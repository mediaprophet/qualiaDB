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
// Additive, default-OFF: routes the output projection through the GPU top-k path
// (keep logits on-GPU, read back K pairs) instead of the full-logit-readback argmax.
static GPU_TOPK: AtomicBool = AtomicBool::new(false);

/// Enable/disable the GPU top-k decode path (`QUALIA_LLM_GPU_TOPK`).
#[inline]
pub fn set_gpu_topk(on: bool) {
    GPU_TOPK.store(on, Ordering::Relaxed);
}

/// Whether the GPU top-k decode path is active (atomic flag OR the env var).
#[inline]
pub fn gpu_topk_enabled() -> bool {
    GPU_TOPK.load(Ordering::Relaxed)
        || matches!(
            std::env::var("QUALIA_LLM_GPU_TOPK").ok().as_deref(),
            Some("1") | Some("true")
        )
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
        gpu_timestamp_supported: false,
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
