#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};


/// The 17 seeded domains of agency (id/label/description/consequential/selfhood) for the picker.
#[command]
pub fn wellfair_list_agency_domains(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&host.list_agency_domains()).map_err(|e| e.to_string())
    })?
}

/// Current delegations (latest version per delegation id).
#[command]
pub fn wellfair_list_agency_delegations(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&host.list_agency_delegations(256)?).map_err(|e| e.to_string())
    })?
}

/// Create a delegation. `agent_dids` is a comma-separated DID list; `precedence` is
/// `primary|secondary|local_temporary`; `consent` is `pending|granted|withdrawn|not_required`.
#[command]
pub fn wellfair_create_agency_delegation(
    app: AppHandle,
    principal_did: String,
    domain: String,
    values_anchor: String,
    agent_dids: String,
    precedence: String,
    consent: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let agents: Vec<String> = agent_dids
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let d = host.create_agency_delegation(
            &principal_did,
            &domain,
            &values_anchor,
            agents,
            &precedence,
            &consent,
        )?;
        serde_json::to_string(&d).map_err(|e| e.to_string())
    })?
}

/// Update a delegation's consent state (`granted|withdrawn|pending|not_required`).
#[command]
pub fn wellfair_set_agency_delegation_consent(
    app: AppHandle,
    delegation_id: String,
    consent: String,
) -> Result<String, String> {
    use qualia_client_core::wellfair::api::agency_consent_from_str;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let parsed = agency_consent_from_str(&consent)?;
        let entry = host.set_agency_delegation_consent(&delegation_id, parsed)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

/// Revoke a delegation (monotonic; appends a superseding revoked version).
#[command]
pub fn wellfair_revoke_agency_delegation(
    app: AppHandle,
    delegation_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let entry = host.revoke_agency_delegation(&delegation_id)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

/// Evaluate the fail-closed ABAC for a delegation. `action` is `read|write|decide`. Returns
/// `{ "permit": bool, "reason": string }` â€” the reason names *why* access was denied.
#[command]
pub fn wellfair_evaluate_agency_access(
    app: AppHandle,
    delegation_id: String,
    action: String,
    data_class: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let decision = host.evaluate_agency_access(&delegation_id, &action, &data_class)?;
        let (permit, reason) = match decision {
            qualia_cooperative_core::agency_delegation::AccessDecision::Permit => {
                (true, String::new())
            }
            qualia_cooperative_core::agency_delegation::AccessDecision::Deny(r) => (false, r),
        };
        Ok(serde_json::json!({ "permit": permit, "reason": reason }).to_string())
    })?
}

