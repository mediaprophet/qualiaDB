//! Seed and resolve ontology sources bundled with the desktop app.
//!
//! These sources provide an offline fallback for essential ontologies whose
//! catalog URLs may be unavailable at runtime. They are also seeded into
//! `{storage}/Index/` on startup so readiness checks can treat them as present.

use std::fs;
use std::path::{Path, PathBuf};

use crate::resource_import;

struct BundledOntologySpec {
    id: &'static str,
    rel_path: &'static str,
}

const BUNDLED_ONTOLOGIES: &[BundledOntologySpec] = &[BundledOntologySpec {
    id: "shacl",
    rel_path: "bundled/ontologies/shacl.ttl",
}];

/// Ontologies seeded into local storage when absent.
pub const DEFAULT_BUNDLED_ONTOLOGIES: &[&str] = &["shacl"];

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn ontology_spec(id: &str) -> Option<&'static BundledOntologySpec> {
    BUNDLED_ONTOLOGIES.iter().find(|spec| spec.id == id)
}

fn join_rel(root: &Path, rel: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    for segment in rel.split('/') {
        out.push(segment);
    }
    out
}

/// Resolve a bundled ontology source file from the packaged app or repo tree.
pub fn resolve_bundled_ontology_source(id: &str) -> Option<PathBuf> {
    let spec = ontology_spec(id)?;

    if let Ok(extra) = std::env::var("QUALIA_BUNDLED_ONTOLOGIES_DIR") {
        let file_name = Path::new(spec.rel_path).file_name()?;
        let candidate = PathBuf::from(extra).join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Some(root) = exe_dir() {
        for rel in [
            spec.rel_path,
            spec.rel_path
                .strip_prefix("bundled/")
                .unwrap_or(spec.rel_path),
        ] {
            let candidate = join_rel(&root, rel);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let candidate = join_rel(&repo_root, spec.rel_path);
    if candidate.is_file() {
        return Some(candidate);
    }

    None
}

fn seed_bundled_ontology_if_missing(
    storage_path: &Path,
    ontology_id: &str,
) -> Result<bool, String> {
    let q42_path = resource_import::index_dir(storage_path).join(format!("{ontology_id}.q42"));
    if q42_path.is_file() {
        return Ok(false);
    }

    let source = resolve_bundled_ontology_source(ontology_id)
        .ok_or_else(|| format!("Bundled ontology source not found for {ontology_id}"))?;

    let catalog = crate::api::load_workspace_catalog();
    let ont = catalog.find_ontology(ontology_id);
    resource_import::ingest_local_rdf(&source, ontology_id, storage_path, ont)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Seed bundled essential ontologies into `{storage}/Index/` when absent.
pub fn seed_bundled_ontologies() -> Result<Vec<String>, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    let storage_path = PathBuf::from(storage);
    fs::create_dir_all(resource_import::index_dir(&storage_path)).map_err(|e| e.to_string())?;

    let mut seeded = Vec::new();
    for ontology_id in DEFAULT_BUNDLED_ONTOLOGIES {
        match seed_bundled_ontology_if_missing(&storage_path, ontology_id) {
            Ok(true) => seeded.push((*ontology_id).to_string()),
            Ok(false) => {}
            Err(e) => eprintln!("[bundled_ontologies] skip {ontology_id}: {e}"),
        }
    }

    Ok(seeded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_shacl_source_resolves_when_tracked() {
        let tracked =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bundled/ontologies/shacl.ttl");
        if tracked.is_file() {
            let src = resolve_bundled_ontology_source("shacl");
            assert!(src.is_some(), "expected bundled SHACL path");
        }
    }
}
