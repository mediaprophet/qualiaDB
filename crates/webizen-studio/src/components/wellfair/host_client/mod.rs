//! Host API client â€” all operating state flows through Tauri invoke, not Dioxus authority.

use super::host_dto::{
    ActorDto, ConsentGrantDto, DelegationRuleDto, GraphCoverageDto, HealthRecordDto,
    PolicyDecisionDto, ReceiptDto, WellfairHostSnapshot,
};
use super::host_dto::ConsentGrantDraft;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

#[cfg(target_arch = "wasm32")]
pub async fn fetch_host_snapshot() -> WellfairHostSnapshot {
    match tauri_invoke("wellfair_host_snapshot", wasm_bindgen::JsValue::NULL).await {
        Ok(js) => {
            if let Some(json) = js.as_string() {
                if let Ok(parsed) = serde_json::from_str::<WellfairHostSnapshot>(&json) {
                    return parsed;
                }
            }
        }
        Err(_) => {}
    }
    super::host_dto::fixture_snapshot()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_host_snapshot() -> WellfairHostSnapshot {
    super::host_dto::fixture_snapshot()
}

#[component]
pub fn HostSnapshotProvider(children: Element) -> Element {
    let mut snapshot = use_signal(WellfairHostSnapshot::default);
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        spawn(async move {
            let next = fetch_host_snapshot().await;
            snapshot.set(next);
        });
    });

    use_context_provider(|| snapshot);
    rsx! { {children} }
}

pub fn use_host_snapshot() -> Signal<WellfairHostSnapshot> {
    consume_context::<Signal<WellfairHostSnapshot>>()
}

#[cfg(target_arch = "wasm32")]
pub async fn ingest_companion_health(bundle_json: &str) -> Result<String, String> {
    let args = wasm_bindgen::JsValue::from_str(bundle_json);
    let js = tauri_invoke("wellfair_ingest_companion_health", args)
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string()
        .ok_or_else(|| "ingest response was not a JSON string".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_companion_health(_bundle_json: &str) -> Result<String, String> {
    Err("Companion ingest requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn import_samsung_folder(folder_path: &str) -> Result<String, String> {
    let args = wasm_bindgen::JsValue::from_str(folder_path);
    let js = tauri_invoke("wellfair_import_samsung_folder", args)
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string()
        .ok_or_else(|| "import response was not a JSON string".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn import_samsung_folder(_folder_path: &str) -> Result<String, String> {
    Err("Samsung folder import requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_companion_pairing() -> Result<String, String> {
    let js = tauri_invoke("wellfair_companion_pairing", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string()
        .ok_or_else(|| "pairing response was not a JSON string".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_companion_pairing() -> Result<String, String> {
    Err("Companion pairing requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_health_records(limit: usize) -> Result<Vec<HealthRecordDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_health_records", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "health records response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_health_records(_limit: usize) -> Result<Vec<HealthRecordDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_receipts(limit: usize) -> Result<Vec<ReceiptDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_receipts", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "receipts response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_receipts(_limit: usize) -> Result<Vec<ReceiptDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_graph_coverage(limit: usize) -> Result<Vec<GraphCoverageDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_query_graph_coverage", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "coverage response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_graph_coverage(_limit: usize) -> Result<Vec<GraphCoverageDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn export_health_package(limit: usize) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_export_health_package", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string()
        .ok_or_else(|| "export response was not a JSON string".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn export_health_package(_limit: usize) -> Result<String, String> {
    Err("Health export requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_identity() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("read_identity", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    if js.is_null() || js.is_undefined() {
        return Ok(serde_json::Value::Null);
    }
    serde_wasm_bindgen::from_value(js).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_identity() -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

#[cfg(target_arch = "wasm32")]
pub async fn save_accessibility(prefs: &super::host_dto::AccessibilityPreferences) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"prefsJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_save_accessibility", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    if js.is_string() {
        Ok(())
    } else {
        Err("save_accessibility returned unexpected response".into())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn save_accessibility(_prefs: &super::host_dto::AccessibilityPreferences) -> Result<(), String> {
    Err("Accessibility save requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_directory_actors() -> Result<Vec<ActorDto>, String> {
    let js = tauri_invoke("get_directory_actors", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(js).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_directory_actors() -> Result<Vec<ActorDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_delegation_rules() -> Result<Vec<DelegationRuleDto>, String> {
    let js = tauri_invoke("get_delegation_rules", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(js).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_delegation_rules() -> Result<Vec<DelegationRuleDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct DirectoryActorWire {
    id: String,
    actor_type: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<String>,
    qualifications: Vec<String>,
    roles: Vec<String>,
    verification_status: String,
    pairwise_did: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_did_uri: Option<String>,
    routing_hints: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct DelegationRuleWire {
    id: String,
    actor_id: String,
    granted_roles: Vec<String>,
    legal_basis: String,
    privacy_mode_limit: String,
    allowed_record_types: Vec<String>,
    restricted_records: Vec<String>,
    is_active: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_directory_actor(
    name: &str,
    actor_type: &str,
    organization: Option<&str>,
    roles: &[String],
) -> Result<(), String> {
    let id = format!("actor-{}", uuid::Uuid::new_v4());
    let pairwise_did = format!("did:q42:pairwise:{id}");
    let actor = DirectoryActorWire {
        id: id.clone(),
        actor_type: actor_type.to_string(),
        name: name.to_string(),
        organization: organization.map(str::to_string),
        qualifications: vec![],
        roles: roles.to_vec(),
        verification_status: "self_asserted".into(),
        pairwise_did,
        root_did_uri: None,
        routing_hints: vec![],
    };
    let js = serde_wasm_bindgen::to_value(&serde_json::json!({ "actor": actor }))
        .map_err(|e| e.to_string())?;
    tauri_invoke("add_directory_actor", js)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_directory_actor(
    _name: &str,
    _actor_type: &str,
    _organization: Option<&str>,
    _roles: &[String],
) -> Result<(), String> {
    Err("Directory actors require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_delegation_rule(
    actor_id: &str,
    legal_basis: &str,
    granted_roles: &[String],
) -> Result<(), String> {
    let rule = DelegationRuleWire {
        id: format!("delegation-{}", uuid::Uuid::new_v4()),
        actor_id: actor_id.to_string(),
        granted_roles: granted_roles.to_vec(),
        legal_basis: legal_basis.to_string(),
        privacy_mode_limit: "minimum_projection".into(),
        allowed_record_types: vec!["health.observation".into(), "medication".into()],
        restricted_records: vec![],
        is_active: true,
    };
    let js = serde_wasm_bindgen::to_value(&serde_json::json!({ "rule": rule }))
        .map_err(|e| e.to_string())?;
    tauri_invoke("add_delegation_rule", js)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_delegation_rule(
    _actor_id: &str,
    _legal_basis: &str,
    _granted_roles: &[String],
) -> Result<(), String> {
    Err("Delegation rules require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn evaluate_policy(
    qapp_id: &str,
    scope: &str,
    sensitivity: &str,
    epistemic: &str,
) -> Result<PolicyDecisionDto, String> {
    let args = js_sys::Object::new();
    for (key, val) in [
        ("qappId", qapp_id),
        ("scope", scope),
        ("sensitivity", sensitivity),
        ("epistemic", epistemic),
    ] {
        js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_evaluate_policy", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "policy response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn evaluate_policy(
    _qapp_id: &str,
    _scope: &str,
    _sensitivity: &str,
    _epistemic: &str,
) -> Result<PolicyDecisionDto, String> {
    Err("Policy evaluation requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn grant_consent(draft: &ConsentGrantDraft, scope: &str) -> Result<ConsentGrantDto, String> {
    let json = serde_json::to_string(draft).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"draftJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"scope".into(), &wasm_bindgen::JsValue::from_str(scope))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_grant_consent", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| "grant response was not a JSON string".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn grant_consent(_draft: &ConsentGrantDraft, _scope: &str) -> Result<ConsentGrantDto, String> {
    Err("Consent grant requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn revoke_consent(grant_id: &str) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"grantId".into(),
        &wasm_bindgen::JsValue::from_str(grant_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_revoke_consent", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "revoke response was not a JSON string".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("revoked").and_then(|b| b.as_bool()).unwrap_or(false))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn revoke_consent(_grant_id: &str) -> Result<bool, String> {
    Err("Consent revoke requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_consents() -> Result<Vec<ConsentGrantDto>, String> {
    let js = tauri_invoke("wellfair_list_consents", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "consents response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_consents() -> Result<Vec<ConsentGrantDto>, String> {
    Ok(vec![])
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddConditionRequest {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icd10_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddAllergyRequest {
    pub substance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_condition(req: &AddConditionRequest) -> Result<HealthRecordDto, String> {
    let json = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"reportJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_condition", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| "add_condition response was not a JSON string".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_condition(_req: &AddConditionRequest) -> Result<HealthRecordDto, String> {
    Err("Add condition requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_allergy(req: &AddAllergyRequest) -> Result<HealthRecordDto, String> {
    let json = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"reportJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_allergy", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| "add_allergy response was not a JSON string".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_allergy(_req: &AddAllergyRequest) -> Result<HealthRecordDto, String> {
    Err("Add allergy requires the Tauri desktop host".into())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddDisputedDiagnosisRequest {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispute_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supporting_notes: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddHousingSafetyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dwelling_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homelessness: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violence_concern: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazards: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_disputed_diagnosis(req: &AddDisputedDiagnosisRequest) -> Result<HealthRecordDto, String> {
    let json = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"reportJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_disputed_diagnosis", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| "add_disputed_diagnosis response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_disputed_diagnosis(_req: &AddDisputedDiagnosisRequest) -> Result<HealthRecordDto, String> {
    Err("Disputed diagnosis requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_housing_safety(req: &AddHousingSafetyRequest) -> Result<HealthRecordDto, String> {
    let json = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"reportJson".into(), &wasm_bindgen::JsValue::from_str(&json))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_housing_safety", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| "add_housing_safety response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_housing_safety(_req: &AddHousingSafetyRequest) -> Result<HealthRecordDto, String> {
    Err("Housing/safety requires the Tauri desktop host".into())
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MedReminderPrefsDto {
    pub enabled: bool,
    pub permission_granted: bool,
    pub permission_granted_at_unix: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DueMedReminderDto {
    pub medication_id: String,
    pub medication_name: String,
    pub schedule_slot: String,
    pub minutes_until_due: i32,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_med_reminder_prefs() -> Result<MedReminderPrefsDto, String> {
    let js = tauri_invoke("wellfair_med_reminder_prefs", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "prefs response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_med_reminder_prefs() -> Result<MedReminderPrefsDto, String> {
    Ok(MedReminderPrefsDto {
        enabled: false,
        permission_granted: false,
        permission_granted_at_unix: None,
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn grant_med_reminder_permission() -> Result<MedReminderPrefsDto, String> {
    let js = tauri_invoke("wellfair_grant_med_reminder_permission", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "grant response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn grant_med_reminder_permission() -> Result<MedReminderPrefsDto, String> {
    Err("Med reminders require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_med_reminders_enabled(enabled: bool) -> Result<MedReminderPrefsDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"enabled".into(), &wasm_bindgen::JsValue::from(enabled))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_set_med_reminders_enabled", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "set enabled response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_med_reminders_enabled(_enabled: bool) -> Result<MedReminderPrefsDto, String> {
    Err("Med reminders require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_due_med_reminders(window_minutes: i32) -> Result<Vec<DueMedReminderDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"window_minutes".into(),
        &wasm_bindgen::JsValue::from(window_minutes),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_due_med_reminders", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "due reminders response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_due_med_reminders(_window_minutes: i32) -> Result<Vec<DueMedReminderDto>, String> {
    Ok(vec![])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyContactDto {
    pub id: String,
    pub display_name: String,
    pub relationship: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub created_at_unix: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepAnalyticsDto {
    pub debt: serde_json::Value,
    pub heatmap: serde_json::Value,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_medication(
    name: &str,
    dose: &str,
    route: &str,
    schedule: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    for (key, val) in [
        ("name", name),
        ("dose", dose),
        ("route", route),
        ("schedule", schedule),
    ] {
        js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_medication", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "medication response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_medication(
    _name: &str,
    _dose: &str,
    _route: &str,
    _schedule: &str,
) -> Result<HealthRecordDto, String> {
    Err("Medication requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn record_administration(
    medication_id: &str,
    medication_name: &str,
    status: &str,
    notes: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"medicationId".into(),
        &wasm_bindgen::JsValue::from_str(medication_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"medicationName".into(),
        &wasm_bindgen::JsValue::from_str(medication_name),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"status".into(),
        &wasm_bindgen::JsValue::from_str(status),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(n) = notes {
        js_sys::Reflect::set(
            &args,
            &"notes".into(),
            &wasm_bindgen::JsValue::from_str(n),
        )
        .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_record_administration", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "administration response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn record_administration(
    _medication_id: &str,
    _medication_name: &str,
    _status: &str,
    _notes: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Administration log requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_diet_entry(
    description: &str,
    meal_type: &str,
    calories_kcal: Option<u32>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"description".into(),
        &wasm_bindgen::JsValue::from_str(description),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"mealType".into(),
        &wasm_bindgen::JsValue::from_str(meal_type),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(c) = calories_kcal {
        js_sys::Reflect::set(
            &args,
            &"caloriesKcal".into(),
            &wasm_bindgen::JsValue::from(c),
        )
        .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_diet_entry", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "diet response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_diet_entry(
    _description: &str,
    _meal_type: &str,
    _calories_kcal: Option<u32>,
) -> Result<HealthRecordDto, String> {
    Err("Diet log requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_sleep_analytics() -> Result<SleepAnalyticsDto, String> {
    let js = tauri_invoke("wellfair_sleep_analytics", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "sleep analytics response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_sleep_analytics() -> Result<SleepAnalyticsDto, String> {
    Err("Sleep analytics requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_emergency_contact(
    display_name: &str,
    relationship: &str,
    phone: Option<&str>,
    email: Option<&str>,
) -> Result<EmergencyContactDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"displayName".into(),
        &wasm_bindgen::JsValue::from_str(display_name),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"relationship".into(),
        &wasm_bindgen::JsValue::from_str(relationship),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(p) = phone {
        js_sys::Reflect::set(&args, &"phone".into(), &wasm_bindgen::JsValue::from_str(p))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    if let Some(e) = email {
        js_sys::Reflect::set(&args, &"email".into(), &wasm_bindgen::JsValue::from_str(e))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_emergency_contact", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "contact response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_emergency_contact(
    _display_name: &str,
    _relationship: &str,
    _phone: Option<&str>,
    _email: Option<&str>,
) -> Result<EmergencyContactDto, String> {
    Err("Emergency contacts require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_emergency_contacts() -> Result<Vec<EmergencyContactDto>, String> {
    let js = tauri_invoke("wellfair_list_emergency_contacts", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "contacts response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_emergency_contacts() -> Result<Vec<EmergencyContactDto>, String> {
    Ok(vec![])
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SanctuaryPrefsDto {
    pub enabled: bool,
    pub locked: bool,
    pub decoy_session: bool,
    #[serde(default)]
    pub armed_at_unix: Option<u32>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct LifeEventWire {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct WelfareCaseWire {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct CaseTaskWire {
    id: String,
    case_id: String,
    title: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct WellbeingObservationWire {
    id: String,
    mood_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    intensity_1_10: Option<u8>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
struct TherapyNoteWire {
    id: String,
    notes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_label: Option<String>,
}

#[cfg(target_arch = "wasm32")]
async fn invoke_report_json(command: &str, report_json: &str) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"reportJson".into(),
        &wasm_bindgen::JsValue::from_str(report_json),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke(command, args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js
        .as_string()
        .ok_or_else(|| format!("{command} response was not JSON"))?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_life_event(title: &str, notes: Option<&str>) -> Result<HealthRecordDto, String> {
    let wire = LifeEventWire {
        id: uuid::Uuid::new_v4().to_string(),
        title: title.to_string(),
        notes: notes.map(str::to_string),
    };
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    invoke_report_json("wellfair_add_life_event", &json).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_life_event(_title: &str, _notes: Option<&str>) -> Result<HealthRecordDto, String> {
    Err("Life events require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_welfare_case(title: &str, summary: Option<&str>) -> Result<HealthRecordDto, String> {
    let wire = WelfareCaseWire {
        id: uuid::Uuid::new_v4().to_string(),
        title: title.to_string(),
        summary: summary.map(str::to_string),
    };
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    invoke_report_json("wellfair_add_welfare_case", &json).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_welfare_case(_title: &str, _summary: Option<&str>) -> Result<HealthRecordDto, String> {
    Err("Welfare cases require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_case_task(case_id: &str, title: &str) -> Result<HealthRecordDto, String> {
    let wire = CaseTaskWire {
        id: uuid::Uuid::new_v4().to_string(),
        case_id: case_id.to_string(),
        title: title.to_string(),
    };
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    invoke_report_json("wellfair_add_case_task", &json).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_case_task(_case_id: &str, _title: &str) -> Result<HealthRecordDto, String> {
    Err("Case tasks require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_wellbeing_observation(
    mood: &str,
    intensity: Option<u8>,
) -> Result<HealthRecordDto, String> {
    let wire = WellbeingObservationWire {
        id: uuid::Uuid::new_v4().to_string(),
        mood_label: mood.to_string(),
        intensity_1_10: intensity,
    };
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    invoke_report_json("wellfair_add_wellbeing_observation", &json).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_wellbeing_observation(
    _mood: &str,
    _intensity: Option<u8>,
) -> Result<HealthRecordDto, String> {
    Err("Wellbeing observations require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_therapy_note(notes: &str, provider: Option<&str>) -> Result<HealthRecordDto, String> {
    let wire = TherapyNoteWire {
        id: uuid::Uuid::new_v4().to_string(),
        notes: notes.to_string(),
        provider_label: provider.map(str::to_string),
    };
    let json = serde_json::to_string(&wire).map_err(|e| e.to_string())?;
    invoke_report_json("wellfair_add_therapy_note", &json).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_therapy_note(_notes: &str, _provider: Option<&str>) -> Result<HealthRecordDto, String> {
    Err("Therapy notes require the Tauri desktop host".into())
}


#[cfg(target_arch = "wasm32")]
pub async fn fetch_sanctuary_prefs() -> Result<SanctuaryPrefsDto, String> {
    let js = tauri_invoke("wellfair_sanctuary_prefs", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "sanctuary prefs response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_sanctuary_prefs() -> Result<SanctuaryPrefsDto, String> {
    Ok(SanctuaryPrefsDto::default())
}

#[cfg(target_arch = "wasm32")]
pub async fn setup_sanctuary(real_pin: &str, decoy_pin: &str) -> Result<SanctuaryPrefsDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"decoyPin".into(),
        &wasm_bindgen::JsValue::from_str(decoy_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_setup_sanctuary", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "setup sanctuary response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_sanctuary(_real_pin: &str, _decoy_pin: &str) -> Result<SanctuaryPrefsDto, String> {
    Err("Sanctuary setup requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn lock_sanctuary() -> Result<SanctuaryPrefsDto, String> {
    let js = tauri_invoke("wellfair_lock_sanctuary", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "lock sanctuary response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn lock_sanctuary() -> Result<SanctuaryPrefsDto, String> {
    Err("Sanctuary lock requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn unlock_sanctuary(pin: &str) -> Result<SanctuaryPrefsDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_unlock_sanctuary", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "unlock sanctuary response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn unlock_sanctuary(_pin: &str) -> Result<SanctuaryPrefsDto, String> {
    Err("Sanctuary unlock requires the Tauri desktop host".into())
}

/// Pending companion live-section request â€” mirrors `wellfare_core::live_share::LiveSectionRequest`.
#[derive(Debug, Clone, Deserialize)]
pub struct LiveShareRequestDto {
    pub id: String,
    pub device_id: String,
    pub purpose: String,
    pub requested_kinds: Vec<String>,
    pub ttl_seconds: u32,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_pending_live_shares(limit: usize) -> Result<Vec<LiveShareRequestDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_pending_live_shares", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "pending live shares response was not a JSON string".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_pending_live_shares(_limit: usize) -> Result<Vec<LiveShareRequestDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
async fn decide_live_share(
    request_id: &str,
    approved: bool,
    projection_kinds: &[String],
    reason: Option<&str>,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"requestId".into(),
        &wasm_bindgen::JsValue::from_str(request_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"approved".into(), &wasm_bindgen::JsValue::from(approved))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let kinds = js_sys::Array::new();
    for kind in projection_kinds {
        kinds.push(&wasm_bindgen::JsValue::from_str(kind));
    }
    js_sys::Reflect::set(&args, &"projectionKinds".into(), &kinds.into())
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(r) = reason {
        js_sys::Reflect::set(&args, &"reason".into(), &wasm_bindgen::JsValue::from_str(r))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_decide_live_share", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    if js.is_string() || js.is_null() || js.is_undefined() {
        Ok(())
    } else {
        Err("decide live share returned unexpected response".into())
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn approve_live_share(
    request_id: &str,
    projection_kinds: &[String],
) -> Result<(), String> {
    decide_live_share(request_id, true, projection_kinds, None).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn approve_live_share(
    _request_id: &str,
    _projection_kinds: &[String],
) -> Result<(), String> {
    Err("Live share approval requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn deny_live_share(request_id: &str, reason: &str) -> Result<(), String> {
    decide_live_share(request_id, false, &[], Some(reason)).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn deny_live_share(_request_id: &str, _reason: &str) -> Result<(), String> {
    Err("Live share denial requires the Tauri desktop host".into())
}


mod finance;
mod clinical;
mod pwa;
mod work_items;
mod guardianship;
mod keychain;
mod decoy;
mod agency;
mod wellbeing;
mod anatomy_qapp;
mod sync_backup;
mod accountability;
mod physiological;
mod anatomy_render;
mod library;
mod encryption;
mod safeguards;
mod disclosure;

pub use finance::*;
pub use clinical::*;
pub use pwa::*;
pub use work_items::*;
pub use guardianship::*;
pub use keychain::*;
pub use decoy::*;
pub use agency::*;
pub use wellbeing::*;
pub use anatomy_qapp::*;
pub use sync_backup::*;
pub use accountability::*;
pub use physiological::*;
pub use anatomy_render::*;
pub use library::*;
pub use encryption::*;
pub use safeguards::*;
pub use disclosure::*;

