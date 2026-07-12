//! Shared per-phase timing metrics — written exactly once per phase by
//! `infer_local_model_inner` (NOT per token, so zero hot-path cost) and read by
//! the harness via [`phase_snapshot`]. Process-global because the decode loop
//! runs on a spawned engine thread. Also the small unit-conversion helpers used
//! by the runner and probes. Pure code motion — behaviour unchanged.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;

// ── Shared phase metrics ──────────────────────────────────────────────────────
// Written exactly once per phase by `infer_local_model_inner` (NOT per token, so
// there is zero hot-path cost), read by the harness via `phase_snapshot`. Process
// global because the decode loop runs on a spawned engine thread.

static LOAD_NS: AtomicU64 = AtomicU64::new(0);
static PREFILL_NS: AtomicU64 = AtomicU64::new(0);
static PREFILL_TOKENS: AtomicU64 = AtomicU64::new(0);
static DECODE_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_TOKENS: AtomicU64 = AtomicU64::new(0);
// Decode-profiler accumulators (summed ACROSS the decode loop; one atomic add per
// token-phase — nanosecond cost, off the GPU critical path). Localize where the
// per-token wall-clock goes: transformer forward (32 layers) vs output projection.
static DECODE_FORWARD_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_OUTPUT_NS: AtomicU64 = AtomicU64::new(0);
// One-shot empty submit→poll(Wait) baseline (total ns over N round-trips), measured once per
// profiled decode so the bench can compare per-round-trip fence latency to real forward time.
static DECODE_EMPTY_RT_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_EMPTY_RT_N: AtomicU64 = AtomicU64::new(0);

/// Record the empty-round-trip baseline: `total_ns` measured over `n` empty submit→wait cycles.
#[inline]
pub fn record_empty_rt(total_ns: u64, n: u64) {
    DECODE_EMPTY_RT_NS.store(total_ns, Ordering::Relaxed);
    DECODE_EMPTY_RT_N.store(n, Ordering::Relaxed);
}
/// Read the empty-round-trip baseline as `(total_ns, n)`.
#[inline]
pub fn empty_rt() -> (u64, u64) {
    (
        DECODE_EMPTY_RT_NS.load(Ordering::Relaxed),
        DECODE_EMPTY_RT_N.load(Ordering::Relaxed),
    )
}

/// Re-export: GPU `submit → poll(Wait)` round-trips counted during the last run (see `gguf_bridge`).
#[inline]
pub fn gpu_wait_count() -> u64 {
    crate::gguf_bridge::gpu_wait_count()
}

/// Accumulate one token's transformer-forward (32-layer) wall-clock.
#[inline]
pub fn add_decode_forward_ns(ns: u64) {
    DECODE_FORWARD_NS.fetch_add(ns, Ordering::Relaxed);
}
/// Accumulate one token's output-projection (argmax/top-k) wall-clock.
#[inline]
pub fn add_decode_output_ns(ns: u64) {
    DECODE_OUTPUT_NS.fetch_add(ns, Ordering::Relaxed);
}
/// Intra-layer split (summed over all 32 layers of one token): attention vs FFN — localizes which
/// shader (fused_attention vs the GEMM kernel) bleeds the compute-bound forward time.
static DECODE_ATTN_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_FFN_NS: AtomicU64 = AtomicU64::new(0);
/// Accumulate one layer's attention (QKV-proj + SDPA + O-proj) wall-clock.
#[inline]
pub fn add_decode_attn_ns(ns: u64) {
    DECODE_ATTN_NS.fetch_add(ns, Ordering::Relaxed);
}
/// Accumulate one layer's FFN (pre-norm SwiGLU) wall-clock.
#[inline]
pub fn add_decode_ffn_ns(ns: u64) {
    DECODE_FFN_NS.fetch_add(ns, Ordering::Relaxed);
}
/// Read the intra-layer attention/FFN accumulators as `(attn_ns, ffn_ns)` (summed over the run).
#[inline]
pub fn decode_attn_ffn() -> (u64, u64) {
    (
        DECODE_ATTN_NS.load(Ordering::Relaxed),
        DECODE_FFN_NS.load(Ordering::Relaxed),
    )
}

/// Clear the phase counters before a measured run.
#[inline]
pub fn reset_phase_metrics() {
    LOAD_NS.store(0, Ordering::Relaxed);
    PREFILL_NS.store(0, Ordering::Relaxed);
    PREFILL_TOKENS.store(0, Ordering::Relaxed);
    DECODE_NS.store(0, Ordering::Relaxed);
    DECODE_TOKENS.store(0, Ordering::Relaxed);
    DECODE_FORWARD_NS.store(0, Ordering::Relaxed);
    DECODE_OUTPUT_NS.store(0, Ordering::Relaxed);
    DECODE_ATTN_NS.store(0, Ordering::Relaxed);
    DECODE_FFN_NS.store(0, Ordering::Relaxed);
    DECODE_EMPTY_RT_NS.store(0, Ordering::Relaxed);
    DECODE_EMPTY_RT_N.store(0, Ordering::Relaxed);
    crate::gguf_bridge::reset_gpu_wait_count();
}

/// Record model load/mmap-adopt + pipeline-build time (engine ready → before prefill).
#[inline]
pub fn record_load_ns(ns: u64) {
    LOAD_NS.store(ns, Ordering::Relaxed);
}

/// Record prefill (prompt KV population) time + tokens prefilled.
#[inline]
pub fn record_prefill(ns: u64, tokens: u64) {
    PREFILL_NS.store(ns, Ordering::Relaxed);
    PREFILL_TOKENS.store(tokens, Ordering::Relaxed);
}

/// Record the autoregressive decode loop time + tokens generated.
#[inline]
pub fn record_decode(ns: u64, tokens: u64) {
    DECODE_NS.store(ns, Ordering::Relaxed);
    DECODE_TOKENS.store(tokens, Ordering::Relaxed);
}

/// Snapshot of the phase counters (last completed inference).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct LlmPhaseSnapshot {
    pub load_ns: u64,
    pub prefill_ns: u64,
    pub prefill_tokens: u64,
    pub decode_ns: u64,
    pub decode_tokens: u64,
    pub decode_forward_ns: u64,
    pub decode_output_ns: u64,
}

/// Read the phase counters recorded by the last inference call.
#[inline]
pub fn phase_snapshot() -> LlmPhaseSnapshot {
    LlmPhaseSnapshot {
        load_ns: LOAD_NS.load(Ordering::Relaxed),
        prefill_ns: PREFILL_NS.load(Ordering::Relaxed),
        prefill_tokens: PREFILL_TOKENS.load(Ordering::Relaxed),
        decode_ns: DECODE_NS.load(Ordering::Relaxed),
        decode_tokens: DECODE_TOKENS.load(Ordering::Relaxed),
        decode_forward_ns: DECODE_FORWARD_NS.load(Ordering::Relaxed),
        decode_output_ns: DECODE_OUTPUT_NS.load(Ordering::Relaxed),
    }
}

// ── Unit conversion helpers ───────────────────────────────────────────────────
// Shared by the runner (`run_bench`) and the probes (`decode_with_metrics`).
// Widened from module-private `fn` to `pub(super)` so sibling submodules reach
// them; not part of the crate-public surface.

#[inline]
pub(super) fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[inline]
pub(super) fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

#[inline]
pub(super) fn tok_per_s(tokens: u64, ns: u64) -> f64 {
    if ns == 0 || tokens == 0 {
        0.0
    } else {
        tokens as f64 / (ns as f64 / 1_000_000_000.0)
    }
}
