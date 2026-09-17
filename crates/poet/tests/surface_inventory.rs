//! BASE-01 inventory integrity checks.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn inventory_matches_the_live_delegation_audit() {
    let root = manifest_dir().join("../..");
    let browser = manifest_dir().join("src/browser");
    let inventory: Value = serde_json::from_str(
        &fs::read_to_string(root.join("docs/poet/surface-inventory.json"))
            .expect("surface inventory"),
    )
    .expect("valid surface inventory JSON");

    let mut delegated_files = BTreeSet::new();
    collect_delegated_files(&browser, &mut delegated_files);
    let surfaces = inventory["surfaces"].as_array().expect("surfaces array");
    assert_eq!(
        inventory["counts"]["delegation_source_audit"].as_u64(),
        Some(delegated_files.len() as u64)
    );
    assert_eq!(
        inventory["counts"]["total_surfaces"].as_u64(),
        Some(surfaces.len() as u64)
    );
    assert_eq!(surfaces.len(), delegated_files.len() + 1);
    assert_eq!(
        inventory["counts"]["thin_generic_delegations"].as_u64(),
        Some(
            surfaces
                .iter()
                .filter(|surface| surface["generic_delegation"].as_bool() == Some(true))
                .count() as u64
        )
    );

    let mut inventory_files = BTreeSet::new();
    let mut surface_ids = BTreeSet::new();
    for surface in surfaces {
        for field in [
            "surface_id",
            "domain",
            "route_or_builder",
            "current_implementation",
            "intended_user_job",
            "record_families",
            "capabilities",
            "backend_state",
            "generic_delegation",
            "remaining_behavior",
            "uat_state",
            "status",
            "status_evidence",
        ] {
            assert!(!surface[field].is_null(), "missing {field}");
        }
        let id = surface["surface_id"].as_str().expect("surface id");
        assert!(surface_ids.insert(id), "duplicate surface id: {id}");
        let file = surface["route_or_builder"]["surface_module"]
            .as_str()
            .expect("surface module");
        assert!(
            inventory_files.insert(file.to_string()),
            "duplicate source file: {file}"
        );
        if surface["generic_delegation"].as_bool() == Some(true) {
            assert!(
                delegated_files.contains(file),
                "generic inventory source is not live: {file}"
            );
            assert_ne!(surface["status"].as_str(), Some("verified_complete"));
        }
    }
    assert!(
        delegated_files.is_subset(&inventory_files),
        "inventory omitted one or more live delegated modules"
    );

    for (file, builder) in [
        (
            "crates/poet/src/browser/project_views/budget.rs",
            "budget_workspace::build_budget_view",
        ),
        (
            "crates/poet/src/browser/health_views/health_overview.rs",
            "overview_workspace::build_health_overview_view",
        ),
        (
            "crates/poet/src/browser/health_views/conditions.rs",
            "conditions_workspace::build_conditions_view",
        ),
        (
            "crates/poet/src/browser/health_views/medications.rs",
            "medications_workspace::build_medications_view",
        ),
        (
            "crates/poet/src/browser/health_views/documents.rs",
            "documents_workspace::build_documents_view",
        ),
        (
            "crates/poet/src/browser/health_views/clinical_reports.rs",
            "reports_workspace::build_clinical_reports_view",
        ),
    ] {
        let surface = surfaces
            .iter()
            .find(|surface| surface["route_or_builder"]["surface_module"].as_str() == Some(file))
            .expect("restored exemplar in inventory");
        assert_eq!(surface["generic_delegation"], false);
        assert_eq!(surface["status"], "partial");
        assert!(surface["route_or_builder"]["delegated_builder"]
            .as_str()
            .expect("delegated builder")
            .contains(builder));
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_delegated_files(directory: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_delegated_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            if source.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("pub use ") && line.contains("build_") && line.contains("_view")
            }) {
                let repository_root = manifest_dir()
                    .join("../..")
                    .canonicalize()
                    .expect("repository root");
                let canonical_path = path.canonicalize().expect("delegated source path");
                let relative = canonical_path
                    .strip_prefix(repository_root)
                    .expect("source under repository root");
                out.insert(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}
