use super::*;
use crate::p64::{IntegrityMode, P64TensorEntry, P64View};

fn entry() -> P64TensorEntry {
    P64TensorEntry {
        name_offset: 77,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1; 4],
        blob_offset: 0,
        blob_size: 0,
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }
}

fn tensor() -> TensorDescriptor<'static> {
    TensorDescriptor {
        name: b"x",
        role_id: 9,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1, 0, 0, 0],
        source_offset: 123,
        source_name_hash: 456,
        alt_dtype: 789,
        precision_views_mask: 12,
        alt_blob_offset: 345,
        blob: &[1, 2, 3, 4],
    }
}

fn input<'a>(profile: BuildProfile, tensors: &'a [TensorDescriptor<'a>]) -> BuildInput<'a> {
    BuildInput {
        profile,
        page_log2: 8,
        flags: match profile {
            BuildProfile::GgufLayerPacked => 0x71,
            BuildProfile::Safetensor => 3,
            BuildProfile::Vision => 1,
        },
        header_reserved: [21; 20],
        hparams_bytes: &[0; 64],
        manifold_bytes: &[0; 64],
        tokenizer_bytes: &[],
        tensors,
    }
}

const BUDGET: BuildBudget = BuildBudget {
    output_bytes: 8192,
    scratch_bytes: 256,
};

#[test]
fn profile_padding_and_full_integrity() {
    let tensors = [tensor()];
    for (profile, expected_len, expected_role, name_offset) in [
        (BuildProfile::GgufLayerPacked, 576, 128, 1),
        (BuildProfile::Safetensor, 516, 0, 1),
        (BuildProfile::Vision, 768, 128, 0),
    ] {
        let mut scratch = [entry()];
        let plan = plan_into(input(profile, &tensors), &mut scratch, BUDGET).unwrap();
        assert_eq!(plan.output_len(), expected_len);
        assert_eq!(plan.header().role_table_offset, expected_role);
        assert_eq!(plan.entries()[0].name_offset, name_offset);
        let mut output = [99; 8192];
        assert_eq!(write_into(&plan, &mut output).unwrap(), expected_len);
        assert!(output[expected_len..].iter().all(|byte| *byte == 99));
        let view = P64View::parse(&output[..expected_len], IntegrityMode::Full).unwrap();
        assert_eq!(view.tensor(0).unwrap().blob(), tensors[0].blob);
        assert_eq!(view.tensor(0).unwrap().entry().alt_blob_offset, 345);
        assert_eq!(view.header().reserved, [21; 20]);
    }
}

#[test]
fn refusal_preserves_scratch_and_output() {
    let tensors = [tensor()];
    let mut scratch = [entry()];
    let budget = BuildBudget {
        output_bytes: 1,
        ..BUDGET
    };
    assert!(matches!(
        plan_into(input(BuildProfile::Vision, &tensors), &mut scratch, budget),
        Err(BuildError::OutputBudget { .. })
    ));
    assert_eq!(scratch[0].name_offset, 77);
    let plan = plan_into(input(BuildProfile::Vision, &tensors), &mut scratch, BUDGET).unwrap();
    let mut output = [42; 512];
    assert!(matches!(
        write_into(&plan, &mut output),
        Err(BuildError::OutputCapacity { .. })
    ));
    assert_eq!(output, [42; 512]);
}

#[test]
fn invalid_rank_and_structural_flags_do_not_mutate() {
    let mut tensors = [tensor()];
    tensors[0].rank = 5;
    let mut scratch = [entry()];
    assert!(matches!(
        plan_into(input(BuildProfile::Vision, &tensors), &mut scratch, BUDGET),
        Err(BuildError::InvalidRank(0))
    ));
    assert_eq!(scratch[0].name_offset, 77);
    tensors[0].rank = 1;
    let mut request = input(BuildProfile::Vision, &tensors);
    request.flags = 0x71;
    assert!(matches!(
        plan_into(request, &mut scratch, BUDGET),
        Err(BuildError::ProfileFlags)
    ));
    assert_eq!(scratch[0].name_offset, 77);
}

#[test]
fn raw_reserved_and_nonfinite_manifold_bytes_are_not_changed() {
    let tensors = [tensor()];
    let mut hp = [0; 64];
    hp[56..].fill(33);
    let mut manifold = [44; 64];
    manifold[..4].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    let mut request = input(BuildProfile::Safetensor, &tensors);
    request.hparams_bytes = &hp;
    request.manifold_bytes = &manifold;
    let mut scratch = [entry()];
    let plan = plan_into(request, &mut scratch, BUDGET).unwrap();
    let mut output = [0; 8192];
    write_into(&plan, &mut output).unwrap();
    assert_eq!(&output[64..128], &hp);
    let start = plan.header().manifold_table_offset as usize;
    assert_eq!(&output[start..start + 64], &manifold);
}

#[test]
fn offset_limit_checked_without_large_allocation() {
    let tensors = [tensor(); 4];
    let mut request = input(BuildProfile::Safetensor, &tensors);
    request.page_log2 = 30;
    let mut scratch = [entry(); 4];
    let budget = BuildBudget {
        output_bytes: usize::MAX,
        scratch_bytes: 256,
    };
    assert!(matches!(
        plan_into(request, &mut scratch, budget),
        Err(BuildError::OffsetLimit | BuildError::ArithmeticOverflow)
    ));
    assert!(scratch.iter().all(|entry| entry.name_offset == 77));
}
