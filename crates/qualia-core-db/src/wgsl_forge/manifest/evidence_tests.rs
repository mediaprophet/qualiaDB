use super::*;

fn evidence_fixture() -> (GeneratedShader, ValidationReport) {
    let source = "@compute @workgroup_size(1) fn main() {}".to_string();
    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
    let shader = GeneratedShader {
        kernel_id: "evidence-fixture".to_string(),
        semantic_hash: "fixture".to_string(),
        source_hash: source_hash.clone(),
        schedule: Schedule::default(),
        source,
    };
    let report = ValidationReport {
        source_hash,
        entry_points: vec!["main".to_string()],
        binding_count: 0,
        naga_validated: true,
        native_tool_validated: None,
    };
    (shader, report)
}

fn assert_level(shader: &GeneratedShader, report: ValidationReport, level: ValidationLevel) {
    let manifest = CertificationManifest::naga_only(shader, report.clone());
    assert_eq!(manifest.validation_level, level);
    assert_eq!(manifest.validation, report);
    assert_eq!(manifest.source_hash, shader.source_hash);
    assert!(manifest.oracle.is_none());
    assert!(manifest.adapter.is_none());
}

#[test]
fn matching_naga_evidence_is_retained() {
    let (shader, report) = evidence_fixture();
    assert_level(&shader, report, ValidationLevel::NagaValidated);
}

#[test]
fn missing_or_native_validation_is_not_naga_validation() {
    let (shader, mut report) = evidence_fixture();
    report.naga_validated = false;
    assert_level(&shader, report.clone(), ValidationLevel::Generated);
    report.native_tool_validated = Some("dxc".to_string());
    assert_level(&shader, report, ValidationLevel::Generated);
}

#[test]
fn mismatched_and_stale_source_evidence_is_not_promoted() {
    let (shader, report) = evidence_fixture();
    let other_hash = blake3::hash(b"different source").to_hex().to_string();
    let mut wrong_report = report.clone();
    wrong_report.source_hash = other_hash.clone();
    assert_level(&shader, wrong_report.clone(), ValidationLevel::Generated);

    let mut wrong_shader = shader.clone();
    wrong_shader.source_hash = other_hash;
    assert_level(&wrong_shader, report.clone(), ValidationLevel::Generated);
    assert_level(&wrong_shader, wrong_report, ValidationLevel::Generated);

    let mut changed_source = shader;
    changed_source
        .source
        .push_str("\n// source changed after validation");
    assert_level(&changed_source, report, ValidationLevel::Generated);
}
