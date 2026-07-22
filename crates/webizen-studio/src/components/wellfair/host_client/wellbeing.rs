//! Wellbeing self-assessment instruments

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


/// An instrument definition (items + ordinal options + severity bands + disclaimer).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssessmentInstrumentDto {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub attribution: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub items: Vec<String>,
    /// (value, label) pairs.
    #[serde(default)]
    pub options: Vec<(u8, String)>,
    /// (min, max, label, interpretation) bands.
    #[serde(default)]
    pub bands: Vec<(u32, u32, String, String)>,
    #[serde(default)]
    pub max_score: u32,
    #[serde(default)]
    pub disclaimer: String,
}

/// A scored assessment result.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssessmentResultDto {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub instrument_id: String,
    #[serde(default)]
    pub responses: Vec<u8>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub band_label: String,
    #[serde(default)]
    pub interpretation: String,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub taken_at_unix: u32,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_assessment_instruments() -> Result<Vec<AssessmentInstrumentDto>, String> {
    let js = tauri_invoke("wellfair_list_assessment_instruments", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "instruments not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_assessment_instruments() -> Result<Vec<AssessmentInstrumentDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn record_assessment(
    instrument_id: &str,
    responses_csv: &str,
) -> Result<AssessmentResultDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"instrumentId".into(), &wasm_bindgen::JsValue::from_str(instrument_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"responses".into(), &wasm_bindgen::JsValue::from_str(responses_csv))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_record_assessment", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "assessment result not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn record_assessment(
    _instrument_id: &str,
    _responses_csv: &str,
) -> Result<AssessmentResultDto, String> {
    Err("Recording an assessment requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_assessments() -> Result<Vec<AssessmentResultDto>, String> {
    let js = tauri_invoke("wellfair_list_assessments", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "assessments not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_assessments() -> Result<Vec<AssessmentResultDto>, String> {
    Ok(vec![])
}

