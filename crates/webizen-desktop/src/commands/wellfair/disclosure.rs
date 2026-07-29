#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

// ── Disclosure traceability (ADR 0011 D5) + duty of inquiry (D8) ──

/// Record a transparency cc (the protective "I informed authority X" note).
#[command]
pub fn wellfair_record_transparency_cc(
    app: AppHandle,
    credential_id: String,
    informed_authority_did: String,
    purpose: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.record_transparency_cc(&credential_id, &informed_authority_did, &purpose)?;
        Ok("{\"recorded\":true}".into())
    })?
}

/// Record a disclosure event (access, or onward-share if `onward_to` set). Returns the event (incl. its
/// tracing fingerprint).
#[command]
pub fn wellfair_record_disclosure(
    app: AppHandle,
    commitment_hex: String,
    credential_id: String,
    recipient_did: String,
    acting_delegate_did: Option<String>,
    onward_to: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let event = host.record_disclosure(
            &commitment_hex,
            &credential_id,
            &recipient_did,
            acting_delegate_did,
            onward_to,
        )?;
        serde_json::to_string(&event).map_err(|e| e.to_string())
    })?
}

/// The disclosure chain for a payload.
#[command]
pub fn wellfair_disclosure_chain(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let chain = host.disclosure_chain(&commitment_hex)?;
        serde_json::to_string(&chain).map_err(|e| e.to_string())
    })?
}

/// The distinct actors who had access to a payload (the leak-suspect set).
#[command]
pub fn wellfair_actors_with_access(
    app: AppHandle,
    commitment_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let actors = host.actors_with_access(&commitment_hex)?;
        serde_json::to_string(&actors).map_err(|e| e.to_string())
    })?
}

/// Trace a leak by fingerprint (hex) → the disclosure + accountable actor (or null).
#[command]
pub fn wellfair_trace_leak(app: AppHandle, fingerprint_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let event = host.trace_leak(&fingerprint_hex)?;
        serde_json::to_string(&serde_json::json!({ "event": event })).map_err(|e| e.to_string())
    })?
}

/// List transparency cc records.
#[command]
pub fn wellfair_list_transparency_ccs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let ccs = host.list_transparency_ccs()?;
        serde_json::to_string(&ccs).map_err(|e| e.to_string())
    })?
}

/// Assess a duty of inquiry (JSON = `DutyOfInquiry`, `ConductAgainstDuty`) → the verdict.
#[command]
pub fn wellfair_assess_duty_of_inquiry(
    app: AppHandle,
    duty_json: String,
    conduct_json: String,
) -> Result<String, String> {
    let duty: qualia_client_core::duty_of_inquiry::DutyOfInquiry =
        serde_json::from_str(&duty_json).map_err(|e| format!("invalid duty JSON: {e}"))?;
    let conduct: qualia_client_core::duty_of_inquiry::ConductAgainstDuty =
        serde_json::from_str(&conduct_json).map_err(|e| format!("invalid conduct JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let verdict = host.assess_duty_of_inquiry(duty, conduct);
        serde_json::to_string(&serde_json::json!({ "verdict": verdict })).map_err(|e| e.to_string())
    })?
}
