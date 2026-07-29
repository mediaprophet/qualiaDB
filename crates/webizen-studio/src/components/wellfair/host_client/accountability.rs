//! Accountability fabric

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// Verify the tamper-evident ledger. Returns `{ "ok": bool, "tamper": <detail|null> }`.
#[cfg(target_arch = "wasm32")]
pub async fn ledger_verify() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_ledger_verify", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "ledger verify not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"limit".into(),
        &wasm_bindgen::JsValue::from_f64(limit as f64),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_ledger_entries", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "ledger entries not JSON".to_string())?;
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
        js_sys::Reflect::set(
            &args,
            &"expiryUnix".into(),
            &wasm_bindgen::JsValue::from_f64(exp as f64),
        )
        .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_grant_consent_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "grant response not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"credentialId".into(),
        &wasm_bindgen::JsValue::from_str(credential_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_revoke_consent_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "revoke response not JSON".to_string())?;
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
    let js = tauri_invoke(
        "wellfair_list_consent_credentials",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "credentials list not JSON".to_string())?;
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
    let json = js
        .as_string()
        .ok_or_else(|| "conduct response not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"credentialId".into(),
        &wasm_bindgen::JsValue::from_str(credential_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_conduct_audit_trail", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "audit trail not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"threshold".into(),
        &wasm_bindgen::JsValue::from_f64(threshold as f64),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_compute_scorecard", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "score-card not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn compute_scorecard(_threshold: usize) -> Result<serde_json::Value, String> {
    Err("The score-card requires the Tauri desktop host".into())
}

/// The person's own weight model + the seed suggestion + whether authored. `{ model, seed, authored }`.
#[cfg(target_arch = "wasm32")]
pub async fn get_weight_model() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_get_weight_model", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "weight model not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"modelJson".into(),
        &wasm_bindgen::JsValue::from_str(model_json),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_weight_model", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_weight_model(_model_json: &str) -> Result<(), String> {
    Err("The score-card requires the Tauri desktop host".into())
}

/// Reset the weight model to the seed suggestion.
#[cfg(target_arch = "wasm32")]
pub async fn reset_weight_model() -> Result<(), String> {
    tauri_invoke("wellfair_reset_weight_model", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset_weight_model() -> Result<(), String> {
    Err("The score-card requires the Tauri desktop host".into())
}
