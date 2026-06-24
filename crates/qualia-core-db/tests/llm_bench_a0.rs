//! A0 (STELLAR §A) — native LLM benchmark baseline.
//!
//! Produces the missing trustworthy native SmolLM2-360M number (cold/warm TTFT,
//! prefill/decode tok/s) through the **real** inference path via the shared
//! `llm_bench` harness. Local-only: skips cleanly when the GGUF models are absent
//! (they are gitignored under `docs/models/`).
//!
//! Run:
//!   cargo test -p qualia-core-db --release --test llm_bench_a0 -- --nocapture
//!
//! Integration tests compile the library WITHOUT `cfg(test)`, so this runs at
//! FULL model depth (all layers, full vocab sweep) — a unit test would only run
//! the 2-layer `TEST_TRANSFORMER_LAYER_CAP` and report a meaningless number.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::llm_bench::{self, BenchConfig};
use std::path::{Path, PathBuf};

/// First existing path among the standard relative roots, else None.
fn find_model(name: &str) -> Option<PathBuf> {
    let candidates = [
        format!("../../docs/models/{name}"),
        format!("docs/models/{name}"),
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

fn results_dir() -> PathBuf {
    for root in ["../../benchmarks/results", "benchmarks/results"] {
        let p = PathBuf::from(root);
        if p.parent().map(|d| d.exists()).unwrap_or(false) {
            return p;
        }
    }
    PathBuf::from("benchmarks/results")
}

#[test]
fn a0_native_llm_baseline() {
    let prompt = "The capital of France is";
    let candidates = [
        ("SmolLM2-360M Q8", "smollm2-360m-instruct-q8_0.gguf", "Q8_0"),
        ("SmolLM2-360M Q4_K_M", "SmolLM2-360M-Instruct-Q4_K_M.gguf", "Q4_K_M"),
    ];

    let cfgs: Vec<BenchConfig> = candidates
        .iter()
        .filter_map(|(label, file, quant)| {
            find_model(file).map(|p| {
                let mut c = BenchConfig::new(*label, p.to_string_lossy(), *quant, prompt);
                c.decode_tokens = 64; // fixed count → stable, comparable tok/s
                c.warm_repeats = 3;
                c
            })
        })
        .collect();

    if cfgs.is_empty() {
        eprintln!("[a0] no SmolLM2 GGUF models under docs/models/ — skipping baseline");
        return;
    }

    let results = llm_bench::run_suite_blocking(&cfgs);
    assert!(
        !results.is_empty(),
        "[a0] harness ran but produced no results — every case failed to load (see warnings)"
    );

    // Console table + per-row detail.
    println!("\n=== A0 native LLM baseline (SmolLM2-360M, A2000) ===");
    println!("{}", llm_bench::results_to_table(&results));
    for r in &results {
        println!(
            "[a0] {:<22} layers={} prompt_tok={} out_tok={} \
cold_ttft={:.0}ms warm_ttft={:.0}ms load={:.0}ms prefill={:.1}t/s decode={:.2}t/s directml={}",
            r.label,
            r.model.n_layer,
            r.prompt_tokens,
            r.output_tokens,
            r.cold_ttft_ms,
            r.warm_ttft_ms,
            r.load_ms,
            r.prefill_tok_s,
            r.decode_tok_s,
            r.model.directml_enabled,
        );
        if r.decode_tok_s <= 0.0 || r.output_tokens == 0 {
            eprintln!(
                "[a0][WARN] {} produced no decoded tokens — native path did not generate \
(this is a real finding, not a perf number).",
                r.label
            );
        }
    }

    // Persist artifacts for external review.
    let dir = results_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json_path = dir.join("llm_a0_baseline.json");
    let csv_path = dir.join("llm_a0_baseline.csv");
    if std::fs::write(&json_path, llm_bench::results_to_json(&results)).is_ok() {
        println!("[a0] wrote {}", json_path.display());
    }
    if std::fs::write(&csv_path, llm_bench::results_to_csv(&results)).is_ok() {
        println!("[a0] wrote {}", csv_path.display());
    }
}

/// A1a correctness: GPU top-k (k=1) must decode byte-identical text to the argmax path. Skips if the
/// model is absent. Run: `cargo test -p qualia-core-db --release --test llm_bench_a0
/// a1a_gpu_topk_matches_argmax_text -- --nocapture`.
#[test]
fn a1a_gpu_topk_matches_argmax_text() {
    // Q4_K_M: attention weights are Q4_K, which the native GPU path supports (Q8_0 is not — separate gap).
    let Some(path) = find_model("SmolLM2-360M-Instruct-Q4_K_M.gguf")
        .or_else(|| find_model("smollm2-360m-instruct-q8_0.gguf"))
    else {
        eprintln!("[a1a] model absent — skipping token-identity check");
        return;
    };
    let model = path.to_string_lossy().to_string();
    // Unambiguous continuation prompt — a working forward must produce real words, not EOS/garbage.
    let prompt = "Once upon a time, there was a";
    let (off, on) = llm_bench::compare_topk_decode_blocking(&model, prompt, 24)
        .expect("compare_topk_decode");
    println!("[a1a] argmax: {off:?}");
    println!("[a1a] topk  : {on:?}");
    assert_eq!(
        off, on,
        "GPU top-k (k=1) must emit identical text to the argmax path"
    );
    // #48 regression guard: native decode must produce coherent text, not EOS/garbage spam.
    assert!(
        !off.trim_start().starts_with("<|endoftext|>") && off.contains(' ') && off.len() > 8,
        "native decode must produce coherent text (regression of #48), got: {off:?}"
    );
    println!("[a1a] token-identity verified + coherent generation: top-k == argmax");
}

/// Decode profiler — localize the ~2 s/token native decode: forward (32 layers) vs output
/// projection, and split **synchronization** (submit→poll(Wait) round-trips) from **kernel compute**
/// via an empty-round-trip baseline on the same device. This decides whether the lever is dispatch
/// fusion (sync-bound) or shader optimization (compute-bound) — no guessing. Skips if model absent.
/// Run: `cargo test -p qualia-core-db --release --test llm_bench_a0 a0_decode_profile -- --nocapture`.
#[test]
fn a0_decode_profile() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf")
        .or_else(|| find_model("SmolLM2-360M-Instruct-Q4_K_M.gguf"))
    else {
        eprintln!("[prof] model absent — skipping decode profile");
        return;
    };
    std::env::set_var("QUALIA_LLM_PROFILE_DECODE", "1");

    let mut cfg = BenchConfig::new(
        "decode-profile",
        path.to_string_lossy(),
        "auto",
        "Once upon a time, there was a",
    );
    cfg.decode_tokens = 16; // bounded; the per-token averages are what matter
    cfg.warm_repeats = 1; // the single warm run leaves the accumulators populated

    let results = llm_bench::run_suite_blocking(&[cfg]);
    assert!(
        !results.is_empty(),
        "[prof] harness produced no result (model failed to load)"
    );

    let snap = llm_bench::phase_snapshot();
    let waits = llm_bench::gpu_wait_count();
    let (ert_total, ert_n) = llm_bench::empty_rt();
    let toks = snap.decode_tokens.max(1);

    let per_ms = |ns: u64| (ns as f64 / toks as f64) / 1e6; // ms/token
    let total_ms = per_ms(snap.decode_ns).max(0.001);
    let fwd_ms = per_ms(snap.decode_forward_ns);
    let out_ms = per_ms(snap.decode_output_ns);
    let other_ms = (total_ms - fwd_ms - out_ms).max(0.0);
    let waits_per_tok = waits as f64 / toks as f64;
    let ert_per_ms = if ert_n > 0 {
        (ert_total as f64 / ert_n as f64) / 1e6
    } else {
        0.0
    };
    let sync_ms = waits_per_tok * ert_per_ms; // est. fixed fence overhead/token
    let sync_pct = 100.0 * sync_ms / total_ms;

    println!("\n=== A0 decode profile (SmolLM2-360M, A2000) ===");
    println!(
        "[prof] tokens={toks}  total={total_ms:.1} ms/tok  ({:.2} tok/s)",
        1000.0 / total_ms
    );
    println!(
        "[prof]   forward (32 layers) = {fwd_ms:.1} ms/tok ({:.0}%)",
        100.0 * fwd_ms / total_ms
    );
    println!(
        "[prof]   output projection   = {out_ms:.1} ms/tok ({:.0}%)",
        100.0 * out_ms / total_ms
    );
    println!("[prof]   host / other        = {other_ms:.1} ms/tok");
    println!("[prof] GPU submit→wait round-trips = {waits_per_tok:.0}/tok");
    println!("[prof] empty round-trip baseline   = {ert_per_ms:.3} ms each (n={ert_n})");
    println!(
        "[prof] est. fence overhead = {sync_ms:.1} ms/tok ({sync_pct:.0}% of token) → {}",
        if sync_pct >= 60.0 {
            "SYNC-BOUND — dispatch fusion is the lever"
        } else if sync_pct <= 25.0 {
            "COMPUTE-BOUND — the kernels are slow; optimize shaders"
        } else {
            "MIXED — both sync and compute matter"
        }
    );

    std::env::remove_var("QUALIA_LLM_PROFILE_DECODE");
}
