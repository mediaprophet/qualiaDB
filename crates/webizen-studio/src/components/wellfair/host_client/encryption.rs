//! Envelope encryption

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// The owner's envelope PUBLIC key (hex) — publishable so others can seal payloads to the owner.
#[cfg(target_arch = "wasm32")]
pub async fn owner_envelope_public() -> Result<String, String> {
    let js = tauri_invoke(
        "wellfair_owner_envelope_public",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "owner key not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("public_hex")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string())
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
        js_sys::Reflect::set(
            &args,
            &"expiryUnix".into(),
            &wasm_bindgen::JsValue::from_f64(exp as f64),
        )
        .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_seal_and_grant_credential", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "seal-grant response not JSON".to_string())?;
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
    js_sys::Reflect::set(
        &args,
        &"credentialId".into(),
        &wasm_bindgen::JsValue::from_str(credential_id),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_open_owner_payload", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "open response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("plaintext")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn open_owner_payload(_credential_id: &str) -> Result<String, String> {
    Err("Envelope encryption requires the Tauri desktop host".into())
}
