#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_sanctuary_vault_configured(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        Ok(serde_json::json!({ "configured": host.sanctuary_vault_configured() }).to_string())
    })?
}

#[command]
pub fn wellfair_setup_sanctuary_vault(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        host.setup_sanctuary_vault(&real_pin, &decoy_pin)?;
        Ok(serde_json::json!({ "configured": true }).to_string())
    })?
}

#[command]
pub fn wellfair_sanctuary_vault_add_note(
    app: AppHandle,
    pin: String,
    body: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let lane = host.add_sanctuary_vault_note(&pin, &body)?;
        Ok(serde_json::json!({ "lane": lane }).to_string())
    })?
}

#[command]
pub fn wellfair_sanctuary_vault_list_notes(app: AppHandle, pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let (lane, notes) = host.list_sanctuary_vault_notes(&pin)?;
        Ok(serde_json::json!({ "lane": lane, "notes": notes }).to_string())
    })?
}

// --- T1.2: OS-keychain vault wrapping (opt-in, off by default; recovery-gated) ---

#[command]
pub fn wellfair_sanctuary_vault_is_keychain_wrapped(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        Ok(serde_json::json!({ "wrapped": host.sanctuary_vault_is_keychain_wrapped() }).to_string())
    })?
}

/// Create a keychain-wrapped vault; returns the one-time recovery code the user MUST record.
#[command]
pub fn wellfair_setup_sanctuary_vault_wrapped(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let recovery_code = host.setup_sanctuary_vault_wrapped(&real_pin, &decoy_pin)?;
        Ok(serde_json::json!({ "configured": true, "recovery_code": recovery_code }).to_string())
    })?
}

#[command]
pub fn wellfair_sanctuary_vault_unlock_with_recovery(
    app: AppHandle,
    pin: String,
    recovery_code: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let lane = host.sanctuary_vault_unlock_with_recovery(&pin, &recovery_code)?;
        Ok(serde_json::json!({ "lane": lane }).to_string())
    })?
}

// --- Vault v2 (S6): per-session decoy audit, realâ†’decoy curation, real-lane review, retention ---

/// Add a note attributing a **decoy** (duress) write to a per-unlock `session_ref` (git-like branch
/// in the audit DAG). Real-lane writes ignore `session_ref`.
#[command]
pub fn wellfair_sanctuary_vault_add_note_in_session(
    app: AppHandle,
    pin: String,
    body: String,
    session_ref: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let lane = host.add_sanctuary_vault_note_in_session(&pin, &body, &session_ref)?;
        Ok(serde_json::json!({ "lane": lane }).to_string())
    })?
}

/// Curate the decoy from a real session â€” seed a plausible note into the decoy lane without the
/// decoy PIN. Requires the **real** PIN.
#[command]
pub fn wellfair_curate_decoy_note(
    app: AppHandle,
    real_pin: String,
    body: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        host.curate_sanctuary_decoy_note(&real_pin, &body)?;
        Ok(serde_json::json!({ "curated": true }).to_string())
    })?
}

/// Review decoy activity from the real lane: decrypt + verify the sealed trail, advance head
/// anchors, and return the integrity verdict + decrypted actions. Requires the **real** PIN.
#[command]
pub fn wellfair_review_decoy_activity(app: AppHandle, real_pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let report = host.review_sanctuary_decoy_activity(&real_pin)?;
        serde_json::to_string(&report).map_err(|e| e.to_string())
    })?
}

/// Read the decoy-audit retention policy (real-session-only; ADR Â§8). Returns `{ "mode": "..." }`.
/// Requires the **real** PIN.
#[command]
pub fn wellfair_get_decoy_retention_mode(app: AppHandle, real_pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let mode = host.get_sanctuary_decoy_retention_mode(&real_pin)?;
        Ok(serde_json::json!({ "mode": mode }).to_string())
    })?
}

/// Set the decoy-audit retention policy (real-session-only; ADR Â§8). `mode` is `"auto_archive"` or
/// `"manual_triage"`. Requires the **real** PIN.
#[command]
pub fn wellfair_set_decoy_retention_mode(
    app: AppHandle,
    real_pin: String,
    mode: String,
) -> Result<String, String> {
    use qualia_client_core::wellfair::api::sanctuary_retention_mode_from_str;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let parsed = sanctuary_retention_mode_from_str(&mode)?;
        host.set_sanctuary_decoy_retention_mode(&real_pin, parsed)?;
        Ok(serde_json::json!({ "mode": mode }).to_string())
    })?
}

