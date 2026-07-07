//! W5b Phase 4b step 6 — the honest A/B: decode tok/s for f32 vs int8 vs dict KV cache (SmolLM2-360M).
//!
//! ΔPPL and memory are already validated (dict ΔPPL ≈ 0, KV 12.9× smaller). This measures the tok/s
//! COST so the trade-off is on the record. Expectation (documented in the Phase 4b plan): on this
//! compute-bound A2000 the dict path is SLOWER (it adds an OMP encode + per-element reconstruct), while
//! its memory win pays off on memory-bound / long-context hardware. Prints the table; asserts the
//! benchmark ran. Skips without the model. Slow-ish (three short decode benches).

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::llm_bench::BenchConfig;
use qualia_core_db::{kv_dict_runtime, llm_bench};
use std::path::{Path, PathBuf};

fn find_model(name: &str) -> Option<PathBuf> {
    [format!("../../docs/models/{name}"), format!("docs/models/{name}")]
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

fn find_artifact() -> Option<PathBuf> {
    [
        "target/kv_dict_smollm2_256atoms_5sparse.q42art",
        "../../target/kv_dict_smollm2_256atoms_5sparse.q42art",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| Path::new(p).exists())
}

fn bench(model: &str, tokens: u32) -> Option<f64> {
    // run_suite_blocking runs the decode on the calling thread; a cargo-test thread's 2 MB stack
    // overflows on the deep GPU-decode frame, so give it a large stack.
    let model = model.to_string();
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let mut c = BenchConfig::new("kv-ab", &model, "Q8_0", "The capital of France is");
            c.decode_tokens = tokens;
            c.warm_repeats = 2;
            llm_bench::run_suite_blocking(&[c])
                .first()
                .map(|r| r.decode_tok_s)
        })
        .ok()?
        .join()
        .ok()
        .flatten()
}

#[test]
fn kv_cache_mode_ab() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/4b-ab] no smollm2 model — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();
    let tokens = 64u32;
    llm_bench::set_cpu_attention(false);

    kv_dict_runtime::deactivate();
    llm_bench::set_kv_int8(false);
    llm_bench::set_kv_dict(false);
    let f32_toks = bench(&model, tokens);

    kv_dict_runtime::deactivate();
    llm_bench::set_kv_int8(true);
    llm_bench::set_kv_dict(false);
    let int8_toks = bench(&model, tokens);

    let dict_toks = if let Some(art) = find_artifact() {
        llm_bench::set_kv_int8(false);
        let _ = kv_dict_runtime::activate(&art);
        let t = bench(&model, tokens);
        kv_dict_runtime::deactivate();
        t
    } else {
        None
    };

    let fmt = |t: Option<f64>| t.map(|x| format!("{x:.2}")).unwrap_or_else(|| "n/a".into());
    println!(
        "\n=== W5b Phase 4b — KV cache mode A/B (SmolLM2-360M, A2000, {tokens}-tok decode) ===\n\
         f32  : {:>7} tok/s   KV 80.0 MiB\n\
         int8 : {:>7} tok/s   KV 21.2 MiB   (ΔPPL +0.05%, 3.77× < f32)\n\
         dict : {:>7} tok/s   KV  6.2 MiB   (ΔPPL ~0%, k=5, 12.9× < f32 / ~3.4× < int8)\n\
         Honest read: dict trades decode compute (in-shader OMP encode + per-element reconstruct) for KV\n\
         memory — expected slower on this compute-bound A2000, a win on memory-bound / long-context HW.",
        fmt(f32_toks),
        fmt(int8_toks),
        fmt(dict_toks),
    );

    assert!(
        f32_toks.map(|t| t.is_finite() && t > 0.0).unwrap_or(false),
        "the decode benchmark must produce a tok/s number"
    );
}
