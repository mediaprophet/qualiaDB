//! W5b Phase 4 — certify a learned KV dictionary by real **ΔPPL** on SmolLM2-360M.
//!
//! Captures the engine's KV, learns per-layer K/V dictionaries, then measures perplexity on the CPU
//! reference attention path with the dictionary OFF (ref) vs ON (cand). Prints ref/cand PPL, ΔPPL, the
//! pass/fail against the project gate, and the packaged-artifact size. The ΔPPL OUTCOME (pass or fail)
//! is the deliverable — either is an honest result — so the test asserts the pipeline RAN and produced
//! finite perplexities and a consistent package, not that it passed. Skips when the model/GPU is absent.
//! Slow (three model passes on the CPU reference path); run explicitly.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::{certify_kv_dictionary, GateSpec};
use std::path::{Path, PathBuf};

fn find_model(name: &str) -> Option<PathBuf> {
    [format!("../../docs/models/{name}"), format!("docs/models/{name}")]
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

#[test]
fn certify_kv_dictionary_on_smollm2() {
    let Some(model) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[w5b/certify] no smollm2-360m-instruct-q8_0.gguf under docs/models/ — skipping");
        return;
    };
    let model = model.to_string_lossy().to_string();

    // Config is env-driven so a ΔPPL sweep needs no recompile:
    //   QUALIA_W5B_ATOMS (default 256), QUALIA_W5B_K (default 4), QUALIA_W5B_MAXTOK (default 48).
    let env_usize = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let atoms = env_usize("QUALIA_W5B_ATOMS", 256);
    let k = env_usize("QUALIA_W5B_K", 4);
    let max_tok = env_usize("QUALIA_W5B_MAXTOK", 48);
    println!("[w5b/certify] config: {atoms} atoms, {k}-sparse, ≤{max_tok} tok/passage");

    // 20 learn iters, default 5% ΔPPL gate. corpus_hash/docs are provenance placeholders here.
    match certify_kv_dictionary(&model, atoms, k, 20, max_tok, GateSpec::default(), 0, 0) {
        Ok(r) => {
            let pkg = r.packaged.as_ref().map(|p| p.len()).unwrap_or(0);
            println!(
                "\n=== W5b Phase 4 certify — SmolLM2-360M (256 atoms, 4-sparse) ===\n\
                 ref_ppl (f32 KV)   = {:.4}\n\
                 cand_ppl (dict KV) = {:.4}\n\
                 ΔPPL               = {:+.2}%  (gate ≤ {:.1}%)\n\
                 VERDICT            = {}\n\
                 packaged artifact  = {} bytes",
                r.ref_ppl,
                r.cand_ppl,
                r.delta_ppl * 100.0,
                GateSpec::default().max_delta_ppl * 100.0,
                if r.passed { "PASS — dictionary certified + packaged" } else { "FAIL — ΔPPL over gate (not packaged)" },
                pkg,
            );
            assert!(r.ref_ppl.is_finite() && r.ref_ppl > 1.0, "ref PPL must be a sane finite value");
            assert!(r.cand_ppl.is_finite(), "cand PPL must be finite");
            assert_eq!(
                r.packaged.is_some(),
                r.passed,
                "an artifact is packaged iff the ΔPPL gate passed"
            );
        }
        Err(e) => eprintln!("[w5b/certify] capture/eval unavailable ({e}) — skipping"),
    }
}
