//! Physiological state

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// The person's declared physiological state + whether they've declared one. `{ state, declared }`.
#[cfg(target_arch = "wasm32")]
pub async fn get_physiological_state() -> Result<serde_json::Value, String> {
    let js = tauri_invoke(
        "wellfair_get_physiological_state",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "physiological state not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_physiological_state() -> Result<serde_json::Value, String> {
    Err("The physiological state requires the Tauri desktop host".into())
}

/// Set the person's declared physiological state (`state_json` = a serialized `PhysiologicalState`).
#[cfg(target_arch = "wasm32")]
pub async fn set_physiological_state(state_json: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"stateJson".into(),
        &wasm_bindgen::JsValue::from_str(state_json),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_physiological_state", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_physiological_state(_state_json: &str) -> Result<(), String> {
    Err("The physiological state requires the Tauri desktop host".into())
}

/// Clear the declared physiological state — revert to the implicit Baseline.
#[cfg(target_arch = "wasm32")]
pub async fn reset_physiological_state() -> Result<(), String> {
    tauri_invoke(
        "wellfair_reset_physiological_state",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset_physiological_state() -> Result<(), String> {
    Err("The physiological state requires the Tauri desktop host".into())
}
