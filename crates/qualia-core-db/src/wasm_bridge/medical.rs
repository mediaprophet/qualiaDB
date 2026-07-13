//! WASM-bindgen API — medical domain (split from wasm_bridge.rs; verbatim, no behaviour change).
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

// ─── Biomedical: clinical risk scores ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FraminghamParams {
    pub age: u8,
    pub sex_male: bool,
    pub total_cholesterol_mmol: f64,
    pub hdl_cholesterol_mmol: f64,
    pub systolic_bp: f64,
    pub bp_treated: bool,
    pub current_smoker: bool,
    pub diabetic: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_framingham_risk_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: FraminghamParams = serde_wasm_bindgen::from_value(val)?;

    // D'Agostino et al. 2008 General Cardiovascular Risk Score (Framingham Heart
    // Study, Circulation 117:743). Self-contained Cox model — no clinical_engine
    // dependency. Cholesterol inputs are mmol/L; the model uses mg/dL (×38.67).
    let tc_mgdl = (p.total_cholesterol_mmol * 38.67).max(1.0);
    let hdl_mgdl = (p.hdl_cholesterol_mmol * 38.67).max(1.0);
    let ln_age = (p.age as f64).max(1.0).ln();
    let ln_tc = tc_mgdl.ln();
    let ln_hdl = hdl_mgdl.ln();
    let ln_sbp = p.systolic_bp.max(1.0).ln();

    let (sum, mean, s0) = if p.sex_male {
        let mut s = 3.06117 * ln_age + 1.12370 * ln_tc - 0.93263 * ln_hdl
            + (if p.bp_treated { 1.99881 } else { 1.93303 }) * ln_sbp;
        if p.current_smoker {
            s += 0.65451;
        }
        if p.diabetic {
            s += 0.57367;
        }
        (s, 23.9802_f64, 0.88936_f64)
    } else {
        let mut s = 2.32888 * ln_age + 1.20904 * ln_tc - 0.70833 * ln_hdl
            + (if p.bp_treated { 2.82263 } else { 2.76157 }) * ln_sbp;
        if p.current_smoker {
            s += 0.52873;
        }
        if p.diabetic {
            s += 0.69154;
        }
        (s, 26.1931_f64, 0.95012_f64)
    };

    // Risk = 1 − S0(10)^exp(Σβx − mean), expressed as a percentage.
    let risk = ((1.0 - s0.powf((sum - mean).exp())) * 100.0).clamp(0.0, 100.0);
    let category = if risk < 10.0 {
        "Low"
    } else if risk < 20.0 {
        "Intermediate"
    } else {
        "High"
    };

    #[derive(Serialize)]
    struct RiskResult {
        risk_10yr_pct: f64,
        category: String,
    }
    Ok(serde_wasm_bindgen::to_value(&RiskResult {
        risk_10yr_pct: risk,
        category: category.to_string(),
    })?)
}

// ─── Biomedical: FHIR observation validation ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FhirObsParams {
    pub loinc_code: String,
    pub value: f64,
    pub unit_ucum: String,
    pub reference_low: Option<f64>,
    pub reference_high: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_fhir_observation_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: FhirObsParams = serde_wasm_bindgen::from_value(val)?;
    // Mocked for WASM due to clinical_engine dependency removal
    #[derive(Serialize)]
    struct ValidationResult {
        is_valid: bool,
        status: String,
        interpretation_code: String,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationResult {
        is_valid: true,
        status: "Mock".to_string(),
        interpretation_code: "N".to_string(),
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn check_drug_interactions_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: DrugInteractionParams = serde_wasm_bindgen::from_value(val)?;
    let hashes: Vec<u64> = p
        .medications
        .iter()
        .map(|m| crate::q_hash(m.to_lowercase().as_str()))
        .collect();
    // Mocked for WASM due to clinical_engine dependency removal
    #[derive(Serialize)]
    struct Interaction {
        mechanism: String,
        severity: String,
    }
    let result: Vec<Interaction> = vec![];
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
