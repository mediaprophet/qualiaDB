//! Product-integrity regression gates.
//!
//! These tests intentionally encode the current thin-delegation count as
//! a ceiling, not an acceptable target. Each restored domain workflow should
//! reduce it until thin generic view replacements are gone.

use std::fs;
use std::path::{Path, PathBuf};

const GENERIC_DELEGATION_CEILING: usize = 112;

#[test]
fn generic_view_collapse_cannot_expand() {
    let browser = manifest_dir().join("src/browser");
    let mut files = Vec::new();
    collect_rs_files(&browser, &mut files);
    let count = files
        .iter()
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|source| {
                source.lines().any(|line| {
                    let line = line.trim_start();
                    line.starts_with("pub use ")
                        && line.contains("build_")
                        && line.contains("_view")
                })
            })
        })
        .count();
    assert!(
        count <= GENERIC_DELEGATION_CEILING,
        "generic view delegation count grew from the audited ceiling of {GENERIC_DELEGATION_CEILING} to {count}; restore a domain workflow instead"
    );
}

#[test]
fn reopened_completion_claims_remain_visible() {
    let root = manifest_dir().join("../..");
    let tracker = fs::read_to_string(root.join("docs/POET_UI_PARITY_IMPLEMENTATION_2026-08-27.md"))
        .expect("POET parity tracker");
    let remediation =
        fs::read_to_string(root.join("docs/POET_PRODUCT_INTEGRITY_REMEDIATION_2026-08-29.md"))
            .expect("product-integrity remediation record");
    assert!(
        tracker.contains("[R]"),
        "tracker must expose withdrawn completion claims"
    );
    assert!(
        remediation.contains("broad non-QApps completion claims withdrawn"),
        "liability correction must remain explicit"
    );
}

#[test]
fn project_budget_uses_the_domain_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/project_views/budget.rs"))
        .expect("budget route");
    assert!(source.contains("budget_workspace::build_budget_view"));
    assert!(!source.contains("persist_ledgers::build_budget_view"));
}

#[test]
fn health_overview_uses_the_person_controlled_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/health_views/health_overview.rs"))
        .expect("health overview route");
    assert!(source.contains("overview_workspace::build_health_overview_view"));
    assert!(!source.contains("persist::build_health_overview_view"));
}

#[test]
fn health_conditions_uses_the_domain_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/health_views/conditions.rs"))
        .expect("health conditions route");
    assert!(source.contains("conditions_workspace::build_conditions_view"));
    assert!(!source.contains("persist::build_conditions_view"));
}

#[test]
fn health_medications_uses_the_domain_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/health_views/medications.rs"))
        .expect("health medications route");
    assert!(source.contains("medications_workspace::build_medications_view"));
    assert!(!source.contains("persist::build_medications_view"));
}

#[test]
fn health_documents_uses_the_domain_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/health_views/documents.rs"))
        .expect("health documents route");
    assert!(source.contains("documents_workspace::build_documents_view"));
    assert!(!source.contains("persist::build_health_documents_view"));
}

#[test]
fn health_reports_uses_the_domain_workspace() {
    let source = fs::read_to_string(manifest_dir().join("src/browser/health_views/clinical_reports.rs"))
        .expect("clinical reports route");
    assert!(source.contains("reports_workspace::build_clinical_reports_view"));
    assert!(!source.contains("persist::build_clinical_reports_view"));
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
