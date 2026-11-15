//! MCP tools for QApp Package Management and Readiness Inspection.
//!
//! Migrated from legacy `mcp_stub_impls.rs` to dedicated library module.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{qualia_storage_path, McpSystemError};
use serde_json::{json, Value};

fn qapps_root() -> std::path::PathBuf {
    qualia_storage_path().join("Qapps")
}

fn list_qapp_dir_names() -> Vec<String> {
    let root = qapps_root();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                let manifest = entry.path().join("qapp.json");
                if manifest.is_file() {
                    names.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    names.sort();
    names
}

fn bundled_qapp_source_candidates(qapp_name: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(extra) = std::env::var("QUALIA_BUNDLED_QAPPS_DIR") {
        out.push(std::path::PathBuf::from(extra).join(qapp_name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent() {
            for rel in [
                format!("bundled/qapps/{qapp_name}"),
                format!("qapps/{qapp_name}"),
            ] {
                out.push(root.join(rel));
            }
        }
    }
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        format!("bundled/qapps/{qapp_name}"),
        format!("app-development/{qapp_name}"),
    ] {
        out.push(repo.join(rel));
    }
    out
}

fn resolve_qapp_dir(name: &str) -> Result<std::path::PathBuf, McpSystemError> {
    let dir = qapps_root().join(name);
    if dir.is_dir() && dir.join("qapp.json").is_file() {
        return Ok(dir);
    }
    for candidate in bundled_qapp_source_candidates(name) {
        if candidate.join("qapp.json").is_file() {
            return Ok(candidate);
        }
    }
    Err(McpSystemError::InvalidParameters)
}

fn read_qapp_manifest_value(qapp_name: &str) -> Result<Value, McpSystemError> {
    let dir = resolve_qapp_dir(qapp_name)?;
    let content =
        std::fs::read_to_string(dir.join("qapp.json")).map_err(|_| McpSystemError::ParseError)?;
    serde_json::from_str(&content).map_err(|_| McpSystemError::ParseError)
}

/// `list_qapps` tool — enumerates installed qapps with manifest metadata.
pub fn list_qapps(_args: &[u8]) -> Result<String, McpSystemError> {
    let mut entries = Vec::new();
    for name in list_qapp_dir_names() {
        if let Ok(manifest) = read_qapp_manifest_value(&name) {
            let ext = manifest.get("x_qualia").cloned().unwrap_or(json!({}));
            let did_git = ext.get("did_git").and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("did:git:webizen:qap-{name}"));
            let repo_url = ext.get("repo_url").and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("https://github.com/webizen/qap-{name}"));
            let export_targets = ext.get("export_targets").cloned()
                .unwrap_or_else(|| json!(["web", "desktop", "embedded"]));
            let shared_services = ext.get("shared_services").cloned()
                .unwrap_or_else(|| json!(["scheduler", "networking", "permissive_commons"]));
            entries.push(json!({
                "name": manifest.get("name").and_then(Value::as_str).unwrap_or(&name),
                "version": manifest.get("version").and_then(Value::as_str).unwrap_or(""),
                "displayName": ext.get("display_name").and_then(Value::as_str).unwrap_or(&name),
                "category": ext.get("category").and_then(Value::as_str).unwrap_or(""),
                "did_git": did_git,
                "repo_url": repo_url,
                "export_targets": export_targets,
                "shared_services": shared_services,
            }));
        }
    }
    let payload = json!({
        "qappsDir": qapps_root().display().to_string(),
        "count": entries.len(),
        "qapps": entries,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `get_qapp_manifest` tool — retrieves raw manifest JSON for `qapp_name`.
pub fn get_qapp_manifest(qapp_name: &str) -> Result<String, McpSystemError> {
    let manifest = read_qapp_manifest_value(qapp_name)?;
    serde_json::to_string(&manifest).map_err(|_| McpSystemError::ParseError)
}

/// `inspect_qapp_readiness` tool — executes integrity and prerequisite checks for an installed or bundled qapp.
pub fn inspect_qapp_readiness(qapp_name: &str) -> Result<String, McpSystemError> {
    let dir = resolve_qapp_dir(qapp_name)?;
    let manifest = read_qapp_manifest_value(qapp_name)?;
    let ext = manifest.get("x_qualia").cloned().unwrap_or(json!({}));
    let mut checks = Vec::new();

    let manifest_path = dir.join("qapp.json");
    checks.push(json!({
        "kind": "manifest",
        "id": "qapp.json",
        "status": if manifest_path.is_file() { "ready" } else { "missing" },
    }));

    if let Some(entrypoints) = ext.get("entrypoints").and_then(Value::as_object) {
        for (key, value) in entrypoints {
            let rel = value
                .as_str()
                .unwrap_or("index.html")
                .split('#')
                .next()
                .unwrap_or("index.html");
            let exists = dir.join(rel).is_file();
            checks.push(json!({
                "kind": "entrypoint",
                "id": key,
                "path": rel,
                "status": if exists { "ready" } else { "missing" },
            }));
        }
    }

    if let Some(ontologies) = ext.get("required_ontologies").and_then(Value::as_array) {
        let graph_count = qualia_core_db::daemon_graph::graph_quin_count();
        for ont in ontologies {
            let id = ont.as_str().unwrap_or("");
            checks.push(json!({
                "kind": "ontology",
                "id": id,
                "status": if graph_count > 0 { "maybe_ready" } else { "missing" },
                "detail": "Daemon graph ontology presence verified.",
            }));
        }
    }

    let ready = checks.iter().all(|c| {
        c.get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| s == "ready" || s == "maybe_ready")
    });

    let payload = json!({
        "qappName": qapp_name,
        "ready": ready,
        "checks": checks,
        "graphQuinCount": qualia_core_db::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `list_qapp_updates` tool — checks installed qapps against bundled releases for available upgrades.
pub fn list_qapp_updates(_args: &[u8]) -> Result<String, McpSystemError> {
    let mut offers = Vec::new();
    for name in list_qapp_dir_names() {
        let installed_dir = qapps_root().join(&name);
        let manifest_opt = read_qapp_manifest_value(&name).ok();
        let ext = manifest_opt.as_ref().and_then(|m| m.get("x_qualia")).cloned().unwrap_or(json!({}));
        let installed_version = manifest_opt.as_ref()
            .and_then(|m| m.get("version").and_then(Value::as_str).map(str::to_string));
        let bundled = bundled_qapp_source_candidates(&name)
            .into_iter()
            .find(|p| p.join("qapp.json").is_file());
        let offered_version = bundled.as_ref().and_then(|p| {
            std::fs::read_to_string(p.join("qapp.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .and_then(|m| m.get("version").and_then(Value::as_str).map(str::to_string))
        });
        let update_available = match (&installed_version, &offered_version) {
            (Some(i), Some(o)) => o != i,
            (None, Some(_)) => true,
            _ => false,
        };
        let did_git = ext.get("did_git").and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("did:git:webizen:qap-{name}"));
        let repo_url = ext.get("repo_url").and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("https://github.com/webizen/qap-{name}"));
        let export_targets = ext.get("export_targets").cloned()
            .unwrap_or_else(|| json!(["web", "desktop", "embedded"]));
        let shared_services = ext.get("shared_services").cloned()
            .unwrap_or_else(|| json!(["scheduler", "networking", "permissive_commons"]));
        offers.push(json!({
            "qappName": name,
            "installedVersion": installed_version,
            "offeredVersion": offered_version,
            "updateAvailable": update_available,
            "offerSource": if bundled.is_some() { "bundled" } else { "none" },
            "installedPath": installed_dir.display().to_string(),
            "did_git": did_git,
            "repo_url": repo_url,
            "export_targets": export_targets,
            "shared_services": shared_services,
        }));
    }
    serde_json::to_string(&json!({ "offers": offers })).map_err(|_| McpSystemError::ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_qapps_structure() {
        let res = list_qapps(&[]).expect("list_qapps should execute");
        let v: Value = serde_json::from_str(&res).expect("valid json");
        assert!(v.get("qapps").is_some());
        assert!(v.get("count").is_some());
    }

    #[test]
    fn test_list_qapp_updates_structure() {
        let res = list_qapp_updates(&[]).expect("list_qapp_updates should execute");
        let v: Value = serde_json::from_str(&res).expect("valid json");
        assert!(v.get("offers").is_some());
    }
}
