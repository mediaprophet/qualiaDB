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

#![cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]

use super::*;
pub(crate) use etalon_inference::native_resident::resident_decode_plan::*;

impl QTensorEngine {
    /// Exact steady-state compute dispatch count for the prepared resident plan.
    pub fn resident_dispatches_per_token(&self) -> Option<u32> {
        match &self.resident_decode {
            ResidentDecodeState::Ready(plan) => Some(plan.dispatches_per_token),
            _ => None,
        }
    }

    /// Exact candidate staging copy size for one resident greedy token.
    pub fn resident_readback_bytes_per_token(&self) -> Option<u32> {
        match &self.resident_decode {
            ResidentDecodeState::Ready(plan) => Some(plan.readback_bytes_per_token),
            _ => None,
        }
    }

    fn plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self.model_bytes().map(|m| m.as_ptr() as u64).unwrap_or(0);
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
            Some(ResidentTokenOutcome::Argmax(a)) => Some(StreamingArgmaxResult {
                best_token_id: a.best_token_id,
                max_logit: a.max_logit,
            }),
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
            ResidentDecodeState::Ineligible(k) if k == key => {
                self.resident_decode = ResidentDecodeState::Ineligible(k);
                return None;
            }
            _ => match self.build_resident_plan(index, key) {
                Some(p) => p,
                None => {
                    wlog("[resident-decode] plan build ineligible — legacy path");
                    self.resident_decode = ResidentDecodeState::Ineligible(key);
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
        etalon_inference::native_resident::ResidentEngine { host: self }
            .run_resident_token(plan, emb, token_idx, out_hidden)
    }

    /// Build the per-model plan: force weight residency, create activation
    /// buffers + static uniform slots + all bind groups. Any missing piece →
    /// `None` (the caller records Ineligible and the legacy path runs).
    fn build_resident_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<ResidentDecodePlan>> {
        etalon_inference::native_resident::planning::ResidentPlanner { host: self }
            .build_resident_plan(index, key)
    }
}
