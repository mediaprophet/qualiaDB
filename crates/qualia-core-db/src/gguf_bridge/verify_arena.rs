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
//! Etalon owns cold numerical plans and prepared execution; this adapter supplies
//! model access, eligibility and embeddings.

#![cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]

use super::*;
pub(crate) use etalon_inference::native_resident::verify_arena_plan::*;

impl QTensorEngine {
    fn verify_plan_key(&self, index: &crate::gguf_sharder::GgufTensorIndex) -> (u64, u64, u32) {
        let base = self.model_bytes().map(|m| m.as_ptr() as u64).unwrap_or(0);
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
            VerifyArenaState::Ineligible(k) if k == key => {
                self.verify_arena = VerifyArenaState::Ineligible(k);
                return None;
            }
            _ => match self.build_verify_plan(index, key) {
                Some(p) => p,
                None => {
                    self.verify_arena = VerifyArenaState::Ineligible(key);
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
        let n = plan.n_embd;
        let mmap = self.clone_model_bytes()?;
        if b.checked_mul(n)? > plan.embedding_scratch.len() {
            return None;
        }
        for (row, &token) in tokens.iter().enumerate() {
            if index.dequantize_token_embedding_into(
                &mmap,
                token,
                &mut plan.embedding_scratch[row * n..(row + 1) * n],
            ) == 0
            {
                return None;
            }
        }
        let mut ids = [0u32; VERIFY_BATCH_MAX];
        let mut logits = [0f32; VERIFY_BATCH_MAX];
        etalon_inference::native_resident::ResidentEngine { host: self }.run_verify_batch(
            plan,
            b,
            start_pos,
            &mut ids[..b],
            &mut logits[..b],
        )?;
        out_argmax.clear();
        out_logit.clear();
        out_argmax.extend_from_slice(&ids[..b]);
        out_logit.extend_from_slice(&logits[..b]);
        Some(())
    }

    fn build_verify_plan(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        key: (u64, u64, u32),
    ) -> Option<Box<VerifyArenaPlan>> {
        etalon_inference::native_resident::planning::ResidentPlanner { host: self }
            .build_verify_plan(index, key)
    }
}
