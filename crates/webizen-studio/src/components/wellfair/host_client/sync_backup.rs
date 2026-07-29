//! Sync transport + backup/restore

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// The admission tally from a sync round against a relay.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyncSummaryDto {
    #[serde(default)]
    pub pushed: usize,
    #[serde(default)]
    pub pulled: usize,
    #[serde(default)]
    pub validated: usize,
    #[serde(default)]
    pub duplicate: usize,
    #[serde(default)]
    pub rejected: usize,
}

/// The count moved by an export/import.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BackupSummaryDto {
    #[serde(default)]
    pub files: usize,
    #[serde(default)]
    pub bytes: u64,
}

#[cfg(target_arch = "wasm32")]
pub async fn sync_with_relay(base_url: &str, since: u64) -> Result<SyncSummaryDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"baseUrl".into(),
        &wasm_bindgen::JsValue::from_str(base_url),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"since".into(),
        &wasm_bindgen::JsValue::from_f64(since as f64),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sync_with_relay", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "sync response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sync_with_relay(_base_url: &str, _since: u64) -> Result<SyncSummaryDto, String> {
    Err("Sync requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn export_backup(path: &str) -> Result<BackupSummaryDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"path".into(),
        &wasm_bindgen::JsValue::from_str(path),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_export_backup", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "export response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn export_backup(_path: &str) -> Result<BackupSummaryDto, String> {
    Err("Backup requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn import_backup(path: &str) -> Result<BackupSummaryDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"path".into(),
        &wasm_bindgen::JsValue::from_str(path),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_import_backup", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "import response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn import_backup(_path: &str) -> Result<BackupSummaryDto, String> {
    Err("Restore requires the Tauri desktop host".into())
}

/// A node health/status snapshot.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DiagnosticsDto {
    #[serde(default)]
    pub crate_version: String,
    #[serde(default)]
    pub sanctuary_configured: bool,
    #[serde(default)]
    pub sanctuary_keychain_wrapped: bool,
    #[serde(default)]
    pub journal_records: usize,
    #[serde(default)]
    pub outbox_queued: usize,
    #[serde(default)]
    pub inbox_validated: usize,
    #[serde(default)]
    pub data_files: usize,
    #[serde(default)]
    pub data_bytes: u64,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_diagnostics() -> Result<DiagnosticsDto, String> {
    let js = tauri_invoke("wellfair_diagnostics", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "diagnostics not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_diagnostics() -> Result<DiagnosticsDto, String> {
    Ok(DiagnosticsDto::default())
}
