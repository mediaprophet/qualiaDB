//! 3D Anatomy Qapp

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


/// One system's entry in the anatomy view (mirror of the host `SystemView`).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AnatomySystemDto {
    #[serde(default)]
    pub system_id: String,
    #[serde(default)]
    pub system_label: String,
    #[serde(default)]
    pub plain_label: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub net_milli: u32,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub detail: Vec<String>,
    #[serde(default)]
    pub dominant_evidence: String,
}

/// The lens-shaped narrative (mirror of the host `AnatomyView`).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AnatomyViewDto {
    #[serde(default)]
    pub lens: String,
    #[serde(default)]
    pub systems: Vec<AnatomySystemDto>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub boundary: String,
    #[serde(default)]
    pub uncertainty_note: String,
}

/// A record with no knowledge mapping yet.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct UnmappedRecordDto {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub label: String,
}

/// The whole anatomy report for a lens (mirror of the host `AnatomyViewReport`; the lens-independent
/// `burdens` field is intentionally not mirrored — the text panel renders the narrative, not colours).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AnatomyViewReportDto {
    #[serde(default)]
    pub view: AnatomyViewDto,
    #[serde(default)]
    pub unmapped: Vec<UnmappedRecordDto>,
    #[serde(default)]
    pub mapped_count: usize,
    #[serde(default)]
    pub total_records: usize,
    #[serde(default)]
    pub disclosure: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_anatomy_view(lens: &str, threshold: u32) -> Result<AnatomyViewReportDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"lens".into(), &wasm_bindgen::JsValue::from_str(lens))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from(threshold))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_compute_anatomy_view", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "anatomy view was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_anatomy_view(_lens: &str, _threshold: u32) -> Result<AnatomyViewReportDto, String> {
    Err("The Anatomy view requires the Tauri desktop host".into())
}

