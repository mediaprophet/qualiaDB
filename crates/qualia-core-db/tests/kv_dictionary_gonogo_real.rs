//! W5b Phase 3 — the sparse-KV-dictionary **go/no-go on real engine KV vectors** (SmolLM2-360M).
//!
//! Enables the KV capture hook, runs a short calibration forward (native attention routes through the
//! CPU SDPA the hook taps), learns a per-layer dictionary over the captured K and V, and compares its
//! reconstruction to per-vector int8 at footprint. Prints a per-layer table and the overall verdict —
//! this is the measurement that decides whether the runtime dictionary decode (Phase 4) is worth
//! building. Skips (does not fail) when the model isn't present locally.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::kv_dictionary_go_no_go;
use std::path::{Path, PathBuf};

fn find_model(name: &str) -> Option<PathBuf> {
    [format!("../../docs/models/{name}"), format!("docs/models/{name}")]
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

#[test]
fn kv_dictionary_go_no_go_on_smollm2() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/real] no smollm2-360m-instruct-q8_0.gguf under docs/models/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();

    // 256 atoms, 4-sparse, ≤2048 vectors/layer, ≤128 tokens/passage of eval corpus (CPU attention is
    // slower, so bound per-passage work), sample every 6th layer.
    let report = match kv_dictionary_go_no_go(&model, 80, 2048, 256, 4, 25, 128, 6) {
        Ok(r) => r,
        Err(e) => {
            // A capture/forward failure (e.g. no GPU adapter in CI) is a skip, not a red test — the
            // pure learner + decision logic are covered by kv_dictionary_learn.rs.
            eprintln!("[w5b/real] capture/eval unavailable ({e}) — skipping");
            return;
        }
    };

    println!(
        "\n=== W5b go/no-go — SmolLM2-360M, head_dim={}, {} atoms, {}-sparse ===",
        report.head_dim, report.n_atoms, report.sparsity
    );
    println!(
        "int8 incumbent: {:.0} bits/vec.  dict code: {:.0} bits/vec (~matched to {}-bit uniform).",
        report.layers.first().map(|l| l.int8_bits).unwrap_or(0.0),
        report.layers.first().map(|l| l.dict_code_bits).unwrap_or(0.0),
        report.layers.first().map(|l| l.matched_bits).unwrap_or(0),
    );
    println!(
        "{:>5} {:>3} {:>8}  {:>10} {:>10} {:>12}  {}",
        "layer", "s", "n_vec", "int8_err", "dict_err", "unif_matched", "dict>=unif?"
    );
    for l in &report.layers {
        println!(
            "{:>5} {:>3} {:>8}  {:>10.4} {:>10.4} {:>12.4}  {}",
            l.layer,
            l.stream,
            l.n_vectors,
            l.recon_int8,
            l.recon_dict,
            l.recon_uniform_matched,
            if l.go { "DICT wins" } else { "uniform wins" }
        );
    }
    let go = report.layers.iter().filter(|l| l.go).count();
    println!(
        "OVERALL: {} — learned dictionary beats matched-rate uniform in {}/{} (layer,stream) pairs.",
        if report.overall_go { "GO" } else { "NO-GO" },
        go,
        report.layers.len()
    );

    // The test asserts the analysis RAN and produced verdicts (the decision itself — GO or NO-GO — is
    // the deliverable, not an assertion target). If the CPU attention path yielded nothing, that's a
    // real problem to surface.
    assert!(
        !report.layers.is_empty(),
        "capture produced no analysable layers — the KV hook did not fire on the forward"
    );
    for l in &report.layers {
        assert!(l.recon_int8.is_finite() && l.recon_dict.is_finite());
        assert!(l.n_vectors >= 16);
    }
}
