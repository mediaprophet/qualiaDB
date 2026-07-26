//! Decode/prefill path-selection counters — record which internal path produced
//! each token (top-k vs argmax, resident vs legacy, speculative steps, sampled
//! tokens, resident-prefill hits). Each is a process-global atomic with
//! `record_*` / `*_counts` / `reset_*` helpers. Pure code motion — unchanged.

use std::sync::atomic::{AtomicU64, Ordering};

// ── Output-projection path counters (Codex P0: make the chosen path visible) ───────────────────
static TOPK_HITS: AtomicU64 = AtomicU64::new(0);
static ARGMAX_FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// Decode loop: the GPU top-k path produced the next token.
#[inline]
pub fn record_topk_hit() {
    TOPK_HITS.fetch_add(1, Ordering::Relaxed);
}
/// Decode loop: fell back to the full-logit-readback argmax path.
#[inline]
pub fn record_argmax_fallback() {
    ARGMAX_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}
/// (top-k hits, argmax fallbacks) since the last reset.
#[inline]
pub fn output_path_counts() -> (u64, u64) {
    (
        TOPK_HITS.load(Ordering::Relaxed),
        ARGMAX_FALLBACKS.load(Ordering::Relaxed),
    )
}
/// Reset the output-projection path counters.
#[inline]
pub fn reset_output_path_counts() {
    TOPK_HITS.store(0, Ordering::Relaxed);
    ARGMAX_FALLBACKS.store(0, Ordering::Relaxed);
}

// ── W1/W9: resident-token decode path counters ─────────────────────────────────
static RESIDENT_HITS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// Decode loop: the resident single-fence path produced this token.
#[inline]
pub fn record_resident_hit() {
    RESIDENT_HITS.fetch_add(1, Ordering::Relaxed);
}
/// Decode loop: resident path was enabled but ineligible/failed — legacy ran instead.
#[inline]
pub fn record_resident_fallback() {
    RESIDENT_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}
/// (resident hits, resident fallbacks) since the last reset.
#[inline]
pub fn resident_path_counts() -> (u64, u64) {
    (
        RESIDENT_HITS.load(Ordering::Relaxed),
        RESIDENT_FALLBACKS.load(Ordering::Relaxed),
    )
}
/// Reset the resident-path counters.
#[inline]
pub fn reset_resident_path_counts() {
    RESIDENT_HITS.store(0, Ordering::Relaxed);
    RESIDENT_FALLBACKS.store(0, Ordering::Relaxed);
}

// Native CUDA mega-pass execution counters.
//
// These are deliberately separate from inference mode and requested backend.
// A mode/env label is intent; only a successful mega-pass call proves that
// CUDA produced the token forward.
static CUDA_MEGA_HITS: AtomicU64 = AtomicU64::new(0);
static CUDA_MEGA_FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// Decode loop: the CUDA all-layer mega-pass completed this token forward.
#[inline]
pub fn record_cuda_mega_hit() {
    CUDA_MEGA_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Decode loop: CUDA mega-pass was explicitly requested but failed eligibility
/// or execution and the ordinary fallback path ran.
#[inline]
pub fn record_cuda_mega_fallback() {
    CUDA_MEGA_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

/// (successful CUDA mega-pass forwards, explicit CUDA fallbacks).
#[inline]
pub fn cuda_mega_path_counts() -> (u64, u64) {
    (
        CUDA_MEGA_HITS.load(Ordering::Relaxed),
        CUDA_MEGA_FALLBACKS.load(Ordering::Relaxed),
    )
}

/// Reset the native CUDA execution counters.
#[inline]
pub fn reset_cuda_mega_path_counts() {
    CUDA_MEGA_HITS.store(0, Ordering::Relaxed);
    CUDA_MEGA_FALLBACKS.store(0, Ordering::Relaxed);
}

// ── W2/W9: sampled-token counter ───────────────────────────────────────────────
static SAMPLED_TOKENS: AtomicU64 = AtomicU64::new(0);

/// Decode loop: the exact CPU sampler produced this token (non-greedy path).
#[inline]
pub fn record_sampled_token() {
    SAMPLED_TOKENS.fetch_add(1, Ordering::Relaxed);
}
/// Sampled tokens since the last reset.
#[inline]
pub fn sampled_token_count() -> u64 {
    SAMPLED_TOKENS.load(Ordering::Relaxed)
}
/// Reset the sampled-token counter.
#[inline]
pub fn reset_sampled_token_count() {
    SAMPLED_TOKENS.store(0, Ordering::Relaxed);
}

// ── W6a/W9: speculative-decode counters ────────────────────────────────────────
static SPEC_STEPS: AtomicU64 = AtomicU64::new(0);
static SPEC_DRAFTED: AtomicU64 = AtomicU64::new(0);
static SPEC_ACCEPTED: AtomicU64 = AtomicU64::new(0);

/// One speculative step ran (a draft was proposed + verified).
#[inline]
pub fn record_spec_step(drafted: u64, accepted: u64) {
    SPEC_STEPS.fetch_add(1, Ordering::Relaxed);
    SPEC_DRAFTED.fetch_add(drafted, Ordering::Relaxed);
    SPEC_ACCEPTED.fetch_add(accepted, Ordering::Relaxed);
}
/// (spec steps, tokens drafted, draft tokens accepted) since the last reset.
#[inline]
pub fn spec_decode_counts() -> (u64, u64, u64) {
    (
        SPEC_STEPS.load(Ordering::Relaxed),
        SPEC_DRAFTED.load(Ordering::Relaxed),
        SPEC_ACCEPTED.load(Ordering::Relaxed),
    )
}
/// Reset the speculative-decode counters.
#[inline]
pub fn reset_spec_decode_counts() {
    SPEC_STEPS.store(0, Ordering::Relaxed);
    SPEC_DRAFTED.store(0, Ordering::Relaxed);
    SPEC_ACCEPTED.store(0, Ordering::Relaxed);
}

// ── W3/W9: resident-prefill path counters ──────────────────────────────────────
static RESIDENT_PREFILL_HITS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_PREFILL_FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// Prefill: the resident single-fence-per-chunk arena populated this chunk's KV.
#[inline]
pub fn record_resident_prefill_hit() {
    RESIDENT_PREFILL_HITS.fetch_add(1, Ordering::Relaxed);
}
/// Prefill: resident path was enabled but ineligible/failed — legacy chunk ran instead.
#[inline]
pub fn record_resident_prefill_fallback() {
    RESIDENT_PREFILL_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}
/// (resident-prefill hits, resident-prefill fallbacks) since the last reset.
#[inline]
pub fn resident_prefill_counts() -> (u64, u64) {
    (
        RESIDENT_PREFILL_HITS.load(Ordering::Relaxed),
        RESIDENT_PREFILL_FALLBACKS.load(Ordering::Relaxed),
    )
}
/// Reset the resident-prefill path counters.
#[inline]
pub fn reset_resident_prefill_counts() {
    RESIDENT_PREFILL_HITS.store(0, Ordering::Relaxed);
    RESIDENT_PREFILL_FALLBACKS.store(0, Ordering::Relaxed);
}
