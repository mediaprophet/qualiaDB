//! W6a — batched speculative-verify forward (per-position argmax).
//!
//! Given `B` tokens at consecutive absolute positions `[start_pos, start_pos+B)`, run ONE batched
//! resident forward (the W3 prefill arena's per-layer batched encode) and, instead of discarding the
//! hidden state, apply the output tail (output RMSNorm → chunked logits GEMV) and return the **greedy
//! argmax at every position**. Prompt-lookup speculative decode uses this to verify a draft in a
//! single forward: `verify_draft_batch([cur, d0, … d_{γ-1}], p)` returns `[a0 … a_γ]`, where `a_i` is
//! the model's greedy next-token given the true prefix through position `p+i`; the caller accepts the
//! longest prefix with `a_i == d_i` (bit-identical to greedy decode — see
//! `docs/plans/inference-W6a-speculative-verify.md`).
//!
//! The forward is byte-identical to the legacy per-token forward at each position (same kernels, same
//! order; the batched RMSNorm reduces in the same sequential order as the CPU path — the property that
//! makes W1/W3 token-identical). int8 KV rides the same `fused_attention` write/read branch for free.
//!
//! Unlike `resident_decode.rs` (one position, GPU block-top-1), verify handles ≤`VERIFY_BATCH_MAX`
//! positions and reads back the whole `B × vocab` logit block once for a CPU argmax per position — the
//! readback is negligible (verify runs once per accepted group, not per token) and avoids per-position
//! GPU top-k bookkeeping. As a side effect the forward populates the KV cache for the accepted prefix,
//! so no separate KV rollback is needed: rejected positions are overwritten by the next real decode.
//!
//! This module duplicates ~80% of `prefill_arena.rs`/`resident_decode.rs`; a shared `batched_forward`
//! helper is the right library-ization (PROJECT RULE §11) but is deferred to the post-feature pass.

#![cfg(not(target_arch = "wasm32"))]

use super::*;

/// Max verify-batch width (MAX_DRAFT + 1 leading token, with headroom).
pub(crate) const VERIFY_BATCH_MAX: usize = 16;

/// 256-byte uniform slot stride.
const SLOT: wgpu::BufferAddress = 256;
/// Uniform slots per layer: 6 GEMM + 3 attention + 5 elementwise.
const SLOTS_PER_LAYER: u64 = 14;
const S_GEMM_K: u64 = 0;
const S_GEMM_V: u64 = 1;
const S_GEMM_O: u64 = 2;
const S_GEMM_GATE: u64 = 3;
const S_GEMM_UP: u64 = 4;
const S_GEMM_DOWN: u64 = 5;
const S_ATTN_KW: u64 = 6;
const S_ATTN_VW: u64 = 7;
const S_ATTN_Q: u64 = 8;
const S_ELEM_RMS_ATTN: u64 = 9;
const S_ELEM_ADD1: u64 = 10;
const S_ELEM_RMS_FFN: u64 = 11;
const S_ELEM_SILU: u64 = 12;
const S_ELEM_ADD2: u64 = 13;

struct VerifyLayerBinds {
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

struct VerifyLayerProtos {
    gemm: [GemmGpuParams; 6],
    attn: [AttentionGpuParams; 3],
    elem: [ElemGpuParams; 5],
}

/// One vocab chunk's logits GEMV (writes `B` rows into `logits_out` at the chunk offset).
struct VerifyChunk {
    gemm: wgpu::BindGroup,
    rows: u32,
    /// GEMM param slot in the arena.
    slot: u64,
    /// Constant GEMM params (only `n_batch` varies per call).
    params: GemmGpuParams,
}

pub(crate) struct VerifyArenaPlan {
    key: (u64, u64, u32),
    n_embd: usize,
    n_ffn: usize,
    kv_dim: usize,
    n_head: u32,
    n_kv_head: u32,
    vocab: usize,
    layout: KvCacheLayout,
    protos: Vec<VerifyLayerProtos>,
    layers: Vec<VerifyLayerBinds>,
    /// Output RMSNorm (hidden_a → normed) bind group; elem param at `tail_rms_slot`.
    rms_out: wgpu::BindGroup,
    tail_rms_slot: u64,
    chunks: Vec<VerifyChunk>,
    param_arena: wgpu::Buffer,
    scratch: Vec<u8>,
    /// Embedded-token upload target + residual stream.
    hidden_a: wgpu::Buffer,
    /// `B × vocab` logits (STORAGE|COPY_SRC).
    logits_out: wgpu::Buffer,
    /// `B × vocab` readback staging (MAP_READ|COPY_DST).
    logits_staging: wgpu::Buffer,
    use_coop: bool,
}

pub(crate) enum VerifyArenaState {
    Unbuilt,
    Ineligible(u64),
    Ready(Box<VerifyArenaPlan>),
}

impl QTensorEngine {
    fn verify_plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self
            .gguf_mmap
            .as_deref()
            .map(|m| m.as_ptr() as u64)
            .unwrap_or(0);
        (base, index.tensor_data_start, index.hyperparams.n_layer)
    }

    /// Batched speculative verify. `tokens` are the ≤`VERIFY_BATCH_MAX` inputs at absolute positions
    /// `[start_pos, start_pos + tokens.len())`; on success `out_argmax` holds the greedy argmax at each
    /// position (same length as `tokens`). Returns `None` on any ineligibility (caller falls back to
    /// per-token decode). Populates the KV cache for those positions as a side effect.
    pub(crate) fn verify_draft_batch(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        tokens: &[u32],
        start_pos: u32,
        out_argmax: &mut Vec<u32>,
        out_logit: &mut Vec<f32>,
    ) -> Option<()> {
        if !crate::llm_bench::resident_weights_enabled()
            || !crate::llm_bench::coop_gemv_enabled()
            || crate::llm_bench::cpu_attention_enabled()
            || tokens.is_empty()
            || tokens.len() > VERIFY_BATCH_MAX
        {
            return None;
        }
        if crate::compute_universe::attention_route_mask().active_bits > 0 {
            return None;
        }
        let key = self.verify_plan_key(index);
        let state = std::mem::replace(&mut self.verify_arena, VerifyArenaState::Unbuilt);
        let mut plan = match state {
            VerifyArenaState::Ready(p) if p.key == key => p,
            VerifyArenaState::Ineligible(k) if k == (key.0 ^ key.1) => {
                self.verify_arena = VerifyArenaState::Ineligible(k);
                return None;
            }
            _ => match self.build_verify_plan(index, key) {
                Some(p) => p,
                None => {
                    self.verify_arena = VerifyArenaState::Ineligible(key.0 ^ key.1);
                    return None;
                }
            },
        };
        let result =
            self.run_verify_batch(&mut plan, index, tokens, start_pos, out_argmax, out_logit);
        self.verify_arena = VerifyArenaState::Ready(plan);
        result
    }

    fn run_verify_batch(
        &self,
        plan: &mut VerifyArenaPlan,
        index: &crate::gguf_sharder::GgufTensorIndex,
        tokens: &[u32],
        start_pos: u32,
        out_argmax: &mut Vec<u32>,
        out_logit: &mut Vec<f32>,
    ) -> Option<()> {
        let b = tokens.len();
        let n_embd = plan.n_embd;
        let vocab = plan.vocab;
        if start_pos as usize + b > plan.layout.max_context as usize {
            return None;
        }
        let mmap = self.gguf_mmap.clone()?;
        let queue = self.gpu_queue();

        // 1) Dequantize the B token embeddings → hidden_a (packed B × n_embd).
        let mut emb = vec![0f32; b * n_embd];
        for (t, &tok) in tokens.iter().enumerate() {
            let n = index.dequantize_token_embedding_into(
                &mmap,
                tok,
                &mut emb[t * n_embd..(t + 1) * n_embd],
            );
            if n == 0 {
                return None;
            }
        }
        queue.write_buffer(&plan.hidden_a, 0, bytemuck::cast_slice(&emb));

        // 2) Rewrite the param arena with this batch's fields.
        let gp = std::mem::size_of::<GemmGpuParams>();
        let ap = std::mem::size_of::<AttentionGpuParams>();
        let ep = std::mem::size_of::<ElemGpuParams>();
        let bu = b as u32;
        {
            let scratch = &mut plan.scratch;
            for (l, proto) in plan.protos.iter().enumerate() {
                let base = l as u64 * SLOTS_PER_LAYER * SLOT;
                for (i, slot) in [
                    S_GEMM_K,
                    S_GEMM_V,
                    S_GEMM_O,
                    S_GEMM_GATE,
                    S_GEMM_UP,
                    S_GEMM_DOWN,
                ]
                .into_iter()
                .enumerate()
                {
                    let mut g = proto.gemm[i];
                    g.n_batch = bu;
                    let off = (base + slot * SLOT) as usize;
                    scratch[off..off + gp].copy_from_slice(bytemuck::bytes_of(&g));
                }
                for (i, slot) in [S_ATTN_KW, S_ATTN_VW, S_ATTN_Q].into_iter().enumerate() {
                    let mut a = proto.attn[i];
                    a.num_tokens_in_batch = bu;
                    a.batch_start_token_idx = start_pos;
                    a.token_idx = start_pos;
                    let off = (base + slot * SLOT) as usize;
                    scratch[off..off + ap].copy_from_slice(bytemuck::bytes_of(&a));
                }
                for (i, slot) in [
                    S_ELEM_RMS_ATTN,
                    S_ELEM_ADD1,
                    S_ELEM_RMS_FFN,
                    S_ELEM_SILU,
                    S_ELEM_ADD2,
                ]
                .into_iter()
                .enumerate()
                {
                    let mut e = proto.elem[i];
                    e.batch = bu;
                    let off = (base + slot * SLOT) as usize;
                    scratch[off..off + ep].copy_from_slice(bytemuck::bytes_of(&e));
                }
            }
            // Tail: output RMSNorm elem (batch = B).
            {
                let mut e = ElemGpuParams {
                    n: n_embd as u32,
                    batch: bu,
                    op: ELEM_OP_RMS_NORM,
                    eps: RMS_NORM_EPS,
                    a_row_stride: 0,
                    b_row_stride: 0,
                    out_row_stride: 0,
                    a_slot: 0,
                    b_slot: 0,
                    out_slot: 0,
                    _pad: 0,
                };
                e.batch = bu;
                let off = (plan.tail_rms_slot * SLOT) as usize;
                scratch[off..off + ep].copy_from_slice(bytemuck::bytes_of(&e));
            }
            // Tail: per-chunk logits GEMV params (full struct; only `n_batch` varies). Must write the
            // WHOLE struct into scratch — the full-arena upload below would otherwise zero these slots.
            for chunk in &plan.chunks {
                let mut g = chunk.params;
                g.n_batch = bu;
                let off = (chunk.slot * SLOT) as usize;
                scratch[off..off + gp].copy_from_slice(bytemuck::bytes_of(&g));
            }
        }
        queue.write_buffer(&plan.param_arena, 0, &plan.scratch);

        // 3) Encode: batched forward (all layers) → output RMSNorm → per-chunk logits GEMV.
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("VerifyBatchEncoder"),
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

        let gemv_wg = |n_out: u32| {
            if plan.use_coop {
                n_out
            } else {
                n_out.div_ceil(64)
            }
        };
        let elem_wg = |n: u32| n.div_ceil(64);
        let (n_embd_u, n_ffn_u, kv_dim_u) = (n_embd as u32, plan.n_ffn as u32, plan.kv_dim as u32);
        let kv_pairs = plan.n_kv_head.max(1) * bu;

        for lb in &plan.layers {
            let seq: [(&wgpu::ComputePipeline, &wgpu::BindGroup, u32, u32); 14] = [
                (rms, &lb.rms_attn, 1, bu),
                (gemv, &lb.k_gemm, gemv_wg(kv_dim_u), bu),
                (attn, &lb.k_write, kv_pairs, 1),
                (gemv, &lb.v_gemm, gemv_wg(kv_dim_u), bu),
                (attn, &lb.v_write, kv_pairs, 1),
                (attn, &lb.q, plan.n_head.max(1), bu),
                (gemv, &lb.o, gemv_wg(n_embd_u), bu),
                (add, &lb.add1, elem_wg(n_embd_u), bu),
                (rms, &lb.rms_ffn, 1, bu),
                (gemv, &lb.gate, gemv_wg(n_ffn_u), bu),
                (gemv, &lb.up, gemv_wg(n_ffn_u), bu),
                (silu, &lb.silu, elem_wg(n_ffn_u), bu),
                (gemv, &lb.down, gemv_wg(n_embd_u), bu),
                (add, &lb.add2, elem_wg(n_embd_u), bu),
            ];
            for (pipe, bg, wg_x, wg_y) in seq {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                cpass.set_pipeline(pipe);
                cpass.set_bind_group(0, bg, &[]);
                cpass.dispatch_workgroups(wg_x, wg_y, 1);
            }
        }
        // Output RMSNorm (hidden_a → normed).
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VerifyOutputNorm"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(rms);
            cpass.set_bind_group(0, &plan.rms_out, &[]);
            cpass.dispatch_workgroups(1, bu, 1);
        }
        // Per-chunk logits GEMV (all B rows) into logits_out.
        for chunk in &plan.chunks {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VerifyLogitsGemv"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(gemv);
            cpass.set_bind_group(0, &chunk.gemm, &[]);
            cpass.dispatch_workgroups(gemv_wg(chunk.rows), bu, 1);
        }
        // Copy the B × vocab logit block to the readback staging.
        let logit_bytes = (b * vocab * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(&plan.logits_out, 0, &plan.logits_staging, 0, logit_bytes);

        // 4) ONE submit, ONE fence, then CPU argmax per position.
        queue.submit(Some(encoder.finish()));
        let slice = plan.logits_staging.slice(..logit_bytes);
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
            let _ = plan.logits_staging.unmap();
            return None;
        }
        out_argmax.clear();
        out_logit.clear();
        {
            let data = slice.get_mapped_range();
            let all: &[f32] = bytemuck::cast_slice(&data[..(b * vocab * 4)]);
            for m in 0..b {
                let row = &all[m * vocab..(m + 1) * vocab];
                let mut best_id = 0u32;
                let mut best_v = f32::NEG_INFINITY;
                for (i, &v) in row.iter().enumerate() {
                    // Ties → lower index (matches CPU argmax / topk_reduction semantics).
                    if v > best_v {
                        best_v = v;
                        best_id = i as u32;
                    }
                }
                out_argmax.push(best_id);
                out_logit.push(best_v);
            }
        }
        plan.logits_staging.unmap();
        Some(())
    }

    fn build_verify_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<VerifyArenaPlan>> {
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
        let bmax = VERIFY_BATCH_MAX;
        if n_layer == 0
            || n_embd == 0
            || n_embd > MAX_HIDDEN_DIM
            || q_dim == 0
            || kv_dim == 0
            || self.attention_mask_buf.is_none()
            || self.kv_cache_gpu.is_none()
        {
            return None;
        }
        let use_coop = crate::llm_bench::coop_gemv_enabled();
        if !use_coop {
            return None;
        }

        // Logits projection must be resident (chunked GEMV shares one encoder).
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

        let device = self.gpu_device().clone();
        let mk_storage = |label: &str, floats: usize, extra: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: ((floats * 4 + 255) & !255).max(4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | extra,
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

        let empty = wgpu::BufferUsages::empty();
        let hidden_a = mk_storage("VerifyHiddenA", bmax * n_embd, empty);
        let hidden_b = mk_storage("VerifyHiddenB", bmax * n_embd, empty);
        let normed = mk_storage("VerifyNormed", bmax * n_embd, empty);
        let attn_out = mk_storage("VerifyAttnOut", bmax * q_dim.max(n_embd), empty);
        let delta = mk_storage("VerifyDelta", bmax * n_embd, empty);
        let kv_proj = mk_storage("VerifyKvProj", bmax * kv_dim, empty);
        let gate_buf = mk_storage("VerifyGate", bmax * n_ffn, empty);
        let up_buf = mk_storage("VerifyUp", bmax * n_ffn, empty);
        let silu_buf = mk_storage("VerifySilu", bmax * n_ffn, empty);
        let logits_out = mk_storage(
            "VerifyLogitsOut",
            bmax * vocab,
            wgpu::BufferUsages::COPY_SRC,
        );
        let logits_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VerifyLogitsStaging"),
            size: ((bmax * vocab * 4 + 255) & !255).max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Resident norm weights: 2L = attn_norm, 2L+1 = ffn_norm, 2·n_layer = output_norm.
        let norm_stride = ((n_embd * 4 + 255) & !255) as wgpu::BufferAddress;
        let norm_res = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VerifyNormWeights"),
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

        let full_chunks = vocab.div_ceil(VOCAB_CHUNK_ROWS);
        let tail_rms_slot = n_layer as u64 * SLOTS_PER_LAYER;
        let tail_gemm_base = tail_rms_slot + 1;
        let total_slots = tail_gemm_base + full_chunks as u64;
        let param_arena = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VerifyParamArena"),
            size: total_slots * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let attn_layout = self.attention_bind_layout.clone();
        let rms_pipe = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?.clone();
        let add_pipe = self.elem_gpu_pipeline(ELEM_OP_ADD_RESIDUAL)?.clone();
        let rms_layout = rms_pipe.get_bind_group_layout(0);
        let add_layout = add_pipe.get_bind_group_layout(0);
        let silu_layout = self.elem_silu_mul_bind_layout.clone();
        let mask_buf = self.attention_mask_buf.as_ref()?.clone();
        let kv_buf = self.kv_cache_gpu.as_ref()?.clone();

        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ap_sz = std::num::NonZeroU64::new(std::mem::size_of::<AttentionGpuParams>() as u64);
        let ep_sz = std::num::NonZeroU64::new(std::mem::size_of::<ElemGpuParams>() as u64);
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
        let slot_off = |l: u64, s: u64| (l * SLOTS_PER_LAYER + s) * SLOT;
        let mk_elem_bg = |layout: &wgpu::BindGroupLayout,
                          a: wgpu::BindingResource,
                          bb: wgpu::BindingResource,
                          out: &wgpu::Buffer,
                          p_off: wgpu::BufferAddress| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VerifyElemBG"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: a,
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: bb,
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: out.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: ubind(&param_arena, p_off, ep_sz),
                    },
                ],
            })
        };
        let mk_gemm_bg = |input: &wgpu::Buffer,
                          weight: wgpu::BindingResource,
                          p_off: wgpu::BufferAddress,
                          out_res: wgpu::BindingResource| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VerifyGemmBG"),
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
                        resource: ubind(&param_arena, p_off, gp_sz),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: out_res,
                    },
                ],
            })
        };
        let layer_f32s = layout.layer_stride as u64;
        let mk_attn_bg = |input: &wgpu::Buffer,
                          weight: &wgpu::Buffer,
                          p_off: wgpu::BufferAddress,
                          layer: u32,
                          out: &wgpu::Buffer| {
            let kv_binding = wgpu::BufferBinding {
                buffer: &kv_buf,
                offset: layer as u64 * layer_f32s * 4,
                size: std::num::NonZeroU64::new((layer_f32s * 4).max(4)),
            };
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VerifyAttnBG"),
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
                        resource: ubind(&param_arena, p_off, ap_sz),
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

        let gemm_proto =
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
        let elem_proto = |n: usize, op: u32| ElemGpuParams {
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

        let mut layers = Vec::with_capacity(n_layer as usize);
        let mut protos = Vec::with_capacity(n_layer as usize);
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
                q_dim as u32,
            );

            protos.push(VerifyLayerProtos {
                gemm: [
                    gemm_proto(&k_info, n_embd, kv_dim, k_raw.len()),
                    gemm_proto(&v_info, n_embd, kv_dim, v_raw.len()),
                    gemm_proto(&o_info, q_dim, n_embd, o_raw.len()),
                    gemm_proto(&gate_info, n_embd, n_ffn, g_raw.len()),
                    gemm_proto(&up_info, n_embd, n_ffn, u_raw.len()),
                    gemm_proto(&down_info, n_ffn, n_embd, d_raw.len()),
                ],
                attn: [k_p, v_p, q_p],
                elem: [
                    elem_proto(n_embd, ELEM_OP_RMS_NORM),
                    elem_proto(n_embd, ELEM_OP_ADD_RESIDUAL),
                    elem_proto(n_embd, ELEM_OP_RMS_NORM),
                    elem_proto(n_ffn, ELEM_OP_SILU_MUL),
                    elem_proto(n_embd, ELEM_OP_ADD_RESIDUAL),
                ],
            });

            let lu = l as u64;
            layers.push(VerifyLayerBinds {
                rms_attn: mk_elem_bg(
                    &rms_layout,
                    hidden_a.as_entire_binding(),
                    norm_bind(2 * lu),
                    &normed,
                    slot_off(lu, S_ELEM_RMS_ATTN),
                ),
                k_gemm: mk_gemm_bg(
                    &normed,
                    k_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_K),
                    kv_proj.as_entire_binding(),
                ),
                k_write: mk_attn_bg(&kv_proj, &k_w, slot_off(lu, S_ATTN_KW), l, &attn_out),
                v_gemm: mk_gemm_bg(
                    &normed,
                    v_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_V),
                    kv_proj.as_entire_binding(),
                ),
                v_write: mk_attn_bg(&kv_proj, &v_w, slot_off(lu, S_ATTN_VW), l, &attn_out),
                q: mk_attn_bg(&normed, &q_w, slot_off(lu, S_ATTN_Q), l, &attn_out),
                o: mk_gemm_bg(
                    &attn_out,
                    o_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_O),
                    delta.as_entire_binding(),
                ),
                add1: mk_elem_bg(
                    &add_layout,
                    hidden_a.as_entire_binding(),
                    delta.as_entire_binding(),
                    &hidden_b,
                    slot_off(lu, S_ELEM_ADD1),
                ),
                rms_ffn: mk_elem_bg(
                    &rms_layout,
                    hidden_b.as_entire_binding(),
                    norm_bind(2 * lu + 1),
                    &normed,
                    slot_off(lu, S_ELEM_RMS_FFN),
                ),
                gate: mk_gemm_bg(
                    &normed,
                    g_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_GATE),
                    gate_buf.as_entire_binding(),
                ),
                up: mk_gemm_bg(
                    &normed,
                    u_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_UP),
                    up_buf.as_entire_binding(),
                ),
                silu: mk_elem_bg(
                    &silu_layout,
                    gate_buf.as_entire_binding(),
                    up_buf.as_entire_binding(),
                    &silu_buf,
                    slot_off(lu, S_ELEM_SILU),
                ),
                down: mk_gemm_bg(
                    &silu_buf,
                    d_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_DOWN),
                    delta.as_entire_binding(),
                ),
                add2: mk_elem_bg(
                    &add_layout,
                    hidden_b.as_entire_binding(),
                    delta.as_entire_binding(),
                    &hidden_a,
                    slot_off(lu, S_ELEM_ADD2),
                ),
            });
        }

        // Output norm weight (slot 2·n_layer) + output-RMSNorm bind group (hidden_a → normed).
        if !upload_norm(2 * n_layer as u64, &out_norm_info) {
            return None;
        }
        let rms_out = mk_elem_bg(
            &rms_layout,
            hidden_a.as_entire_binding(),
            norm_bind(2 * n_layer as u64),
            &normed,
            tail_rms_slot * SLOT,
        );

        // Per-chunk logits GEMV: normed (B × n_embd) → logits_out chunk slice. The chunk's output
        // binding is offset by `row_start` so the GEMV's `m*vocab + row` lands at the global vocab
        // position (chunk_start is a multiple of VOCAB_CHUNK_ROWS ⇒ 256-byte aligned).
        let mut chunks = Vec::with_capacity(full_chunks);
        for c in 0..full_chunks {
            let row_start = c * VOCAB_CHUNK_ROWS;
            let rows = VOCAB_CHUNK_ROWS.min(vocab - row_start);
            if rows > self.gemm_max_out_dim as usize {
                return None;
            }
            let byte_len = rows as u64 * logits_row_bytes;
            let slot = tail_gemm_base + c as u64;
            // Params are written into `scratch` every call (the full-arena upload in run overwrites the
            // whole buffer), so no build-time write_buffer here — it would just be clobbered.
            let params = GemmGpuParams {
                n_in: n_embd as u32,
                n_out: rows as u32,
                weight_ggml_type: logits_info.ggml_type,
                weight_row_elems: logits_info.dims[0] as u32,
                weight_byte_len: byte_len as u32,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: vocab as u32,
            };
            let weight = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &logits_buf,
                offset: row_start as u64 * logits_row_bytes,
                size: std::num::NonZeroU64::new(byte_len),
            });
            let out_res = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &logits_out,
                offset: (row_start * 4) as wgpu::BufferAddress,
                size: std::num::NonZeroU64::new(((bmax * vocab - row_start) * 4) as u64),
            });
            let gemm = mk_gemm_bg(&normed, weight, slot * SLOT, out_res);
            chunks.push(VerifyChunk {
                gemm,
                rows: rows as u32,
                slot,
                params,
            });
        }

        log::info!(
            "LLM_VERIFY|batched-verify|built: {} layers + {} vocab chunks, B_max={}",
            n_layer,
            full_chunks,
            bmax,
        );
        Some(Box::new(VerifyArenaPlan {
            key,
            n_embd,
            n_ffn,
            kv_dim,
            n_head: n_head as u32,
            n_kv_head: n_kv as u32,
            vocab,
            layout,
            protos,
            layers,
            rms_out,
            tail_rms_slot,
            chunks,
            param_arena,
            scratch: vec![0u8; (total_slots * SLOT) as usize],
            hidden_a,
            logits_out,
            logits_staging,
            use_coop,
        }))
    }
}
