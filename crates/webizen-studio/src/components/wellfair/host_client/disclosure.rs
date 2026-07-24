//! Disclosure traceability + duty of inquiry

use super::*;

#[cfg(target_arch = "wasm32")]
use super::safeguards::invoke_str_arg;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


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
