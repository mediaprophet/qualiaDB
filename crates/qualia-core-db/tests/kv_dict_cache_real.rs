//! W5b Phase 4b steps 4-5a — the dict-coded KV cache, end-to-end on SmolLM2-360M.
//!
//! Proves the compressed cache works AND the engine runs it WITHOUT the forge feature (this test needs
//! no `wgsl-forge`): load the certified k=5 artifact, enable dict mode, and run perplexity — the KV
//! arena now stores sparse codes (reconstructed on the CPU attention read path), and the ΔPPL must
//! stay under the gate and track the certified value. Skips if the model or the saved artifact is
//! absent. Slow (two CPU-reference perplexity passes).

#![cfg(not(target_arch = "wasm32"))]

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

#[test]
fn dict_coded_cache_runs_and_tracks_certified_delta() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/4b] no smollm2 model — skipping");
        return;
    };
    let Some(art) = find_artifact() else {
        eprintln!("[w5b/4b] no k=5 artifact under target/ (run the certify sweep first) — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();

    // Same attention path + f32 KV for the reference (dict OFF) so the delta is purely the dict cache.
    llm_bench::set_cpu_attention(true);
    llm_bench::set_attention_preproject(false);
    llm_bench::set_attention_o_fuse(false);
    llm_bench::set_kv_int8(false);
    llm_bench::set_kv_dict(false);
    kv_dict_runtime::disable();
    kv_dict_runtime::clear();

    let (ref_ppl, _) = llm_bench::perplexity_eval_blocking(&model, 48).expect("reference PPL");

    // Load the certified dictionary + switch the KV cache to dict-coded storage.
    let info = kv_dict_runtime::load_certified(&art).expect("certified k=5 artifact must load");
    llm_bench::set_kv_dict(true);
    let cand = llm_bench::perplexity_eval_blocking(&model, 48);
    llm_bench::set_kv_dict(false);
    kv_dict_runtime::disable();
    kv_dict_runtime::clear();
    let (cand_ppl, _) = cand.expect("dict-cache PPL");

    let delta = (cand_ppl - ref_ppl) / ref_ppl;
    println!(
        "\n=== W5b Phase 4b — dict-coded KV cache (k={}, head_dim={}) ===\n\
         ref_ppl (f32 KV)   = {:.4}\n\
         cand_ppl (dict KV) = {:.4}\n\
         ΔPPL               = {:+.2}%   (certified {:+.2}%, gate ≤ 5.0%)",
        info.sparsity,
        info.head_dim,
        ref_ppl,
        cand_ppl,
        delta * 100.0,
        info.delta_ppl * 100.0,
    );

    assert!(cand_ppl.is_finite() && cand_ppl > 1.0, "dict-cache decode must stay coherent");
    assert!(
        delta < 0.05,
        "dict-cache ΔPPL must stay under the 5% gate (got {:+.2}%)",
        delta * 100.0
    );
    // The dict cache reconstructs the same lossy vectors the artifact was certified at (± f16 coeff
    // rounding), so its ΔPPL should be near the certified value — a loose band catches gross breakage.
    assert!(
        (delta - info.delta_ppl).abs() < 0.02,
        "dict-cache ΔPPL {:+.2}% should track the certified {:+.2}%",
        delta * 100.0,
        info.delta_ppl * 100.0
    );
}
