//! W5b Phase 3 — the sparse-KV-dictionary **go/no-go on real engine KV vectors** (SmolLM2-360M).
//!
//! Two independent capture routes, so the verdict can be cross-checked:
//!   * `go_no_go_gpu_readback`  — reads the REAL fast-decode-path K/V straight from VRAM (f32 KV).
//!   * `go_no_go_cpu_reference` — routes attention through the CPU reference SDPA and taps the hook.
//! Each learns a per-layer dictionary over captured K/V and compares it to matched-rate uniform
//! quantization (int8 shown for context). Prints a per-layer table + the overall verdict. Skips (does
//! not fail) when the model isn't present or a GPU/forward is unavailable.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::{
    kv_dictionary_go_no_go, kv_dictionary_go_no_go_gpu, KvDictReport,
};
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

fn print_report(route: &str, report: &KvDictReport) {
    println!(
        "\n=== W5b go/no-go [{route}] — SmolLM2-360M, head_dim={}, {} atoms, {}-sparse ===",
        report.head_dim, report.n_atoms, report.sparsity
    );
    println!(
        "int8 incumbent: {:.0} bits/vec.  dict code: {:.0} bits/vec (~matched to {}-bit uniform).",
        report.layers.first().map(|l| l.int8_bits).unwrap_or(0.0),
        report
            .layers
            .first()
            .map(|l| l.dict_code_bits)
            .unwrap_or(0.0),
        report.layers.first().map(|l| l.matched_bits).unwrap_or(0),
    );
    println!(
        "{:>5} {:>3} {:>8}  {:>10} {:>10} {:>12}  {}",
        "layer", "s", "n_vec", "int8_err", "dict_err", "unif_matched", "winner"
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
        "OVERALL [{route}]: {} — learned dictionary beats matched-rate uniform in {}/{} pairs.",
        if report.overall_go { "GO" } else { "NO-GO" },
        go,
        report.layers.len()
    );
}

fn assert_analysed(report: &KvDictReport) {
    assert!(
        !report.layers.is_empty(),
        "capture produced no analysable layers"
    );
    for l in &report.layers {
        assert!(l.recon_int8.is_finite() && l.recon_dict.is_finite());
        assert!(l.n_vectors >= 16);
    }
}

#[test]
fn go_no_go_gpu_readback() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/gpu] no smollm2-360m-instruct-q8_0.gguf under docs/models/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();
    // 256 atoms, 4-sparse, ≤2048 vec/layer, ≤128 tok/passage, sample every 6th layer.
    match kv_dictionary_go_no_go_gpu(&model, 2048, 256, 4, 25, 128, 6) {
        Ok(report) => {
            print_report("gpu-readback", &report);
            assert_analysed(&report);
        }
        Err(e) => eprintln!("[w5b/gpu] capture/eval unavailable ({e}) — skipping"),
    }
}

#[test]
fn go_no_go_cpu_reference() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/cpu] no smollm2-360m-instruct-q8_0.gguf under docs/models/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();
    // CPU reference path is slower → ≤64 tok/passage.
    match kv_dictionary_go_no_go(&model, 80, 2048, 256, 4, 25, 64, 6) {
        Ok(report) => {
            print_report("cpu-reference", &report);
            assert_analysed(&report);
        }
        Err(e) => eprintln!("[w5b/cpu] capture/eval unavailable ({e}) — skipping"),
    }
}
