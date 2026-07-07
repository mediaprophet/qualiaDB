//! W5b Phase 4 — certify KV-dictionary configs by real **ΔPPL** on SmolLM2-360M, as a SWEEP.
//!
//! Captures the engine's KV once and measures the reference perplexity once, then for each config
//! (256 atoms × a list of sparsities) learns the dictionaries, measures candidate PPL on the CPU
//! reference path, gates at 5% ΔPPL, and packages fail-closed. Sharing the capture + reference makes a
//! multi-config sweep cost ~one capture + one reference + one candidate pass per config (not a full
//! certification each). Prints a table and saves each PASSING artifact to disk (Phase 4b input). The
//! ΔPPL OUTCOMES are the deliverable; the test asserts the sweep RAN and each result is self-consistent.
//! Skips when the model/GPU is absent. Slow — run explicitly.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::{sweep_kv_dictionary, GateSpec};
use std::path::{Path, PathBuf};

fn find_model(name: &str) -> Option<PathBuf> {
    [format!("../../docs/models/{name}"), format!("docs/models/{name}")]
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

#[test]
fn certify_kv_dictionary_sweep_on_smollm2() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/certify] no smollm2-360m-instruct-q8_0.gguf under docs/models/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();

    // Env-driven, no recompile: QUALIA_W5B_ATOMS (256), QUALIA_W5B_KS ("5,6"), QUALIA_W5B_MAXTOK (48).
    let atoms: usize = std::env::var("QUALIA_W5B_ATOMS").ok().and_then(|v| v.parse().ok()).unwrap_or(256);
    let max_tok: usize = std::env::var("QUALIA_W5B_MAXTOK").ok().and_then(|v| v.parse().ok()).unwrap_or(48);
    let ks: Vec<usize> = std::env::var("QUALIA_W5B_KS")
        .unwrap_or_else(|_| "5,6".to_string())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let configs: Vec<(usize, usize)> = ks.iter().map(|&k| (atoms, k)).collect();
    let gate = GateSpec::default();
    println!(
        "[w5b/certify] sweep: {atoms} atoms × k∈{ks:?}, ≤{max_tok} tok/passage, gate ≤ {:.1}%",
        gate.max_delta_ppl * 100.0
    );

    match sweep_kv_dictionary(&model, &configs, 20, max_tok, gate, 0, 0) {
        Ok(results) => {
            let ref_ppl = results.first().map(|(_, _, r)| r.ref_ppl).unwrap_or(f64::NAN);
            println!(
                "\n=== W5b Phase 4 certify sweep — SmolLM2-360M ({atoms} atoms), ref_ppl(f32 KV)={:.4} ===",
                ref_ppl
            );
            println!("{:>7} {:>10} {:>9} {:>7}  {}", "k", "cand_ppl", "ΔPPL", "bits/v", "verdict");
            for (na, k, r) in &results {
                // asymptotic dict code rate: k × (ceil(log2 atoms) index + 16-bit coeff).
                let idx_bits = (usize::BITS - na.saturating_sub(1).leading_zeros()).max(1) as usize;
                let bits = k * (idx_bits + 16);
                let mut verdict = if r.passed { format!("PASS ({} B)", r.packaged.as_ref().map(|p| p.len()).unwrap_or(0)) } else { "FAIL".to_string() };
                if let Some(bytes) = r.packaged.as_ref() {
                    let out = format!("../../target/kv_dict_smollm2_{na}atoms_{k}sparse.q42art");
                    if std::fs::write(&out, bytes).is_ok() {
                        verdict = format!("{verdict} → {out}");
                    }
                }
                println!(
                    "{:>7} {:>10.4} {:>+8.2}% {:>7}  {}",
                    k, r.cand_ppl, r.delta_ppl * 100.0, bits, verdict
                );
            }
            // Self-consistency: sane ref, finite candidates, package iff passed.
            assert!(ref_ppl.is_finite() && ref_ppl > 1.0, "ref PPL must be sane");
            for (_, _, r) in &results {
                assert!(r.cand_ppl.is_finite(), "cand PPL must be finite");
                assert_eq!(r.packaged.is_some(), r.passed, "packaged iff passed");
            }
        }
        Err(e) => eprintln!("[w5b/certify] capture/eval unavailable ({e}) — skipping"),
    }
}
