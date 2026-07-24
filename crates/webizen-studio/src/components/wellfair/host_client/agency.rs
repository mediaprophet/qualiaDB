//! Agency layer

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


/// A domain of agency (for the delegation-creation picker).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgencyDomainDto {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub consequential: bool,
    #[serde(default)]
    pub selfhood: bool,
}

/// A supported-agency delegation (display subset; the host returns the full record — serde ignores
/// the extra fields).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgencyDelegationDto {
    pub id: String,
    #[serde(default)]
    pub principal_did: String,
    #[serde(default)]
    pub agent_dids: Vec<String>,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub precedence: String,
    #[serde(default)]
    pub consent: String,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub values_anchor: String,
    #[serde(default)]
    pub valid_from_unix: u32,
}

/// The result of an ABAC access evaluation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgencyDecisionDto {
    #[serde(default)]
    pub permit: bool,
    #[serde(default)]
    pub reason: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_agency_domains() -> Result<Vec<AgencyDomainDto>, String> {
    let js = tauri_invoke("wellfair_list_agency_domains", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "agency domains not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_agency_domains() -> Result<Vec<AgencyDomainDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_agency_delegations() -> Result<Vec<AgencyDelegationDto>, String> {
    let js = tauri_invoke("wellfair_list_agency_delegations", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "agency delegations not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_agency_delegations() -> Result<Vec<AgencyDelegationDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn create_agency_delegation(
    principal_did: &str,
    domain: &str,
    values_anchor: &str,
    agent_dids_csv: &str,
    precedence: &str,
    consent: &str,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    let set = |k: &str, v: &str| {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v))
            .map_err(|_| "failed to build invoke args".to_string())
    };
    set("principalDid", principal_did)?;
    set("domain", domain)?;
    set("valuesAnchor", values_anchor)?;
    set("agentDids", agent_dids_csv)?;
    set("precedence", precedence)?;
    set("consent", consent)?;
    tauri_invoke("wellfair_create_agency_delegation", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub async fn create_agency_delegation(
    _principal_did: &str,
    _domain: &str,
    _values_anchor: &str,
    _agent_dids_csv: &str,
    _precedence: &str,
    _consent: &str,
) -> Result<(), String> {
    Err("Creating a delegation requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_agency_delegation_consent(delegation_id: &str, consent: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"delegationId".into(), &wasm_bindgen::JsValue::from_str(delegation_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"consent".into(), &wasm_bindgen::JsValue::from_str(consent))
        .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_set_agency_delegation_consent", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_agency_delegation_consent(_delegation_id: &str, _consent: &str) -> Result<(), String> {
    Err("Updating consent requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn revoke_agency_delegation(delegation_id: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"delegationId".into(), &wasm_bindgen::JsValue::from_str(delegation_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_revoke_agency_delegation", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn revoke_agency_delegation(_delegation_id: &str) -> Result<(), String> {
    Err("Revoking a delegation requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn evaluate_agency_access(
    delegation_id: &str,
    action: &str,
    data_class: &str,
) -> Result<AgencyDecisionDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"delegationId".into(), &wasm_bindgen::JsValue::from_str(delegation_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"action".into(), &wasm_bindgen::JsValue::from_str(action))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"dataClass".into(), &wasm_bindgen::JsValue::from_str(data_class))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_evaluate_agency_access", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "agency decision not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn evaluate_agency_access(
    _delegation_id: &str,
    _action: &str,
    _data_class: &str,
) -> Result<AgencyDecisionDto, String> {
    Err("Evaluating access requires the Tauri desktop host".into())
}

