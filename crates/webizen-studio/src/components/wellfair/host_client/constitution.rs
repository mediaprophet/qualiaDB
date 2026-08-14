//! Body constitution (measurements / characteristics / attributes)

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// `{ constitution, declared, fit }`.
#[cfg(target_arch = "wasm32")]
pub async fn get_body_constitution() -> Result<serde_json::Value, String> {
    let js = tauri_invoke(
        "wellfair_get_body_constitution",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "body constitution not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_body_constitution() -> Result<serde_json::Value, String> {
    Err("Body constitution requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_body_constitution(constitution_json: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"constitutionJson".into(),
        &wasm_bindgen::JsValue::from_str(constitution_json),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_set_body_constitution", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "set constitution not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_body_constitution(_constitution_json: &str) -> Result<serde_json::Value, String> {
    Err("Body constitution requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn reset_body_constitution() -> Result<(), String> {
    tauri_invoke(
        "wellfair_reset_body_constitution",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset_body_constitution() -> Result<(), String> {
    Err("Body constitution requires the Tauri desktop host".into())
}
