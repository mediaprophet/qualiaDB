// ─── Constants ──────────────────────────────────────────────────────────────
/// Hard memory ceiling for the LLM runtime within the 512MB system floor.
/// Leaves the remaining 384MB for the Webizen VM, SLG Arena, and WASM stack.
pub const LLM_MEMORY_BUDGET_BYTES: u64 = 128 * 1024 * 1024; // 128 MB

/// Maximum tokens the agent may generate in a single turn. Enforces deterministic
/// compute cost — no runaway generation that blocks the edge device.
pub const MAX_OUTPUT_TOKENS: u32 = 2048;

/// Token budget for the autoregressive loop (`MAX_OUTPUT_TOKENS` in release).
// NOTE: `pub(super)` widening (was a private module-level const) so the decode
// path in `decode.rs` can read it across the new submodule boundary.
#[cfg(test)]
pub(super) const DECODE_TOKEN_BUDGET: u32 = 16;
/// MC2b harness iteration: CPU SDPA decode is very slow in wasm; trim budget until Option B.
#[cfg(all(not(test), target_arch = "wasm32"))]
pub(super) const DECODE_TOKEN_BUDGET: u32 = 32;
// Codex P0: default per-turn decode cap. Was MAX_OUTPUT_TOKENS (2048) → at ~3 tok/s a no-EOS reply
// ran ~11 min and the app looked frozen. 256 keeps a turn bounded; MAX_OUTPUT_TOKENS stays the
// absolute ceiling and the cooperative deadline (INFERENCE_TIMEOUT_MS, checked INSIDE the decode
// loop) bounds wall-clock time independently.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(super) const DECODE_TOKEN_BUDGET: u32 = 256;

/// Layer cap for transformer forward during unit tests (full depth in release).
// NOTE: `pub(super)` widening — read by `decode.rs` and `decode_helpers.rs`.
#[cfg(test)]
pub(super) const TEST_TRANSFORMER_LAYER_CAP: u32 = 2;
#[cfg(not(test))]
pub(super) const TEST_TRANSFORMER_LAYER_CAP: u32 = 0;

/// Vocab chunk cap during unit tests (full sweep in release).
// NOTE: `pub(super)` widening — read by `decode.rs` and `decode_helpers.rs`.
#[cfg(test)]
pub(super) const TEST_VOCAB_CHUNK_CAP: u32 = 4;
#[cfg(not(test))]
pub(super) const TEST_VOCAB_CHUNK_CAP: u32 = 0;

/// Default maximum milliseconds for a local inference call (interactive).
/// Batch/overnight profile raises this via `llm_bench::inference_timeout_ms()`.
pub const INFERENCE_TIMEOUT_MS: u64 = 30_000;

/// Effective timeout: batch profile / env may extend (e.g. 8h overnight jobs).
// NOTE: `pub(super)` widening — read by `decode.rs` and `runtime.rs`.
#[inline]
pub(super) fn effective_inference_timeout_ms() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return crate::llm_bench::inference_timeout_ms();
    }
    #[cfg(target_arch = "wasm32")]
    {
        INFERENCE_TIMEOUT_MS
    }
}
