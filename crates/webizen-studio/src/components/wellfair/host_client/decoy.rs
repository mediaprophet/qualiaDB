//! Decoy-retention mode + decoy activity review

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// Returns the current decoy-retention mode ("auto_archive" | "manual_triage"). Real-session-only:
/// requires the real PIN (the setting lives in the real lane).
#[cfg(target_arch = "wasm32")]
pub async fn get_decoy_retention_mode(real_pin: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_get_decoy_retention_mode", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "decoy retention response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("auto_archive")
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_decoy_retention_mode(_real_pin: &str) -> Result<String, String> {
    // Fail closed like every other wrapper in this file: never fabricate a
    // security-relevant duress-audit-retention value off the desktop host. A
    // false "auto_archive" here would tell the caller a policy is in effect
    // when none was read — the exact false-success signal the sanctuary vault
    // is built to avoid.
    Err("Decoy-retention mode requires the Tauri desktop host".into())
}

/// Saves the decoy-retention mode ("auto_archive" | "manual_triage"). Requires the real PIN.
#[cfg(target_arch = "wasm32")]
pub async fn set_decoy_retention_mode(real_pin: &str, mode: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"mode".into(),
        &wasm_bindgen::JsValue::from_str(mode),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_set_decoy_retention_mode", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_decoy_retention_mode(_real_pin: &str, _mode: &str) -> Result<(), String> {
    // Fail closed like every other mutating wrapper in this file: never report
    // a security-relevant duress-audit-retention write as succeeding when
    // nothing was written.
    Err("Decoy-retention mode requires the Tauri desktop host".into())
}

// --- Decoy activity review + curation (vault v2 S6; real-session-only) ---

/// One decrypted decoy-session action surfaced to the real lane.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DecoyActionDto {
    #[serde(default)]
    pub branch_ref: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub actor_did: String,
    #[serde(default)]
    pub unix: u32,
    #[serde(default)]
    pub payload: String,
}

/// The decoy-activity review report. `integrity` is a raw JSON value: the string `"ok"` when clean,
/// otherwise an object naming the tampered branch (`chain_broken` / `witnessed_prefix_altered`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DecoyActivityReportDto {
    #[serde(default)]
    pub integrity: serde_json::Value,
    #[serde(default)]
    pub session_count: usize,
    #[serde(default)]
    pub actions: Vec<DecoyActionDto>,
    #[serde(default)]
    pub retention_mode: String,
}

impl DecoyActivityReportDto {
    /// True when the audit log verified clean (integrity == "ok").
    pub fn is_clean(&self) -> bool {
        self.integrity.as_str() == Some("ok")
    }
}

/// Review decoy activity from the real lane. Requires the real PIN.
#[cfg(target_arch = "wasm32")]
pub async fn review_decoy_activity(real_pin: &str) -> Result<DecoyActivityReportDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_review_decoy_activity", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "decoy review response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn review_decoy_activity(_real_pin: &str) -> Result<DecoyActivityReportDto, String> {
    Err("Decoy review requires the Tauri desktop host".into())
}

/// Seed a plausible cover note into the decoy lane from a real session (no decoy PIN). Requires the
/// real PIN.
#[cfg(target_arch = "wasm32")]
pub async fn curate_decoy_note(real_pin: &str, body: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"body".into(),
        &wasm_bindgen::JsValue::from_str(body),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_curate_decoy_note", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn curate_decoy_note(_real_pin: &str, _body: &str) -> Result<(), String> {
    Err("Decoy curation requires the Tauri desktop host".into())
}
