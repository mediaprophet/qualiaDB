//! W3 — native GPU-resident single-fence-per-chunk prefill.
//!
//! The legacy prefill path (`forward.rs::dispatch_prefill_chunk` →
//! `dispatch_prefill_layer_batch`) batches the K/V projection but then runs Q +
//! o_proj + FFN **per token, per layer** through the CPU-orchestrated
//! `dispatch_attention_q_ffn_token`, each op paying its own `submit → poll(wait)`
//! fence and CPU readback. For a 10-token prompt over 32 layers that is ~640
//! blocking fences before the first decode token — the dominant slice of TTFT on
//! edge/mobile and under load.
//!
//! This module keeps the WHOLE prompt chunk (`B ≤ PREFILL_CHUNK_SIZE` tokens)
//! resident in VRAM and encodes every layer's batched forward into ONE command
//! encoder, ONE submit, ONE fence. It is the batched mirror of the resident
//! single-fence decode (`resident_decode.rs`) with two differences: a batch
//! dimension `B` (every op is dispatched over `B` rows — the shaders already take
//! a `batch`/`n_batch`/`num_tokens_in_batch` param and a `wg_id.y` batch axis),
//! and **no output tail** — prefill's only product is the populated KV cache (a
//! side effect of the K/V-write passes), so there is no output RMSNorm, no logits
//! GEMV, no top-k, and no readback. Decode re-embeds the last prompt token, so the
//! final hidden state is discarded. The batched RMSNorm reduces in the same
//! sequential order as the CPU `rms_norm_inplace`, so the KV it writes is expected
//! bit-identical to the legacy path (asserted by the `a3a` differential test).
//!
//! int8 KV (W5a) works for free: the K/V-write passes go through the SAME
//! `fused_attention.wgsl` write path that already branches on `params.kv_quant`,
//! and the per-layer KV binding uses `layout.layer_stride` (int8-aware).
//!
//! Causality: with no sparse-attention route active, `mask_active == 0` and the
//! per-token online-softmax loop bound `logical <= abs_pos` (`abs_pos =
//! batch_start + token_in_batch`) enforces causality directly — so batched Q needs
//! no per-token mask buffer. When a route IS active the arena is ineligible and the
//! legacy per-token path runs (which builds each token's route mask).
//!
//! Toggle: `QUALIA_LLM_RESIDENT_PREFILL` / `set_resident_prefill` (default OFF
//! until the `a3a` KV-identity gate passes). Any per-model ineligibility
//! (unsupported quant, coop-GEMV off, active route, layer cap, oversize dims)
//! falls back to the legacy `dispatch_prefill_chunk` path unchanged.

#![cfg(not(target_arch = "wasm32"))]

use super::*;

/// 256-byte uniform slot stride (WebGPU min uniform offset alignment).
const SLOT: wgpu::BufferAddress = 256;
/// Uniform slots per layer: 7 GEMM (K/V/Q/O/gate/up/down) + 3 attention + 5 elementwise.
const SLOTS_PER_LAYER: u64 = 15;
// Slot indices within a layer.
const S_GEMM_K: u64 = 0;
const S_GEMM_V: u64 = 1;
const S_GEMM_Q: u64 = 2;
const S_GEMM_O: u64 = 3;
const S_GEMM_GATE: u64 = 4;
const S_GEMM_UP: u64 = 5;
const S_GEMM_DOWN: u64 = 6;
const S_ATTN_KW: u64 = 7;
const S_ATTN_VW: u64 = 8;
const S_ATTN_Q: u64 = 9;
const S_ELEM_RMS_ATTN: u64 = 10;
const S_ELEM_ADD1: u64 = 11;
const S_ELEM_RMS_FFN: u64 = 12;
const S_ELEM_SILU: u64 = 13;
const S_ELEM_ADD2: u64 = 14;

/// One transformer layer's pre-built bind groups (encode order).
struct PrefillLayerBinds {
    rms_attn: wgpu::BindGroup,
    /// Dual K+V when SoA (shared act); else use k_gemm/v_gemm.
    dual_kv: Option<wgpu::BindGroup>,
    k_gemm: wgpu::BindGroup,
    k_write: wgpu::BindGroup,
    v_gemm: wgpu::BindGroup,
    v_write: wgpu::BindGroup,
    q_gemm: wgpu::BindGroup,
    q: wgpu::BindGroup,
    /// O-proj + residual → hidden_b (coop residual GEMV); drops separate add1.
    o_resid: wgpu::BindGroup,
    rms_ffn: wgpu::BindGroup,
    /// T-A1 fused SwiGLU when present; else gate/up/silu.
    fused_ffn: Option<wgpu::BindGroup>,
    gate: wgpu::BindGroup,
    up: wgpu::BindGroup,
    silu: wgpu::BindGroup,
    /// Down-proj + residual → hidden_a; drops separate add2.
    down_resid: wgpu::BindGroup,
}

/// Per-layer prototype params: everything set except the per-call batch fields
/// (`n_batch`/`batch`/`num_tokens_in_batch`/`batch_start_token_idx`).
struct PrefillLayerProtos {
    /// K, V, Q, O, gate, up, down.
    gemm: [GemmGpuParams; 7],
    /// k_write, v_write, q-sdpa.
    attn: [AttentionGpuParams; 3],
    /// rms_attn, add1, rms_ffn, silu, add2.
    elem: [ElemGpuParams; 5],
}

pub(crate) struct PrefillArenaPlan {
    key: (u64, u64, u32),
    n_embd: usize,
    n_ffn: usize,
    kv_dim: usize,
    q_dim: usize,
    n_head: u32,
    n_kv_head: u32,
    layout: KvCacheLayout,
    protos: Vec<PrefillLayerProtos>,
    layers: Vec<PrefillLayerBinds>,
    /// Uniform arena: `n_layer × 14` slots, rewritten per call with the batch fields.
    param_arena: wgpu::Buffer,
    /// Reused CPU staging for the param arena (no per-call heap alloc).
    scratch: Vec<u8>,
    /// Embedded-token upload target; also the residual stream (layer in/out).
    hidden_a: wgpu::Buffer,
    use_coop: bool,
}

pub(crate) enum PrefillArenaState {
    Unbuilt,
    /// Build failed for this model — don't retry every prefill.
    Ineligible(u64),
    Ready(Box<PrefillArenaPlan>),
}

impl QTensorEngine {
    fn prefill_plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self
            .gguf_mmap
            .as_deref()
            .map(|m| m.as_ptr() as u64)
            .unwrap_or(0);
        (base, index.tensor_data_start, index.hyperparams.n_layer)
    }

    /// Single-fence-per-chunk resident prefill. Populates the KV cache for the
    /// `n_tokens` prompt positions starting at `batch_start_token_idx`. Returns
    /// `Some(())` on success, or `None` on any ineligibility (the caller then runs
    /// the legacy per-layer path).
    pub(crate) fn dispatch_prefill_chunk_resident(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &[f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        max_layers: u32,
    ) -> Option<()> {
        if !crate::llm_bench::resident_weights_enabled()
            || !crate::llm_bench::coop_gemv_enabled()
            || crate::llm_bench::cpu_attention_enabled()
            || n_tokens == 0
            || n_tokens as usize > PREFILL_CHUNK_SIZE
        {
            return None;
        }
        // A sparse-attention route makes Q masking per-token — legacy handles that.
        if crate::compute_universe::attention_route_mask().active_bits > 0 {
            return None;
        }
        let key = self.prefill_plan_key(index);
        let state = std::mem::replace(&mut self.prefill_arena, PrefillArenaState::Unbuilt);
        let mut plan = match state {
            PrefillArenaState::Ready(p) if p.key == key => p,
            PrefillArenaState::Ineligible(k) if k == (key.0 ^ key.1) => {
                self.prefill_arena = PrefillArenaState::Ineligible(k);
                return None;
            }
            _ => match self.build_prefill_plan(index, key) {
                Some(p) => p,
                None => {
                    self.prefill_arena = PrefillArenaState::Ineligible(key.0 ^ key.1);
                    return None;
                }
            },
        };
        let result = self.run_prefill_chunk(
            &mut plan,
            batch_hidden,
            emb_dim,
            n_tokens,
            batch_start_token_idx,
            max_layers,
        );
        self.prefill_arena = PrefillArenaState::Ready(plan);
        result
    }

    fn run_prefill_chunk(
        &self,
        plan: &mut PrefillArenaPlan,
        batch_hidden: &[f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        max_layers: u32,
    ) -> Option<()> {
        let b = n_tokens as usize;
        let n_embd = plan.n_embd;
        if emb_dim < n_embd
            || b * n_embd > batch_hidden.len()
            || batch_start_token_idx as usize + b > plan.layout.max_context as usize
        {
            return None;
        }
        let n_layer_total = plan.layers.len() as u32;
        let limit = if max_layers == 0 {
            n_layer_total
        } else {
            max_layers.min(n_layer_total)
        };
        let queue = self.gpu_queue();

        // 1) Upload the B embedded tokens (packed B × n_embd, contiguous rows).
        queue.write_buffer(
            &plan.hidden_a,
            0,
            bytemuck::cast_slice(&batch_hidden[..b * n_embd]),
        );

        // 2) Rewrite the param arena with this chunk's batch fields. `protos` (read) and
        //    `scratch` (write) are disjoint fields, so the split borrow is safe.
        let gp = std::mem::size_of::<GemmGpuParams>();
        let ap = std::mem::size_of::<AttentionGpuParams>();
        let ep = std::mem::size_of::<ElemGpuParams>();
        let bu = n_tokens; // B as u32
        {
            let scratch = &mut plan.scratch;
            for (l, proto) in plan.protos.iter().enumerate() {
                let base = l as u64 * SLOTS_PER_LAYER * SLOT;
                for (i, slot) in [
                    S_GEMM_K,
                    S_GEMM_V,
                    S_GEMM_Q,
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
                    a.batch_start_token_idx = batch_start_token_idx;
                    a.token_idx = batch_start_token_idx;
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
        }
        queue.write_buffer(&plan.param_arena, 0, &plan.scratch);

        // 3) Encode every layer's batched forward into ONE encoder.
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ResidentPrefillEncoder"),
                });
        let gemv = if plan.use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let gemv_resid = if plan.use_coop {
            &self.coop_gemv_residual_pipeline
        } else {
            &self.pipeline
        };
        let rms = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?;
        let silu = &self.elem_silu_mul_pipeline;
        let attn = &self.attention_pipeline;
        let fused_pipe = if plan.use_coop {
            &self.ffn_fused_coop_pipeline
        } else {
            &self.ffn_fused_pipeline
        };

        let gemv_wg = |n_out: u32| {
            if plan.use_coop {
                n_out // prefill keeps single-row coop (batch axis is y)
            } else {
                n_out.div_ceil(64)
            }
        };
        let elem_wg = |n: u32| n.div_ceil(64);
        let (n_embd_u, n_ffn_u, kv_dim_u) = (n_embd as u32, plan.n_ffn as u32, plan.kv_dim as u32);
        let q_dim_u = plan.q_dim as u32;
        let kv_pairs = plan.n_kv_head.max(1) * bu;

        // ONE compute pass for the whole chunk (all layers). Previously each of the
        // 15 ops/layer opened its own begin_compute_pass — ~420 driver passes/chunk
        // and catastrophic TTFT. Mirror resident_decode: one pass, many dispatches.
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ResidentPrefillChunk"),
                timestamp_writes: None,
            });
            for lb in plan.layers.iter().take(limit as usize) {
                // ~11–13 dispatches/layer with residual-fused O/down (+ optional fused FFN).
                let dispatch = |cpass: &mut wgpu::ComputePass,
                                pipe: &wgpu::ComputePipeline,
                                bg: &wgpu::BindGroup,
                                wg_x: u32,
                                wg_y: u32| {
                    cpass.set_pipeline(pipe);
                    cpass.set_bind_group(0, bg, &[]);
                    cpass.dispatch_workgroups(wg_x, wg_y, 1);
                };
                dispatch(&mut cpass, rms, &lb.rms_attn, 1, bu);
                if let Some(ref dual) = lb.dual_kv {
                    // Match decode: dual multi-row is opt-in (A/B lost on A2000).
                    let dual_mr = matches!(
                        std::env::var("QUALIA_LLM_DUAL_MR").ok().as_deref(),
                        Some("1") | Some("true")
                    );
                    const DUAL_ROWS: u32 = 4;
                    if dual_mr {
                        dispatch(
                            &mut cpass,
                            &self.dual_gemv_mr_pipeline,
                            dual,
                            gemv_wg(kv_dim_u).div_ceil(DUAL_ROWS).max(1),
                            bu,
                        );
                    } else {
                        dispatch(
                            &mut cpass,
                            &self.dual_gemv_pipeline,
                            dual,
                            gemv_wg(kv_dim_u),
                            bu,
                        );
                    }
                } else {
                    dispatch(&mut cpass, gemv, &lb.k_gemm, gemv_wg(kv_dim_u), bu);
                    dispatch(&mut cpass, gemv, &lb.v_gemm, gemv_wg(kv_dim_u), bu);
                }
                dispatch(&mut cpass, attn, &lb.k_write, kv_pairs, 1);
                dispatch(&mut cpass, attn, &lb.v_write, kv_pairs, 1);
                dispatch(&mut cpass, gemv, &lb.q_gemm, gemv_wg(q_dim_u), bu);
                dispatch(&mut cpass, attn, &lb.q, plan.n_head.max(1), bu);
                // O·attn + residual(hidden_a) → hidden_b
                dispatch(
                    &mut cpass,
                    gemv_resid,
                    &lb.o_resid,
                    gemv_wg(n_embd_u),
                    bu,
                );
                dispatch(&mut cpass, rms, &lb.rms_ffn, 1, bu);
                if let Some(ref fbg) = lb.fused_ffn {
                    dispatch(&mut cpass, fused_pipe, fbg, gemv_wg(n_ffn_u), bu);
                } else {
                    dispatch(&mut cpass, gemv, &lb.gate, gemv_wg(n_ffn_u), bu);
                    dispatch(&mut cpass, gemv, &lb.up, gemv_wg(n_ffn_u), bu);
                    dispatch(&mut cpass, silu, &lb.silu, elem_wg(n_ffn_u), bu);
                }
                // Down·mid + residual(hidden_b) → hidden_a
                dispatch(
                    &mut cpass,
                    gemv_resid,
                    &lb.down_resid,
                    gemv_wg(n_embd_u),
                    bu,
                );
            }
        }

        // ONE submit, ONE fence — the KV cache is populated as a side effect.
        queue.submit(Some(encoder.finish()));
        self.poll_wait();
        Some(())
    }

    /// Build the per-model prefill plan: force weight residency, create batched
    /// activation buffers (`PREFILL_CHUNK_SIZE` rows), the uniform param arena, and
    /// all per-layer bind groups. Any missing piece → `None`.
    fn build_prefill_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<PrefillArenaPlan>> {
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
        let bmax = PREFILL_CHUNK_SIZE;
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
            return None; // batched non-coop GEMV grid is not wired; legacy handles it
        }

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

        // Batched activation buffers (B_max rows each).
        let hidden_a = mk_storage("PrefillHiddenA", bmax * n_embd);
        let hidden_b = mk_storage("PrefillHiddenB", bmax * n_embd);
        let normed = mk_storage("PrefillNormed", bmax * n_embd);
        let attn_out = mk_storage("PrefillAttnOut", bmax * q_dim.max(n_embd));
        // Residual is fused into O/down GEMV — no separate delta buffer.
        // Separate K/V proj slots so dual GEMV can write both in one dispatch.
        let k_proj = mk_storage("PrefillKProj", bmax * kv_dim);
        let v_proj = mk_storage("PrefillVProj", bmax * kv_dim);
        let q_proj = mk_storage("PrefillQProj", bmax * q_dim);
        let gate_buf = mk_storage("PrefillGate", bmax * n_ffn);
        let up_buf = mk_storage("PrefillUp", bmax * n_ffn);
        let silu_buf = mk_storage("PrefillSilu", bmax * n_ffn);

        // Resident norm weights: slot 2L = attn_norm, 2L+1 = ffn_norm (no output norm — prefill).
        let norm_stride = ((n_embd * 4 + 255) & !255) as wgpu::BufferAddress;
        let norm_res = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PrefillNormWeights"),
            size: norm_stride * (2 * n_layer as u64),
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

        let param_arena = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PrefillParamArena"),
            size: n_layer as u64 * SLOTS_PER_LAYER * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind-group layouts + pipelines.
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let attn_layout = self.attention_bind_layout.clone();
        let rms_pipe = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?.clone();
        let rms_layout = rms_pipe.get_bind_group_layout(0);
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
                label: Some("PrefillElemBG"),
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
                          out: &wgpu::Buffer,
                          residual: &wgpu::Buffer| {
            // CoopGemvBGL: binding 4 = residual (dummy = input for non-residual GEMVs).
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PrefillGemmBG"),
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
                        resource: out.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: residual.as_entire_binding(),
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
                label: Some("PrefillAttnBG"),
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

        // Prototype GEMM params (batched-tight: in/out row strides default to n_in/n_out).
        let gemm_proto = |ggml_type: u32,
                          n_in: usize,
                          n_out: usize,
                          row_elems: u32,
                          raw_len: usize| GemmGpuParams {
            n_in: n_in as u32,
            n_out: n_out as u32,
            weight_ggml_type: ggml_type,
            weight_row_elems: row_elems,
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
            let (q_in, q_out) = Self::matmul_dims(&q_info);
            let (k_in, k_out) = Self::matmul_dims(&k_info);
            let (v_in, v_out) = Self::matmul_dims(&v_info);
            let (o_in, o_out) = Self::matmul_dims(&o_info);
            let (g_in, g_out) = Self::matmul_dims(&gate_info);
            let (u_in, u_out) = Self::matmul_dims(&up_info);
            let (d_in, d_out) = Self::matmul_dims(&down_info);
            if q_in != n_embd
                || q_out != q_dim
                || k_in != n_embd
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
            let o_w = res(o_raw)?;
            let ffn_bind = |info: &GgufTensorInfo, raw: &[u8]| -> Option<(wgpu::Buffer, u32, u32, u32)> {
                if let Some(p) = self.promote_matrix_to_f16_resident(info, raw) {
                    return Some(p);
                }
                let b = res(raw)?;
                Some((b, info.ggml_type, raw.len() as u32, info.dims[0] as u32))
            };
            let (g_w, g_ty, g_blen, g_row) = ffn_bind(&gate_info, g_raw)?;
            let (u_w, u_ty, u_blen, u_row) = ffn_bind(&up_info, u_raw)?;
            let (d_w, d_ty, d_blen, d_row) = ffn_bind(&down_info, d_raw)?;

            if !upload_norm(2 * l as u64, &attn_norm) || !upload_norm(2 * l as u64 + 1, &ffn_norm) {
                return None;
            }

            // Prototype attention params (batch fields patched per call).
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
            // Q: coop GEMV preprojects into `q_proj`; SDPA reads with proj_row_stride = q_dim.
            let mut q_p = Self::attention_gpu_params(
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
            q_p.proj_row_stride = q_dim as u32;

            protos.push(PrefillLayerProtos {
                gemm: [
                    gemm_proto(
                        k_info.ggml_type,
                        n_embd,
                        kv_dim,
                        k_info.dims[0] as u32,
                        k_raw.len(),
                    ),
                    gemm_proto(
                        v_info.ggml_type,
                        n_embd,
                        kv_dim,
                        v_info.dims[0] as u32,
                        v_raw.len(),
                    ),
                    gemm_proto(
                        q_info.ggml_type,
                        n_embd,
                        q_dim,
                        q_info.dims[0] as u32,
                        q_raw.len(),
                    ),
                    gemm_proto(
                        o_info.ggml_type,
                        q_dim,
                        n_embd,
                        o_info.dims[0] as u32,
                        o_raw.len(),
                    ),
                    gemm_proto(g_ty, n_embd, n_ffn, g_row, g_blen as usize),
                    gemm_proto(u_ty, n_embd, n_ffn, u_row, u_blen as usize),
                    gemm_proto(d_ty, n_ffn, n_embd, d_row, d_blen as usize),
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
            // Fused SwiGLU when gate/up share a supported quant (same gate as resident_decode).
            let fused_ok = crate::llm_bench::ffn_fusion_enabled()
                && g_ty == u_ty
                && matches!(
                    g_ty,
                    crate::ggml_quants::GGML_TYPE_Q4_0
                        | crate::ggml_quants::GGML_TYPE_Q5_0
                        | crate::ggml_quants::GGML_TYPE_Q8_0
                        | crate::ggml_quants::GGML_TYPE_Q4_K
                        | crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                        | crate::ggml_quants::GGML_TYPE_Q6_K
                );
            let fused_ffn = if fused_ok {
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("PrefillFusedFfn"),
                    layout: &self.ffn_fused_bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: normed.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: g_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: u_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: ubind(&param_arena, slot_off(lu, S_GEMM_GATE), gp_sz),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: silu_buf.as_entire_binding(),
                        },
                    ],
                }))
            } else {
                None
            };
            let dual_kv = if k_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && v_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && use_coop
            {
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("PrefillDualKv"),
                    layout: &self.dual_gemv_bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: normed.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: k_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: ubind(&param_arena, slot_off(lu, S_GEMM_K), gp_sz),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: k_proj.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: v_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: v_proj.as_entire_binding(),
                        },
                    ],
                }))
            } else {
                None
            };
            layers.push(PrefillLayerBinds {
                rms_attn: mk_elem_bg(
                    &rms_layout,
                    hidden_a.as_entire_binding(),
                    norm_bind(2 * lu),
                    &normed,
                    slot_off(lu, S_ELEM_RMS_ATTN),
                ),
                dual_kv,
                k_gemm: mk_gemm_bg(
                    &normed,
                    k_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_K),
                    &k_proj,
                    &normed,
                ),
                k_write: mk_attn_bg(&k_proj, &k_w, slot_off(lu, S_ATTN_KW), l, &attn_out),
                v_gemm: mk_gemm_bg(
                    &normed,
                    v_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_V),
                    &v_proj,
                    &normed,
                ),
                v_write: mk_attn_bg(&v_proj, &v_w, slot_off(lu, S_ATTN_VW), l, &attn_out),
                q_gemm: mk_gemm_bg(
                    &normed,
                    q_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_Q),
                    &q_proj,
                    &normed,
                ),
                // hidden = precomputed Q; weight unused when proj_row_stride != 0.
                q: mk_attn_bg(&q_proj, &q_w, slot_off(lu, S_ATTN_Q), l, &attn_out),
                // O·attn + residual(hidden_a) → hidden_b
                o_resid: mk_gemm_bg(
                    &attn_out,
                    o_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_O),
                    &hidden_b,
                    &hidden_a,
                ),
                rms_ffn: mk_elem_bg(
                    &rms_layout,
                    hidden_b.as_entire_binding(),
                    norm_bind(2 * lu + 1),
                    &normed,
                    slot_off(lu, S_ELEM_RMS_FFN),
                ),
                fused_ffn,
                gate: mk_gemm_bg(
                    &normed,
                    g_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_GATE),
                    &gate_buf,
                    &normed,
                ),
                up: mk_gemm_bg(
                    &normed,
                    u_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_UP),
                    &up_buf,
                    &normed,
                ),
                silu: mk_elem_bg(
                    &silu_layout,
                    gate_buf.as_entire_binding(),
                    up_buf.as_entire_binding(),
                    &silu_buf,
                    slot_off(lu, S_ELEM_SILU),
                ),
                // Down·mid + residual(hidden_b) → hidden_a
                down_resid: mk_gemm_bg(
                    &silu_buf,
                    d_w.as_entire_binding(),
                    slot_off(lu, S_GEMM_DOWN),
                    &hidden_a,
                    &hidden_b,
                ),
            });
        }

        let use_fused = layers.iter().any(|l| l.fused_ffn.is_some());
        let use_dual = layers.iter().any(|l| l.dual_kv.is_some());
        // 11 with fused; -1 for dual K+V vs separate.
        let passes = if use_fused {
            if use_dual {
                10
            } else {
                11
            }
        } else if use_dual {
            12
        } else {
            13
        };
        log::info!(
            "LLM_PREFILL|resident-arena|built: {} layers, {} dispatches/layer (1 compute pass/chunk), fused_ffn={} dual_kv={}",
            n_layer,
            passes,
            use_fused,
            use_dual,
        );
        Some(Box::new(PrefillArenaPlan {
            key,
            n_embd,
            n_ffn,
            kv_dim,
            q_dim,
            n_head: n_head as u32,
            n_kv_head: n_kv as u32,
            layout,
            protos,
            layers,
            param_arena,
            scratch: vec![0u8; (n_layer as u64 * SLOTS_PER_LAYER * SLOT) as usize],
            hidden_a,
            use_coop,
        }))
    }
}
