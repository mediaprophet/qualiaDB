use super::*;

fn source() -> (Vec<u8>, SafeTensorPlan) {
    let bytes = 3.25f32.to_le_bytes().to_vec();
    let plan = SafeTensorPlan {
        tensors: vec![crate::safetensor::SafeTensorEntry {
            name: "unknown".into(),
            dtype: "F32".into(),
            shape: vec![],
            begin: 0,
            end: 4,
        }],
        data_start: 0,
        is_mlx: false,
    };
    (bytes, plan)
}

#[test]
fn caller_workspace_retains_scalar_and_report_contract() {
    let (bytes, plan) = source();
    let needed = safetensor_transcode_requirements(&bytes, &plan, 8).unwrap();
    let mut layouts = [empty_layout()];
    let mut entries = [empty_entry()];
    let mut requirements = [WorkspaceRequirements::default()];
    let mut checksums = [0];
    let mut manifolds = [0; 64];
    let mut output = Vec::new();
    let report = transcode_safetensor_plan_to_p64(
        &bytes,
        &plan,
        8,
        &mut output,
        SafetensorTranscodeWorkspace {
            layouts: &mut layouts,
            entries: &mut entries,
            requirements: &mut requirements,
            checksums: &mut checksums,
            manifolds: &mut manifolds,
        },
        SafetensorTranscodeBudget {
            output_bytes: 4096,
            scratch_bytes: needed.scratch_bytes,
        },
    )
    .unwrap();
    assert_eq!(report.n_tensors, 1);
    assert_eq!(report.peak_working_bytes, 4);
    assert_eq!(report.bytes_written, output.len());
    let parsed =
        etalon_model::p64::P64View::parse(&output, etalon_model::p64::IntegrityMode::Full).unwrap();
    let tensor = parsed.tensor(0).unwrap();
    assert_eq!(tensor.entry().rank, 1);
    assert_eq!(tensor.entry().dimensions, [0; 4]);
    assert_eq!(tensor.blob(), bytes);
}

#[test]
fn workspace_budget_counts_spare_capacity_before_mutation() {
    let (bytes, plan) = source();
    let needed = safetensor_transcode_requirements(&bytes, &plan, 8).unwrap();
    let mut layouts = [empty_layout(); 2];
    let mut entries = [empty_entry()];
    let mut requirements = [WorkspaceRequirements::default()];
    let mut checksums = [91];
    let mut manifolds = [92; 64];
    let mut output = Vec::new();
    let error = transcode_safetensor_plan_to_p64(
        &bytes,
        &plan,
        8,
        &mut output,
        SafetensorTranscodeWorkspace {
            layouts: &mut layouts,
            entries: &mut entries,
            requirements: &mut requirements,
            checksums: &mut checksums,
            manifolds: &mut manifolds,
        },
        SafetensorTranscodeBudget {
            output_bytes: 4096,
            scratch_bytes: needed.scratch_bytes,
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        SafetensorTranscodeError::Validation(
            SafetensorTranscodeValidationError::ScratchBudget { .. }
        )
    ));
    assert!(output.is_empty());
    assert_eq!(checksums, [91]);
    assert_eq!(manifolds, [92; 64]);
    assert_eq!(layouts[0].blob_len, 0);
}

#[test]
fn forged_public_plan_ranges_and_dimensions_fail_closed() {
    let (bytes, mut plan) = source();
    plan.data_start = usize::MAX;
    assert_eq!(
        safetensor_transcode_requirements(&bytes, &plan, 8),
        Err(SafetensorTranscodeValidationError::DataStart)
    );
    plan.data_start = 0;
    plan.tensors[0].begin = 5;
    assert_eq!(
        safetensor_transcode_requirements(&bytes, &plan, 8),
        Err(SafetensorTranscodeValidationError::Offsets(0))
    );
    plan.tensors[0].begin = 0;
    plan.tensors[0].end = 0;
    plan.tensors[0].shape = vec![0, usize::MAX];
    if usize::BITS > 32 {
        assert_eq!(
            safetensor_transcode_requirements(&bytes, &plan, 8),
            Err(SafetensorTranscodeValidationError::Dimension)
        );
    }
}
