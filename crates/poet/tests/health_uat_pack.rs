//! HLT-08 Health completion UAT pack — source contracts for the eight workflows.
//!
//! These tests lock the person-facing selectors and fail-closed copy. They do
//! not substitute for browser UAT against a running daemon.

use std::fs;
use std::path::PathBuf;

fn poet_src(relative: &str) -> String {
    fs::read_to_string(manifest_dir().join(relative)).unwrap_or_else(|_| panic!("{relative}"))
}

#[test]
fn add_measurement_requires_entered_values_and_units() {
    let source = poet_src("src/browser/health_views/overview_workspace.rs");
    assert!(source.contains("data-health-save"));
    assert!(source.contains("data-health-input=\"sys_bp\""));
    assert!(source.contains("data-health-input=\"dia_bp\""));
    assert!(source.contains("systolic <= diastolic"));
    assert!(source.contains("mmHg"));
    assert!(
        !source.contains("placeholder=\"120\""),
        "systolic placeholder must not present 120 mmHg as a patient value"
    );
    assert!(!source.contains("placeholder=\"80\""));
    assert!(!source.contains("placeholder=\"68\""));
}

#[test]
fn reload_refreshes_local_health_record() {
    let source = poet_src("src/browser/health_views/overview_workspace.rs");
    assert!(source.contains("data-health-refresh"));
    assert!(source.contains("refresh_health_home"));
    assert!(source.contains("Refreshing your local health record"));
}

#[test]
fn trend_and_table_are_both_present() {
    let source = poet_src("src/browser/health_views/vitals_chart.rs");
    assert!(source.contains("vitals-metric-nav"));
    assert!(source.contains("data-view-mode"));
    assert!(source.contains("Switch to accessible table"));
    assert!(source.contains("Switch to visual chart"));
}

#[test]
fn correction_appends_receipt_and_does_not_erase() {
    let inspection = poet_src("src/browser/health_views/record_inspection.rs");
    let model = poet_src("src/browser/health_views/model.rs");
    assert!(inspection.contains("does not erase or mutate original health records"));
    assert!(inspection.contains("Save correction receipt"));
    assert!(inspection.contains("health-correction-reason"));
    assert!(model.contains("health_correction"));
    assert!(model.contains("build_correction_receipt_payload"));
}

#[test]
fn grant_is_category_scoped_and_named_recipient() {
    let workspace = poet_src("src/browser/health_views/disclosure_workspace.rs");
    let model = poet_src("src/browser/health_views/disclosure_model.rs");
    assert!(workspace.contains("data-disclosure-grant"));
    assert!(workspace.contains("Grant clinical access"));
    let start = model
        .find("pub const CATEGORY_OPTIONS")
        .expect("CATEGORY_OPTIONS");
    let end = model
        .find("pub const PURPOSE_OPTIONS")
        .expect("PURPOSE_OPTIONS");
    let categories = &model[start..end];
    assert!(categories.contains("\"vitals\""));
    assert!(categories.contains("\"medications\""));
    assert!(categories.contains("\"conditions\""));
    assert!(categories.contains("\"lab_results\""));
    assert!(categories.contains("\"documents\""));
    assert!(
        !categories.contains("clinical_notes"),
        "grantable categories must match ConsentScope flags only"
    );
}

#[test]
fn revoke_is_one_action_on_an_active_grant() {
    let list = poet_src("src/browser/health_views/disclosure_list.rs");
    assert!(list.contains("data-revoke-grant"));
    assert!(list.contains("Revoke access"));
}

#[test]
fn ingest_is_extracted_text_only() {
    let source = poet_src("src/browser/health_views/documents_workspace.rs");
    assert!(source.contains("data-doc-text"));
    assert!(source.contains("Binary PDF"));
    assert!(source.contains("health-dropzone-disabled"));
    assert!(source.contains("Document.ingest"));
    assert!(
        !source.contains("type=\"file\""),
        "binary file upload must stay disabled"
    );
}

#[test]
fn offline_recovery_holds_mutation_and_does_not_invent_scores() {
    let overview = poet_src("src/browser/health_views/overview_workspace.rs");
    let calculators = poet_src("src/browser/health_views/calculators/workspace.rs");
    let documents = poet_src("src/browser/health_views/documents_workspace.rs");
    let disclosure = poet_src("src/browser/health_views/disclosure_workspace.rs");
    assert!(overview.contains("gate_offline"));
    assert!(overview.contains("unavailable until the local QualiaDB daemon is running"));
    assert!(calculators.contains("No score is invented"));
    assert!(documents.contains("gate_docs_offline"));
    assert!(disclosure.contains("gate_disclosure_offline"));
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
