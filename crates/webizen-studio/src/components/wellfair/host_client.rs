//! Host API client — all operating state flows through Tauri invoke, not Dioxus authority.

use super::host_dto::{
    ActorDto, ConsentGrantDto, DelegationRuleDto, HealthRecordDto, PolicyDecisionDto,
    ReceiptDto, WellfairHostSnapshot,
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

    use_effect(move || {
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