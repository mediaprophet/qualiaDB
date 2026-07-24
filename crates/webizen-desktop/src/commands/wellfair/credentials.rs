#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};


#[command]
pub fn wellfair_add_credential(
    app: AppHandle,
    issuer_did: String,
    subject_did: String,
    credential_type: String,
    claims_json: String,
    expires_at_unix: Option<u32>,
) -> Result<String, String> {
    let claims: Vec<(String, String)> = serde_json::from_str(&claims_json)
        .map_err(|e| format!("invalid claims JSON (expected [[key,value],…]): {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let mut cred = wellfare_core::credentials::CredentialRecord::new(
            issuer_did,
            subject_did,
            credential_type,
            wellfair_now_unix(),
        );
        cred.claims = claims;
        cred.expires_at_unix = expires_at_unix;
        let committed = host.add_credential(&cred)?;
        serde_json::to_string(&committed).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_get_credential(app: AppHandle, record_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let cred = host.get_credential(&record_id)?;
        serde_json::to_string(&cred).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_present_credential(
    app: AppHandle,
    record_id: String,
    selected_keys_json: String,
) -> Result<String, String> {
    let keys: Vec<String> = serde_json::from_str(&selected_keys_json)
        .map_err(|e| format!("invalid selected keys JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let presentation = host.present_credential(&record_id, &keys)?;
        serde_json::to_string(&presentation).map_err(|e| e.to_string())
    })?
}

