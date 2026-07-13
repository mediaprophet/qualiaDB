//! WASM-bindgen API — meta domain (split from wasm_bridge.rs; verbatim, no behaviour change).
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
#[derive(Deserialize)]
pub struct SimulationParams {
    pub initial_price: f64,
    pub drift: f64,
    pub volatility: f64,
    pub time_horizon: i32,
    pub simulation_steps: i32,
}

// ─── Biomedical: drug interaction check ──────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct DrugInteractionParams {
    /// List of medication names (will be q_hashed internally).
    pub medications: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ThermochemParams {
    pub delta_h_j_mol: f64,
    pub delta_s_j_mol_k: f64,
    pub temp_k: f64,
    pub pka: Option<f64>,
    pub conc_base: Option<f64>,
    pub conc_acid: Option<f64>,
    pub activation_energy_j_mol: Option<f64>,
    pub pre_exponential_a: Option<f64>,
}

// ─── PID Control Step ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct PidStepParams {
    pub setpoint: f64,
    pub current_value: f64,
    pub prev_error: f64,
    pub integral: f64,
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub dt: f64,
}

/// Resolves two conflicting NQuin entries using Last-Writer-Wins semantics.
/// The Lamport clock is encoded in the metadata field; on ties, higher object wins.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn resolve_lww_wasm(local_val: JsValue, remote_val: JsValue) -> Result<JsValue, JsValue> {
    let local: QuinJson =
        serde_wasm_bindgen::from_value(local_val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let remote: QuinJson = serde_wasm_bindgen::from_value(remote_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Lamport clock in metadata upper 32 bits
    let local_clock = local.metadata >> 32;
    let remote_clock = remote.metadata >> 32;

    let winner = if remote_clock > local_clock {
        remote
    } else if local_clock > remote_clock {
        local
    } else if remote.object > local.object {
        remote
    } else {
        local
    };
    Ok(serde_wasm_bindgen::to_value(&winner)?)
}

// ─── Black-Scholes ───────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct BlackScholesParams {
    pub spot: f64,
    pub strike: f64,
    pub rate: f64,
    pub vol: f64,
    pub time_years: f64,
    pub is_call: bool,
}

/// Cumulative standard normal distribution (Horner rational approximation).
#[cfg(target_arch = "wasm32")]
pub(crate) fn phi_norm(x: f64) -> f64 {
    const A: [f64; 5] = [
        0.254829592,
        -0.284496736,
        1.421413741,
        -1.453152027,
        1.061405429,
    ];
    const P: f64 = 0.3275911;
    let sign = if x < 0.0 { -1.0_f64 } else { 1.0_f64 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + P * ax);
    let poly = ((((A[4] * t + A[3]) * t + A[2]) * t + A[1]) * t + A[0]) * t;
    let y = 1.0 - poly * f64::exp(-ax * ax);
    0.5 * (1.0 + sign * y)
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct EngineInfo {
    version: &'static str,
    engine: &'static str,
    target: &'static str,
    profile: &'static str,
    capabilities: Vec<&'static str>,
}

/// Returns the qualia-core-db crate version baked in at compile time (matches daemon `/health`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_engine_version() -> String {
    crate::ENGINE_VERSION.to_string()
}

// ─── LLM Inference Engine ────────────────────────────────────────────────────

/// Structured engine metadata for browser UIs and diagnostics.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_engine_info() -> Result<JsValue, JsValue> {
    let info = EngineInfo {
        version: crate::ENGINE_VERSION,
        engine: "qualia-core-db",
        target: "wasm32",
        profile: crate::wasm_capabilities::compiled_profile(),
        capabilities: crate::wasm_capabilities::compiled_capabilities().to_vec(),
    };
    serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Capability names available in this WASM build.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn list_capabilities_wasm() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(crate::wasm_capabilities::compiled_capabilities())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
