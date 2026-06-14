use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use serde_wasm_bindgen;

use crate::parser::{parse_weight_csv, parse_sleep_csv, parse_heart_rate_csv, parse_steps_csv};
use crate::rdf::{weight_to_turtle, sleep_to_turtle, heart_rate_to_turtle, steps_to_turtle};

#[wasm_bindgen]
pub fn parse_weight_csv_json(content: &str) -> Result<JsValue, JsValue> {
    match parse_weight_csv(content) {
        Ok(records) => serde_wasm_bindgen::to_value(&records).map_err(|e| JsValue::from_str(&format!("serde error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn parse_sleep_csv_json(content: &str) -> Result<JsValue, JsValue> {
    match parse_sleep_csv(content) {
        Ok(records) => serde_wasm_bindgen::to_value(&records).map_err(|e| JsValue::from_str(&format!("serde error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn parse_heart_rate_csv_json(content: &str) -> Result<JsValue, JsValue> {
    match parse_heart_rate_csv(content) {
        Ok(records) => serde_wasm_bindgen::to_value(&records).map_err(|e| JsValue::from_str(&format!("serde error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn parse_steps_csv_json(content: &str) -> Result<JsValue, JsValue> {
    match parse_steps_csv(content) {
        Ok(records) => serde_wasm_bindgen::to_value(&records).map_err(|e| JsValue::from_str(&format!("serde error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn weight_turtle_from_csv(content: &str) -> Result<String, JsValue> {
    match parse_weight_csv(content) {
        Ok(records) => Ok(weight_to_turtle(&records)),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn sleep_turtle_from_csv(content: &str) -> Result<String, JsValue> {
    match parse_sleep_csv(content) {
        Ok(records) => Ok(sleep_to_turtle(&records)),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn heart_rate_turtle_from_csv(content: &str) -> Result<String, JsValue> {
    match parse_heart_rate_csv(content) {
        Ok(records) => Ok(heart_rate_to_turtle(&records)),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[wasm_bindgen]
pub fn steps_turtle_from_csv(content: &str) -> Result<String, JsValue> {
    match parse_steps_csv(content) {
        Ok(records) => Ok(steps_to_turtle(&records)),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

// ==========================================
// QUALIADB OPFS STORE
// ==========================================

// See qualia_bindings.rs for the QualiaStore implementation

// ==========================================
// SPARQL HEALTH STORE  (W4)
// ==========================================

/// In-memory RDF store exposed to JS.  Load Turtle, run SPARQL SELECT/ASK/CONSTRUCT.
#[wasm_bindgen]
pub struct WasmHealthStore {
    inner: crate::store::HealthStore,
}

#[wasm_bindgen]
impl WasmHealthStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmHealthStore, JsValue> {
        crate::store::HealthStore::new()
            .map(|inner| WasmHealthStore { inner })
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Load a Turtle document into the store (appends — call on a fresh store to replace).
    pub fn load_turtle(&mut self, turtle: &str) -> Result<(), JsValue> {
        self.inner.load_turtle(turtle).map_err(|e| JsValue::from_str(&e))
    }

    /// Execute a SPARQL query; returns JSON SPARQL results string.
    pub fn query(&self, sparql: &str) -> Result<String, JsValue> {
        self.inner.query(sparql).map_err(|e| JsValue::from_str(&e))
    }
}

// ==========================================
// SHACL VALIDATION  (W5)
// ==========================================

/// Validate a Turtle document against built-in health shapes (SPARQL ASK constraints).
/// Returns a JSON string: `{"valid":bool,"checked":N,"violations":[{"shape":"...","message":"..."}]}`
#[wasm_bindgen]
pub fn validate_health_turtle(turtle: &str) -> String {
    crate::shapes::validate_turtle(turtle).to_json()
}

// ==========================================
// VAULT DATA → TURTLE  (wf: namespace)
// ==========================================

/// Serialize vault medication records (JSON array from wf-meds IDB store) → Turtle.
#[wasm_bindgen]
pub fn vault_meds_to_turtle(json: &str) -> Result<String, JsValue> {
    crate::rdf::vault_meds_to_turtle(json).map_err(|e| JsValue::from_str(&e))
}

/// Serialize vault diet log entries (JSON array from wf-dl IDB store) → Turtle.
#[wasm_bindgen]
pub fn vault_diet_to_turtle(json: &str) -> Result<String, JsValue> {
    crate::rdf::vault_diet_to_turtle(json).map_err(|e| JsValue::from_str(&e))
}

/// Serialize vault biometric records (JSON array from wf-biometrics IDB store) → Turtle.
#[wasm_bindgen]
pub fn vault_biometrics_to_turtle(json: &str) -> Result<String, JsValue> {
    crate::rdf::vault_biometrics_to_turtle(json).map_err(|e| JsValue::from_str(&e))
}

// ==========================================
// N3 LOGIC RULE ENGINE  (A6)
// ==========================================

/// Evaluate all 7 N3 clinical rules against a Turtle document.
///
/// Returns a JSON array of triggered patterns:
/// `[{"pattern":"ChronicSleepDebt","confidence":"high","routingLane":2,"n3Source":"sleep_debt.n3"},...]`
///
/// Empty array = no concerns found in the supplied health data.
/// Routing lane 2 = BilateralMicroCommons (N3Logic implication rules requiring identity context).
/// Routing lane 0 = PassthroughStandard (simple threshold flags).
#[wasm_bindgen]
pub fn evaluate_n3_rules(turtle: &str) -> String {
    crate::n3_rules::evaluate_n3_rules_turtle(turtle)
}

// ==========================================
// WEBIZEN VM  (W6)
// ==========================================

/// Evaluate a named policy constraint against a single quint (s,p,o,c,m).
///
/// Supported constraint names:
///   "cooperative_obligation" — PermissiveCommons work obligation gate (lane 1)
///   "guardian_identity"      — BilateralMicroCommons guardian auth gate (lane 2)
///   "commercial_block"       — BilateralMicroCommons anti-commercial gate (lane 2)
///
/// Returns JSON: `{"passed":bool,"routingLane":N}`
#[wasm_bindgen]
pub fn validate_health_quin(
    constraint: &str,
    s: u64, p: u64, o: u64, c: u64, m: u64,
) -> String {
    let (passed, lane) = crate::webizen::evaluate_policy_constraint(constraint, s, p, o, c, m);
    format!("{{\"passed\":{},\"routingLane\":{}}}", passed, lane)
}
