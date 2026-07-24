//! Project collaborators

#![allow(non_snake_case)]




pub fn list_project_collaborators(project_id: Option<String>) -> Result<serde_json::Value, String> {
    let rows = crate::project_collab::list(project_id.as_deref());
    serde_json::to_value(rows).map_err(|e| e.to_string())
}

/// Local-first cooperative project list (works without vault).
pub fn list_coop_projects() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::project_collab::list_project_summaries()).map_err(|e| e.to_string())
}

/// Create a cooperative project on this device without requiring Sanctuary unlock.
pub fn create_coop_project(name: String, description: String) -> Result<serde_json::Value, String> {
    let p = crate::project_collab::create_local_project(&name, &description)?;
    serde_json::to_value(p).map_err(|e| e.to_string())
}

/// Admit a person or agent DID to a project roster (always local). When vault is unlocked,
/// the desktop also records Wellfair `ProjectMembership` via `wellfair_add_project_membership`.
pub fn add_project_collaborator(
    project_id: String,
    project_name: String,
    member_did: String,
    display_name: String,
    role: String,
) -> Result<serde_json::Value, String> {
    let row = crate::project_collab::add(
        &project_id,
        &project_name,
        &member_did,
        &display_name,
        &role,
    )?;
    serde_json::to_value(row).map_err(|e| e.to_string())
}

pub fn remove_project_collaborator(
    project_id: String,
    member_did: String,
) -> Result<serde_json::Value, String> {
    crate::project_collab::remove(&project_id, &member_did)?;
    Ok(serde_json::json!({ "removed": true, "project_id": project_id, "member_did": member_did }))
}

/// Answer a connection challenge — prove this node controls its identity key ("it's actually me").
#[cfg(not(target_arch = "wasm32"))]
pub fn answer_connection_challenge(
    challenge_json: String,
    my_did: String,
) -> Result<serde_json::Value, String> {
    let challenge: crate::handshake::Challenge =
        serde_json::from_str(&challenge_json).map_err(|e| format!("bad challenge: {e}"))?;
    let id = crate::node_identity::NodeIdentity::load_or_create()?;
    let resp = crate::handshake::answer_challenge(&challenge, &my_did, &id.signing_key());
    serde_json::to_value(resp).map_err(|e| e.to_string())
}

