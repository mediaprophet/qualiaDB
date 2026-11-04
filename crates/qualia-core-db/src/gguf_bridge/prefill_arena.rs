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

#![cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]

use super::*;
pub(crate) use etalon_inference::native_resident::prefill_arena_plan::*;

impl QTensorEngine {
    fn prefill_plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self.model_bytes().map(|m| m.as_ptr() as u64).unwrap_or(0);
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
            PrefillArenaState::Ineligible(k) if k == key => {
                self.prefill_arena = PrefillArenaState::Ineligible(k);
                return None;
            }
            _ => match self.build_prefill_plan(index, key) {
                Some(p) => p,
                None => {
                    self.prefill_arena = PrefillArenaState::Ineligible(key);
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
        etalon_inference::native_resident::ResidentEngine { host: self }.run_prefill_chunk(
            plan,
            batch_hidden,
            emb_dim,
            n_tokens,
            batch_start_token_idx,
            max_layers,
        )
    }

    /// Build the per-model prefill plan: force weight residency, create batched
    /// activation buffers (`PREFILL_CHUNK_SIZE` rows), the uniform param arena, and
    /// all per-layer bind groups. Any missing piece → `None`.
    fn build_prefill_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<PrefillArenaPlan>> {
        etalon_inference::native_resident::planning::ResidentPlanner { host: self }
            .build_prefill_plan(index, key)
    }
}
