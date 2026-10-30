use super::{
    certify_etalon_kernel, generate_and_certify_via_etalon, generate_with_validation_via_etalon,
    BuiltinKernel, Schedule, TargetBackend,
};

#[test]
fn generated_wgsl_retains_actual_validation_evidence() {
    let (shader, validation) = generate_with_validation_via_etalon(
        BuiltinKernel::AffineF32,
        Schedule::default(),
        TargetBackend::Wgsl,
    )
    .unwrap();
    let report = validation.expect("WGSL returns its validation report");
    assert_eq!(report, certify_etalon_kernel(&shader.source).unwrap());
    assert_eq!(report.source_hash, shader.source_hash);
    assert_eq!(report.native_tool_validated, None);
    if report.naga_validated {
        assert_eq!(report.entry_points, ["affine_f32"]);
        assert!(report.binding_count > 0);
    } else {
        assert!(report.entry_points.is_empty());
        assert_eq!(report.binding_count, 0);
    }
    assert_eq!(
        shader,
        generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Wgsl,
        )
        .unwrap()
    );
}

#[test]
fn native_emission_does_not_claim_validation() {
    let (shader, validation) = generate_with_validation_via_etalon(
        BuiltinKernel::AffineF32,
        Schedule::default(),
        TargetBackend::Hlsl,
    )
    .unwrap();
    assert!(!shader.source.is_empty());
    assert_eq!(validation, None);
    assert_eq!(
        shader,
        generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Hlsl,
        )
        .unwrap()
    );
}

#[test]
fn supplied_wgsl_uses_validation_without_forge_ir() {
    let source = "@compute @workgroup_size(1) fn supplied_entry() {}";
    let report = certify_etalon_kernel(source).unwrap();
    assert_eq!(
        report.source_hash,
        blake3::hash(source.as_bytes()).to_hex().to_string()
    );
    assert_eq!(report.native_tool_validated, None);
    if report.naga_validated {
        assert_eq!(report.entry_points, ["supplied_entry"]);
        assert_eq!(report.binding_count, 0);
        assert!(certify_etalon_kernel("invalid WGSL").is_err());
    } else {
        assert!(
            !certify_etalon_kernel("invalid WGSL")
                .unwrap()
                .naga_validated
        );
    }
}
