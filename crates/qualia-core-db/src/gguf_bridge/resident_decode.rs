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
//! per layer it encodes RMSNorm (elem op) → K/V/Q preprojection (coop GEMV) +
//! KV-cache write → Q-SDPA (reads precomputed Q) → O-projection → residual add
//! (elem) → RMSNorm (elem) → fused FFN expansion (`fused_ffn.wgsl` when gate/up
//! share a supported quant; else gate/up GEMV + SiLU·mul) → down GEMV → residual
//! add (elem), then the output RMSNorm and the chunked logits GEMV + top-1
//! block reduction — all into ONE command encoder, ONE submit, ONE fence, with
//! a ~400-byte candidate readback.
//! It is the native mirror of the proven wasm MC8 fused-encoder design
//! (`mc8_wasm/`), built from the same kernels the legacy path already runs:
//! `coop_gemv`/`main` GEMV, `fused_ffn.wgsl`, `fused_attention.wgsl`,
//! `wasm_elementwise.wgsl`, `topk_reduction.wgsl`. The GPU RMSNorm reduces in
//! the same sequential order as the CPU `rms_norm_inplace`, so decode output is
//! expected token-identical to the legacy path (asserted by the `a1d` differential test).
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
/// Resident logits buffer cap. Sized to cover Llama-3.2 128256 vocab in **one**
/// GEMV+topk wave (E3). Was 32768 (= 4 chunks); full-vocab eliminates the multi-chunk tax.
const RESIDENT_LOGITS_CHUNK: usize = 131_072;

/// Static GEMM param slots per layer: K, V, Q, O, gate, up, down.
const GEMM_SLOTS_PER_LAYER: u64 = 7;
/// Dynamic attention param slots per layer: K-write, V-write, Q-SDPA.
const ATTN_SLOTS_PER_LAYER: u64 = 3;

/// Elem param slots (shared across layers): rms(n_embd), add(n_embd), silu(n_ffn).
const ELEM_SLOT_RMS: wgpu::BufferAddress = 0;
const ELEM_SLOT_ADD: wgpu::BufferAddress = SLOT;
const ELEM_SLOT_SILU: wgpu::BufferAddress = SLOT * 2;
const ELEM_SLOTS: u64 = 3;

/// One transformer layer's pre-built bind groups (encode order).
struct LayerBinds {
    rms_attn: wgpu::BindGroup,
    /// Triple Q+K+V GEMV (shared act, GQA) when SoA — preferred over dual+q.
    triple_qkv: Option<wgpu::BindGroup>,
    /// Dual K+V GEMV (shared act) when SoA and triple unavailable; else k_gemm/v_gemm.
    dual_kv: Option<wgpu::BindGroup>,
    k_gemm: wgpu::BindGroup,
    k_write: wgpu::BindGroup,
    v_gemm: wgpu::BindGroup,
    v_write: wgpu::BindGroup,
    /// Coop GEMV: normed → q_proj (decoupled from attention shader).
    q_gemm: wgpu::BindGroup,
    /// Q-SDPA over precomputed Q (`proj_row_stride = q_dim`).
    q: wgpu::BindGroup,
    /// O-proj + residual (coop_gemv_residual) → hidden_b.
    o_resid: wgpu::BindGroup,
    rms_ffn: wgpu::BindGroup,
    /// T-A1: silu(gate·x)·(up·x) in one pass when set; else use gate/up/silu.
    fused_ffn: Option<wgpu::BindGroup>,
    gate: Option<wgpu::BindGroup>,
    up: Option<wgpu::BindGroup>,
    silu: Option<wgpu::BindGroup>,
    /// Down-proj + residual (coop_gemv_residual) → hidden_a.
    down_resid: wgpu::BindGroup,
}

/// ggml types supported by coop fused FFN (incl. Q4_K_SOA for 3B layouts). Not F16.
fn fused_ffn_quant_supported(ggml_type: u32) -> bool {
    use crate::ggml_quants::{
        GGML_TYPE_Q4_0, GGML_TYPE_Q4_K, GGML_TYPE_Q4_K_SOA, GGML_TYPE_Q5_0, GGML_TYPE_Q6_K,
        GGML_TYPE_Q8_0,
    };
    matches!(
        ggml_type,
        GGML_TYPE_Q4_0
            | GGML_TYPE_Q5_0
            | GGML_TYPE_Q8_0
            | GGML_TYPE_Q4_K
            | GGML_TYPE_Q4_K_SOA
            | GGML_TYPE_Q6_K
    )
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
    q_dim: usize,
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
    /// Post-output-norm hidden (logits GEMV input); `COPY_SRC` for sample-path readback.
    normed: wgpu::Buffer,
    /// Candidate readback staging: `[vals(total_cands) | idxs(total_cands)]`.
    staging: wgpu::Buffer,
    /// Full-hidden MAP_READ staging for sampler-compatible resident (n_embd f32).
    hidden_staging: wgpu::Buffer,
    use_coop: bool,
    /// Multi-row coop GEMV (8 rows/WG) — armed for Q4_K_SOA decode (3B bandwidth path).
    use_multirow: bool,
    /// Warp GEMV (32 thr/row) for Q4_K_SOA — more FMA/thread than 256-wide coop.
    use_warp: bool,
    /// Multi-row fused FFN (4 rows/WG) for Q4_K_SOA.
    use_ffn_mr: bool,
    /// Warp fused FFN (32 thr/row) for Q4_K_SOA.
    use_ffn_warp: bool,
    /// True when every layer has fused_ffn bind groups (T-A1).
    /// Read by lab audit via `ffn_fusion_in_resident()`; kept on plan for diagnostics.
    #[allow(dead_code)]
    use_fused_ffn: bool,
}

pub(crate) enum ResidentDecodeState {
    Unbuilt,
    /// Build failed for this model — don't retry every token.
    Ineligible(u64),
    Ready(Box<ResidentDecodePlan>),
}

/// Greedy → argmax token; sample path → post-norm hidden was written to the caller buffer.
enum ResidentTokenOutcome {
    Argmax(StreamingArgmaxResult),
    HiddenReady,
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

    /// Single-fence resident-token decode (greedy top-1). Returns the argmax token, or `None`
    /// on any ineligibility (caller falls back to the legacy per-layer path).
    pub fn dispatch_token_forward_resident(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb: &[f32],
        token_idx: u32,
    ) -> Option<StreamingArgmaxResult> {
        match self.with_resident_plan(index, |this, plan| {
            this.run_resident_token(plan, emb, token_idx, None)
        })? {
            Some(ResidentTokenOutcome::Argmax(a)) => Some(a),
            _ => None,
        }
    }

    /// Sampler-compatible resident forward: same single-fence layer stack + output
    /// RMSNorm, then read back the normed hidden into `out_hidden` so the caller can
    /// project full logits + sample on CPU without the legacy ~107-fence path.
    ///
    /// `emb` is the token embedding input; `out_hidden` receives post-output-norm state
    /// (may be the same logical buffer only if the caller copies input first — they must
    /// not alias while the upload of `emb` is live; pass distinct slices).
    pub fn dispatch_token_forward_resident_hidden(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb: &[f32],
        token_idx: u32,
        out_hidden: &mut [f32],
    ) -> bool {
        matches!(
            self.with_resident_plan(index, |this, plan| {
                this.run_resident_token(plan, emb, token_idx, Some(out_hidden))
            }),
            Some(Some(ResidentTokenOutcome::HiddenReady))
        )
    }

    fn with_resident_plan<R>(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        f: impl FnOnce(&Self, &mut ResidentDecodePlan) -> R,
    ) -> Option<R> {
        if !crate::llm_bench::resident_decode_enabled()
            || !crate::llm_bench::resident_weights_enabled()
            || crate::llm_bench::cpu_attention_enabled()
            || crate::llm_bench::kv_dict_enabled()
        {
            return None;
        }
        // Note: do NOT skip resident for mode=cuda — the wgpu mega-pass is still faster
        // end-to-end than legacy host-layered CUDA GEMVs. CUDA FFN block helps the
        // non-resident / fused-resident FFN cold path only.
        let key = self.plan_key(index);
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
        let result = f(self, &mut plan);
        self.resident_decode = ResidentDecodeState::Ready(plan);
        Some(result)
    }

    fn run_resident_token(
        &self,
        plan: &mut ResidentDecodePlan,
        emb: &[f32],
        token_idx: u32,
        out_hidden: Option<&mut [f32]>,
    ) -> Option<ResidentTokenOutcome> {
        let n_embd = plan.n_embd;
        if emb.len() < n_embd || token_idx >= plan.layout.max_context {
            return None;
        }
        let queue = self.gpu_queue();

        // 1) Per-token uploads: embedding + dynamic attention params.
        // Skip full KV-mask upload when route mask is inactive (common path).
        queue.write_buffer(&plan.hidden_a, 0, bytemuck::cast_slice(&emb[..n_embd]));

        let (mask_words, mask_active) =
            crate::compute_universe::attention_kv_mask_u32(token_idx, plan.layout.max_context);
        if mask_active != 0 {
            queue.write_buffer(
                self.attention_mask_buf.as_ref()?,
                0,
                bytemuck::cast_slice(&mask_words),
            );
        }

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

        // 2) Encode the WHOLE token in ONE compute pass: all layers + output norm + logits.
        // Previously: 15 passes/layer × 28–32 layers + norm + per-chunk logits ≈ 450+ passes.
        // Then: 1 pass/layer. Now: **one pass for the entire forward**. Driver overhead collapses.
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ResidentTokenEncoder"),
                });
        let rms = self.elem_gpu_pipeline(ELEM_OP_RMS_NORM)?;
        let silu = &self.elem_silu_mul_pipeline;
        let attn = &self.attention_pipeline;
        let sample_path = out_hidden.is_some();
        let topk = if sample_path {
            None
        } else {
            Some(self.output_topk_pipeline.as_ref()?)
        };

        // Projection GEMV (Q, K/V fallback): always 1-row/WG.
        // Residual O/down + logits: multi-row when plan.use_multirow.
        let gemv_proj_wg = |n_out: u32| {
            if plan.use_coop {
                n_out
            } else {
                n_out.div_ceil(64)
            }
        };
        let gemv_large_wg = |n_out: u32| {
            if plan.use_coop && plan.use_multirow {
                crate::llm_bench::coop_gemv_workgroups(n_out)
            } else if plan.use_coop {
                n_out
            } else {
                n_out.div_ceil(64)
            }
        };
        let elem_wg = |n: u32| n.div_ceil(64);
        let (n_embd_u, n_ffn_u) = (n_embd as u32, plan.n_ffn as u32);
        let kv_dim_u = plan.kv_dim as u32;
        let q_dim_u = plan.q_dim as u32;

        {
            // Lab L1.1: optional TIMESTAMP_QUERY around the mega-pass (FusedBlock phase).
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ResidentFullToken"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            // Fused FFN: multi-row (4 rows/WG) for Q4 SoA; else coop 256; else naive.
            const FFN_MR_ROWS: u32 = 4;
            let (fused_pipe, fused_wg) = if plan.use_coop && plan.use_ffn_mr {
                (
                    &self.ffn_fused_mr_pipeline,
                    n_ffn_u.div_ceil(FFN_MR_ROWS).max(1),
                )
            } else if plan.use_coop && plan.use_ffn_warp {
                (&self.ffn_fused_warp_pipeline, n_ffn_u)
            } else if plan.use_coop {
                (&self.ffn_fused_coop_pipeline, n_ffn_u)
            } else {
                (&self.ffn_fused_pipeline, n_ffn_u.div_ceil(64))
            };
            let gemv_proj = if plan.use_coop {
                &self.coop_gemv_pipeline
            } else {
                &self.pipeline
            };
            let gemv_large = if plan.use_coop && plan.use_multirow {
                &self.coop_gemv_mr_pipeline
            } else if plan.use_coop && plan.use_warp {
                &self.coop_gemv_warp_pipeline
            } else if plan.use_coop {
                &self.coop_gemv_pipeline
            } else {
                &self.pipeline
            };
            let gemv_resid = if plan.use_multirow {
                &self.coop_gemv_residual_mr_pipeline
            } else if plan.use_warp {
                &self.coop_gemv_residual_warp_pipeline
            } else {
                &self.coop_gemv_residual_pipeline
            };
            for lb in &plan.layers {
                // RMS attn
                cpass.set_pipeline(rms);
                cpass.set_bind_group(0, &lb.rms_attn, &[]);
                cpass.dispatch_workgroups(1, 1, 1);
                // Triple Q+K+V (shared act) or dual K+V + Q or split.
                if let Some(ref tri) = lb.triple_qkv {
                    cpass.set_pipeline(&self.triple_gemv_pipeline);
                    cpass.set_bind_group(0, tri, &[]);
                    cpass.dispatch_workgroups(q_dim_u.max(1), 1, 1);
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.k_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.v_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                } else if let Some(ref dual) = lb.dual_kv {
                    // Dual multi-row (4 rows/WG): opt-in. A/B on A2000 3B lost vs 1-row dual
                    // (~8.75 vs ~9.0). QUALIA_LLM_DUAL_MR=1 to force.
                    let dual_mr = matches!(
                        std::env::var("QUALIA_LLM_DUAL_MR").ok().as_deref(),
                        Some("1") | Some("true")
                    );
                    const DUAL_ROWS: u32 = 4;
                    if dual_mr {
                        cpass.set_pipeline(&self.dual_gemv_mr_pipeline);
                        cpass.set_bind_group(0, dual, &[]);
                        cpass.dispatch_workgroups(kv_dim_u.div_ceil(DUAL_ROWS).max(1), 1, 1);
                    } else {
                        cpass.set_pipeline(&self.dual_gemv_pipeline);
                        cpass.set_bind_group(0, dual, &[]);
                        cpass.dispatch_workgroups(kv_dim_u.max(1), 1, 1);
                    }
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.k_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.v_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                    cpass.set_pipeline(gemv_proj);
                    cpass.set_bind_group(0, &lb.q_gemm, &[]);
                    cpass.dispatch_workgroups(gemv_proj_wg(q_dim_u), 1, 1);
                } else {
                    cpass.set_pipeline(gemv_proj);
                    cpass.set_bind_group(0, &lb.k_gemm, &[]);
                    cpass.dispatch_workgroups(gemv_proj_wg(kv_dim_u), 1, 1);
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.k_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                    cpass.set_pipeline(gemv_proj);
                    cpass.set_bind_group(0, &lb.v_gemm, &[]);
                    cpass.dispatch_workgroups(gemv_proj_wg(kv_dim_u), 1, 1);
                    cpass.set_pipeline(attn);
                    cpass.set_bind_group(0, &lb.v_write, &[]);
                    cpass.dispatch_workgroups(plan.n_kv_head.max(1), 1, 1);
                    cpass.set_pipeline(gemv_proj);
                    cpass.set_bind_group(0, &lb.q_gemm, &[]);
                    cpass.dispatch_workgroups(gemv_proj_wg(q_dim_u), 1, 1);
                }
                // SDPA + O resid + RMS ffn
                cpass.set_pipeline(attn);
                cpass.set_bind_group(0, &lb.q, &[]);
                cpass.dispatch_workgroups(plan.n_head.max(1), 1, 1);
                cpass.set_pipeline(gemv_resid);
                cpass.set_bind_group(0, &lb.o_resid, &[]);
                cpass.dispatch_workgroups(gemv_large_wg(n_embd_u), 1, 1);
                cpass.set_pipeline(rms);
                cpass.set_bind_group(0, &lb.rms_ffn, &[]);
                cpass.dispatch_workgroups(1, 1, 1);
                // T-A1 FFN expansion: fused_ffn OR gate+up+silu.
                if let Some(ref fbg) = lb.fused_ffn {
                    cpass.set_pipeline(fused_pipe);
                    cpass.set_bind_group(0, fbg, &[]);
                    cpass.dispatch_workgroups(fused_wg, 1, 1);
                } else if let (Some(g), Some(u), Some(s)) =
                    (lb.gate.as_ref(), lb.up.as_ref(), lb.silu.as_ref())
                {
                    for (pipe, bg, wg_x) in [
                        (gemv_large, g, gemv_large_wg(n_ffn_u)),
                        (gemv_large, u, gemv_large_wg(n_ffn_u)),
                        (silu, s, elem_wg(n_ffn_u)),
                    ] {
                        cpass.set_pipeline(pipe);
                        cpass.set_bind_group(0, bg, &[]);
                        cpass.dispatch_workgroups(wg_x, 1, 1);
                    }
                }
                // Down + residual fused. Multi-row opt-in only (A/B lost on A2000 3B).
                cpass.set_pipeline(gemv_resid);
                cpass.set_bind_group(0, &lb.down_resid, &[]);
                cpass.dispatch_workgroups(gemv_large_wg(n_embd_u), 1, 1);
            }
            // Output RMSNorm → `plan.normed`.
            cpass.set_pipeline(rms);
            cpass.set_bind_group(0, &plan.rms_out, &[]);
            cpass.dispatch_workgroups(1, 1, 1);

            // Logits + topk (E3: one full-vocab chunk when cap ≥ vocab).
            // Device maxComputeWorkgroupsPerDimension is typically 65535 — for
            // vocab > 60k always use multi-row GEMV (8 rows/WG) even if residual
            // multirow is off, so a single 128k dispatch stays legal.
            if !sample_path {
                let topk = topk?;
                let logits_mr = plan.use_coop
                    && plan.out_chunks.iter().any(|c| c.rows > 60_000);
                let logits_pipe = if logits_mr {
                    &self.coop_gemv_mr_pipeline
                } else {
                    gemv_large
                };
                let logits_wg = |rows: u32| {
                    if logits_mr {
                        crate::llm_bench::coop_gemv_workgroups(rows)
                    } else {
                        gemv_large_wg(rows)
                    }
                };
                for chunk in &plan.out_chunks {
                    cpass.set_pipeline(logits_pipe);
                    cpass.set_bind_group(0, &chunk.gemm, &[]);
                    cpass.dispatch_workgroups(logits_wg(chunk.rows), 1, 1);
                    cpass.set_pipeline(topk);
                    cpass.set_bind_group(0, &chunk.topk, &[]);
                    cpass.dispatch_workgroups(chunk.cand_count as u32, 1, 1);
                }
            }
            drop(cpass);
            crate::llm_gpu_profiler::resolve(&mut encoder);

            if sample_path {
                let hb = (n_embd * 4) as wgpu::BufferAddress;
                encoder.copy_buffer_to_buffer(&plan.normed, 0, &plan.hidden_staging, 0, hb);
            } else {
                let cand_val = self.topk_cand_val_buf.as_ref()?;
                let cand_idx = self.topk_cand_idx_buf.as_ref()?;
                // One copy of the full candidate pack (cand_base laid them out contiguously).
                let cand_bytes = (plan.total_cands * 4) as wgpu::BufferAddress;
                encoder.copy_buffer_to_buffer(cand_val, 0, &plan.staging, 0, cand_bytes);
                encoder.copy_buffer_to_buffer(
                    cand_idx,
                    0,
                    &plan.staging,
                    cand_bytes,
                    cand_bytes,
                );
            }
        }

        // 3) ONE submit, ONE fence, tiny readback.
        queue.submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);

        if let Some(out) = out_hidden {
            if out.len() < n_embd {
                return None;
            }
            let map_bytes = (n_embd * 4) as wgpu::BufferAddress;
            let slice = plan.hidden_staging.slice(..map_bytes);
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
                let _ = plan.hidden_staging.unmap();
                return None;
            }
            {
                let data = slice.get_mapped_range().expect("wgpu buffer map_range failed");
                let floats: &[f32] = bytemuck::cast_slice(&data[..n_embd * 4]);
                out[..n_embd].copy_from_slice(floats);
            }
            plan.hidden_staging.unmap();
            return Some(ResidentTokenOutcome::HiddenReady);
        }

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
            let data = slice.get_mapped_range().expect("wgpu buffer map_range failed");
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
            Some(ResidentTokenOutcome::Argmax(StreamingArgmaxResult {
                best_token_id,
                max_logit,
            }))
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
        // Normed hidden is also COPY_SRC for the sampler-compatible readback path.
        let mk_storage_copy_src = |label: &str, floats: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: ((floats * 4 + 255) & !255).max(4) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
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
        let normed = mk_storage_copy_src("ResidentNormed", n_embd);
        let attn_out = mk_storage("ResidentAttnOut", q_dim.max(n_embd));
        // Residual add is fused into coop_gemv_residual (no separate delta buffer).
        // Separate K/V proj slots so dual GEMV can write both in one dispatch.
        let k_proj = mk_storage("ResidentKProj", kv_dim);
        let v_proj = mk_storage("ResidentVProj", kv_dim);
        // Q preprojection target (coop GEMV); attention reads with proj_row_stride = q_dim.
        let q_proj = mk_storage("ResidentQProj", q_dim);
        let gate_buf = mk_storage("ResidentGate", n_ffn);
        let up_buf = mk_storage("ResidentUp", n_ffn);
        let silu_buf = mk_storage("ResidentSilu", n_ffn);
        let logits_chunk = mk_storage("ResidentLogitsChunk", RESIDENT_LOGITS_CHUNK);

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
        let full_chunks = vocab.div_ceil(RESIDENT_LOGITS_CHUNK);
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
        let hidden_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentHiddenStaging"),
            size: ((n_embd * 4).max(16)) as wgpu::BufferAddress,
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
        let rms_layout = rms_pipe.get_bind_group_layout(0);
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
            // Binding 4 = dummy residual (shared CoopGemvBGL; input is a safe stand-in).
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: input.as_entire_binding(),
                    },
                ],
            })
        };
        let resid_layout = &self.coop_gemv_residual_bind_layout;
        let mk_resid_bg = |label: &str,
                           input: &wgpu::Buffer,
                           weight: wgpu::BindingResource,
                           p_slot: u64,
                           residual: &wgpu::Buffer,
                           out: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: resid_layout,
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: residual.as_entire_binding(),
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

        let gemm_params = |ggml_type: u32,
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

        let mut layers = Vec::with_capacity(n_layer as usize);
        let mut layer_protos = Vec::with_capacity(n_layer as usize);
        let mut layer_gate_ggml_type: u32 = 0;
        for l in 0..n_layer {
            let t = index.get_layer_tensors(l);
            let (q_info, k_info, v_info) = (t.attn_q?, t.attn_k?, t.attn_v?);
            let o_info = t.attn_output?;
            let (gate_info, up_info, down_info) = (t.ffn_gate?, t.ffn_up?, t.ffn_down?);
            if l == 0 {
                layer_gate_ggml_type = gate_info.ggml_type;
            }
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
            // CUDA multi-weight preload only when CUDA_DECODE is on. Resident mega-pass
            // is the default winner (~6.7 tok/s); duplicating ~1.8 GiB SoA into the CUDA
            // slab while wgpu also holds weights steals VRAM and does not help resident.
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            if crate::prefer_tensor_core_gemm()
                && matches!(
                    std::env::var("QUALIA_LLM_CUDA_DECODE").ok().as_deref(),
                    Some("1") | Some("true")
                )
            {
                use crate::ggml_quants::GGML_TYPE_Q4_K_SOA;
                if l == 0 {
                    if let Some(layout) = self.kv_layout.as_ref() {
                        if !layout.int8 && layout.dict_k == 0 {
                            let _ = crate::ensure_device_kv_cache(
                                layout.max_context,
                                layout.n_layer,
                                layout.n_kv_head,
                                layout.head_dim,
                                layout.slot_kv_elems,
                                layout.layer_stride,
                                layout.total_f32_elems,
                            );
                        }
                    }
                }
                let mut pack: Vec<(&[u8], usize, usize)> = Vec::with_capacity(7);
                if q_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((q_raw, q_in, q_out));
                }
                if k_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((k_raw, n_embd, kv_dim));
                }
                if v_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((v_raw, n_embd, kv_dim));
                }
                if o_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((o_raw, o_in, o_out));
                }
                if gate_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((g_raw, g_in, g_out));
                }
                if up_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((u_raw, u_in, u_out));
                }
                if down_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                    pack.push((d_raw, d_in, d_out));
                }
                if !pack.is_empty() {
                    let _ = crate::preload_q4k_soa_weights(&pack);
                }
                // Host dense TC cache prewarm (implements prewarm_cuda_weight).
                // Opt-in: QUALIA_LLM_CUDA_TC_PREWARM=1 — densify is cold and VRAM-adjacent;
                // SoA device preload above is the default CUDA_DECODE win path.
                let tc_prewarm = matches!(
                    std::env::var("QUALIA_LLM_CUDA_TC_PREWARM").ok().as_deref(),
                    Some("1") | Some("true")
                );
                if tc_prewarm {
                    let mut warmed = 0u32;
                    let mut try_warm =
                        |info: &crate::gguf_sharder::GgufTensorInfo, raw: &[u8], n_in: usize, n_out: usize| {
                            if QTensorEngine::prewarm_cuda_weight(info, raw, n_in, n_out) {
                                warmed += 1;
                            }
                        };
                    if q_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&q_info, q_raw, q_in, q_out);
                    }
                    if k_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&k_info, k_raw, n_embd, kv_dim);
                    }
                    if v_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&v_info, v_raw, n_embd, kv_dim);
                    }
                    if o_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&o_info, o_raw, o_in, o_out);
                    }
                    if gate_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&gate_info, g_raw, g_in, g_out);
                    }
                    if up_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&up_info, u_raw, u_in, u_out);
                    }
                    if down_info.ggml_type == GGML_TYPE_Q4_K_SOA {
                        try_warm(&down_info, d_raw, d_in, d_out);
                    }
                    if l == 0 && warmed > 0 {
                        log::info!(
                            "LLM_LOAD|cuda_tc_prewarm|layer0|dense_entries={}|cache_len={}",
                            warmed,
                            crate::weight_cache_len()
                        );
                    }
                }
            }
            let res = |raw: &[u8]| self.resident_weight_buffer(raw.as_ptr() as u64, raw);
            let (q_w, k_w, v_w) = (res(q_raw)?, res(k_raw)?, res(v_raw)?);
            let o_w = res(o_raw)?;
            // FFN quant→f16 promotion: bind f16 for gate/up/down when eligible (fast coop path).
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
            if l == 0
                && (g_ty == crate::ggml_quants::GGML_TYPE_F16
                    || u_ty == crate::ggml_quants::GGML_TYPE_F16
                    || d_ty == crate::ggml_quants::GGML_TYPE_F16)
            {
                log::info!(
                    "LLM_LOAD|ffn-f16|promoted gate/up/down (types g={} u={} d={})",
                    g_ty,
                    u_ty,
                    d_ty
                );
            }

            if !upload_norm(2 * l as u64, &attn_norm) || !upload_norm(2 * l as u64 + 1, &ffn_norm) {
                return None;
            }

            // Static GEMM param slots for this layer: K, V, Q, O, gate, up, down.
            let gbase = l as u64 * GEMM_SLOTS_PER_LAYER;
            for (i, (ggml_type, n_in, n_out, row_elems, raw_len)) in [
                (
                    k_info.ggml_type,
                    n_embd,
                    kv_dim,
                    k_info.dims[0] as u32,
                    k_raw.len(),
                ),
                (
                    v_info.ggml_type,
                    n_embd,
                    kv_dim,
                    v_info.dims[0] as u32,
                    v_raw.len(),
                ),
                (
                    q_info.ggml_type,
                    n_embd,
                    q_dim,
                    q_info.dims[0] as u32,
                    q_raw.len(),
                ),
                (
                    o_info.ggml_type,
                    q_dim,
                    n_embd,
                    o_info.dims[0] as u32,
                    o_raw.len(),
                ),
                (g_ty, n_embd, n_ffn, g_row, g_blen as usize),
                (u_ty, n_embd, n_ffn, u_row, u_blen as usize),
                (d_ty, n_ffn, n_embd, d_row, d_blen as usize),
            ]
            .into_iter()
            .enumerate()
            {
                queue.write_buffer(
                    &static_arena,
                    (gbase + i as u64) * SLOT,
                    bytemuck::bytes_of(&gemm_params(ggml_type, n_in, n_out, row_elems, raw_len)),
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
            // Q: coop GEMV preprojects into `q_proj`; SDPA pass only reads + RoPE.
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
                0,
            );
            q_p.proj_row_stride = q_dim as u32;
            layer_protos.push(LayerProtos {
                k_write: k_p,
                v_write: v_p,
                q: q_p,
            });

            let dyn_base = l as u64 * ATTN_SLOTS_PER_LAYER;
            // Triple Q+K+V: opt-in only. A/B on A2000 3B: dual+Q ~8.94 vs triple ~8.57
            // (triple fires q_dim WGs with 3× dequant; dual is lighter). QUALIA_LLM_TRIPLE_QKV=1.
            let want_triple = matches!(
                std::env::var("QUALIA_LLM_TRIPLE_QKV").ok().as_deref(),
                Some("1") | Some("true")
            );
            let triple_qkv = if want_triple
                && q_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && k_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && v_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && use_coop
            {
                // Dedicated params: n_out=q_dim, weight_byte_len packs n_kv (GQA).
                let tp = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ResTripleQkvParams"),
                    size: SLOT,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut p = gemm_params(
                    q_info.ggml_type,
                    n_embd,
                    q_dim,
                    q_info.dims[0] as u32,
                    q_raw.len(),
                );
                // triple_gemv.wgsl reads n_kv from weight_byte_len.
                p.weight_byte_len = kv_dim as u32;
                queue.write_buffer(&tp, 0, bytemuck::bytes_of(&p));
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ResTripleQkv"),
                    layout: &self.triple_gemv_bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: normed.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: q_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: tp.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: q_proj.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: k_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: k_proj.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: v_w.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: v_proj.as_entire_binding(),
                        },
                    ],
                }))
            } else {
                None
            };
            let dual_kv = if triple_qkv.is_none()
                && k_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && v_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && use_coop
            {
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ResDualKv"),
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
                            resource: ubind(&static_arena, gbase * SLOT, gp_sz),
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
            layers.push(LayerBinds {
                rms_attn: mk_elem_bg(
                    "ResRmsAttn",
                    &rms_layout,
                    hidden_a.as_entire_binding(),
                    norm_bind(2 * l as u64),
                    &normed,
                    ELEM_SLOT_RMS,
                ),
                triple_qkv,
                dual_kv,
                k_gemm: mk_gemm_bg(
                    "ResKGemm",
                    &normed,
                    k_w.as_entire_binding(),
                    gbase,
                    &k_proj,
                ),
                k_write: mk_attn_bg("ResKWrite", &k_proj, &k_w, dyn_base, l, &attn_out),
                v_gemm: mk_gemm_bg(
                    "ResVGemm",
                    &normed,
                    v_w.as_entire_binding(),
                    gbase + 1,
                    &v_proj,
                ),
                v_write: mk_attn_bg("ResVWrite", &v_proj, &v_w, dyn_base + 1, l, &attn_out),
                q_gemm: mk_gemm_bg(
                    "ResQGemm",
                    &normed,
                    q_w.as_entire_binding(),
                    gbase + 2,
                    &q_proj,
                ),
                // hidden binding = precomputed Q; weight unused when proj_row_stride != 0.
                q: mk_attn_bg("ResQSdpa", &q_proj, &q_w, dyn_base + 2, l, &attn_out),
                // O·attn + residual(hidden_a) → hidden_b (one dispatch).
                o_resid: mk_resid_bg(
                    "ResOResid",
                    &attn_out,
                    o_w.as_entire_binding(),
                    gbase + 3,
                    &hidden_a,
                    &hidden_b,
                ),
                rms_ffn: mk_elem_bg(
                    "ResRmsFfn",
                    &rms_layout,
                    hidden_b.as_entire_binding(),
                    norm_bind(2 * l as u64 + 1),
                    &normed,
                    ELEM_SLOT_RMS,
                ),
                // T-A1: fuse when flag on, gate/up same quant, and fused_ffn.wgsl supports it.
                // Q4_K_SOA / F16-promoted stay on coop GEMV + separate SiLU (faster for those).
                fused_ffn: {
                    let want = crate::llm_bench::ffn_fusion_enabled()
                        && g_ty == u_ty
                        && fused_ffn_quant_supported(g_ty);
                    if want {
                        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("ResFusedFfn"),
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
                                    // Gate params describe both streams (same type+dims).
                                    resource: ubind(&static_arena, (gbase + 4) * SLOT, gp_sz),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: silu_buf.as_entire_binding(),
                                },
                            ],
                        }))
                    } else {
                        None
                    }
                },
                gate: {
                    let want = !(crate::llm_bench::ffn_fusion_enabled()
                        && g_ty == u_ty
                        && fused_ffn_quant_supported(g_ty));
                    if want {
                        Some(mk_gemm_bg(
                            "ResGate",
                            &normed,
                            g_w.as_entire_binding(),
                            gbase + 4,
                            &gate_buf,
                        ))
                    } else {
                        None
                    }
                },
                up: {
                    let want = !(crate::llm_bench::ffn_fusion_enabled()
                        && g_ty == u_ty
                        && fused_ffn_quant_supported(g_ty));
                    if want {
                        Some(mk_gemm_bg(
                            "ResUp",
                            &normed,
                            u_w.as_entire_binding(),
                            gbase + 5,
                            &up_buf,
                        ))
                    } else {
                        None
                    }
                },
                silu: {
                    let want = !(crate::llm_bench::ffn_fusion_enabled()
                        && g_ty == u_ty
                        && fused_ffn_quant_supported(g_ty));
                    if want {
                        Some(mk_elem_bg(
                            "ResSilu",
                            &silu_layout,
                            gate_buf.as_entire_binding(),
                            up_buf.as_entire_binding(),
                            &silu_buf,
                            ELEM_SLOT_SILU,
                        ))
                    } else {
                        None
                    }
                },
                // Down·silu + residual(hidden_b) → hidden_a (one dispatch).
                down_resid: mk_resid_bg(
                    "ResDownResid",
                    &silu_buf,
                    d_w.as_entire_binding(),
                    gbase + 6,
                    &hidden_b,
                    &hidden_a,
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
        // cand_base packs multi-chunk candidates contiguously for one mega-pass.
        let mut out_chunks = Vec::with_capacity(full_chunks);
        let mut cand_base_acc = 0u32;
        for c in 0..full_chunks {
            let row_start = c * RESIDENT_LOGITS_CHUNK;
            let rows = RESIDENT_LOGITS_CHUNK.min(vocab - row_start);
            // Resident logits buffer is sized to RESIDENT_LOGITS_CHUNK (not stack gemm max).
            if rows > RESIDENT_LOGITS_CHUNK {
                return None;
            }
            let cand_count = rows.div_ceil(block_size);
            let byte_len = rows as u64 * logits_row_bytes;
            let gemm_slot = n_layer as u64 * GEMM_SLOTS_PER_LAYER + c as u64;
            queue.write_buffer(
                &static_arena,
                gemm_slot * SLOT,
                bytemuck::bytes_of(&gemm_params(
                    logits_info.ggml_type,
                    n_embd,
                    rows,
                    logits_info.dims[0] as u32,
                    byte_len as usize,
                )),
            );
            let tparams = crate::topk::topk_params_bytes_with_base(
                rows as u32,
                1,
                block_size as u32,
                cand_base_acc,
            );
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
                cand_count,
            });
            cand_base_acc += cand_count as u32;
        }

        let dyn_bytes = (n_layer as u64 * ATTN_SLOTS_PER_LAYER * SLOT) as usize;
        let use_fused_ffn = !layers.is_empty() && layers.iter().all(|lb| lb.fused_ffn.is_some());
        // Residual-fused O/down + triple QKV when SoA: ~9 dispatches/layer with fused FFN.
        let use_triple = layers.iter().any(|lb| lb.triple_qkv.is_some());
        let use_dual = layers.iter().any(|lb| lb.dual_kv.is_some());
        let passes_per_layer = if use_fused_ffn {
            if use_triple {
                9
            } else if use_dual {
                10
            } else {
                11
            }
        } else {
            13
        };
        let passes_token = n_layer as usize * passes_per_layer + 1 + out_chunks.len() * 2;
        crate::llm_bench::set_ffn_fusion_in_resident(use_fused_ffn);
        // Layer weights may be Q4_K_SOA while logits stay Q6_K (3B .soa.p64: 2520 B/row logits).
        let is_q4_soa = layer_gate_ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
            || logits_info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA;
        // Residual multirow: opt-in only (A/B lost vs 1-row on A2000 3B).
        // Logits always use multirow geometry when n_out > 60k (device max WG dim).
        let use_multirow = use_coop
            && is_q4_soa
            && matches!(
                std::env::var("QUALIA_LLM_MULTIROW").ok().as_deref(),
                Some("1") | Some("true")
            );
        let use_warp = use_coop
            && is_q4_soa
            && !use_multirow
            && matches!(
                std::env::var("QUALIA_LLM_WARP_GEMV").ok().as_deref(),
                Some("1") | Some("true")
            );
        // FFN multi-row: opt-in only (A/B lost on A2000 3B SoA).
        let use_ffn_warp = use_coop
            && is_q4_soa
            && matches!(
                std::env::var("QUALIA_LLM_FFN_WARP").ok().as_deref(),
                Some("1") | Some("true")
            );
        let use_ffn_mr = use_coop
            && is_q4_soa
            && !use_ffn_warp
            && matches!(
                std::env::var("QUALIA_LLM_FFN_MR").ok().as_deref(),
                Some("1") | Some("true")
            );
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            let cuda_dense_cache = crate::weight_cache_len();
            let cuda_soa_weights = crate::q4k_device_weight_count();
            log::info!(
                "LLM_DECODE|resident-plan|built: {} layers, {} passes/token, fused_ffn={} triple_qkv={} dual_kv={} ffn_mr={} multirow={} cuda_soa_weights={} cuda_dense_cache={}",
                n_layer,
                passes_token,
                use_fused_ffn,
                use_triple,
                use_dual,
                use_ffn_mr,
                use_multirow,
                cuda_soa_weights,
                cuda_dense_cache,
            );
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "cuda")))]
        log::info!(
            "LLM_DECODE|resident-plan|built: {} layers, {} passes/token, fused_ffn={} ffn_mr={} ffn_warp={}",
            n_layer,
            passes_token,
            use_fused_ffn,
            use_ffn_mr,
            use_ffn_warp,
        );
        Some(Box::new(ResidentDecodePlan {
            key,
            n_embd,
            n_ffn,
            kv_dim,
            q_dim,
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
            normed,
            staging,
            hidden_staging,
            use_coop,
            use_multirow,
            use_warp,
            use_ffn_mr,
            use_ffn_warp,
            use_fused_ffn,
        }))
    }
}
