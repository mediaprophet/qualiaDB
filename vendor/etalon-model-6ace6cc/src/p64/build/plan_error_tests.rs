use super::*;
use crate::p64::P64TensorEntry;

const BUDGET: BuildBudget = BuildBudget {
    output_bytes: usize::MAX,
    scratch_bytes: usize::MAX,
};

fn slot() -> P64TensorEntry {
    P64TensorEntry {
        name_offset: 77,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [0; 4],
        blob_offset: 0,
        blob_size: 0,
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }
}

fn descriptor() -> TensorDescriptor<'static> {
    TensorDescriptor {
        name: b"x",
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1, 0, 0, 0],
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
        blob: &[1, 2, 3, 4],
    }
}

fn prepared<'a>(tensors: &'a [TensorDescriptor<'a>]) -> BuildInput<'a> {
    BuildInput {
        profile: BuildProfile::Safetensor,
        page_log2: 8,
        flags: 3,
        header_reserved: [0; 20],
        hparams_bytes: &[0; 64],
        manifold_bytes: &[0; 64],
        tokenizer_bytes: &[],
        tensors,
    }
}

fn sized<'a>(tensors: &'a [TensorLayout<'a>]) -> LayoutInput<'a> {
    LayoutInput {
        profile: LayoutProfile::Standard(BuildProfile::Safetensor),
        page_log2: 8,
        flags: 3,
        header_reserved: [0; 20],
        hparams_bytes: &[0; 64],
        manifold_bytes: &[0; 64],
        tokenizer_bytes: &[],
        schedule_bytes: &[],
        tensors,
    }
}

#[test]
fn detailed_prepared_success_preserves_serialized_bytes() {
    let tensors = [descriptor()];
    let mut old_slots = [slot()];
    let mut new_slots = [slot()];
    let old = plan_into(prepared(&tensors), &mut old_slots, BUDGET).unwrap();
    let new = plan_into_detailed(prepared(&tensors), &mut new_slots, BUDGET).unwrap();
    let mut old_bytes = [0; 1024];
    let mut new_bytes = [0; 1024];
    assert_eq!(
        write_into(&old, &mut old_bytes),
        write_into(&new, &mut new_bytes)
    );
    assert_eq!(old_bytes, new_bytes);
}

#[test]
fn detailed_refusal_keeps_old_order_and_scratch() {
    let mut tensors = [descriptor()];
    tensors[0].rank = 0;
    let mut input = prepared(&tensors);
    input.page_log2 = 7;
    let mut slots = [slot()];
    let error = plan_into_detailed(input, &mut slots, BUDGET).unwrap_err();
    assert_eq!(error.kind, BuildError::InvalidPageSize);
    assert_eq!(
        error.context,
        PlanErrorContext {
            field: PlanField::PageSize,
            tensor_index: None,
            operation: PlanOperation::Validate,
        }
    );
    assert_eq!(
        plan_into(input, &mut slots, BUDGET).unwrap_err(),
        error.kind
    );
    assert_eq!(slots[0].name_offset, 77);
    input.page_log2 = 8;
    let error = plan_into_detailed(input, &mut slots, BUDGET).unwrap_err();
    assert_eq!(error.kind, BuildError::InvalidRank(0));
    assert_eq!(error.context.field, PlanField::TensorRank);
    assert_eq!(error.context.tensor_index, Some(0));
    assert_eq!(slots[0].name_offset, 77);
}

#[test]
fn virtual_lengths_distinguish_blob_size_start_and_container_end() {
    for (length, count, field, index) in [
        (
            u64::from(u32::MAX) + 1,
            1,
            PlanField::TensorBlobSize,
            Some(0),
        ),
        (u64::from(u32::MAX), 2, PlanField::TensorBlobOffset, Some(1)),
        (u64::from(u32::MAX), 1, PlanField::ContainerEnd, None),
    ] {
        let mut tensors = [TensorLayout::from(descriptor()); 2];
        tensors[0].blob_len = length;
        let mut slots = [slot(); 2];
        let input = sized(&tensors[..count]);
        let error = plan_layout_into_detailed(input, &mut slots, BUDGET).unwrap_err();
        if usize::BITS == 32 && length == u64::from(u32::MAX) {
            assert_eq!(error.kind, BuildError::ArithmeticOverflow);
            assert_eq!(error.context.field, PlanField::TensorBlobEnd);
            assert_eq!(error.context.tensor_index, Some(0));
            assert_eq!(error.context.operation, PlanOperation::Add);
        } else {
            assert_eq!(error.kind, BuildError::OffsetLimit);
            assert_eq!(error.context.field, field);
            assert_eq!(error.context.tensor_index, index);
            assert_eq!(error.context.operation, PlanOperation::ConvertOffset);
        }
        assert_eq!(
            plan_layout_into(input, &mut slots, BUDGET).unwrap_err(),
            error.kind
        );
        assert!(slots.iter().all(|entry| entry.name_offset == 77));
    }
}

#[test]
fn detailed_output_budget_failure_projects_exact_numbers() {
    let tensors = [TensorLayout::from(descriptor())];
    let input = sized(&tensors);
    let budget = BuildBudget {
        output_bytes: 515,
        ..BUDGET
    };
    let mut slots = [slot()];
    let error = plan_layout_into_detailed(input, &mut slots, budget).unwrap_err();
    assert_eq!(
        error.kind,
        BuildError::OutputBudget {
            required_bytes: 516,
            available_bytes: 515
        }
    );
    assert_eq!(error.context.field, PlanField::OutputBudget);
    assert_eq!(
        plan_layout_into(input, &mut slots, budget).unwrap_err(),
        error.kind
    );
    assert_eq!(slots[0].name_offset, 77);
}
