#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

/// The owner's envelope PUBLIC key (hex) â€” publishable so others can seal payloads to the owner.
#[command]
pub fn wellfair_owner_envelope_public(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&serde_json::json!({ "public_hex": host.owner_envelope_public_hex() }))
            .map_err(|e| e.to_string())
    })?
}

/// Seal a real plaintext payload and grant a consent credential over it (real envelope encryption).
/// Empty `agent_public_hex` seals to the owner (self-custody, openable here); a supplied X25519 public key
/// grants that agent access instead.
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_seal_and_grant_credential(
    app: AppHandle,
    agent_did: String,
    agent_public_hex: String,
    scope: String,
    purpose: String,
    plaintext: String,
    expiry_unix: Option<u64>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let cred = host.seal_and_grant_consent_credential(
            &agent_did,
            &agent_public_hex,
            &scope,
            &purpose,
            &plaintext,
            expiry_unix,
        )?;
        serde_json::to_string(&cred).map_err(|e| e.to_string())
    })?
}

/// Open an owner-sealed payload through a credential (works while live; fails once revoked).
#[command]
pub fn wellfair_open_owner_payload(app: AppHandle, credential_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let plaintext = host.open_owner_payload(&credential_id)?;
        serde_json::to_string(&serde_json::json!({ "plaintext": plaintext })).map_err(|e| e.to_string())
    })?
}

