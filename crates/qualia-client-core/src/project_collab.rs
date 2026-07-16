//! Lightweight **project collaborator roster** for Talk → Projects.
//!
//! Wellfair journal membership records are the vault-backed source of truth when the host is
//! unlocked. This module keeps a small JSON roster under `app_meta_dir` so the UI can always list
//! who was invited to a project (people + agents/bots), even when the vault is locked — and so
//! invite → "admit to project" does not depend on parsing the full health journal first.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::state::app_meta_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCollaborator {
    pub project_id: String,
    pub project_name: String,
    pub member_did: String,
    #[serde(default)]
    pub display_name: String,
    /// `steward` | `contributor` | `observer` | `agent`
    pub role: String,
    pub added_at: u64,
}

fn path() -> PathBuf {
    app_meta_dir().join("project_collaborators.json")
}

fn load() -> Vec<ProjectCollaborator> {
    fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(rows: &[ProjectCollaborator]) -> Result<(), String> {
    let p = path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        p,
        serde_json::to_string_pretty(rows).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// List collaborators, optionally filtered to one project id.
pub fn list(project_id: Option<&str>) -> Vec<ProjectCollaborator> {
    let all = load();
    match project_id {
        Some(id) if !id.is_empty() => all
            .into_iter()
            .filter(|c| c.project_id == id)
            .collect(),
        _ => all,
    }
}

/// Upsert a collaborator on a project (keyed by project_id + member_did).
pub fn add(
    project_id: &str,
    project_name: &str,
    member_did: &str,
    display_name: &str,
    role: &str,
) -> Result<ProjectCollaborator, String> {
    let project_id = project_id.trim();
    let member_did = member_did.trim();
    if project_id.is_empty() || member_did.is_empty() {
        return Err("project_id and member_did are required".into());
    }
    let role = match role.trim().to_ascii_lowercase().as_str() {
        "steward" => "steward",
        "observer" => "observer",
        "agent" => "agent",
        _ => "contributor",
    };
    let mut all = load();
    all.retain(|c| !(c.project_id == project_id && c.member_did == member_did));
    let row = ProjectCollaborator {
        project_id: project_id.to_string(),
        project_name: project_name.trim().to_string(),
        member_did: member_did.to_string(),
        display_name: display_name.trim().to_string(),
        role: role.to_string(),
        added_at: now(),
    };
    all.push(row.clone());
    save(&all)?;
    Ok(row)
}

pub fn remove(project_id: &str, member_did: &str) -> Result<(), String> {
    let mut all = load();
    let before = all.len();
    all.retain(|c| !(c.project_id == project_id && c.member_did == member_did));
    if all.len() == before {
        return Err("collaborator not found".into());
    }
    save(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_remove() {
        let pid = format!("board-test-{}", now());
        let did = format!("did:test:{}", now());
        let row = add(&pid, "Test Project", &did, "Alice", "contributor").expect("add");
        assert_eq!(row.project_id, pid);
        assert!(list(Some(&pid)).iter().any(|c| c.member_did == did));
        remove(&pid, &did).expect("remove");
        assert!(!list(Some(&pid)).iter().any(|c| c.member_did == did));
    }
}
