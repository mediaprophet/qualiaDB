//! Lightweight **cooperative project registry + collaborator roster** for Talk → Projects.
//!
//! Works **without** an unlocked Sanctuary vault so cooperative help is not blocked on first-run
//! vault setup. When the vault *is* unlocked, Wellfair journal membership remains the durable
//! clinical/ledger path; this module is the always-available social roster and local project list.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// A cooperative project that exists on this device (local-first; vault optional).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub created_at: u64,
    /// `local` = created here without vault; `wellfair` = also mirrored to vault journal when unlocked.
    #[serde(default = "default_source_local")]
    pub source: String,
}

fn default_source_local() -> String {
    "local".into()
}

/// Summary row for the Projects UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub member_count: usize,
    pub source: String,
}

fn path() -> PathBuf {
    app_meta_dir().join("project_collaborators.json")
}

fn projects_path() -> PathBuf {
    app_meta_dir().join("coop_projects.json")
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

fn load_projects() -> Vec<LocalProject> {
    fs::read_to_string(projects_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_projects(rows: &[LocalProject]) -> Result<(), String> {
    let p = projects_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        p,
        serde_json::to_string_pretty(rows).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// Create (or upsert by name) a **local** cooperative project — no vault required.
pub fn create_local_project(name: &str, description: &str) -> Result<LocalProject, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("project name is required".into());
    }
    let mut all = load_projects();
    if let Some(existing) = all.iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
        return Ok(existing.clone());
    }
    let id = format!("local-{}", now());
    let proj = LocalProject {
        id: id.clone(),
        name: name.to_string(),
        description: description.trim().to_string(),
        created_at: now(),
        source: "local".into(),
    };
    all.push(proj.clone());
    save_projects(&all)?;
    // Steward: local profile DID when available.
    let profile = crate::user_profile::load_profile();
    let self_did = if profile.public_did.is_empty() {
        crate::user_profile::resolve_public_did(&profile)
    } else {
        profile.public_did.clone()
    };
    if !self_did.is_empty() {
        let _ = add(&id, name, &self_did, &profile.display_name, "steward");
    }
    Ok(proj)
}

/// Register a wellfair-backed project id into the local registry (after vault create).
pub fn register_wellfair_project(id: &str, name: &str, description: &str) -> Result<LocalProject, String> {
    let id = id.trim();
    let name = name.trim();
    if id.is_empty() || name.is_empty() {
        return Err("id and name required".into());
    }
    let mut all = load_projects();
    all.retain(|p| p.id != id);
    let proj = LocalProject {
        id: id.to_string(),
        name: name.to_string(),
        description: description.trim().to_string(),
        created_at: now(),
        source: "wellfair".into(),
    };
    all.push(proj.clone());
    save_projects(&all)?;
    Ok(proj)
}

/// List projects for the UI: local registry + any ids seen only in the collaborator roster.
pub fn list_project_summaries() -> Vec<ProjectSummary> {
    let mut by_id: BTreeMap<String, ProjectSummary> = BTreeMap::new();
    for p in load_projects() {
        let n = list(Some(&p.id)).len();
        by_id.insert(
            p.id.clone(),
            ProjectSummary {
                id: p.id,
                name: p.name,
                member_count: n,
                source: p.source,
            },
        );
    }
    for c in load() {
        by_id
            .entry(c.project_id.clone())
            .and_modify(|s| {
                if s.name.is_empty() && !c.project_name.is_empty() {
                    s.name = c.project_name.clone();
                }
                // recount below
            })
            .or_insert_with(|| ProjectSummary {
                id: c.project_id.clone(),
                name: if c.project_name.is_empty() {
                    c.project_id.clone()
                } else {
                    c.project_name.clone()
                },
                member_count: 0,
                source: "roster".into(),
            });
    }
    // Final member counts
    for s in by_id.values_mut() {
        s.member_count = list(Some(&s.id)).len();
    }
    let mut out: Vec<_> = by_id.into_values().collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
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

    #[test]
    fn create_local_project_and_summaries() {
        let name = format!("CoopTest-{}", now());
        let p = create_local_project(&name, "desc").expect("create");
        assert!(!p.id.is_empty());
        let again = create_local_project(&name, "desc").expect("idempotent");
        assert_eq!(p.id, again.id);
        let sums = list_project_summaries();
        assert!(sums.iter().any(|s| s.name == name));
    }
}
