//! A0 — native LLM benchmark harness (STELLAR §A; decisions D17 + D22).
//!
//! This is the **shared** measurement surface for the performance push. It drives
//! the *real* inference path ([`LocalLlmAgent::infer_local_model_streaming`]) and
//! reads per-phase timing recorded *inside that same path* — so the existing
//! F16/Q8 path and the future ternary/top-k paths are measured by **one** harness
//! rather than a forked benchmark loop (D22 "shared-improvement" rule). A speedup
//! that shows up here is a real, attributable, end-to-end number, not a kernel
//! microbenchmark.
//!
//! What it reports (per model / weight policy):
//!   * **cold TTFT** — model not resident: wall-clock from call to first token
//!     (bundles mmap load + pipeline create + prefill + first decode);
//!   * **warm TTFT** — model resident (mmap adopted, pipelines still rebuilt per
//!     call in the current architecture — that cost is intentionally *included*,
//!     it is what A7 will attack);
//!   * **prefill / decode tok/s** from the internal phase split;
//!   * the **load / prefill / decode** wall-clock breakdown.
//!
//! Honest scope of *this* increment (A0.1): timings are **host wall-clock**.
//! GPU timestamp-query kernel isolation (D17) — requesting `TIMESTAMP_QUERY` on
//! the shared device and wrapping passes with `timestamp_writes` — is the A0.2
//! follow-on; [`BenchResult::gpu_timestamp_supported`] is `false` until then.
//!
//! Native-only: the WASM decode path is a different beast and is benchmarked in
//! the browser harness.
#![cfg(not(target_arch = "wasm32"))]

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::llm_agent::{AgentBackend, LocalLlmAgent};

// ── Decode budget override ────────────────────────────────────────────────────
// A bounded, fixed decode-token count gives stable, comparable tok/s. The real
// decode loop reads this once per call; 0 = use the production `DECODE_TOKEN_BUDGET`.

static DECODE_BUDGET_OVERRIDE: AtomicU32 = AtomicU32::new(0);

/// Set a fixed decode-token budget for benchmarking (0 = production default).
#[inline]
pub fn set_decode_budget_override(n: u32) {
    DECODE_BUDGET_OVERRIDE.store(n, Ordering::Relaxed);
}

/// Current decode-budget override (0 = none).
#[inline]
pub fn decode_budget_override() -> u32 {
    DECODE_BUDGET_OVERRIDE.load(Ordering::Relaxed)
}

// ── A1a GPU top-k toggle (D18) ────────────────────────────────────────────────
// Default-OFF — and the A/B (perf_topk_ab_smollm2) proves WHY flipping it on is NOT the win the
// Codex review implied: (1) dispatch_output_topk_chunked gates on the NARROW ggml_gpu_quant_supported
// (Q4_K/Q6_K only), so for Q8_0/F16/Q4_0 models top-k returns None and silently falls to argmax
// (topk_hits=0 measured); and (2) even fully engaged, the output projection is only ~6 of ~120
// submit→wait round-trips/token — the per-LAYER attention+FFN ops are the real ~110. So top-k is at
// best a ~5% lever; the win is layer-forward residency/fusion (cut waits/token), not this flag.
// Kept toggleable for when the gate+shader support the wider quant set. `QUALIA_LLM_GPU_TOPK=1/0`.
static GPU_TOPK: AtomicBool = AtomicBool::new(false);

/// Enable/disable the GPU top-k decode path (`QUALIA_LLM_GPU_TOPK`).
#[inline]
pub fn set_gpu_topk(on: bool) {
    GPU_TOPK.store(on, Ordering::Relaxed);
}

/// Whether the GPU top-k decode path is active. The env var overrides the flag in BOTH directions
/// (`0`/`false` → off, `1`/`true` → on); otherwise the process default (ON) applies.
#[inline]
pub fn gpu_topk_enabled() -> bool {
    match std::env::var("QUALIA_LLM_GPU_TOPK").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => GPU_TOPK.load(Ordering::Relaxed),
    }
}

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

// ── A1b ternary-FFN toggle (D3/D7) ────────────────────────────────────────────
// Additive, default-OFF: when a `.q42` ternary container is booted, routes its FFN
// GEMMs through the resident 2-bit GPU kernel (`TernaryFfnResident`). OFF runs the
// SAME ternary weights via the CPU oracle — so ON-vs-OFF isolates the GPU-kernel win
// on identical weights, and ternary-container-vs-Q8 (a0) is the headline FFN number.
static TERNARY_FFN: AtomicBool = AtomicBool::new(false);

/// Enable/disable the resident 2-bit GPU ternary-FFN path (`QUALIA_LLM_TERNARY_FFN`).
#[inline]
pub fn set_ternary_ffn(on: bool) {
    TERNARY_FFN.store(on, Ordering::Relaxed);
}

/// Whether the GPU ternary-FFN path is active (atomic flag OR the env var). When false, ternary
/// FFN GEMMs fall back to the CPU oracle (correct, slower) — the toggle's OFF baseline.
#[inline]
pub fn ternary_ffn_enabled() -> bool {
    TERNARY_FFN.load(Ordering::Relaxed)
        || matches!(
            std::env::var("QUALIA_LLM_TERNARY_FFN").ok().as_deref(),
            Some("1") | Some("true")
        )
}

// ── Phase 2: resident weights toggle ──────────────────────────────────────────
// Default ON (native). Each layer's q/k/v/o/gate/up/down weight is uploaded to its own resident
// VRAM buffer once (keyed by the GGUF tensor byte_offset) and reused every token, instead of
// re-`write_buffer`ing the (up to ~50 MB for a 3B FFN tensor) weight into the shared GEMM buffer
// on every GEMM, every token. For a 3B F16 model that re-upload is ~5 GB/token of PCIe traffic —
// the decode bottleneck. Set `QUALIA_LLM_RESIDENT_WEIGHTS=0` to force the per-token re-upload (the
// A/B OFF baseline) — useful for measuring the win or on VRAM-constrained GPUs.
static RESIDENT_WEIGHTS: AtomicBool = AtomicBool::new(true);

/// Enable/disable the resident per-tensor weight buffers (`QUALIA_LLM_RESIDENT_WEIGHTS`).
#[inline]
pub fn set_resident_weights(on: bool) {
    RESIDENT_WEIGHTS.store(on, Ordering::Relaxed);
}

/// Whether native GEMM should bind resident per-tensor weight buffers (upload-once) rather than
/// re-uploading the weight every token. Env forces either direction; otherwise the atomic flag.
#[inline]
pub fn resident_weights_enabled() -> bool {
    match std::env::var("QUALIA_LLM_RESIDENT_WEIGHTS").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => RESIDENT_WEIGHTS.load(Ordering::Relaxed),
    }
}

// ── #48 correctness path: CPU attention reference ─────────────────────────────
// Route native attention through the wasm-proven CPU SDPA (`cpu_attention_pass`) instead of the
// GPU attention shader (whose output is currently unbounded). Correct-but-slower; opt-in.
static CPU_ATTENTION: AtomicBool = AtomicBool::new(false);

/// Enable/disable the native CPU-attention reference path (`QUALIA_LLM_CPU_ATTENTION`).
#[inline]
pub fn set_cpu_attention(on: bool) {
    CPU_ATTENTION.store(on, Ordering::Relaxed);
}

/// Whether native attention should use the CPU reference.
///
/// **Default OFF** (use the GPU attention path) — as of #49 the GPU path also honors `norm_weight`
/// for prefill K/V and produces coherent output, and it is faster. The CPU SDPA reference remains
/// available as a correctness fallback / cross-check via `QUALIA_LLM_CPU_ATTENTION=1` or
/// [`set_cpu_attention`].
#[inline]
pub fn cpu_attention_enabled() -> bool {
    CPU_ATTENTION.load(Ordering::Relaxed)
        || matches!(
            std::env::var("QUALIA_LLM_CPU_ATTENTION").ok().as_deref(),
            Some("1") | Some("true")
        )
}

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

// ── Config / result ───────────────────────────────────────────────────────────

/// One benchmark case: a model + prompt to drive through the real path.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    /// Human-readable row label, e.g. "SmolLM2-360M Q8".
    pub label: String,
    /// Path to the GGUF on disk.
    pub model_path: String,
    /// Descriptive quantization tag for the report (e.g. "Q8_0").
    pub quantization: String,
    /// Prompt to run.
    pub prompt: String,
    /// Fixed decode-token count for a bounded, comparable measurement (0 = production default).
    pub decode_tokens: u32,
    /// Warm repeats to average over (≥1). Cold is always a single fresh run.
    pub warm_repeats: u32,
}

impl BenchConfig {
    pub fn new(
        label: impl Into<String>,
        model_path: impl Into<String>,
        quantization: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            model_path: model_path.into(),
            quantization: quantization.into(),
            prompt: prompt.into(),
            decode_tokens: 64,
            warm_repeats: 3,
        }
    }
}

/// Model metadata captured at residency mount (best-effort).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModelMeta {
    pub n_layer: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub mapped_bytes: u64,
    pub kv_cache_bytes: u64,
    pub directml_enabled: bool,
}

/// A single benchmark row — JSON/CSV serializable.
#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub label: String,
    pub model_path: String,
    pub quantization: String,
    pub model: ModelMeta,

    pub prompt_tokens: u64,
    pub output_tokens: u64,

    // Cold: model not resident (includes disk mmap + pipeline build + prefill).
    pub cold_ttft_ms: f64,
    pub cold_total_ms: f64,

    // Warm: model resident (mmap adopted; pipelines rebuilt per call by design).
    pub warm_ttft_ms: f64,
    pub warm_total_ms: f64,

    // Phase split from internal metrics (averaged over warm repeats).
    pub load_ms: f64,
    pub prefill_ms: f64,
    pub prefill_tok_s: f64,
    pub decode_ms: f64,
    pub decode_tok_s: f64,

    /// Whether GPU timestamp-query kernel isolation contributed to these numbers.
    /// `false` for A0.1 (wall-clock); set when A0.2 lands.
    pub gpu_timestamp_supported: bool,
    pub note: String,
}

// ── Internal one-shot timing ──────────────────────────────────────────────────

struct RunTiming {
    ttft: Duration,
    total: Duration,
    output_tokens: u64,
}

/// Drive one real inference, timestamping the first/last streamed token.
fn timed_infer(agent: &LocalLlmAgent, prompt: &str) -> RunTiming {
    let stamps: Arc<Mutex<Vec<Instant>>> = Arc::new(Mutex::new(Vec::with_capacity(64)));
    let stamps_cb = Arc::clone(&stamps);
    let start = Instant::now();
    let cb = move |_delta: String| {
        if let Ok(mut v) = stamps_cb.lock() {
            v.push(Instant::now());
        }
    };
    let (_text, _prov, tokens, _quin) =
        agent.infer_local_model_streaming(prompt, "", Some(cb));
    let total = start.elapsed();
    let v = stamps.lock().map(|g| g.clone()).unwrap_or_default();
    let ttft = v.first().map(|f| f.duration_since(start)).unwrap_or(total);
    RunTiming {
        ttft,
        total,
        output_tokens: tokens as u64,
    }
}

#[inline]
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[inline]
fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

#[inline]
fn tok_per_s(tokens: u64, ns: u64) -> f64 {
    if ns == 0 || tokens == 0 {
        0.0
    } else {
        tokens as f64 / (ns as f64 / 1_000_000_000.0)
    }
}

// ── Public entry ──────────────────────────────────────────────────────────────

/// Run one benchmark case end-to-end (cold then warm) against the real path.
///
/// Must be called from within a multi-thread Tokio runtime context (the residency
/// mount uses `block_in_place`). Returns `Err` if the model file is absent.
pub fn run_bench(cfg: &BenchConfig) -> Result<BenchResult, String> {
    if !std::path::Path::new(&cfg.model_path).exists() {
        return Err(format!("model not found: {}", cfg.model_path));
    }

    let agent = LocalLlmAgent::with_local_backend(
        "did:qualia:bench",
        AgentBackend::Local {
            model_path: cfg.model_path.clone(),
            context_window: 4096,
            quantization: cfg.quantization.clone(),
            vision_projector_path: None,
            modality: "text".into(),
            architecture: None,
        },
    );

    // Bound decode for a stable, comparable measurement.
    set_decode_budget_override(cfg.decode_tokens);

    // ── COLD: ensure the model is NOT resident, then measure a fresh run. ──
    crate::resident_model::clear_resident_model();
    reset_phase_metrics();
    let cold = timed_infer(&agent, &cfg.prompt);

    // ── Make resident so warm runs adopt the mmap (skip disk load). ──
    let model_id = crate::q_hash(&cfg.model_path);
    let mut meta = ModelMeta::default();
    if let Ok(report) = crate::resident_model::mount_resident_gguf(model_id, &cfg.model_path) {
        meta = ModelMeta {
            n_layer: report.n_layer,
            n_head: report.n_head,
            n_kv_head: report.n_kv_head,
            mapped_bytes: report.mapped_bytes,
            kv_cache_bytes: report.kv_cache_bytes,
            directml_enabled: report.directml_enabled,
        };
    }

    // ── WARM: average over repeats. ──
    let repeats = cfg.warm_repeats.max(1);
    let mut warm_ttft = Duration::ZERO;
    let mut warm_total = Duration::ZERO;
    let mut acc_load = 0.0f64;
    let mut acc_prefill_ns = 0u64;
    let mut acc_prefill_tok = 0u64;
    let mut acc_decode_ns = 0u64;
    let mut acc_decode_tok = 0u64;
    let mut last_warm = RunTiming {
        ttft: Duration::ZERO,
        total: Duration::ZERO,
        output_tokens: 0,
    };
    for _ in 0..repeats {
        reset_phase_metrics();
        let w = timed_infer(&agent, &cfg.prompt);
        let snap = phase_snapshot();
        warm_ttft += w.ttft;
        warm_total += w.total;
        acc_load += ns_to_ms(snap.load_ns);
        acc_prefill_ns += snap.prefill_ns;
        acc_prefill_tok += snap.prefill_tokens;
        acc_decode_ns += snap.decode_ns;
        acc_decode_tok += snap.decode_tokens;
        last_warm = w;
    }
    let n = repeats as u32;

    crate::resident_model::clear_resident_model();
    set_decode_budget_override(0); // restore production default

    let prompt_tokens = if repeats > 0 {
        acc_prefill_tok / repeats as u64 + 1 // prefill covers prompt_len-1
    } else {
        0
    };

    Ok(BenchResult {
        label: cfg.label.clone(),
        model_path: cfg.model_path.clone(),
        quantization: cfg.quantization.clone(),
        model: meta,
        prompt_tokens,
        output_tokens: last_warm.output_tokens,
        cold_ttft_ms: ms(cold.ttft),
        cold_total_ms: ms(cold.total),
        warm_ttft_ms: ms(warm_ttft) / n as f64,
        warm_total_ms: ms(warm_total) / n as f64,
        load_ms: acc_load / n as f64,
        prefill_ms: ns_to_ms(acc_prefill_ns) / n as f64,
        prefill_tok_s: tok_per_s(acc_prefill_tok, acc_prefill_ns),
        decode_ms: ns_to_ms(acc_decode_ns) / n as f64,
        decode_tok_s: tok_per_s(acc_decode_tok, acc_decode_ns),
        // W2/D17: report the real device capability (TIMESTAMP_QUERY negotiation), not a hardcoded
        // false. Per-kernel µs come from the dedicated `w2_gpu_phase_profile` test (a profiled run
        // perturbs the headline tok/s, so the baseline run is left unprofiled).
        gpu_timestamp_supported: crate::gpu_context::shared_gpu().timestamps_supported,
        note: String::new(),
    })
}

/// Run a suite of cases, skipping any whose model file is absent.
pub fn run_suite(cfgs: &[BenchConfig]) -> Vec<BenchResult> {
    let mut out = Vec::new();
    for c in cfgs {
        match run_bench(c) {
            Ok(r) => out.push(r),
            Err(e) => log::warn!("llm_bench|skip|{}|{}", c.label, e),
        }
    }
    out
}

/// Run a suite inside a fresh multi-thread Tokio runtime.
///
/// `mount_resident_gguf` uses `block_in_place`, which requires a multi-thread
/// runtime context — this wrapper provides one so callers (tests, CLI) don't have
/// to. Safe to call from a plain (non-async) thread.
pub fn run_suite_blocking(cfgs: &[BenchConfig]) -> Vec<BenchResult> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio multi-thread runtime for llm_bench");
    rt.block_on(async { run_suite(cfgs) })
}

/// A1a correctness: decode the same prompt with the GPU top-k path **off** then **on** (same resident
/// model, deterministic argmax) and return both strings. Since k=1 top-k == argmax, the texts must be
/// byte-identical — this verifies the GEMM→top-k wiring, not just the kernel (which is oracle-tested).
pub fn compare_topk_decode(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, String), String> {
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("model not found: {model_path}"));
    }
    let agent = LocalLlmAgent::with_local_backend(
        "did:qualia:bench",
        AgentBackend::Local {
            model_path: model_path.to_string(),
            context_window: 4096,
            quantization: "auto".into(),
            vision_projector_path: None,
            modality: "text".into(),
            architecture: None,
        },
    );
    set_decode_budget_override(decode_tokens);
    let model_id = crate::q_hash(model_path);
    let _ = crate::resident_model::mount_resident_gguf(model_id, model_path);

    set_gpu_topk(false);
    let (off_text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);
    set_gpu_topk(true);
    let (on_text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);

    set_gpu_topk(false);
    set_decode_budget_override(0);
    crate::resident_model::clear_resident_model();
    Ok((off_text, on_text))
}

/// `compare_topk_decode` inside a fresh multi-thread runtime (residency mount needs `block_in_place`).
pub fn compare_topk_decode_blocking(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, String), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async { compare_topk_decode(model_path, prompt, decode_tokens) })
}

/// A1b: mount a model (auto-detecting `Q42W` vs GGUF by magic) and run ONE decode of `prompt` for
/// `decode_tokens`, returning `(text, decode_tok_s)`. For a ternary `.q42` the FFN routing follows
/// the global `set_ternary_ffn` toggle, so a caller can measure GPU-ON vs CPU-OFF on identical
/// weights. Caller sets the toggle before invoking. (Use the `_blocking` wrapper from sync code.)
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_with_metrics(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, f64), String> {
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("model not found: {model_path}"));
    }
    let is_q42 = {
        use std::io::Read;
        let mut buf = [0u8; 4];
        std::fs::File::open(model_path)
            .and_then(|mut f| f.read_exact(&mut buf))
            .map(|_| &buf == b"Q42W")
            .unwrap_or(false)
    };
    let agent = LocalLlmAgent::with_local_backend(
        "did:qualia:bench",
        AgentBackend::Local {
            model_path: model_path.to_string(),
            context_window: 4096,
            quantization: "auto".into(),
            vision_projector_path: None,
            modality: "text".into(),
            architecture: None,
        },
    );
    set_decode_budget_override(decode_tokens);
    let model_id = crate::q_hash(model_path);
    if is_q42 {
        crate::resident_model::mount_resident_q42(model_id, model_path)?;
    } else {
        let _ = crate::resident_model::mount_resident_gguf(model_id, model_path);
    }
    reset_phase_metrics();
    let (text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);
    let snap = phase_snapshot();
    let decode_tok_s = tok_per_s(snap.decode_tokens, snap.decode_ns);
    set_decode_budget_override(0);
    crate::resident_model::clear_resident_model();
    Ok((text, decode_tok_s))
}

/// W3 — GPU↔CPU GEMM parity probe (test/diagnostic). Builds a fresh engine, synthesizes a random
/// Q8_0 weight matrix (`n_out` rows × `n_in`; `n_in` must be a multiple of 32) + input from `seed`,
/// runs the GPU kernel and the CPU reference on **identical** bytes, and returns
/// `(max_abs_err, mean_abs_err, max_ulp, gpu_gemm_passes_profiled)`. A non-zero pass count proves the
/// GPU path actually executed — the engine readback falls back to CPU when no tokio handle is present,
/// so the `rt.enter()` below installs one to force the real GPU path.
#[cfg(not(target_arch = "wasm32"))]
pub fn gemm_parity_probe_blocking(
    n_in: usize,
    n_out: usize,
    seed: u64,
) -> Result<(f32, f64, u64, u64), String> {
    use crate::gguf_sharder::GgufTensorInfo;
    if n_in == 0 || n_out == 0 || n_in % 32 != 0 {
        return Err("n_in must be a non-zero multiple of 32 (Q8_0 block size); n_out > 0".into());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let mut engine = rt
        .block_on(crate::gguf_bridge::QTensorEngine::try_new())
        .map_err(|e| format!("engine init: {e}"))?;
    let _guard = rt.enter(); // install a tokio handle on this thread so the GPU readback path runs

    // Deterministic LCG → values in [-1, 1).
    let mut s = seed | 1;
    let mut rng = move || -> f32 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    };

    let row_bytes = crate::llm_kernel_parity::q8_0_bytes(n_in);
    let mut raw = vec![0u8; row_bytes * n_out];
    let mut row_f32 = vec![0f32; n_in];
    for r in 0..n_out {
        for x in row_f32.iter_mut() {
            *x = rng();
        }
        if !crate::llm_kernel_parity::quantize_q8_0_from_f32(
            &row_f32,
            &mut raw[r * row_bytes..(r + 1) * row_bytes],
        ) {
            return Err("q8_0 quantize failed".into());
        }
    }
    let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();

    let info = GgufTensorInfo {
        dims: [n_in as u64, n_out as u64, 1, 1],
        n_dims: 2,
        ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
        byte_offset: 0,
    };

    crate::llm_gpu_profiler::set_enabled(true);
    crate::llm_gpu_profiler::reset();
    let mut gpu_out = vec![0f32; n_out];
    let mut cpu_out = vec![0f32; n_out];
    let ok = engine.gemm_parity_probe(&info, &raw, &input, &mut gpu_out, &mut cpu_out, n_in, n_out);
    let calls = crate::llm_gpu_profiler::snapshot()
        .iter()
        .find(|t| matches!(t.phase, crate::llm_gpu_profiler::Phase::Gemm))
        .map(|t| t.calls)
        .unwrap_or(0);
    crate::llm_gpu_profiler::set_enabled(false);
    if !ok {
        return Err("gemm_parity_probe: GPU or CPU path returned false".into());
    }
    Ok((
        crate::llm_kernel_parity::max_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::mean_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::max_ulp_diff(&gpu_out, &cpu_out),
        calls,
    ))
}

/// W3/F16 — GPU↔CPU parity for the new **F16** GEMM path (`unpack2x16float` in the shader vs the CPU
/// `dequant_f16` reference). Synthesizes a random F16 weight matrix (`n_out` rows × `n_in`; no block
/// constraint) + input from `seed`, runs both on identical bytes, returns
/// `(max_abs_err, mean_abs_err, max_ulp, gpu_gemm_passes)`. Same witness rule as the Q8 probe.
#[cfg(not(target_arch = "wasm32"))]
pub fn gemm_parity_probe_f16_blocking(
    n_in: usize,
    n_out: usize,
    seed: u64,
) -> Result<(f32, f64, u64, u64), String> {
    use crate::gguf_sharder::GgufTensorInfo;
    if n_in == 0 || n_out == 0 {
        return Err("n_in and n_out must be > 0".into());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let mut engine = rt
        .block_on(crate::gguf_bridge::QTensorEngine::try_new())
        .map_err(|e| format!("engine init: {e}"))?;
    let _guard = rt.enter();

    let mut s = seed | 1;
    let mut rng = move || -> f32 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    };

    let row_bytes = crate::llm_kernel_parity::f16_bytes(n_in);
    let mut raw = vec![0u8; row_bytes * n_out];
    let mut row_f32 = vec![0f32; n_in];
    for r in 0..n_out {
        for x in row_f32.iter_mut() {
            *x = rng();
        }
        if !crate::llm_kernel_parity::quantize_f16_from_f32(
            &row_f32,
            &mut raw[r * row_bytes..(r + 1) * row_bytes],
        ) {
            return Err("f16 quantize failed".into());
        }
    }
    let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();

    let info = GgufTensorInfo {
        dims: [n_in as u64, n_out as u64, 1, 1],
        n_dims: 2,
        ggml_type: crate::ggml_quants::GGML_TYPE_F16,
        byte_offset: 0,
    };

    crate::llm_gpu_profiler::set_enabled(true);
    crate::llm_gpu_profiler::reset();
    let mut gpu_out = vec![0f32; n_out];
    let mut cpu_out = vec![0f32; n_out];
    let ok = engine.gemm_parity_probe(&info, &raw, &input, &mut gpu_out, &mut cpu_out, n_in, n_out);
    let calls = crate::llm_gpu_profiler::snapshot()
        .iter()
        .find(|t| matches!(t.phase, crate::llm_gpu_profiler::Phase::Gemm))
        .map(|t| t.calls)
        .unwrap_or(0);
    crate::llm_gpu_profiler::set_enabled(false);
    if !ok {
        return Err("gemm_parity_probe (f16): GPU or CPU path returned false".into());
    }
    Ok((
        crate::llm_kernel_parity::max_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::mean_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::max_ulp_diff(&gpu_out, &cpu_out),
        calls,
    ))
}

/// W1 — teacher-forced perplexity of `model_path` over the eval corpus, run through Qualia's **native**
/// engine (never an external runtime). For each corpus passage: `reset_kv_cache`, then per position
/// embed → `dispatch_transformer_forward` → `apply_output_norm_inplace` → `dispatch_output_logits_into`
/// → NLL of the true next token; PPL = `exp(ΣNLL / Σtokens)`. `max_tok` = 0 scores the whole passage,
/// >0 caps it (to bound the slow F16-on-CPU path for big models). Returns `(perplexity, tokens_scored)`.
/// Runs on a dedicated thread with a current-thread tokio runtime (mirrors the decode path) so the
/// engine's GPU readback works. Handles both GGUF and `.q42` containers.
#[cfg(not(target_arch = "wasm32"))]
pub fn perplexity_eval_blocking(model_path: &str, max_tok: usize) -> Result<(f64, usize), String> {
    use crate::gguf_bridge::QTensorEngine;
    use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

    let corpus = crate::llm_eval::load_corpus().map_err(|e| format!("corpus load: {e}"))?;
    if corpus.is_empty() {
        return Err("eval corpus is empty".into());
    }
    let model_path = model_path.to_string();

    std::thread::spawn(move || -> Result<(f64, usize), String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let _g = rt.enter();

        let mut engine = QTensorEngine::new();
        let mut magic = [0u8; 4];
        let is_q42 = {
            use std::io::Read;
            std::fs::File::open(&model_path)
                .and_then(|mut f| f.read_exact(&mut magic))
                .map(|_| &magic == b"Q42W")
                .unwrap_or(false)
        };
        if is_q42 {
            let f = std::fs::File::open(&model_path).map_err(|e| e.to_string())?;
            let mmap = unsafe { memmap2::Mmap::map(&f) }.map_err(|e| e.to_string())?;
            engine
                .adopt_resident_q42_mmap(std::sync::Arc::new(mmap))
                .map_err(|e| format!("q42 adopt: {e}"))?;
        } else {
            engine.load_gguf(&model_path);
        }

        let mmap = engine
            .gguf_mmap
            .clone()
            .ok_or_else(|| "model did not memory-map (load failed)".to_string())?;
        let is_q42_mmap = mmap.len() >= 4 && mmap[0..4] == *b"Q42W";
        let tok = if is_q42_mmap {
            crate::q42_weight::Q42TensorIndex::from_q42(&mmap)
                .ok()
                .and_then(|qi| GgufTokenizer::from_q42_section(qi.tokenizer_bytes(&mmap)))
                .unwrap_or_default()
        } else {
            GgufTokenizer::from_gguf(&mmap)
        };
        let tensor_idx = if is_q42_mmap {
            crate::q42_weight::Q42TensorIndex::from_q42(&mmap)
                .map(|qi| qi.to_gguf_index())
                .map_err(|e| format!("q42 index: {e}"))?
        } else {
            GgufTensorIndex::from_gguf(&mmap)
        };

        let emb_dim = tensor_idx.emb_dim();
        if emb_dim == 0 {
            return Err("embedding dimension is 0 (tensor index parse failed)".into());
        }
        let vocab = tok.vocab_len().max(1) as usize;

        let mut emb_buf = vec![0f32; emb_dim.max(8192)];
        let mut scratch_a = vec![0f32; 16384];
        let mut scratch_b = vec![0f32; 16384];
        let mut logits = vec![0f32; vocab];
        let mmap_bytes: &[u8] = &mmap;

        let mut total_nll = 0.0f64;
        let mut total_tok = 0usize;
        for passage in &corpus {
            let toks = tok.encode(passage);
            if toks.len() < 2 {
                continue;
            }
            let limit = if max_tok > 0 {
                (max_tok + 1).min(toks.len())
            } else {
                toks.len()
            };
            engine.reset_kv_cache();
            for i in 0..limit - 1 {
                let n_emb = tensor_idx.dequantize_token_embedding_into(
                    mmap_bytes,
                    toks[i],
                    &mut emb_buf[..emb_dim],
                );
                if n_emb == 0 {
                    return Err(format!("embedding lookup failed for token {}", toks[i]));
                }
                // AWQ calibration: reset the per-forward layer cursor so the FFN hook tags layers
                // 0..n_layer-1 correctly (no-op when AWQ capture is off).
                crate::llm_awq::begin_forward();
                let _ = engine.dispatch_transformer_forward(
                    &tensor_idx,
                    &mut emb_buf[..emb_dim],
                    emb_dim,
                    &mut scratch_a,
                    &mut scratch_b,
                    i as u32,
                    0, // 0 = all layers (full model depth)
                );
                let _ = engine.apply_output_norm_inplace(&tensor_idx, &mut emb_buf[..emb_dim], emb_dim);
                let n = engine.dispatch_output_logits_into(
                    &tensor_idx,
                    &emb_buf[..emb_dim],
                    emb_dim,
                    &mut logits,
                );
                if n == 0 {
                    return Err("output projection produced no logits".into());
                }
                let nll = crate::llm_eval::token_nll(&logits[..n], toks[i + 1] as usize);
                if nll.is_finite() {
                    total_nll += nll;
                    total_tok += 1;
                }
            }
        }
        if total_tok == 0 {
            return Err("no tokens scored".into());
        }
        Ok((crate::llm_eval::perplexity(total_nll, total_tok), total_tok))
    })
    .join()
    .map_err(|_| "perplexity eval thread panicked".to_string())?
}

/// AWQ α-sweep on the ternary FFN (AWQ steps 1–3 end to end): capture activation salience from the Q8
/// reference at `gguf_path`, then for each α compile an AWQ-scaled ternary `.q42`
/// (`compile_gguf_to_q42_ternary_ffn_awq`), evaluate its perplexity + unique-word coherence, and return
/// `(reference_ppl, [(alpha, ppl, uniq)])`. α=0.0 is plain ternary (the baseline). `max_tok` caps
/// tokens/passage to bound the sweep. Honest: this measures whether AWQ rescues ternary — it does not
/// assume it does. Needs a GPU.
#[cfg(not(target_arch = "wasm32"))]
pub fn awq_sweep_blocking(
    gguf_path: &str,
    alphas: &[f32],
    max_tok: usize,
    quant: crate::q42_weight::FfnQuant,
) -> Result<(f64, Vec<(f32, f64, f64)>), String> {
    use crate::q42_weight::compile_gguf_to_q42_ffn_quant_awq;

    let bytes = std::fs::read(gguf_path).map_err(|e| format!("read gguf: {e}"))?;
    let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&bytes);
    let n_layer = idx.hyperparams.n_layer;
    let n_embd = idx.hyperparams.n_embd;
    if n_layer == 0 || n_embd == 0 {
        return Err("gguf parse failed (n_layer/n_embd = 0)".into());
    }

    // 1. Capture per-channel salience + the Q8 reference PPL in one calibration forward.
    set_ternary_ffn(false);
    crate::llm_awq::enable(n_layer, n_embd)?;
    let (ref_ppl, _) = perplexity_eval_blocking(gguf_path, max_tok)?;
    let stats = crate::llm_awq::snapshot();
    crate::llm_awq::disable();
    if stats.is_empty() {
        return Err("AWQ: no activation stats captured".into());
    }

    // 2. Sweep: AWQ-scaled .q42 per α → eval PPL + coherence. Ternary needs the resident 2-bit path;
    //    Q4_0 runs through the standard quantized GEMM (no ternary toggle).
    set_ternary_ffn(matches!(quant, crate::q42_weight::FfnQuant::Ternary));
    let tmp = std::env::temp_dir();
    let mut results = Vec::with_capacity(alphas.len());
    for &alpha in alphas {
        let scales = if alpha == 0.0 { None } else { Some(stats.as_slice()) };
        let q42 = compile_gguf_to_q42_ffn_quant_awq(&bytes, 14, scales, alpha, quant)
            .map_err(|e| format!("AWQ compile (alpha={alpha}): {e}"))?;
        let path = tmp.join(format!("awq_sweep_a{:.2}.q42", alpha));
        std::fs::write(&path, &q42).map_err(|e| format!("write q42: {e}"))?;
        let ps = path.to_string_lossy().to_string();
        let (ppl, _) = perplexity_eval_blocking(&ps, max_tok)?;
        let (text, _) = decode_with_metrics_blocking(&ps, "Once upon a time, there was a", 24)?;
        let uniq = crate::llm_eval::unique_word_ratio(&text);
        let _ = std::fs::remove_file(&path);
        results.push((alpha, ppl, uniq));
    }
    set_ternary_ffn(false);
    Ok((ref_ppl, results))
}

/// `decode_with_metrics` inside a fresh multi-thread runtime (residency mount needs `block_in_place`).
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_with_metrics_blocking(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, f64), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async { decode_with_metrics(model_path, prompt, decode_tokens) })
}

// ── Reporting ─────────────────────────────────────────────────────────────────

/// Pretty-printed JSON for a result set.
pub fn results_to_json(results: &[BenchResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

/// CSV (header + one row per result).
pub fn results_to_csv(results: &[BenchResult]) -> String {
    let mut s = String::new();
    s.push_str(
        "label,quantization,n_layer,mapped_bytes,prompt_tokens,output_tokens,\
cold_ttft_ms,cold_total_ms,warm_ttft_ms,warm_total_ms,\
load_ms,prefill_ms,prefill_tok_s,decode_ms,decode_tok_s,directml,gpu_ts\n",
    );
    for r in results {
        s.push_str(&format!(
            "{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.2},{:.3},{:.2},{},{}\n",
            r.label.replace(',', " "),
            r.quantization,
            r.model.n_layer,
            r.model.mapped_bytes,
            r.prompt_tokens,
            r.output_tokens,
            r.cold_ttft_ms,
            r.cold_total_ms,
            r.warm_ttft_ms,
            r.warm_total_ms,
            r.load_ms,
            r.prefill_ms,
            r.prefill_tok_s,
            r.decode_ms,
            r.decode_tok_s,
            r.model.directml_enabled,
            r.gpu_timestamp_supported,
        ));
    }
    s
}

/// Human-readable table for stdout.
pub fn results_to_table(results: &[BenchResult]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{:<22} {:>6} {:>10} {:>10} {:>10} {:>11} {:>11}\n",
        "model", "layers", "coldTTFT", "warmTTFT", "prefill/s", "decode/s", "decode_ms"
    ));
    s.push_str(&"-".repeat(86));
    s.push('\n');
    for r in results {
        s.push_str(&format!(
            "{:<22} {:>6} {:>9.0}m {:>9.0}m {:>10.1} {:>11.2} {:>10.1}\n",
            r.label,
            r.model.n_layer,
            r.cold_ttft_ms,
            r.warm_ttft_ms,
            r.prefill_tok_s,
            r.decode_tok_s,
            r.decode_ms,
        ));
    }
    s
}
