#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

// --- Guardianship approval escrow (M-of-N co-signature for proxy actions; T1.5) --------------

/// A supporter records a condition on the principal's behalf. The write escrows for guardian
/// co-signature; returns the `SubmitOutcome` (Suspended with the pending proposal id).
#[command]
pub fn wellfair_propose_proxy_condition(
    app: AppHandle,
    proxy_did: String,
    label: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = wellfare_core::conditions::ConditionReport::new(label);
        let outcome = host.propose_proxy_condition(&proxy_did, &report)?;
        serde_json::to_string(&outcome).map_err(|e| e.to_string())
    })?
}

/// Pending and resolved guardianship proposals for the approval tray.
#[command]
pub fn wellfair_list_guardianship_proposals(
    app: AppHandle,
    limit: usize,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let proposals = host.list_guardianship_proposals(limit)?;
        serde_json::to_string(&proposals).map_err(|e| e.to_string())
    })?
}

/// A guardian co-signs (approve) or objects (deny). On ratification the escrowed record commits.
#[command]
pub fn wellfair_vote_guardianship_proposal(
    app: AppHandle,
    proposal_id: String,
    guardian_did: String,
    approve: bool,
    reason: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let view = host.vote_guardianship_proposal(&proposal_id, &guardian_did, approve, reason)?;
        serde_json::to_string(&view).map_err(|e| e.to_string())
    })?
}
