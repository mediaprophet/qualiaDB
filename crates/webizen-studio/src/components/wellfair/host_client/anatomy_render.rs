//! 3D Anatomy render + asset cache

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// Render the whole-body 3D Anatomy snapshot at `(azimuth, elevation)` degrees. The PNG is served at
/// `webizen://localhost/anatomy/body.png`; bump the epoch query-string to refetch after this call.
#[cfg(target_arch = "wasm32")]
pub async fn render_body_snapshot(azimuth: f64, elevation: f64) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"azimuth".into(),
        &wasm_bindgen::JsValue::from_f64(azimuth),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"elevation".into(),
        &wasm_bindgen::JsValue::from_f64(elevation),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_render_body_snapshot", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn render_body_snapshot(_azimuth: f64, _elevation: f64) -> Result<(), String> {
    Err("The 3D Anatomy render surface requires the Tauri desktop host".into())
}

// ── 3D Anatomy asset cache (S5.8 — user-triggered real-mesh acquisition) ───────────────────────

/// The status of a model's body-asset cache.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct BodyAssetsStatus {
    pub model: String,
    pub cached: bool,
    pub organ_count: usize,
    pub total_ten_d_bytes: usize,
    pub acquired_at_unix: u64,
}

/// The per-organ percept for the cached organ set.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct OrganPerceptDto {
    pub organ_key: String,
    pub system_id: String,
    pub percept: SystemPerceptDto,
}

/// The dual-modality percept (colour + pitch) for one body system.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SystemPerceptDto {
    pub system_id: String,
    pub sigma: f32,
    pub rgba: [f32; 4],
    pub frequency_hz: f32,
}

/// The final report from an acquisition run.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct AcquireReport {
    pub model: String,
    pub organs_cached: usize,
    pub organs_failed: usize,
    pub organs_unmapped: usize,
    pub total_glb_bytes: usize,
    pub total_ten_d_bytes: usize,
    pub failed: Vec<(String, String)>,
    pub unmapped: Vec<String>,
}

/// Per-organ progress during acquisition (emitted via `anatomy-acquire-progress` events).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct AcquireProgress {
    pub stage: String,
    pub organ_key: String,
    pub done: usize,
    pub total: usize,
    pub bytes: usize,
    pub message: String,
}

/// Check whether the body assets for a model are cached + complete.
#[cfg(target_arch = "wasm32")]
pub async fn body_assets_status(model: &str) -> Result<BodyAssetsStatus, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"model".into(),
        &wasm_bindgen::JsValue::from_str(model),
    )
    .map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_body_assets_status", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = raw
        .as_string()
        .ok_or_else(|| "body assets status response not JSON".to_string())?;
    serde_json::from_str::<BodyAssetsStatus>(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn body_assets_status(_model: &str) -> Result<BodyAssetsStatus, String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

/// Acquire (download + compile + cache) the body assets for a model — user-triggered. Returns the final
/// report; progress is observed via `listen_anatomy_acquire_progress` (the caller wires the event listener).
#[cfg(target_arch = "wasm32")]
pub async fn acquire_body_assets(model: &str) -> Result<AcquireReport, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"model".into(),
        &wasm_bindgen::JsValue::from_str(model),
    )
    .map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_acquire_body_assets", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = raw
        .as_string()
        .ok_or_else(|| "acquire body assets response not JSON".to_string())?;
    serde_json::from_str::<AcquireReport>(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn acquire_body_assets(_model: &str) -> Result<AcquireReport, String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

/// The per-organ percepts for the cached organ set (so the browser portal knows what colour to paint each).
#[cfg(target_arch = "wasm32")]
pub async fn cached_body_organ_percepts(
    model: &str,
) -> Result<(Vec<OrganPerceptDto>, Vec<String>), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"model".into(),
        &wasm_bindgen::JsValue::from_str(model),
    )
    .map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_cached_body_organ_percepts", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = raw
        .as_string()
        .ok_or_else(|| "organ percepts response not JSON".to_string())?;
    #[derive(serde::Deserialize)]
    struct Resp {
        painted: Vec<OrganPerceptDto>,
        unmapped: Vec<String>,
    }
    let resp: Resp = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok((resp.painted, resp.unmapped))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn cached_body_organ_percepts(
    _model: &str,
) -> Result<(Vec<OrganPerceptDto>, Vec<String>), String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

/// Clear the cache for a model.
#[cfg(target_arch = "wasm32")]
pub async fn clear_body_cache(model: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"model".into(),
        &wasm_bindgen::JsValue::from_str(model),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_clear_body_cache", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn clear_body_cache(_model: &str) -> Result<(), String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}
