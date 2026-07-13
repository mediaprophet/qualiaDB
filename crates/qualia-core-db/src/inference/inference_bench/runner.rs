//! The public benchmark entry points — drive one case (cold then warm) through
//! the real inference path, average warm repeats, and assemble a [`BenchResult`];
//! plus the suite runners and their blocking (own-runtime) wrappers. Includes the
//! internal one-shot timing (`RunTiming` / `timed_infer`). Pure code motion.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::llm_agent::{AgentBackend, LocalLlmAgent};

use super::metrics::{ms, ns_to_ms, tok_per_s};
use super::*;

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
    let (_text, _prov, tokens, _quin) = agent.infer_local_model_streaming(prompt, "", Some(cb));
    let total = start.elapsed();
    let v = stamps.lock().map(|g| g.clone()).unwrap_or_default();
    let ttft = v.first().map(|f| f.duration_since(start)).unwrap_or(total);
    RunTiming {
        ttft,
        total,
        output_tokens: tokens as u64,
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
    if let Ok(report) = crate::resident_model::mount_resident_gguf(model_id, &cfg.model_path, false)
    {
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

    let shared_gpu = crate::gpu_context::shared_gpu();

    Ok(BenchResult {
        label: cfg.label.clone(),
        model_path: cfg.model_path.clone(),
        quantization: cfg.quantization.clone(),
        model: meta,
        gpu: BenchGpuMeta::from_shared_context(shared_gpu),
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
        gpu_timestamp_supported: shared_gpu.timestamps_supported,
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
