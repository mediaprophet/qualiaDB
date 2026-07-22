#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_add_project(
    app: AppHandle,
    name: String,
    description: String,
    licensing_ontologies: Vec<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let project = wellfare_core::projects::Project::new(name, description, licensing_ontologies, wellfair_now_unix());
        let committed = host.add_project(&project)?;
        serde_json::to_string(&committed).map_err(|e| e.to_string())
    })?
}

/// Admit a DID to a cooperative project with a role (steward | contributor | observer).
#[command]
pub fn wellfair_add_project_membership(
    app: AppHandle,
    project_id: String,
    member_did: String,
    role: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| {
                "Host API not initialized â€” unlock Sanctuary vault first (Talk â†’ Projects shows the CTA)"
                    .to_string()
            })?;
        let role = match role.trim().to_ascii_lowercase().as_str() {
            "steward" => wellfare_core::projects::ProjectRole::Steward,
            "observer" => wellfare_core::projects::ProjectRole::Observer,
            _ => wellfare_core::projects::ProjectRole::Contributor,
        };
        let membership = wellfare_core::projects::ProjectMembership::new(
            project_id,
            member_did,
            role,
            wellfair_now_unix(),
        );
        let committed = host.add_project_membership(&membership)?;
        serde_json::to_string(&committed).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_contribution(
    app: AppHandle,
    project_id: String,
    contributor_did: String,
    description: String,
    effort_minutes: u32,
    capital_cents: u64,
    roi_multiplier: f32,
    privacy_level: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
    
        let privacy = match privacy_level.as_str() {
            "Private" => wellfare_core::projects::ContributionPrivacy::Private,
            "Permissive" => wellfare_core::projects::ContributionPrivacy::Permissive,
            _ => wellfare_core::projects::ContributionPrivacy::Public,
        };

        let contribution = wellfare_core::projects::Contribution::new(
            project_id,
            contributor_did,
            description,
            effort_minutes,
            capital_cents,
            roi_multiplier,
            privacy,
            wellfair_now_unix(),
        );
        let committed = host.add_contribution(&contribution)?;
        serde_json::to_string(&committed).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_project_obligations(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        // Includes validated inbound contributions (replay-safe cross-node convergence).
        let obligations = host.synced_project_obligations(limit)?;
        serde_json::to_string(&obligations).map_err(|e| e.to_string())
    })?
}
