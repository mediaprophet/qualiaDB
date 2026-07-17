//! Host API client — all operating state flows through Tauri invoke, not Dioxus authority.

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

/// Pending companion live-section request — mirrors `wellfare_core::live_share::LiveSectionRequest`.
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

// --- Finance / Cooperative Projects / Credentials (Phase 5 / Phase 3) ---

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

/// A field-selected presentation — the disclosed subset only (NOT cryptographic disclosure).
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

// --- Clinical documents / Welfare support / Sync inbox ---

#[cfg(target_arch = "wasm32")]
pub async fn add_clinical_report(
    title: &str,
    report_type: &str,
    body: &str,
    author_label: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"title".into(), &wasm_bindgen::JsValue::from_str(title))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"reportType".into(), &wasm_bindgen::JsValue::from_str(report_type))
        .map_err(|_| "failed to build invoke args".to_string())?;
    // 0 → the host stamps "now".
    js_sys::Reflect::set(&args, &"observedAtUnix".into(), &wasm_bindgen::JsValue::from(0u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"body".into(), &wasm_bindgen::JsValue::from_str(body))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(a) = author_label {
        js_sys::Reflect::set(&args, &"authorLabel".into(), &wasm_bindgen::JsValue::from_str(a))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_clinical_report", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "clinical response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_clinical_report(
    _title: &str,
    _report_type: &str,
    _body: &str,
    _author_label: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Clinical reports require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_clinical_attachment_from_path(
    path: &str,
    media_type: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(m) = media_type {
        js_sys::Reflect::set(&args, &"mediaType".into(), &wasm_bindgen::JsValue::from_str(m))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_clinical_attachment_from_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "attachment response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_clinical_attachment_from_path(
    _path: &str,
    _media_type: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Clinical attachments require the Tauri desktop host".into())
}

/// Export an attachment's bytes to a destination path; returns the host's JSON summary.
#[cfg(target_arch = "wasm32")]
pub async fn export_attachment(record_id: &str, dest_path: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"recordId".into(), &wasm_bindgen::JsValue::from_str(record_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"destPath".into(), &wasm_bindgen::JsValue::from_str(dest_path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_export_attachment", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string().ok_or_else(|| "export response not JSON".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn export_attachment(_record_id: &str, _dest_path: &str) -> Result<String, String> {
    Err("Attachment export requires the Tauri desktop host".into())
}

/// Parse a host `Option<String>` command response (JSON `null` or a JSON string) into
/// `Option<String>`. The dialog commands return the chosen path, or `null` on cancel.
#[cfg(target_arch = "wasm32")]
fn parse_optional_path(js: wasm_bindgen::JsValue) -> Result<Option<String>, String> {
    if js.is_null() || js.is_undefined() {
        return Ok(None);
    }
    if let Some(s) = js.as_string() {
        // Serde may hand back either a bare JS string or a JSON-encoded string.
        if let Ok(inner) = serde_json::from_str::<Option<String>>(&s) {
            return Ok(inner);
        }
        return Ok(Some(s));
    }
    serde_wasm_bindgen::from_value(js).map_err(|e| e.to_string())
}

/// Open a native OS file-open dialog on the desktop host; returns the chosen absolute path
/// (or `None` if the user cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_file_path() -> Result<Option<String>, String> {
    let js = tauri_invoke("wellfair_pick_file_path", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_file_path() -> Result<Option<String>, String> {
    Ok(None)
}

/// Open a native OS file-save dialog on the desktop host, seeded with `default_name`;
/// returns the chosen absolute path (or `None` if the user cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_save_path(default_name: &str) -> Result<Option<String>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"defaultName".into(),
        &wasm_bindgen::JsValue::from_str(default_name),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_pick_save_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_save_path(_default_name: &str) -> Result<Option<String>, String> {
    Ok(None)
}

/// Open a native OS folder-picker; returns the chosen directory (or `None` if cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_directory() -> Result<Option<String>, String> {
    let js = tauri_invoke("wellfair_pick_directory", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_directory() -> Result<Option<String>, String> {
    Ok(None)
}

// --- WP2: Package & Publish a qapp as an installable PWA bundle ---

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn publish_qapp_pwa(
    target_dir: &str,
    id: &str,
    name: &str,
    kind: &str,
    description: &str,
    capabilities: &str,
    wasm_filename: &str,
) -> Result<Vec<String>, String> {
    let args = js_sys::Object::new();
    for (k, v) in [
        ("targetDir", target_dir),
        ("id", id),
        ("name", name),
        ("kind", kind),
        ("description", description),
        ("capabilities", capabilities),
        ("wasmFilename", wasm_filename),
    ] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_publish_qapp_pwa", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub async fn publish_qapp_pwa(
    _target_dir: &str,
    _id: &str,
    _name: &str,
    _kind: &str,
    _description: &str,
    _capabilities: &str,
    _wasm_filename: &str,
) -> Result<Vec<String>, String> {
    Err("Package & Publish requires the Tauri desktop host".into())
}

// --- Cooperative work items / Kanban board ---

#[derive(Debug, Clone, Deserialize)]
pub struct BoardCardDto {
    pub work_item_id: String,
    pub title: String,
    pub item_type: String,
    pub priority: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BoardColumnDto {
    pub status: String,
    pub cards: Vec<BoardCardDto>,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_work_item(
    project_id: &str,
    item_type: &str,
    title: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"projectId".into(), &wasm_bindgen::JsValue::from_str(project_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"itemType".into(), &wasm_bindgen::JsValue::from_str(item_type))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"title".into(), &wasm_bindgen::JsValue::from_str(title))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_work_item", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "work item response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_work_item(
    _project_id: &str,
    _item_type: &str,
    _title: &str,
) -> Result<HealthRecordDto, String> {
    Err("Work items require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_work_item_status(work_item_id: &str, status: &str) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"workItemId".into(), &wasm_bindgen::JsValue::from_str(work_item_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"status".into(), &wasm_bindgen::JsValue::from_str(status))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_work_item_status", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "status response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_work_item_status(_work_item_id: &str, _status: &str) -> Result<HealthRecordDto, String> {
    Err("Work items require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_work_item_board(project_id: &str) -> Result<Vec<BoardColumnDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"projectId".into(), &wasm_bindgen::JsValue::from_str(project_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(256u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_work_item_board", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "board response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_work_item_board(_project_id: &str) -> Result<Vec<BoardColumnDto>, String> {
    Ok(vec![])
}

// --- Guardianship approval escrow (M-of-N co-signature for proxy actions; T1.5) ---

#[derive(Debug, Clone, Deserialize)]
pub struct GuardianshipProposalDto {
    pub proposal_id: String,
    pub principal_did: String,
    pub proxy_did: String,
    pub escrowed_kind: String,
    pub reason: String,
    pub created_unix: u32,
    /// "pending" | "ratified" | "denied".
    pub state: String,
    pub approvals: u8,
    pub threshold: u8,
    pub denied_by: Option<String>,
    pub denial_reason: Option<String>,
    pub committed: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn propose_proxy_condition(proxy_did: &str, label: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"proxyDid".into(), &wasm_bindgen::JsValue::from_str(proxy_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"label".into(), &wasm_bindgen::JsValue::from_str(label))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_propose_proxy_condition", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string().ok_or_else(|| "proxy proposal response not JSON".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn propose_proxy_condition(_proxy_did: &str, _label: &str) -> Result<String, String> {
    Err("Guardianship requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_guardianship_proposals(
    limit: usize,
) -> Result<Vec<GuardianshipProposalDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_guardianship_proposals", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "proposals response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_guardianship_proposals(
    _limit: usize,
) -> Result<Vec<GuardianshipProposalDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn vote_guardianship_proposal(
    proposal_id: &str,
    guardian_did: &str,
    approve: bool,
    reason: Option<&str>,
) -> Result<GuardianshipProposalDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"proposalId".into(), &wasm_bindgen::JsValue::from_str(proposal_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"guardianDid".into(), &wasm_bindgen::JsValue::from_str(guardian_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"approve".into(), &wasm_bindgen::JsValue::from_bool(approve))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(r) = reason {
        js_sys::Reflect::set(&args, &"reason".into(), &wasm_bindgen::JsValue::from_str(r))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_vote_guardianship_proposal", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "vote response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn vote_guardianship_proposal(
    _proposal_id: &str,
    _guardian_did: &str,
    _approve: bool,
    _reason: Option<&str>,
) -> Result<GuardianshipProposalDto, String> {
    Err("Guardianship requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_assistance_need(
    category: &str,
    description: &str,
    urgency: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"category".into(), &wasm_bindgen::JsValue::from_str(category))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"description".into(), &wasm_bindgen::JsValue::from_str(description))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"urgency".into(), &wasm_bindgen::JsValue::from_str(urgency))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_assistance_need", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "assistance response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_assistance_need(
    _category: &str,
    _description: &str,
    _urgency: &str,
) -> Result<HealthRecordDto, String> {
    Err("Assistance needs require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_welfare_stream(
    program_name: &str,
    reference: Option<&str>,
    status: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"programName".into(), &wasm_bindgen::JsValue::from_str(program_name))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(r) = reference {
        js_sys::Reflect::set(&args, &"reference".into(), &wasm_bindgen::JsValue::from_str(r))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    js_sys::Reflect::set(&args, &"status".into(), &wasm_bindgen::JsValue::from_str(status))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_welfare_stream", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "welfare stream response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_welfare_stream(
    _program_name: &str,
    _reference: Option<&str>,
    _status: &str,
) -> Result<HealthRecordDto, String> {
    Err("Welfare streams require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_government_letter(
    sender: &str,
    subject: &str,
    action_required: bool,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"sender".into(), &wasm_bindgen::JsValue::from_str(sender))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"subject".into(), &wasm_bindgen::JsValue::from_str(subject))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"actionRequired".into(), &wasm_bindgen::JsValue::from(action_required))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_government_letter", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "letter response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_government_letter(
    _sender: &str,
    _subject: &str,
    _action_required: bool,
) -> Result<HealthRecordDto, String> {
    Err("Government letters require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_government_letter_attachment_from_path(
    sender: &str,
    subject: &str,
    action_required: bool,
    path: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"sender".into(), &wasm_bindgen::JsValue::from_str(sender))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"subject".into(), &wasm_bindgen::JsValue::from_str(subject))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"actionRequired".into(), &wasm_bindgen::JsValue::from(action_required))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_government_letter_attachment_from_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "letter attachment response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_government_letter_attachment_from_path(
    _sender: &str,
    _subject: &str,
    _action_required: bool,
    _path: &str,
) -> Result<HealthRecordDto, String> {
    Err("Government letter attachments require the Tauri desktop host".into())
}

/// One quarantined-inbox row (subset of the host `InboxRecord`).
#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxOpDto {
    pub operation_id: String,
    pub kind: String,
    pub lamport: u64,
    pub sensitivity: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxOutcomeDto {
    pub state: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxRecordDto {
    pub operation: SyncInboxOpDto,
    pub outcome: SyncInboxOutcomeDto,
    pub admitted_unix: u32,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_sync_inbox(limit: usize) -> Result<Vec<SyncInboxRecordDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_sync_inbox", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "sync inbox response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_sync_inbox(_limit: usize) -> Result<Vec<SyncInboxRecordDto>, String> {
    Ok(vec![])
}

/// A note held in the encrypted Sanctuary vault (never leaves the desktop unencrypted).
#[derive(Debug, Clone, Deserialize)]
pub struct SanctuaryVaultNoteDto {
    pub id: String,
    pub body: String,
    pub created_at_unix: u32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Deserialize)]
struct SanctuaryVaultListDto {
    lane: String,
    notes: Vec<SanctuaryVaultNoteDto>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Deserialize)]
struct SanctuaryVaultConfiguredDto {
    configured: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_configured() -> Result<bool, String> {
    let js = tauri_invoke("wellfair_sanctuary_vault_configured", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "vault status not JSON".to_string())?;
    let dto: SanctuaryVaultConfiguredDto = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(dto.configured)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_configured() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
pub async fn setup_sanctuary_vault(real_pin: &str, decoy_pin: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"decoyPin".into(), &wasm_bindgen::JsValue::from_str(decoy_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_setup_sanctuary_vault", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_sanctuary_vault(_real_pin: &str, _decoy_pin: &str) -> Result<(), String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

// --- T1.2: OS-keychain vault wrapping (opt-in, off by default; recovery-gated) ---

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_is_keychain_wrapped() -> Result<bool, String> {
    let js = tauri_invoke("wellfair_sanctuary_vault_is_keychain_wrapped", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("wrapped").and_then(|b| b.as_bool()).unwrap_or(false))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_is_keychain_wrapped() -> Result<bool, String> {
    Ok(false)
}

/// Create a keychain-wrapped vault; returns the one-time recovery code to display to the user.
#[cfg(target_arch = "wasm32")]
pub async fn setup_sanctuary_vault_wrapped(
    real_pin: &str,
    decoy_pin: &str,
) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"decoyPin".into(), &wasm_bindgen::JsValue::from_str(decoy_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_setup_sanctuary_vault_wrapped", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("recovery_code")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_sanctuary_vault_wrapped(_real_pin: &str, _decoy_pin: &str) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_unlock_with_recovery(
    pin: &str,
    recovery_code: &str,
) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"recoveryCode".into(), &wasm_bindgen::JsValue::from_str(recovery_code))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_unlock_with_recovery", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("lane").and_then(|l| l.as_str()).unwrap_or("real").to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_unlock_with_recovery(_pin: &str, _recovery_code: &str) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

/// Returns the lane label ("real" | "decoy") the PIN opened.
#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_add_note(pin: &str, body: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"body".into(), &wasm_bindgen::JsValue::from_str(body))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_add_note", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "add note response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("lane").and_then(|l| l.as_str()).unwrap_or("real").to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_add_note(_pin: &str, _body: &str) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

/// Returns (lane label, notes) for whichever lane the PIN opens.
#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_list_notes(pin: &str) -> Result<(String, Vec<SanctuaryVaultNoteDto>), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_list_notes", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list notes response not JSON".to_string())?;
    let dto: SanctuaryVaultListDto = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok((dto.lane, dto.notes))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_list_notes(_pin: &str) -> Result<(String, Vec<SanctuaryVaultNoteDto>), String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

// --- Decoy-retention mode (real-session-only setting; vault v2 slice B) ---
// Controls what happens to intercepted decoy-session activity:
//   "auto_archive"  — a record is kept automatically (default)
//   "manual_triage" — nothing is kept until the owner reviews it next time
// The Tauri commands are wired by the orchestrator; this bridge only names them.

/// Returns the current decoy-retention mode ("auto_archive" | "manual_triage"). Real-session-only:
/// requires the real PIN (the setting lives in the real lane).
#[cfg(target_arch = "wasm32")]
pub async fn get_decoy_retention_mode(real_pin: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
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
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"mode".into(), &wasm_bindgen::JsValue::from_str(mode))
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
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
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
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"body".into(), &wasm_bindgen::JsValue::from_str(body))
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

// --- Agency layer: supported-agency delegations (ADR §7–§10) ---------------------------------

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

// --- Wellbeing self-assessment instruments (T2.2; PHQ-9 / GAD-7) -----------------------------

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

// --- 3D Anatomy Qapp (S4b) -------------------------------------------------------------------

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

// --- Sync transport (T3.1) + backup/restore (T3.3) -------------------------------------------

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
    js_sys::Reflect::set(&args, &"baseUrl".into(), &wasm_bindgen::JsValue::from_str(base_url))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"since".into(), &wasm_bindgen::JsValue::from_f64(since as f64))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sync_with_relay", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "sync response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sync_with_relay(_base_url: &str, _since: u64) -> Result<SyncSummaryDto, String> {
    Err("Sync requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn export_backup(path: &str) -> Result<BackupSummaryDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_export_backup", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "export response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn export_backup(_path: &str) -> Result<BackupSummaryDto, String> {
    Err("Backup requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn import_backup(path: &str) -> Result<BackupSummaryDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_import_backup", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "import response not JSON".to_string())?;
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
    let json = js.as_string().ok_or_else(|| "diagnostics not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_diagnostics() -> Result<DiagnosticsDto, String> {
    Ok(DiagnosticsDto::default())
}

// ── Accountability fabric (ADR 0011) — tamper-evident ledger + revocable consent credentials ──
//
// Bridges the desktop backend (store + host API + Tauri commands) to the Studio panel. Responses are
// returned as `serde_json::Value` so the panel renders from JSON without duplicating the client-core types
// as Studio DTOs. Each has a non-wasm stub (these need the Tauri host).

/// Verify the tamper-evident ledger. Returns `{ "ok": bool, "tamper": <detail|null> }`.
#[cfg(target_arch = "wasm32")]
pub async fn ledger_verify() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_ledger_verify", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "ledger verify not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn ledger_verify() -> Result<serde_json::Value, String> {
    Err("The accountability ledger requires the Tauri desktop host".into())
}

/// The most-recent ledger entries (newest first), capped to `limit`.
#[cfg(target_arch = "wasm32")]
pub async fn ledger_entries(limit: usize) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from_f64(limit as f64))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_ledger_entries", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "ledger entries not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn ledger_entries(_limit: usize) -> Result<serde_json::Value, String> {
    Err("The accountability ledger requires the Tauri desktop host".into())
}

/// Grant a consent credential to an agent over a committed payload (subject = vault owner).
#[cfg(target_arch = "wasm32")]
pub async fn grant_consent_credential(
    agent_did: &str,
    scope: &str,
    purpose: &str,
    commitment_hex: &str,
    wrapped_key_hex: &str,
    expiry_unix: Option<u64>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (key, val) in [
        ("agentDid", agent_did),
        ("scope", scope),
        ("purpose", purpose),
        ("commitmentHex", commitment_hex),
        ("wrappedKeyHex", wrapped_key_hex),
    ] {
        js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    if let Some(exp) = expiry_unix {
        js_sys::Reflect::set(&args, &"expiryUnix".into(), &wasm_bindgen::JsValue::from_f64(exp as f64))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_grant_consent_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "grant response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn grant_consent_credential(
    _agent_did: &str,
    _scope: &str,
    _purpose: &str,
    _commitment_hex: &str,
    _wrapped_key_hex: &str,
    _expiry_unix: Option<u64>,
) -> Result<serde_json::Value, String> {
    Err("Consent credentials require the Tauri desktop host".into())
}

/// Revoke a consent credential — crypto-enforced (the wrapped key is destroyed). Returns whether one was live.
#[cfg(target_arch = "wasm32")]
pub async fn revoke_consent_credential(credential_id: &str) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"credentialId".into(), &wasm_bindgen::JsValue::from_str(credential_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_revoke_consent_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "revoke response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("revoked").and_then(|x| x.as_bool()).unwrap_or(false))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn revoke_consent_credential(_credential_id: &str) -> Result<bool, String> {
    Err("Consent credentials require the Tauri desktop host".into())
}

/// List stored consent credentials (active and revoked — revoked rows remain as the audit anchor).
#[cfg(target_arch = "wasm32")]
pub async fn list_consent_credentials() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_consent_credentials", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "credentials list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_consent_credentials() -> Result<serde_json::Value, String> {
    Err("Consent credentials require the Tauri desktop host".into())
}

/// Record an agent's conduct under a credential — signed, into the durable trail + tamper-evident ledger.
#[cfg(target_arch = "wasm32")]
pub async fn record_conduct(
    agent_did: &str,
    credential_id: &str,
    action: &str,
    reason: &str,
    commitment_hex: &str,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (key, val) in [
        ("agentDid", agent_did),
        ("credentialId", credential_id),
        ("action", action),
        ("reason", reason),
        ("commitmentHex", commitment_hex),
    ] {
        js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_record_conduct", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "conduct response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn record_conduct(
    _agent_did: &str,
    _credential_id: &str,
    _action: &str,
    _reason: &str,
    _commitment_hex: &str,
) -> Result<serde_json::Value, String> {
    Err("Conduct records require the Tauri desktop host".into())
}

/// The audit view — every conduct record taken under one credential (survives its revocation).
#[cfg(target_arch = "wasm32")]
pub async fn conduct_audit_trail(credential_id: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"credentialId".into(), &wasm_bindgen::JsValue::from_str(credential_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_conduct_audit_trail", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "audit trail not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn conduct_audit_trail(_credential_id: &str) -> Result<serde_json::Value, String> {
    Err("Conduct records require the Tauri desktop host".into())
}

/// The accumulative, traceable well-being **score-card** over the person's own records — forum-internum /
/// Sanctuary-class; a set of Hypotheses + pathway-starts, never a diagnosis, never a rating.
#[cfg(target_arch = "wasm32")]
pub async fn compute_scorecard(threshold: usize) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_compute_scorecard", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "score-card not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn compute_scorecard(_threshold: usize) -> Result<serde_json::Value, String> {
    Err("The score-card requires the Tauri desktop host".into())
}

/// The person's own weight model + the seed suggestion + whether authored. `{ model, seed, authored }`.
#[cfg(target_arch = "wasm32")]
pub async fn get_weight_model() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_get_weight_model", wasm_bindgen::JsValue::NULL).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "weight model not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_weight_model() -> Result<serde_json::Value, String> {
    Err("The score-card requires the Tauri desktop host".into())
}

/// Set the person's own weight model (`model_json` = a serialized `WeightModel`).
#[cfg(target_arch = "wasm32")]
pub async fn set_weight_model(model_json: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"modelJson".into(), &wasm_bindgen::JsValue::from_str(model_json)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_weight_model", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_weight_model(_model_json: &str) -> Result<(), String> {
    Err("The score-card requires the Tauri desktop host".into())
}

/// Reset the weight model to the seed suggestion.
#[cfg(target_arch = "wasm32")]
pub async fn reset_weight_model() -> Result<(), String> {
    tauri_invoke("wellfair_reset_weight_model", wasm_bindgen::JsValue::NULL).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset_weight_model() -> Result<(), String> {
    Err("The score-card requires the Tauri desktop host".into())
}

// ── Physiological state (P6 — the reproductive-continuum declaration) ──────────────────────────

/// The person's declared physiological state + whether they've declared one. `{ state, declared }`.
#[cfg(target_arch = "wasm32")]
pub async fn get_physiological_state() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_get_physiological_state", wasm_bindgen::JsValue::NULL).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "physiological state not JSON".to_string())?;
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
    js_sys::Reflect::set(&args, &"stateJson".into(), &wasm_bindgen::JsValue::from_str(state_json)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_physiological_state", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_physiological_state(_state_json: &str) -> Result<(), String> {
    Err("The physiological state requires the Tauri desktop host".into())
}

/// Clear the declared physiological state — revert to the implicit Baseline.
#[cfg(target_arch = "wasm32")]
pub async fn reset_physiological_state() -> Result<(), String> {
    tauri_invoke("wellfair_reset_physiological_state", wasm_bindgen::JsValue::NULL).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset_physiological_state() -> Result<(), String> {
    Err("The physiological state requires the Tauri desktop host".into())
}

// ── 3D Anatomy render surface (S5.7 — whole-body percept snapshot) ────────────────────────────

/// Render the whole-body 3D Anatomy snapshot at `(azimuth, elevation)` degrees. The PNG is served at
/// `webizen://localhost/anatomy/body.png`; bump the epoch query-string to refetch after this call.
#[cfg(target_arch = "wasm32")]
pub async fn render_body_snapshot(azimuth: f64, elevation: f64) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"azimuth".into(), &wasm_bindgen::JsValue::from_f64(azimuth)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"elevation".into(), &wasm_bindgen::JsValue::from_f64(elevation)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_render_body_snapshot", args.into()).await.map_err(|e| format!("{e:?}"))?;
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
    js_sys::Reflect::set(&args, &"model".into(), &wasm_bindgen::JsValue::from_str(model)).map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_body_assets_status", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = raw.as_string().ok_or_else(|| "body assets status response not JSON".to_string())?;
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
    js_sys::Reflect::set(&args, &"model".into(), &wasm_bindgen::JsValue::from_str(model)).map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_acquire_body_assets", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = raw.as_string().ok_or_else(|| "acquire body assets response not JSON".to_string())?;
    serde_json::from_str::<AcquireReport>(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn acquire_body_assets(_model: &str) -> Result<AcquireReport, String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

/// The per-organ percepts for the cached organ set (so the browser portal knows what colour to paint each).
#[cfg(target_arch = "wasm32")]
pub async fn cached_body_organ_percepts(model: &str) -> Result<(Vec<OrganPerceptDto>, Vec<String>), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"model".into(), &wasm_bindgen::JsValue::from_str(model)).map_err(|_| "args".to_string())?;
    let raw = tauri_invoke("wellfair_cached_body_organ_percepts", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = raw.as_string().ok_or_else(|| "organ percepts response not JSON".to_string())?;
    #[derive(serde::Deserialize)]
    struct Resp { painted: Vec<OrganPerceptDto>, unmapped: Vec<String> }
    let resp: Resp = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok((resp.painted, resp.unmapped))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn cached_body_organ_percepts(_model: &str) -> Result<(Vec<OrganPerceptDto>, Vec<String>), String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

/// Clear the cache for a model.
#[cfg(target_arch = "wasm32")]
pub async fn clear_body_cache(model: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"model".into(), &wasm_bindgen::JsValue::from_str(model)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_clear_body_cache", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn clear_body_cache(_model: &str) -> Result<(), String> {
    Err("The 3D Anatomy asset cache requires the Tauri desktop host".into())
}

// ── Hypermedia asset library: ingest a document → find it by meaning ──

/// The person-authored facets attached at ingest — an optional date (timeline), place (map), project and
/// purpose. All `None`/empty ⇒ ingest derives what it can from the content alone.
#[derive(Debug, Clone, Default)]
pub struct IngestFacets {
    pub occurred_at: Option<i64>,
    pub place_label: Option<String>,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub project: Option<String>,
    pub purpose: Option<String>,
    pub sensitivity: Option<String>,
    pub section: Option<String>,
    pub commons_visibility: Option<String>,
}

/// Ingest a text document (derive topics + searchable text; guardianship flag→notify), optionally placing it
/// on the timeline/map via person-authored `facets`. Returns a summary.
#[cfg(target_arch = "wasm32")]
pub async fn ingest_document(uri: &str, media_type: &str, text: &str, guardian_did: Option<String>, facets: &IngestFacets, sensitivity: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (k, v) in [("uri", uri), ("mediaType", media_type), ("text", text), ("sensitivity", sensitivity)] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v)).map_err(|_| "args".to_string())?;
    }
    if let Some(g) = guardian_did {
        js_sys::Reflect::set(&args, &"guardianDid".into(), &wasm_bindgen::JsValue::from_str(&g)).map_err(|_| "args".to_string())?;
    }
    if let Some(t) = facets.occurred_at {
        js_sys::Reflect::set(&args, &"occurredAt".into(), &wasm_bindgen::JsValue::from_f64(t as f64)).map_err(|_| "args".to_string())?;
    }
    if let Some(l) = &facets.place_label {
        js_sys::Reflect::set(&args, &"placeLabel".into(), &wasm_bindgen::JsValue::from_str(l)).map_err(|_| "args".to_string())?;
    }
    if let Some(v) = facets.lat {
        js_sys::Reflect::set(&args, &"lat".into(), &wasm_bindgen::JsValue::from_f64(v as f64)).map_err(|_| "args".to_string())?;
    }
    if let Some(v) = facets.lon {
        js_sys::Reflect::set(&args, &"lon".into(), &wasm_bindgen::JsValue::from_f64(v as f64)).map_err(|_| "args".to_string())?;
    }
    if let Some(p) = &facets.project {
        js_sys::Reflect::set(&args, &"project".into(), &wasm_bindgen::JsValue::from_str(p)).map_err(|_| "args".to_string())?;
    }
    if let Some(p) = &facets.purpose {
        js_sys::Reflect::set(&args, &"purpose".into(), &wasm_bindgen::JsValue::from_str(p)).map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.sensitivity {
        js_sys::Reflect::set(&args, &"sensitivity".into(), &wasm_bindgen::JsValue::from_str(s)).map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.section {
        js_sys::Reflect::set(&args, &"section".into(), &wasm_bindgen::JsValue::from_str(s)).map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.commons_visibility {
        js_sys::Reflect::set(&args, &"commonsVisibility".into(), &wasm_bindgen::JsValue::from_str(s)).map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_document", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "ingest response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_document(_u: &str, _m: &str, _t: &str, _g: Option<String>, _f: &IngestFacets, _s: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Ingest a binary asset (photo/audio) from hex-encoded bytes — a photo's EXIF time/GPS auto-populate the
/// timeline/map. Returns a summary.
#[cfg(target_arch = "wasm32")]
pub async fn ingest_file_hex(uri: &str, media_type: &str, bytes_hex: &str, caption: &str, guardian_did: Option<String>, sensitivity: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (k, v) in [("uri", uri), ("mediaType", media_type), ("bytesHex", bytes_hex), ("caption", caption), ("sensitivity", sensitivity)] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v)).map_err(|_| "args".to_string())?;
    }
    if let Some(g) = guardian_did {
        js_sys::Reflect::set(&args, &"guardianDid".into(), &wasm_bindgen::JsValue::from_str(&g)).map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_file_hex", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "ingest response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_file_hex(_u: &str, _m: &str, _b: &str, _c: &str, _g: Option<String>, _s: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Search the library by facet (topic/depicts/place/project/purpose). Returns entry summaries.
#[cfg(target_arch = "wasm32")]
pub async fn search_library(facet: &str, value: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"facet".into(), &wasm_bindgen::JsValue::from_str(facet)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"value".into(), &wasm_bindgen::JsValue::from_str(value)).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_search_library", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "search response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library(_f: &str, _v: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Everything in the library (newest first). Optional section filter.
#[cfg(target_arch = "wasm32")]
pub async fn list_library() -> Result<serde_json::Value, String> {
    list_library_section(None).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_library() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_library_section(section: Option<&str>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    if let Some(s) = section {
        js_sys::Reflect::set(&args, &"section".into(), &wasm_bindgen::JsValue::from_str(s))
            .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_list_library", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_library_section(_s: Option<&str>) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_library_commons(asset_uri: &str, visibility: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"assetUri".into(), &wasm_bindgen::JsValue::from_str(asset_uri))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"visibility".into(), &wasm_bindgen::JsValue::from_str(visibility))
        .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_set_library_commons", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "commons response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_library_commons(_u: &str, _v: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn library_commons_share_card(asset_uri: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"assetUri".into(), &wasm_bindgen::JsValue::from_str(asset_uri))
        .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_library_commons_share_card", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "share card not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn library_commons_share_card(_u: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// The timeline query — entries whose event instant falls within `[start, end]` (unix seconds).
#[cfg(target_arch = "wasm32")]
pub async fn search_library_time(start: i64, end: i64) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"start".into(), &wasm_bindgen::JsValue::from_f64(start as f64)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"end".into(), &wasm_bindgen::JsValue::from_f64(end as f64)).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_search_library_time", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "timeline response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library_time(_s: i64, _e: i64) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn search_library_text(query: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"query".into(), &wasm_bindgen::JsValue::from_str(query))
        .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_search_library_text", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "search text not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library_text(_q: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Multi-facet library query. `filter` is a JSON object matching FacetFilter;
/// `sort` is newest|oldest|title_asc|title_desc|media_type|category.
#[cfg(target_arch = "wasm32")]
pub async fn query_library_faceted(
    filter: &serde_json::Value,
    sort: &str,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    let filter_json = serde_json::to_string(filter).map_err(|e| e.to_string())?;
    js_sys::Reflect::set(
        &args,
        &"filterJson".into(),
        &wasm_bindgen::JsValue::from_str(&filter_json),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"sort".into(),
        &wasm_bindgen::JsValue::from_str(sort),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_query_library_faceted", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "faceted query not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn query_library_faceted(
    _filter: &serde_json::Value,
    _sort: &str,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn seed_studio_qapps() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_seed_studio_qapps", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "seed qapps not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn seed_studio_qapps() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn ingest_legislation_text(
    text: &str,
    register_id: Option<&str>,
    jurisdiction: Option<&str>,
    title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"text".into(), &wasm_bindgen::JsValue::from_str(text))
        .map_err(|_| "args".to_string())?;
    if let Some(id) = register_id {
        js_sys::Reflect::set(&args, &"registerId".into(), &wasm_bindgen::JsValue::from_str(id))
            .map_err(|_| "args".to_string())?;
    }
    if let Some(j) = jurisdiction {
        js_sys::Reflect::set(
            &args,
            &"jurisdiction".into(),
            &wasm_bindgen::JsValue::from_str(j),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(t) = title_hint {
        js_sys::Reflect::set(&args, &"titleHint".into(), &wasm_bindgen::JsValue::from_str(t))
            .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_legislation_text", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "legislation ingest not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_legislation_text(
    _text: &str,
    _register_id: Option<&str>,
    _jurisdiction: Option<&str>,
    _title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn ingest_legislation_pdf_hex(
    hex_bytes: &str,
    register_id: Option<&str>,
    jurisdiction: Option<&str>,
    title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"hexBytes".into(),
        &wasm_bindgen::JsValue::from_str(hex_bytes),
    )
    .map_err(|_| "args".to_string())?;
    if let Some(id) = register_id {
        js_sys::Reflect::set(&args, &"registerId".into(), &wasm_bindgen::JsValue::from_str(id))
            .map_err(|_| "args".to_string())?;
    }
    if let Some(j) = jurisdiction {
        js_sys::Reflect::set(
            &args,
            &"jurisdiction".into(),
            &wasm_bindgen::JsValue::from_str(j),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(t) = title_hint {
        js_sys::Reflect::set(&args, &"titleHint".into(), &wasm_bindgen::JsValue::from_str(t))
            .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_legislation_pdf_hex", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "legislation pdf ingest not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_legislation_pdf_hex(
    _hex: &str,
    _register_id: Option<&str>,
    _jurisdiction: Option<&str>,
    _title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_qapp_catalog_categories() -> Result<serde_json::Value, String> {
    let js = tauri_invoke(
        "wellfair_list_qapp_catalog_categories",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "qapp categories not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_qapp_catalog_categories() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn library_stats() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_library_stats", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "stats not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn library_stats() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn remove_library_entry(asset_uri: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"assetUri".into(),
        &wasm_bindgen::JsValue::from_str(asset_uri),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_remove_library_entry", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "remove not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_library_entry(_u: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn export_library_graph() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_export_library_graph", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "export not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn export_library_graph() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

// ── Real envelope encryption over the consent credential (ADR 0011 D1/D2) ──

/// The owner's envelope PUBLIC key (hex) — publishable so others can seal payloads to the owner.
#[cfg(target_arch = "wasm32")]
pub async fn owner_envelope_public() -> Result<String, String> {
    let js = tauri_invoke("wellfair_owner_envelope_public", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "owner key not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("public_hex").and_then(|x| x.as_str()).unwrap_or_default().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn owner_envelope_public() -> Result<String, String> {
    Err("Envelope encryption requires the Tauri desktop host".into())
}

/// Seal a real plaintext payload and grant a consent credential over it (real envelope encryption).
#[cfg(target_arch = "wasm32")]
pub async fn seal_and_grant_credential(
    agent_did: &str,
    agent_public_hex: &str,
    scope: &str,
    purpose: &str,
    plaintext: &str,
    expiry_unix: Option<u64>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (key, val) in [
        ("agentDid", agent_did),
        ("agentPublicHex", agent_public_hex),
        ("scope", scope),
        ("purpose", purpose),
        ("plaintext", plaintext),
    ] {
        js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    if let Some(exp) = expiry_unix {
        js_sys::Reflect::set(&args, &"expiryUnix".into(), &wasm_bindgen::JsValue::from_f64(exp as f64))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_seal_and_grant_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "seal-grant response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn seal_and_grant_credential(
    _agent_did: &str,
    _agent_public_hex: &str,
    _scope: &str,
    _purpose: &str,
    _plaintext: &str,
    _expiry_unix: Option<u64>,
) -> Result<serde_json::Value, String> {
    Err("Envelope encryption requires the Tauri desktop host".into())
}

/// Open an owner-sealed payload through a credential — returns the decrypted plaintext (fails once revoked).
#[cfg(target_arch = "wasm32")]
pub async fn open_owner_payload(credential_id: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"credentialId".into(), &wasm_bindgen::JsValue::from_str(credential_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_open_owner_payload", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "open response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("plaintext").and_then(|x| x.as_str()).unwrap_or_default().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn open_owner_payload(_credential_id: &str) -> Result<String, String> {
    Err("Envelope encryption requires the Tauri desktop host".into())
}

// ── Safeguard switches (ADR 0011 D6/D7): dead-man + incapacity ──

/// Generic single-string-arg invoke returning parsed JSON `Value` (for the switch commands).
#[cfg(target_arch = "wasm32")]
async fn invoke_str_arg(cmd: &str, key: &str, val: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke(cmd, args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Arm a dead-man switch from primitive fields. `disposition` = `"make_public"` | `"release_to"`.
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn arm_dead_mans_switch(
    commitment_hex: &str,
    lapse_after_secs: u64,
    parties: Vec<String>,
    threshold: usize,
    disposition: &str,
    disposition_parties: Vec<String>,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"lapseAfterSecs".into(), &wasm_bindgen::JsValue::from_f64(lapse_after_secs as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"disposition".into(), &wasm_bindgen::JsValue::from_str(disposition))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"dispositionParties".into(), &serde_wasm_bindgen::to_value(&disposition_parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_arm_dead_mans_switch", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn arm_dead_mans_switch(
    _commitment_hex: &str,
    _lapse_after_secs: u64,
    _parties: Vec<String>,
    _threshold: usize,
    _disposition: &str,
    _disposition_parties: Vec<String>,
) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// "I'm alive" — touch a dead-man switch's heartbeat.
#[cfg(target_arch = "wasm32")]
pub async fn dead_mans_alive(commitment_hex: &str) -> Result<bool, String> {
    let v = invoke_str_arg("wellfair_dead_mans_alive", "commitmentHex", commitment_hex).await?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn dead_mans_alive(_commitment_hex: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Record a party attestation toward a dead-man switch. `kind` = `no_contact` | `believed_dead` | `abandon`.
#[cfg(target_arch = "wasm32")]
pub async fn attest_dead_mans(commitment_hex: &str, party_did: &str, kind: &str) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"partyDid".into(), &wasm_bindgen::JsValue::from_str(party_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"kind".into(), &wasm_bindgen::JsValue::from_str(kind))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_attest_dead_mans", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn attest_dead_mans(_commitment_hex: &str, _party_did: &str, _kind: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact a dead-man switch — returns the disposition JSON (or null).
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_enact_dead_mans", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// List armed dead-man switches.
#[cfg(target_arch = "wasm32")]
pub async fn list_dead_mans_switches() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_dead_mans_switches", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_dead_mans_switches() -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact a dead-man switch AND release the keys to the disposition parties. `party_keys` = `(did, pubkey_hex)`.
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans_release(commitment_hex: &str, party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"partyKeys".into(), &serde_wasm_bindgen::to_value(&party_keys).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_enact_dead_mans_release", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans_release(_commitment_hex: &str, _party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Split a payload's DEK into Shamir social-recovery shares. Returns `{ threshold, shares: [{party, share}] }`.
#[cfg(target_arch = "wasm32")]
pub async fn split_dek_recovery(commitment_hex: &str, threshold: usize, parties: Vec<String>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_split_dek_recovery", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn split_dek_recovery(_c: &str, _t: usize, _p: Vec<String>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Social-recovery enactment: reconstruct from friends' `shares` (a JSON array of Shamir shares) and release.
#[cfg(target_arch = "wasm32")]
pub async fn reconstruct_and_release(commitment_hex: &str, shares: serde_json::Value, party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"shares".into(), &serde_wasm_bindgen::to_value(&shares).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"partyKeys".into(), &serde_wasm_bindgen::to_value(&party_keys).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_reconstruct_and_release", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reconstruct_and_release(_c: &str, _s: serde_json::Value, _p: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Publish a peer's envelope (X25519) public key into their peer record (remote-key distribution).
#[cfg(target_arch = "wasm32")]
pub async fn set_peer_envelope_key(did: &str, pubkey_hex: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"did".into(), &wasm_bindgen::JsValue::from_str(did)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"pubkeyHex".into(), &wasm_bindgen::JsValue::from_str(pubkey_hex)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_peer_envelope_key", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_peer_envelope_key(_did: &str, _pubkey_hex: &str) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact + release resolving the disposition parties' keys from the peer store. `{ result, missing_keys_for }`.
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans_release_via_peers(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_enact_dead_mans_release_via_peers", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans_release_via_peers(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Arm an incapacity switch from primitive fields. `kind` = `involuntary_psychiatric` | `serious_injury` | ...
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn arm_incapacity_switch(
    principal_did: &str,
    kind: &str,
    advocate_did: &str,
    parties: Vec<String>,
    threshold: usize,
    require_official_instrument: bool,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"principalDid".into(), &wasm_bindgen::JsValue::from_str(principal_did))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"kind".into(), &wasm_bindgen::JsValue::from_str(kind))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"advocateDid".into(), &wasm_bindgen::JsValue::from_str(advocate_did))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"requireOfficialInstrument".into(), &wasm_bindgen::JsValue::from_bool(require_official_instrument))
        .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_arm_incapacity_switch", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn arm_incapacity_switch(
    _principal_did: &str,
    _kind: &str,
    _advocate_did: &str,
    _parties: Vec<String>,
    _threshold: usize,
    _require_official_instrument: bool,
) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Activate advocacy on a validated incapacity trigger.
#[cfg(target_arch = "wasm32")]
pub async fn activate_incapacity(
    principal_did: &str,
    attesting_parties: Vec<String>,
    official_instrument: Option<String>,
) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"principalDid".into(), &wasm_bindgen::JsValue::from_str(principal_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let parties = serde_wasm_bindgen::to_value(&attesting_parties).map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&args, &"attestingParties".into(), &parties)
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(instr) = official_instrument {
        js_sys::Reflect::set(&args, &"officialInstrument".into(), &wasm_bindgen::JsValue::from_str(&instr))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_activate_incapacity", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("activated").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn activate_incapacity(
    _principal_did: &str,
    _attesting_parties: Vec<String>,
    _official_instrument: Option<String>,
) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Regain capacity — the advocate stands down.
#[cfg(target_arch = "wasm32")]
pub async fn regain_capacity(principal_did: &str) -> Result<bool, String> {
    let v = invoke_str_arg("wellfair_regain_capacity", "principalDid", principal_did).await?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn regain_capacity(_principal_did: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// List armed incapacity switches.
#[cfg(target_arch = "wasm32")]
pub async fn list_incapacity_switches() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_incapacity_switches", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_incapacity_switches() -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

// ── Disclosure traceability (ADR 0011 D5) + duty of inquiry (D8) ──

/// Record a transparency cc (the protective "I informed authority X" note).
#[cfg(target_arch = "wasm32")]
pub async fn record_transparency_cc(credential_id: &str, informed_authority_did: &str, purpose: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    for (k, v) in [("credentialId", credential_id), ("informedAuthorityDid", informed_authority_did), ("purpose", purpose)] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v)).map_err(|_| "args".to_string())?;
    }
    tauri_invoke("wellfair_record_transparency_cc", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn record_transparency_cc(_c: &str, _a: &str, _p: &str) -> Result<(), String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// Record a disclosure event (onward-share if `onward_to` set). Returns the event JSON (incl. fingerprint).
#[cfg(target_arch = "wasm32")]
pub async fn record_disclosure(
    commitment_hex: &str,
    credential_id: &str,
    recipient_did: &str,
    acting_delegate_did: Option<String>,
    onward_to: Option<String>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (k, v) in [("commitmentHex", commitment_hex), ("credentialId", credential_id), ("recipientDid", recipient_did)] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v)).map_err(|_| "args".to_string())?;
    }
    if let Some(d) = acting_delegate_did {
        js_sys::Reflect::set(&args, &"actingDelegateDid".into(), &wasm_bindgen::JsValue::from_str(&d)).map_err(|_| "args".to_string())?;
    }
    if let Some(t) = onward_to {
        js_sys::Reflect::set(&args, &"onwardTo".into(), &wasm_bindgen::JsValue::from_str(&t)).map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_record_disclosure", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn record_disclosure(_c: &str, _cr: &str, _r: &str, _d: Option<String>, _o: Option<String>) -> Result<serde_json::Value, String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// The disclosure chain for a payload.
#[cfg(target_arch = "wasm32")]
pub async fn disclosure_chain(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_disclosure_chain", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn disclosure_chain(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// The distinct actors who had access to a payload (leak-suspect set).
#[cfg(target_arch = "wasm32")]
pub async fn actors_with_access(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_actors_with_access", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn actors_with_access(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// Trace a leak by fingerprint (hex) → `{ event }` (null if no match).
#[cfg(target_arch = "wasm32")]
pub async fn trace_leak(fingerprint_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_trace_leak", "fingerprintHex", fingerprint_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn trace_leak(_fingerprint_hex: &str) -> Result<serde_json::Value, String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// List transparency cc records.
#[cfg(target_arch = "wasm32")]
pub async fn list_transparency_ccs() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_transparency_ccs", wasm_bindgen::JsValue::NULL).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_transparency_ccs() -> Result<serde_json::Value, String> {
    Err("Disclosure trace requires the Tauri desktop host".into())
}

/// Assess a duty of inquiry (`duty_json`, `conduct_json`) → the verdict string.
#[cfg(target_arch = "wasm32")]
pub async fn assess_duty_of_inquiry(duty_json: &str, conduct_json: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"dutyJson".into(), &wasm_bindgen::JsValue::from_str(duty_json)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"conductJson".into(), &wasm_bindgen::JsValue::from_str(conduct_json)).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_assess_duty_of_inquiry", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("verdict").and_then(|x| x.as_str()).unwrap_or("").to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn assess_duty_of_inquiry(_duty_json: &str, _conduct_json: &str) -> Result<String, String> {
    Err("Duty of inquiry requires the Tauri desktop host".into())
}