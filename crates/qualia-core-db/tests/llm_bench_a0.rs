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
    println!("[a1a] token-identity verified: top-k == argmax");
}
