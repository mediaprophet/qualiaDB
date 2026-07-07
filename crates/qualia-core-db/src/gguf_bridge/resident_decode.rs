//! Native GPU-resident single-fence token decode.
//!
//! The legacy native decode path round-trips the hidden state through the CPU
//! twice per layer (attention readback + FFN readback), paying ~2 blocking
//! `submit → poll(wait)` fences and two CPU RMSNorm/residual-adds per layer —
//! ~107 fences/token measured on SmolLM2-360M (A2000, Vulkan), ~24% of the
//! token in pure fence latency plus the GPU idling through every CPU
//! turnaround.
//!
//! This module keeps the hidden state resident in VRAM for the WHOLE token:
//! per layer it encodes RMSNorm (elem op) → K/V preprojection + KV-cache write
//! → Q-SDPA → O-projection → residual add (elem) → RMSNorm (elem) → gate/up
//! GEMVs → SiLU·mul (elem) → down GEMV → residual add (elem), then the output
//! RMSNorm and the chunked logits GEMV + top-1 block reduction — all into ONE
//! command encoder, ONE submit, ONE fence, with a ~400-byte candidate readback.
//! It is the native mirror of the proven wasm MC8 fused-encoder design
//! (`mc8_wasm/`), built from the same kernels the legacy path already runs:
//! `coop_gemv`/`main` GEMV, `fused_attention.wgsl`, `wasm_elementwise.wgsl`,
//! `topk_reduction.wgsl`. The GPU RMSNorm reduces in the same sequential order
//! as the CPU `rms_norm_inplace`, so decode output is expected token-identical
//! to the legacy path (asserted by the `a1d` differential test).
//!
//! All bind groups and static uniform slots are created ONCE per model (the
//! weights are Phase-2 resident, so bindings are stable). Per token the driver
//! writes: the embedded token (n_embd floats), the per-layer attention uniform
//! block (token_idx / KV mask fields — one `write_buffer`), and the KV mask
//! words. Everything else is pre-encoded state.
//!
//! Toggle: `QUALIA_LLM_RESIDENT_DECODE` / `set_resident_decode` (default ON).
//! Any ineligibility (unsupported quant, missing resident logits, layer cap,
//! sieve mask) falls back to the legacy per-layer path unchanged.

#![cfg(not(target_arch = "wasm32"))]

use super::*;

/// 256-byte uniform slot stride (WebGPU min uniform offset alignment).
const SLOT: wgpu::BufferAddress = 256;

/// Static GEMM param slots per layer: K, V, O, gate, up, down.
const GEMM_SLOTS_PER_LAYER: u64 = 6;
/// Dynamic attention param slots per layer: K-write, V-write, Q.
const ATTN_SLOTS_PER_LAYER: u64 = 3;

/// Elem param slots (shared across layers): rms(n_embd), add(n_embd), silu(n_ffn).
const ELEM_SLOT_RMS: wgpu::BufferAddress = 0;
const ELEM_SLOT_ADD: wgpu::BufferAddress = SLOT;
const ELEM_SLOT_SILU: wgpu::BufferAddress = SLOT * 2;
const ELEM_SLOTS: u64 = 3;

/// One transformer layer's pre-built bind groups (encode order).
struct LayerBinds {
    rms_attn: wgpu::BindGroup,
    k_gemm: wgpu::BindGroup,
    k_write: wgpu::BindGroup,
    v_gemm: wgpu::BindGroup,
    v_write: wgpu::BindGroup,
    q: wgpu::BindGroup,
    o: wgpu::BindGroup,
    add1: wgpu::BindGroup,
    rms_ffn: wgpu::BindGroup,
    gate: wgpu::BindGroup,
    up: wgpu::BindGroup,
    silu: wgpu::BindGroup,
    down: wgpu::BindGroup,
    add2: wgpu::BindGroup,
}

/// One output-projection vocab chunk (GEMV + top-1 reduction).
struct OutChunk {
    gemm: wgpu::BindGroup,
    topk: wgpu::BindGroup,
    rows: u32,
    cand_count: usize,
}

/// Per-layer prototype attention params; per token only the position/mask
/// fields are patched before the single dynamic-arena upload.
struct LayerProtos {
    k_write: AttentionGpuParams,
    v_write: AttentionGpuParams,
    q: AttentionGpuParams,
}

pub(crate) struct ResidentDecodePlan {
    /// (mmap base, tensor_data_start, n_layer) — invalidates on model swap.
    key: (u64, u64, u32),
    n_embd: usize,
    n_ffn: usize,
    kv_dim: usize,
    n_head: u32,
    n_kv_head: u32,
    layout: KvCacheLayout,
    layer_protos: Vec<LayerProtos>,
    layers: Vec<LayerBinds>,
    rms_out: wgpu::BindGroup,
    out_chunks: Vec<OutChunk>,
    total_cands: usize,
    /// Dynamic (per-token) attention uniform arena: `n_layer × 3` slots.
    attn_dyn_arena: wgpu::Buffer,
    /// Reused CPU staging for the dynamic arena (no per-token heap alloc).
    dyn_scratch: Vec<u8>,
    /// Embedded-token upload target; also the residual stream (layer in/out).
    hidden_a: wgpu::Buffer,
    /// Candidate readback staging: `[vals(total_cands) | idxs(total_cands)]`.
    staging: wgpu::Buffer,
    use_coop: bool,
}

pub(crate) enum ResidentDecodeState {
    Unbuilt,
    /// Build failed for this model — don't retry every token.
    Ineligible(u64),
    Ready(Box<ResidentDecodePlan>),
}

impl QTensorEngine {
    fn plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self
            .gguf_mmap
            .as_deref()
            .map(|m| m.as_ptr() as u64)
            .unwrap_or(0);
        (base, index.tensor_data_start, index.hyperparams.n_layer)
    }

    /// Single-fence resident-token decode. Returns the argmax token, or `None`
    /// on any ineligibility (caller falls back to the legacy per-layer path).
    pub fn dispatch_token_forward_resident(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb: &[f32],
        token_idx: u32,
    ) -> Option<StreamingArgmaxResult> {
        if !crate::llm_bench::resident_decode_enabled()
            || !crate::llm_bench::resident_weights_enabled()
            || crate::llm_bench::cpu_attention_enabled()
            // W5b Phase 4b: dict-coded KV needs the write to run through the attention pass
            // (`write_kv_head` encodes to codes); the resident fast path bypasses it.
            || crate::llm_bench::kv_dict_enabled()
        {
            return None;
        }
        let key = self.plan_key(index);
        // Take the state out so plan borrows don't conflict with `&self` calls.
        let state = std::mem::replace(&mut self.resident_decode, ResidentDecodeState::Unbuilt);
        let mut plan = match state {
            ResidentDecodeState::Ready(p) if p.key == key => p,
            ResidentDecodeState::Ineligible(k) if k == (key.0 ^ key.1) => {
                self.resident_decode = ResidentDecodeState::Ineligible(k);
                return None;
            }
            _ => match self.build_resident_plan(index, key) {
                Some(p) => p,
                None => {
                    wlog("[resident-decode] plan build ineligible — legacy path");
                    self.resident_decode = ResidentDecodeState::Ineligible(key.0 ^ key.1);
                    return None;
                }
            },
        };
        let result = self.run_resident_token(&mut plan, emb, token_idx);
        self.resident_decode = ResidentDecodeState::Ready(plan);
        result
    }

    fn run_resident_token(
        &self,
        plan: &mut ResidentDecodePlan,
        emb: &[f32],
        token_idx: u32,
    ) -> Option<StreamingArgmaxResult> {
        let n_embd = plan.n_embd;
        if emb.len() < n_embd || token_idx >= plan.layout.max_context {
            return None;
        }
        let queue = self.gpu_queue();

        // 1) Per-token uploads: embedded token, KV mask, dynamic attention params.
        queue.write_buffer(&plan.hidden_a, 0, bytemuck::cast_slice(&emb[..n_embd]));

        let (mask_words, mask_active) =
            crate::compute_universe::attention_kv_mask_u32(token_idx, plan.layout.max_context);
        queue.write_buffer(
            self.attention_mask_buf.as_ref()?,
            0,
            bytemuck::cast_slice(&mask_words),
        );

        let ap_size = std::mem::size_of::<AttentionGpuParams>();
        for (l, protos) in plan.layer_protos.iter().enumerate() {
            let base = l as u64 * ATTN_SLOTS_PER_LAYER * SLOT;
            let mut k = protos.k_write;
            k.token_idx = token_idx;
            k.batch_start_token_idx = token_idx;
            let mut v = protos.v_write;
            v.token_idx = token_idx;
            v.batch_start_token_idx = token_idx;
            let mut q = protos.q;
            q.token_idx = token_idx;
            q.batch_start_token_idx = token_idx;
            q.mask_active = mask_active;
            q.mask_word_count = if mask_active != 0 {
                KV_ATTENTION_MASK_WORDS as u32
            } else {
                0
            };
            for (slot, p) in [(0u64, &k), (1, &v), (2, &q)] {
                let off = (base + slot * SLOT) as usize;
                plan.dyn_scratch[off..off + ap_size].copy_from_slice(bytemuck::bytes_of(p));
            }
        }
        queue.write_buffer(&plan.attn_dyn_arena, 0, &plan.dyn_scratch);

        // 2) Encode the whole token: 32 layers + output norm + logits top-1.
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ResidentTokenEncoder"),
                });
        let gemv = if plan.use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let rms = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?;
        let add = self.elem_gpu_pipeline(ELEM_OP_ADD_RESIDUAL)?;
        let silu = &self.elem_silu_mul_pipeline;
        let attn = &self.attention_pipeline;
        let topk = self.output_topk_pipeline.as_ref()?;

        let gemv_wg = |n_out: u32| {
            if plan.use_coop {
                n_out
            } else {
                n_out.div_ceil(64)
            }
        };
        let elem_wg = |n: u32| n.div_ceil(64);
        let (n_embd_u, n_ffn_u) = (n_embd as u32, plan.n_ffn as u32);
        let kv_dim_u = plan.kv_dim as u32;

        {
            for lb in &plan.layers {
                let seq: [(&wgpu::ComputePipeline, &wgpu::BindGroup, u32); 14] = [
                    (rms, &lb.rms_attn, 1),
                    (gemv, &lb.k_gemm, gemv_wg(kv_dim_u)),
                    (attn, &lb.k_write, plan.n_kv_head.max(1)),
                    (gemv, &lb.v_gemm, gemv_wg(kv_dim_u)),
                    (attn, &lb.v_write, plan.n_kv_head.max(1)),
                    (attn, &lb.q, plan.n_head.max(1)),
                    (gemv, &lb.o, gemv_wg(n_embd_u)),
                    (add, &lb.add1, elem_wg(n_embd_u)),
                    (rms, &lb.rms_ffn, 1),
                    (gemv, &lb.gate, gemv_wg(n_ffn_u)),
                    (gemv, &lb.up, gemv_wg(n_ffn_u)),
                    (silu, &lb.silu, elem_wg(n_ffn_u)),
                    (gemv, &lb.down, gemv_wg(n_embd_u)),
                    (add, &lb.add2, elem_wg(n_embd_u)),
                ];
                for (pipe, bg, wg_x) in seq {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: None,
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(pipe);
                    cpass.set_bind_group(0, bg, &[]);
                    cpass.dispatch_workgroups(wg_x, 1, 1);
                }
            }
            // Output RMSNorm.
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("ResidentOutputNorm"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(rms);
                cpass.set_bind_group(0, &plan.rms_out, &[]);
                cpass.dispatch_workgroups(1, 1, 1);
            }
            // Chunked logits GEMV + top-1 reduction + candidate staging copies.
            let cand_val = self.topk_cand_val_buf.as_ref()?;
            let cand_idx = self.topk_cand_idx_buf.as_ref()?;
            let mut cand_offset = 0usize;
            for chunk in &plan.out_chunks {
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("ResidentLogitsGemv"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(gemv);
                    cpass.set_bind_group(0, &chunk.gemm, &[]);
                    cpass.dispatch_workgroups(gemv_wg(chunk.rows), 1, 1);
                }
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("ResidentLogitsTopk"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(topk);
                    cpass.set_bind_group(0, &chunk.topk, &[]);
                    cpass.dispatch_workgroups(chunk.cand_count as u32, 1, 1);
                }
                let cand_bytes = (chunk.cand_count * 4) as wgpu::BufferAddress;
                let val_dst = (cand_offset * 4) as wgpu::BufferAddress;
                let idx_dst = ((plan.total_cands + cand_offset) * 4) as wgpu::BufferAddress;
                encoder.copy_buffer_to_buffer(cand_val, 0, &plan.staging, val_dst, cand_bytes);
                encoder.copy_buffer_to_buffer(cand_idx, 0, &plan.staging, idx_dst, cand_bytes);
                cand_offset += chunk.cand_count;
            }
        }

        // 3) ONE submit, ONE fence, tiny readback.
        queue.submit(Some(encoder.finish()));

        let map_bytes = (plan.total_cands * 8) as wgpu::BufferAddress;
        let slice = plan.staging.slice(..map_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        let mapped_ok = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false)
        } else {
            false
        };
        if !mapped_ok {
            let _ = plan.staging.unmap();
            return None;
        }

        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;
        {
            let data = slice.get_mapped_range();
            let val_bytes = plan.total_cands * 4;
            let vals: &[f32] = bytemuck::cast_slice(&data[..val_bytes]);
            let idxs: &[u32] = bytemuck::cast_slice(&data[val_bytes..val_bytes * 2]);
            let mut offset = 0usize;
            let mut row_start = 0u32;
            for chunk in &plan.out_chunks {
                for i in 0..chunk.cand_count {
                    let pos = offset + i;
                    let v = vals[pos];
                    let token_id = row_start + idxs[pos];
                    if v > f32::NEG_INFINITY
                        && (v > max_logit || (v == max_logit && token_id < best_token_id))
                    {
                        max_logit = v;
                        best_token_id = token_id;
                    }
                }
                offset += chunk.cand_count;
                row_start += chunk.rows;
            }
        }
        plan.staging.unmap();

        if max_logit == f32::NEG_INFINITY {
            None
        } else {
            Some(StreamingArgmaxResult {
                best_token_id,
                max_logit,
            })
        }
    }

    /// Build the per-model plan: force weight residency, create activation
    /// buffers + static uniform slots + all bind groups. Any missing piece →
    /// `None` (the caller records Ineligible and the legacy path runs).
    fn build_resident_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<ResidentDecodePlan>> {
        let mmap_arc = self.gguf_mmap.clone()?;
        let mmap: &[u8] = &mmap_arc;
        let h = index.hyperparams;
        let layout = self.kv_layout?;
        let n_embd = h.n_embd as usize;
        let n_layer = h.n_layer;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        let q_dim = n_head * head_dim;
        let kv_dim = n_kv * head_dim;
        if n_layer == 0
            || n_embd == 0
            || n_embd > MAX_HIDDEN_DIM
            || q_dim == 0
            || kv_dim == 0
            || self.output_topk_pipeline.is_none()
            || self.attention_mask_buf.is_none()
        {
            return None;
        }

        // Logits projection must be resident (per-chunk re-upload cannot share
        // one encoder), and every weight must be GPU-eligible.
        let logits_buf = self.mc8_logits_resident_buf.clone()?;
        let logits_row_bytes = self.mc8_logits_row_bytes as u64;
        let logits_info = *index.logits_projection_info()?;
        let (logits_in, vocab) = Self::matmul_dims(&logits_info);
        if logits_in != n_embd
            || vocab == 0
            || !ggml_gpu_gemm_supported(logits_info.ggml_type)
            || logits_row_bytes == 0
        {
            return None;
        }
        let out_norm_info = *index.output_norm_info()?;

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let device = self.gpu_device().clone();
        let mk_storage = |label: &str, floats: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: ((floats * 4 + 255) & !255).max(4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let n_ffn = {
            let t0 = index.get_layer_tensors(0);
            let gate = t0.ffn_gate.as_ref()?;
            Self::matmul_dims(gate).1
        };
        if n_ffn == 0 || n_ffn > MAX_STACK_GEMM_DIM {
            return None;
        }

        let hidden_a = mk_storage("ResidentHiddenA", n_embd);
        let hidden_b = mk_storage("ResidentHiddenB", n_embd);
        let normed = mk_storage("ResidentNormed", n_embd);
        let attn_out = mk_storage("ResidentAttnOut", q_dim.max(n_embd));
        let delta = mk_storage("ResidentDelta", n_embd);
        let kv_proj = mk_storage("ResidentKvProj", kv_dim);
        let gate_buf = mk_storage("ResidentGate", n_ffn);
        let up_buf = mk_storage("ResidentUp", n_ffn);
        let silu_buf = mk_storage("ResidentSilu", n_ffn);
        let logits_chunk = mk_storage("ResidentLogitsChunk", VOCAB_CHUNK_ROWS);

        // All layers' norm weights resident: slot 2L = attn_norm, 2L+1 = ffn_norm,
        // slot 2·n_layer = output_norm. 256-aligned stride.
        let norm_stride = ((n_embd * 4 + 255) & !255) as wgpu::BufferAddress;
        let norm_res = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentNormWeights"),
            size: norm_stride * (2 * n_layer as u64 + 1),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queue = self.gpu_queue().clone();
        let upload_norm = |slot: u64, info: &GgufTensorInfo| -> bool {
            let mut w = [0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, info, &mut w) < n_embd {
                return false;
            }
            queue.write_buffer(
                &norm_res,
                slot * norm_stride,
                bytemuck::cast_slice(&w[..n_embd]),
            );
            true
        };

        // Static uniform arena: per-layer GEMM slots + output-chunk GEMM slots
        // + shared elem slots + per-chunk topk slots.
        let full_chunks = vocab.div_ceil(VOCAB_CHUNK_ROWS);
        let gemm_slots = n_layer as u64 * GEMM_SLOTS_PER_LAYER + full_chunks as u64;
        let static_arena = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentStaticParams"),
            size: (gemm_slots + ELEM_SLOTS + full_chunks as u64) * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let elem_base = gemm_slots * SLOT;
        let topk_base = (gemm_slots + ELEM_SLOTS) * SLOT;

        let attn_dyn_arena = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentAttnDynParams"),
            size: n_layer as u64 * ATTN_SLOTS_PER_LAYER * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Candidate staging (own buffer — never contends with the legacy path's).
        let block_size = crate::topk::TOPK_BLOCK_SIZE;
        let total_cands = vocab.div_ceil(block_size);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentTopkStaging"),
            size: ((total_cands * 8).max(8)) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Shared elem params (written once).
        let elem_p = |n: usize, op: u32| ElemGpuParams {
            n: n as u32,
            batch: 1,
            op,
            eps: RMS_NORM_EPS,
            a_row_stride: 0,
            b_row_stride: 0,
            out_row_stride: 0,
            a_slot: 0,
            b_slot: 0,
            out_slot: 0,
            _pad: 0,
        };
        queue.write_buffer(
            &static_arena,
            elem_base + ELEM_SLOT_RMS,
            bytemuck::bytes_of(&elem_p(n_embd, ELEM_OP_RMS_NORM)),
        );
        queue.write_buffer(
            &static_arena,
            elem_base + ELEM_SLOT_ADD,
            bytemuck::bytes_of(&elem_p(n_embd, ELEM_OP_ADD_RESIDUAL)),
        );
        queue.write_buffer(
            &static_arena,
            elem_base + ELEM_SLOT_SILU,
            bytemuck::bytes_of(&elem_p(n_ffn, ELEM_OP_SILU_MUL)),
        );

        // Bind-group layouts + helpers.
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let attn_layout = self.attention_bind_layout.clone();
        let rms_pipe = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?.clone();
        let add_pipe = self.elem_gpu_pipeline(ELEM_OP_ADD_RESIDUAL)?.clone();
        let rms_layout = rms_pipe.get_bind_group_layout(0);
        let add_layout = add_pipe.get_bind_group_layout(0);
        let silu_layout = self.elem_silu_mul_bind_layout.clone();
        let topk_layout = self.output_topk_bind_layout.clone()?;
        let mask_buf = self.attention_mask_buf.as_ref()?.clone();
        let kv_buf = self.kv_cache_gpu.as_ref()?.clone();
        let cand_val = self.topk_cand_val_buf.as_ref()?.clone();
        let cand_idx = self.topk_cand_idx_buf.as_ref()?.clone();

        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ap_sz = std::num::NonZeroU64::new(std::mem::size_of::<AttentionGpuParams>() as u64);
        let ep_sz = std::num::NonZeroU64::new(std::mem::size_of::<ElemGpuParams>() as u64);
        let tp_sz = std::num::NonZeroU64::new(16);
        fn ubind(
            buf: &wgpu::Buffer,
            off: wgpu::BufferAddress,
            sz: Option<std::num::NonZeroU64>,
        ) -> wgpu::BindingResource<'_> {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: buf,
                offset: off,
                size: sz,
            })
        }
        let norm_bind = |slot: u64| {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &norm_res,
                offset: slot * norm_stride,
                size: std::num::NonZeroU64::new((n_embd * 4) as u64),
            })
        };
        let mk_elem_bg = |label: &str,
                          layout: &wgpu::BindGroupLayout,
                          a: wgpu::BindingResource,
                          b: wgpu::BindingResource,
                          out: &wgpu::Buffer,
                          p_off: wgpu::BufferAddress| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: a,
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: b,
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: out.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: ubind(&static_arena, elem_base + p_off, ep_sz),
                    },
                ],
            })
        };
        let mk_gemm_bg = |label: &str,
                          input: &wgpu::Buffer,
                          weight: wgpu::BindingResource,
                          p_slot: u64,
                          out: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &gemm_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weight,
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ubind(&static_arena, p_slot * SLOT, gp_sz),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: out.as_entire_binding(),
                    },
                ],
            })
        };
        let layer_f32s = layout.layer_stride as u64;
        let mk_attn_bg = |label: &str,
                          input: &wgpu::Buffer,
                          weight: &wgpu::Buffer,
                          dyn_slot: u64,
                          layer: u32,
                          out: &wgpu::Buffer| {
            let kv_binding = wgpu::BufferBinding {
                buffer: &kv_buf,
                offset: layer as u64 * layer_f32s * 4,
                size: std::num::NonZeroU64::new((layer_f32s * 4).max(4)),
            };
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &attn_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weight.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ubind(&attn_dyn_arena, dyn_slot * SLOT, ap_sz),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Buffer(kv_binding),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: out.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: mask_buf.as_entire_binding(),
                    },
                ],
            })
        };

        let gemm_params =
            |info: &GgufTensorInfo, n_in: usize, n_out: usize, raw_len: usize| GemmGpuParams {
                n_in: n_in as u32,
                n_out: n_out as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len: raw_len as u32,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };

        let mut layers = Vec::with_capacity(n_layer as usize);
        let mut layer_protos = Vec::with_capacity(n_layer as usize);
        for l in 0..n_layer {
            let t = index.get_layer_tensors(l);
            let (q_info, k_info, v_info) = (t.attn_q?, t.attn_k?, t.attn_v?);
            let o_info = t.attn_output?;
            let (gate_info, up_info, down_info) = (t.ffn_gate?, t.ffn_up?, t.ffn_down?);
            let attn_norm = t.attn_norm?;
            let ffn_norm = t.ffn_norm?;
            for i in [&q_info, &k_info, &v_info] {
                if !ggml_gpu_attention_shader_supported(i.ggml_type)
                    || !ggml_gpu_gemm_supported(i.ggml_type)
                {
                    return None;
                }
            }
            for i in [&o_info, &gate_info, &up_info, &down_info] {
                if !ggml_gpu_gemm_supported(i.ggml_type) {
                    return None;
                }
            }
            // Shape contract (mirrors the legacy per-layer checks).
            let (k_in, k_out) = Self::matmul_dims(&k_info);
            let (v_in, v_out) = Self::matmul_dims(&v_info);
            let (o_in, o_out) = Self::matmul_dims(&o_info);
            let (g_in, g_out) = Self::matmul_dims(&gate_info);
            let (u_in, u_out) = Self::matmul_dims(&up_info);
            let (d_in, d_out) = Self::matmul_dims(&down_info);
            if k_in != n_embd
                || v_in != n_embd
                || k_out != kv_dim
                || v_out != kv_dim
                || o_in != q_dim
                || o_out != n_embd
                || g_in != n_embd
                || u_in != n_embd
                || g_out != n_ffn
                || u_out != n_ffn
                || d_in != n_ffn
                || d_out != n_embd
            {
                return None;
            }
            let fetch = |i: &GgufTensorInfo| {
                crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, i).ok()
            };
            let (q_raw, k_raw, v_raw) = (fetch(&q_info)?, fetch(&k_info)?, fetch(&v_info)?);
            let (o_raw, g_raw, u_raw, d_raw) = (
                fetch(&o_info)?,
                fetch(&gate_info)?,
                fetch(&up_info)?,
                fetch(&down_info)?,
            );
            let res = |raw: &[u8]| self.resident_weight_buffer(raw.as_ptr() as u64, raw);
            let (q_w, k_w, v_w) = (res(q_raw)?, res(k_raw)?, res(v_raw)?);
            let (o_w, g_w, u_w, d_w) = (res(o_raw)?, res(g_raw)?, res(u_raw)?, res(d_raw)?);

            if !upload_norm(2 * l as u64, &attn_norm) || !upload_norm(2 * l as u64 + 1, &ffn_norm) {
                return None;
            }

            // Static GEMM param slots for this layer: K,V,O,gate,up,down.
            let gbase = l as u64 * GEMM_SLOTS_PER_LAYER;
            for (i, (info, n_in, n_out, raw_len)) in [
                (&k_info, n_embd, kv_dim, k_raw.len()),
                (&v_info, n_embd, kv_dim, v_raw.len()),
                (&o_info, q_dim, n_embd, o_raw.len()),
                (&gate_info, n_embd, n_ffn, g_raw.len()),
                (&up_info, n_embd, n_ffn, u_raw.len()),
                (&down_info, n_ffn, n_embd, d_raw.len()),
            ]
            .into_iter()
            .enumerate()
            {
                queue.write_buffer(
                    &static_arena,
                    (gbase + i as u64) * SLOT,
                    bytemuck::bytes_of(&gemm_params(info, n_in, n_out, raw_len)),
                );
            }

            // Prototype attention params (position/mask patched per token).
            let mut k_p = Self::attention_gpu_params(
                &h,
                &layout,
                l,
                0,
                &k_info,
                k_raw.len(),
                1,
                1,
                0,
                0,
                0,
                0,
            );
            k_p.proj_row_stride = kv_dim as u32;
            let mut v_p = Self::attention_gpu_params(
                &h,
                &layout,
                l,
                0,
                &v_info,
                v_raw.len(),
                2,
                1,
                0,
                0,
                0,
                0,
            );
            v_p.proj_row_stride = kv_dim as u32;
            let q_p = Self::attention_gpu_params(
                &h,
                &layout,
                l,
                0,
                &q_info,
                q_raw.len(),
                0,
                1,
                0,
                0,
                0,
                0,
            );
            layer_protos.push(LayerProtos {
                k_write: k_p,
                v_write: v_p,
                q: q_p,
            });

            let dyn_base = l as u64 * ATTN_SLOTS_PER_LAYER;
            layers.push(LayerBinds {
                rms_attn: mk_elem_bg(
                    "ResRmsAttn",
                    &rms_layout,
                    hidden_a.as_entire_binding(),
                    norm_bind(2 * l as u64),
                    &normed,
                    ELEM_SLOT_RMS,
                ),
                k_gemm: mk_gemm_bg(
                    "ResKGemm",
                    &normed,
                    k_w.as_entire_binding(),
                    gbase,
                    &kv_proj,
                ),
                k_write: mk_attn_bg("ResKWrite", &kv_proj, &k_w, dyn_base, l, &attn_out),
                v_gemm: mk_gemm_bg(
                    "ResVGemm",
                    &normed,
                    v_w.as_entire_binding(),
                    gbase + 1,
                    &kv_proj,
                ),
                v_write: mk_attn_bg("ResVWrite", &kv_proj, &v_w, dyn_base + 1, l, &attn_out),
                q: mk_attn_bg("ResQ", &normed, &q_w, dyn_base + 2, l, &attn_out),
                o: mk_gemm_bg(
                    "ResO",
                    &attn_out,
                    o_w.as_entire_binding(),
                    gbase + 2,
                    &delta,
                ),
                add1: mk_elem_bg(
                    "ResAdd1",
                    &add_layout,
                    hidden_a.as_entire_binding(),
                    delta.as_entire_binding(),
                    &hidden_b,
                    ELEM_SLOT_ADD,
                ),
                rms_ffn: mk_elem_bg(
                    "ResRmsFfn",
                    &rms_layout,
                    hidden_b.as_entire_binding(),
                    norm_bind(2 * l as u64 + 1),
                    &normed,
                    ELEM_SLOT_RMS,
                ),
                gate: mk_gemm_bg(
                    "ResGate",
                    &normed,
                    g_w.as_entire_binding(),
                    gbase + 3,
                    &gate_buf,
                ),
                up: mk_gemm_bg(
                    "ResUp",
                    &normed,
                    u_w.as_entire_binding(),
                    gbase + 4,
                    &up_buf,
                ),
                silu: mk_elem_bg(
                    "ResSilu",
                    &silu_layout,
                    gate_buf.as_entire_binding(),
                    up_buf.as_entire_binding(),
                    &silu_buf,
                    ELEM_SLOT_SILU,
                ),
                down: mk_gemm_bg(
                    "ResDown",
                    &silu_buf,
                    d_w.as_entire_binding(),
                    gbase + 5,
                    &delta,
                ),
                add2: mk_elem_bg(
                    "ResAdd2",
                    &add_layout,
                    hidden_b.as_entire_binding(),
                    delta.as_entire_binding(),
                    &hidden_a,
                    ELEM_SLOT_ADD,
                ),
            });
        }

        // Output norm weights + bind group (reads the final hidden_a).
        if !upload_norm(2 * n_layer as u64, &out_norm_info) {
            return None;
        }
        let rms_out = mk_elem_bg(
            "ResRmsOut",
            &rms_layout,
            hidden_a.as_entire_binding(),
            norm_bind(2 * n_layer as u64),
            &normed,
            ELEM_SLOT_RMS,
        );

        // Output chunks: logits GEMV (resident weight sub-range) + top-1 reduce.
        let mut out_chunks = Vec::with_capacity(full_chunks);
        for c in 0..full_chunks {
            let row_start = c * VOCAB_CHUNK_ROWS;
            let rows = VOCAB_CHUNK_ROWS.min(vocab - row_start);
            if rows > self.gemm_max_out_dim as usize {
                return None;
            }
            let byte_len = rows as u64 * logits_row_bytes;
            let gemm_slot = n_layer as u64 * GEMM_SLOTS_PER_LAYER + c as u64;
            queue.write_buffer(
                &static_arena,
                gemm_slot * SLOT,
                bytemuck::bytes_of(&gemm_params(&logits_info, n_embd, rows, byte_len as usize)),
            );
            let tparams = crate::topk::topk_params_bytes(rows as u32, 1, block_size as u32);
            queue.write_buffer(&static_arena, topk_base + c as u64 * SLOT, &tparams);
            let weight = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &logits_buf,
                offset: row_start as u64 * logits_row_bytes,
                size: std::num::NonZeroU64::new(byte_len),
            });
            let gemm = mk_gemm_bg("ResLogits", &normed, weight, gemm_slot, &logits_chunk);
            let topk = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ResTopk"),
                layout: &topk_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: logits_chunk.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ubind(&static_arena, topk_base + c as u64 * SLOT, tp_sz),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cand_val.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: cand_idx.as_entire_binding(),
                    },
                ],
            });
            out_chunks.push(OutChunk {
                gemm,
                topk,
                rows: rows as u32,
                cand_count: rows.div_ceil(block_size),
            });
        }

        let dyn_bytes = (n_layer as u64 * ATTN_SLOTS_PER_LAYER * SLOT) as usize;
        log::info!(
            "LLM_DECODE|resident-plan|built: {} layers, {} passes/token, 1 fence/token",
            n_layer,
            n_layer as usize * 14 + 1 + out_chunks.len() * 2,
        );
        Some(Box::new(ResidentDecodePlan {
            key,
            n_embd,
            n_ffn,
            kv_dim,
            n_head: n_head as u32,
            n_kv_head: n_kv as u32,
            layout,
            layer_protos,
            layers,
            rms_out,
            out_chunks,
            total_cands,
            attn_dyn_arena,
            dyn_scratch: vec![0u8; dyn_bytes],
            hidden_a,
            staging,
            use_coop,
        }))
    }
}
