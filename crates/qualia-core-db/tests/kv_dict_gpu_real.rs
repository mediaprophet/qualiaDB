//! W5b Phase 4b step 5b — the dict-coded KV cache on the **GPU** decode path (SmolLM2-360M).
//!
//! Exercises the fused_attention.wgsl dict path: `write_kv_head` OMP-encodes K/V to codes, and
//! `read_k`/`read_v` reconstruct them from the resident atoms. Both the f32 reference and the dict
//! candidate run the GPU attention pass (cpu_attention OFF, preproject/o_fuse OFF), so the ΔPPL isolates
//! the dictionary and the GPU decode is genuinely tested. The verdict must stay under the gate and track
//! the certified value (and the CPU-path result). Skips without the model / saved artifact. Slow.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::{kv_dict_runtime, llm_bench};
use std::path::{Path, PathBuf};

fn find_model(name: &str) -> Option<PathBuf> {
    [
        format!("../../docs/models/{name}"),
        format!("docs/models/{name}"),
    ]
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
fn dict_coded_cache_on_gpu_path() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/4b-gpu] no smollm2 model — skipping");
        return;
    };
    let Some(art) = find_artifact() else {
        eprintln!("[w5b/4b-gpu] no k=5 artifact under target/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();

    // GPU attention pass for both runs (write_kv_head + read_k/read_v), f32 baseline.
    llm_bench::set_cpu_attention(false);
    llm_bench::set_attention_preproject(false);
    llm_bench::set_attention_o_fuse(false);
    llm_bench::set_kv_int8(false);
    llm_bench::set_kv_dict(false);
    kv_dict_runtime::deactivate();

    let (ref_ppl, _) =
        llm_bench::perplexity_eval_blocking(&model, 48).expect("GPU f32 reference PPL");

    // Activate the dict-coded cache (GPU shader encode + reconstruct).
    let info = kv_dict_runtime::activate(&art).expect("certified k=5 artifact must load");
    let cand = llm_bench::perplexity_eval_blocking(&model, 48);
    kv_dict_runtime::deactivate();
    let (cand_ppl, _) = cand.expect("GPU dict PPL");

    let delta = (cand_ppl - ref_ppl) / ref_ppl;
    println!(
        "\n=== W5b Phase 4b — dict-coded KV cache on the GPU path (k={}, head_dim={}) ===\n\
         ref_ppl (f32 GPU)  = {:.4}\n\
         cand_ppl (dict GPU)= {:.4}\n\
         ΔPPL               = {:+.2}%   (certified {:+.2}%, CPU-path +0.59%, gate ≤ 5.0%)",
        info.sparsity,
        info.head_dim,
        ref_ppl,
        cand_ppl,
        delta * 100.0,
        info.delta_ppl * 100.0,
    );

    assert!(
        cand_ppl.is_finite() && cand_ppl > 1.0,
        "GPU dict decode must be coherent"
    );
    assert!(
        delta < 0.05,
        "GPU dict ΔPPL must stay under the 5% gate (got {:+.2}%)",
        delta * 100.0
    );
    // GPU OMP encode + f16 codes should track the CPU-path / certified value (a loose band catches
    // gross shader bugs — a wrong reconstruction blows PPL up).
    assert!(
        (delta - info.delta_ppl).abs() < 0.03,
        "GPU dict ΔPPL {:+.2}% should track the certified {:+.2}%",
        delta * 100.0,
        info.delta_ppl * 100.0
    );
}
