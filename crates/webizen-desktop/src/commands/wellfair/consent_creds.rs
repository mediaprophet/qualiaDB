#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

/// Grant a consent credential to an agent over a committed payload (subject = vault owner).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_grant_consent_credential(
    app: AppHandle,
    agent_did: String,
    scope: String,
    purpose: String,
    commitment_hex: String,
    wrapped_key_hex: String,
    expiry_unix: Option<u64>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let cred = host.grant_consent_credential(
            &agent_did,
            &scope,
            &purpose,
            &commitment_hex,
            &wrapped_key_hex,
            expiry_unix,
        )?;
        serde_json::to_string(&cred).map_err(|e| e.to_string())
    })?
}

/// Revoke a consent credential â€” crypto-enforced (the wrapped key is destroyed). `{ "revoked": bool }`.
#[command]
pub fn wellfair_revoke_consent_credential(
    app: AppHandle,
    credential_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let revoked = host.revoke_consent_credential(&credential_id)?;
        serde_json::to_string(&serde_json::json!({ "revoked": revoked })).map_err(|e| e.to_string())
    })?
}

/// List stored consent credentials (active and revoked â€” revoked rows remain as the audit anchor).
#[command]
pub fn wellfair_list_consent_credentials(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let creds = host.list_consent_credentials()?;
        serde_json::to_string(&creds).map_err(|e| e.to_string())
    })?
}

/// Record an agent's conduct under a credential â€” signed, into the durable trail + tamper-evident ledger.
#[command]
pub fn wellfair_record_conduct(
    app: AppHandle,
    agent_did: String,
    credential_id: String,
    action: String,
    reason: String,
    commitment_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let record = host.record_conduct(&agent_did, &credential_id, &action, &reason, &commitment_hex)?;
        serde_json::to_string(&record).map_err(|e| e.to_string())
    })?
}

/// The audit view â€” every conduct record taken under one credential (survives its revocation).
#[command]
pub fn wellfair_conduct_audit_trail(
    app: AppHandle,
    credential_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let trail = host.conduct_audit_trail(&credential_id)?;
        serde_json::to_string(&trail).map_err(|e| e.to_string())
    })?
}

