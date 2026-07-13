//! WASM-bindgen API — bio domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
use super::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn align_sequences_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let params: AlignmentParams = serde_wasm_bindgen::from_value(val)?;
    let result = if params.mode == "protein" {
        crate::domains::biological::bioinformatics::align_protein(
            params.query.as_bytes(),
            params.target.as_bytes(),
        )
    } else {
        crate::domains::biological::bioinformatics::align_nucleotide(
            params.query.as_bytes(),
            params.target.as_bytes(),
        )
    };
    #[derive(Serialize)]
    struct AlignResult {
        score: i32,
        identity_pct: f32,
        num_matches: usize,
        num_gaps: usize,
        aligned_query: String,
        aligned_target: String,
    }
    Ok(serde_wasm_bindgen::to_value(&AlignResult {
        score: result.score,
        identity_pct: result.identity_pct,
        num_matches: result.num_matches,
        num_gaps: result.num_gaps,
        aligned_query: String::from_utf8_lossy(&result.aligned_query).into_owned(),
        aligned_target: String::from_utf8_lossy(&result.aligned_target).into_owned(),
    })?)
}

// ─── Bioinformatics: FASTA validation ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FastaParams {
    pub header: String,
    pub sequence: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_fasta_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let params: FastaParams = serde_wasm_bindgen::from_value(val)?;
    let record = crate::domains::biological::bioinformatics::validate_fasta_record(
        &params.header,
        params.sequence.as_bytes(),
    );
    #[derive(Serialize)]
    struct FastaResult {
        is_valid: bool,
        alphabet: String,
        invalid_chars: Vec<char>,
    }
    Ok(serde_wasm_bindgen::to_value(&FastaResult {
        is_valid: record.is_valid,
        alphabet: format!("{:?}", record.alphabet),
        invalid_chars: record.invalid_chars,
    })?)
}

// ─── Quantum DFT: receptor binding affinity ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn predict_receptor_binding_wasm() -> f64 {
    // Molecule and receptor Quins would be loaded from the OPFS graph in production.
    // Returns binding affinity in kcal/mol (more negative = stronger binding).
    let demo_molecule = crate::NQuin {
        subject: crate::q_hash("demo:ligand"),
        predicate: crate::q_hash("HAS_ELECTRON"),
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let demo_receptor = crate::NQuin {
        subject: crate::q_hash("demo:receptor"),
        predicate: crate::q_hash("HAS_ELECTRON"),
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    crate::quantum_dft::pinn_predict_receptor_binding(&[demo_molecule], &[demo_receptor])
}
