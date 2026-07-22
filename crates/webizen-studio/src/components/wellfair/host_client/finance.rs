//! Finance / Cooperative Projects / Credentials

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


#[derive(Debug, Clone, Default, Deserialize)]
pub struct CurrencyBalanceDto {
    pub currency: String,
    pub net_cents: i64,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BalanceReportDto {
    pub by_currency: Vec<CurrencyBalanceDto>,
    pub total_entries: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObligationDto {
    pub project_id: String,
    pub contributor_did: String,
    pub total_effort_minutes: u64,
    #[serde(default)]
    pub total_capital_cents: u64,
    #[serde(default)]
    pub resolved_obligation_score: f64,
    pub contribution_count: usize,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_ledger_entry(
    description: &str,
    amount_cents: i64,
    currency: &str,
    category: Option<&str>,
    attached_asset_uri: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"description".into(), &wasm_bindgen::JsValue::from_str(description))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"amountCents".into(), &wasm_bindgen::JsValue::from(amount_cents as f64))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"currency".into(), &wasm_bindgen::JsValue::from_str(currency))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(c) = category {
        js_sys::Reflect::set(&args, &"category".into(), &wasm_bindgen::JsValue::from_str(c))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    if let Some(uri) = attached_asset_uri {
        js_sys::Reflect::set(&args, &"attachedAssetUri".into(), &wasm_bindgen::JsValue::from_str(uri))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_ledger_entry", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "ledger response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_ledger_entry(
    _description: &str,
    _amount_cents: i64,
    _currency: &str,
    _category: Option<&str>,
    _attached_asset_uri: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Ledger entries require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_ledger_balance(limit: usize) -> Result<BalanceReportDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_ledger_balance", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "balance response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_ledger_balance(_limit: usize) -> Result<BalanceReportDto, String> {
    Ok(BalanceReportDto::default())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_project(name: &str, description: &str, ontologies: Vec<String>) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"name".into(), &wasm_bindgen::JsValue::from_str(name))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"description".into(), &wasm_bindgen::JsValue::from_str(description))
        .map_err(|_| "failed to build invoke args".to_string())?;
    
    let js_ontologies = js_sys::Array::new();
    for o in ontologies {
        js_ontologies.push(&wasm_bindgen::JsValue::from_str(&o));
    }
    js_sys::Reflect::set(&args, &"licensingOntologies".into(), &js_ontologies)
        .map_err(|_| "failed to build invoke args".to_string())?;

    let js = tauri_invoke("wellfair_add_project", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "project response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_project(_name: &str, _description: &str, _ontologies: Vec<String>) -> Result<HealthRecordDto, String> {
    Err("Projects require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_contribution(
    project_id: &str,
    contributor_did: &str,
    description: &str,
    effort_minutes: u32,
    capital_cents: u64,
    roi_multiplier: f32,
    privacy_level: &str,
    attached_asset_uri: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"projectId".into(), &wasm_bindgen::JsValue::from_str(project_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"contributorDid".into(), &wasm_bindgen::JsValue::from_str(contributor_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"description".into(), &wasm_bindgen::JsValue::from_str(description))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"effortMinutes".into(), &wasm_bindgen::JsValue::from(effort_minutes))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"capitalCents".into(), &wasm_bindgen::JsValue::from(capital_cents as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"roiMultiplier".into(), &wasm_bindgen::JsValue::from_f64(roi_multiplier as f64))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"privacyLevel".into(), &wasm_bindgen::JsValue::from_str(privacy_level))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(uri) = attached_asset_uri {
        js_sys::Reflect::set(&args, &"attachedAssetUri".into(), &wasm_bindgen::JsValue::from_str(uri))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_contribution", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "contribution response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_contribution(
    _project_id: &str,
    _contributor_did: &str,
    _description: &str,
    _effort_minutes: u32,
    _capital_cents: u64,
    _roi_multiplier: f32,
    _privacy_level: &str,
    _attached_asset_uri: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Contributions require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_project_obligations(limit: usize) -> Result<Vec<ObligationDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_project_obligations", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "obligations response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_project_obligations(_limit: usize) -> Result<Vec<ObligationDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn add_credential(
    issuer_did: &str,
    subject_did: &str,
    credential_type: &str,
    claims: &[(String, String)],
    expires_at_unix: Option<u32>,
) -> Result<HealthRecordDto, String> {
    let claims_json = serde_json::to_string(claims).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"issuerDid".into(), &wasm_bindgen::JsValue::from_str(issuer_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"subjectDid".into(), &wasm_bindgen::JsValue::from_str(subject_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"credentialType".into(), &wasm_bindgen::JsValue::from_str(credential_type))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"claimsJson".into(), &wasm_bindgen::JsValue::from_str(&claims_json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(exp) = expires_at_unix {
        js_sys::Reflect::set(&args, &"expiresAtUnix".into(), &wasm_bindgen::JsValue::from(exp))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "credential response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_credential(
    _issuer_did: &str,
    _subject_did: &str,
    _credential_type: &str,
    _claims: &[(String, String)],
    _expires_at_unix: Option<u32>,
) -> Result<HealthRecordDto, String> {
    Err("Credentials require the Tauri desktop host".into())
}

/// Full stored credential (from its content-addressed blob), including claim key/value pairs.
#[derive(Debug, Clone, Deserialize)]
pub struct CredentialFullDto {
    pub id: String,
    pub credential_type: String,
    pub issuer_did: String,
    pub claims: Vec<(String, String)>,
    pub verification_state: String,
}

/// A field-selected presentation â€” the disclosed subset only (NOT cryptographic disclosure).
#[derive(Debug, Clone, Deserialize)]
pub struct PresentationDto {
    pub credential_type: String,
    pub issuer_did: String,
    pub disclosed_claims: Vec<(String, String)>,
    pub verification_state: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn get_credential(record_id: &str) -> Result<Option<CredentialFullDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"recordId".into(), &wasm_bindgen::JsValue::from_str(record_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_get_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "credential response not JSON".to_string())?;
    if json.trim() == "null" {
        return Ok(None);
    }
    serde_json::from_str(&json).map(Some).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_credential(_record_id: &str) -> Result<Option<CredentialFullDto>, String> {
    Ok(None)
}

#[cfg(target_arch = "wasm32")]
pub async fn present_credential(
    record_id: &str,
    selected_keys: &[String],
) -> Result<PresentationDto, String> {
    let keys_json = serde_json::to_string(selected_keys).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"recordId".into(), &wasm_bindgen::JsValue::from_str(record_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"selectedKeysJson".into(), &wasm_bindgen::JsValue::from_str(&keys_json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_present_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "presentation response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn present_credential(
    _record_id: &str,
    _selected_keys: &[String],
) -> Result<PresentationDto, String> {
    Err("Credential presentation requires the Tauri desktop host".into())
}

