//! Independent compatibility tests: detailed diagnostics must not change planning.
use etalon_model::p64::build::*;
use etalon_model::p64::P64TensorEntry;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
#[allow(dead_code)]
#[path = "support/p64_writer_fixture.rs"]
mod fixture;

struct Counter;
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
fn tick() {
    let _ = ACTIVE.try_with(|a| {
        if a.get() {
            let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        }
    });
}
// Test allocator forwards all requests unchanged and counts only this test thread.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        tick();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
const BUDGET: BuildBudget = BuildBudget {
    output_bytes: usize::MAX,
    scratch_bytes: usize::MAX,
};
const ZERO: [u8; 64] = [0; 64];
fn tensor(n: u64) -> TensorLayout<'static> {
    TensorLayout {
        name: b"raw",
        role_id: 65535,
        dtype: 65535,
        manifold_idx: 0,
        rank: 1,
        dimensions: [0; 4],
        source_offset: 19,
        source_name_hash: u64::MAX,
        alt_dtype: 9,
        precision_views_mask: 65535,
        alt_blob_offset: 77,
        blob_len: n,
    }
}
fn input<'a>(tensors: &'a [TensorLayout<'a>]) -> LayoutInput<'a> {
    LayoutInput {
        profile: LayoutProfile::Standard(BuildProfile::Safetensor),
        page_log2: 8,
        flags: 3,
        header_reserved: [37; 20],
        hparams_bytes: &ZERO,
        manifold_bytes: &ZERO,
        tokenizer_bytes: &[],
        schedule_bytes: &[],
        tensors,
    }
}
fn unchanged(entries: &[P64TensorEntry]) {
    for e in entries {
        assert_eq!(
            fixture::entry_bytes(*e),
            fixture::entry_bytes(fixture::sentinel())
        );
    }
}
fn refuse(
    input: LayoutInput<'_>,
    budget: BuildBudget,
    capacity: usize,
    expected: BuildError,
    field: PlanField,
    index: Option<usize>,
    operation: PlanOperation,
) {
    let mut old = [fixture::sentinel(); 4];
    let mut detailed = old;
    assert_eq!(
        plan_layout_into(input, &mut old[..capacity], budget).unwrap_err(),
        expected
    );
    let error = plan_layout_into_detailed(input, &mut detailed[..capacity], budget).unwrap_err();
    assert_eq!(error.kind, expected);
    assert_eq!(
        error.context,
        PlanErrorContext {
            field,
            tensor_index: index,
            operation
        }
    );
    unchanged(&old);
    unchanged(&detailed);
}
#[test]
fn frozen_validation_categories_and_context_leave_all_scratch_unchanged() {
    use PlanField::*;
    use PlanOperation::*;
    let ts = [tensor(4)];
    let base = input(&ts);
    refuse(
        input(&[]),
        BUDGET,
        4,
        BuildError::EmptyTensors,
        TensorCount,
        None,
        Validate,
    );
    refuse(
        base,
        BUDGET,
        0,
        BuildError::ScratchCapacity {
            required_entries: 1,
            available_entries: 0,
        },
        EntryScratch,
        None,
        Validate,
    );
    refuse(
        base,
        BuildBudget {
            scratch_bytes: 63,
            ..BUDGET
        },
        4,
        BuildError::ScratchBudget {
            required_bytes: 64,
            available_bytes: 63,
        },
        EntryScratch,
        None,
        Validate,
    );
    refuse(
        LayoutInput {
            page_log2: 7,
            ..base
        },
        BUDGET,
        4,
        BuildError::InvalidPageSize,
        PageSize,
        None,
        Validate,
    );
    refuse(
        LayoutInput { flags: 1, ..base },
        BUDGET,
        4,
        BuildError::ProfileFlags,
        ProfileFlags,
        None,
        Validate,
    );
    refuse(
        LayoutInput {
            tokenizer_bytes: b"x",
            ..base
        },
        BUDGET,
        4,
        BuildError::ProfileSections,
        ProfileSections,
        None,
        Validate,
    );
    refuse(
        LayoutInput {
            manifold_bytes: &[],
            ..base
        },
        BUDGET,
        4,
        BuildError::ManifoldCount,
        ManifoldTable,
        None,
        Validate,
    );
    refuse(
        LayoutInput {
            schedule_bytes: &ZERO,
            ..base
        },
        BUDGET,
        4,
        BuildError::ProfileSections,
        ScheduleTable,
        None,
        Validate,
    );
    let bad = [
        tensor(4),
        TensorLayout {
            rank: 5,
            ..tensor(4)
        },
    ];
    refuse(
        input(&bad),
        BUDGET,
        4,
        BuildError::InvalidRank(1),
        TensorRank,
        Some(1),
        Validate,
    );
    let bad = [
        tensor(4),
        TensorLayout {
            manifold_idx: 1,
            ..tensor(4)
        },
    ];
    refuse(
        input(&bad),
        BUDGET,
        4,
        BuildError::InvalidManifold(1),
        TensorManifold,
        Some(1),
        Validate,
    );
    let mut hp = ZERO;
    hp[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    refuse(
        LayoutInput {
            hparams_bytes: &hp,
            ..base
        },
        BUDGET,
        4,
        BuildError::ArithmeticOverflow,
        ManifoldCount,
        None,
        Add,
    );
    // One raw four-byte blob starts at512, with no tail padding.
    refuse(
        base,
        BuildBudget {
            output_bytes: 515,
            ..BUDGET
        },
        4,
        BuildError::OutputBudget {
            required_bytes: 516,
            available_bytes: 515,
        },
        OutputBudget,
        None,
        Validate,
    );
}
#[test]
fn virtual_lengths_distinguish_blob_size_offset_and_final_end_without_payload_allocations() {
    use PlanField::*;
    use PlanOperation::*;
    let ts = [tensor(u32::MAX as u64 + 1)];
    refuse(
        input(&ts),
        BUDGET,
        4,
        BuildError::OffsetLimit,
        TensorBlobSize,
        Some(0),
        ConvertOffset,
    );
    #[cfg(target_pointer_width = "64")]
    {
        let ts = [tensor(u32::MAX as u64)];
        refuse(
            input(&ts),
            BUDGET,
            4,
            BuildError::OffsetLimit,
            ContainerEnd,
            None,
            ConvertOffset,
        );
        let ts = [tensor(u32::MAX as u64), tensor(0)];
        refuse(
            input(&ts),
            BUDGET,
            4,
            BuildError::OffsetLimit,
            TensorBlobOffset,
            Some(1),
            ConvertOffset,
        );
    }
}
#[test]
fn detailed_prepared_plans_preserve_nine_actual_legacy_images_and_old_error_projection() {
    for golden in &fixture::GOLDENS {
        let tensors = fixture::descriptors(golden);
        let input = fixture::input(golden, &tensors);
        let mut old = [fixture::sentinel(); 8];
        let mut precise = old;
        let a = plan_into(input, &mut old, BUDGET).unwrap();
        let b = plan_into_detailed(input, &mut precise, BUDGET).unwrap();
        assert_eq!(a.output_len(), b.output_len());
        let mut output = vec![0; golden.bytes.len()];
        write_into(&b, &mut output).unwrap();
        assert_eq!(output, golden.bytes, "{}", golden.name);
        for (x, y) in a.entries().iter().zip(b.entries()) {
            assert_eq!(fixture::entry_bytes(*x), fixture::entry_bytes(*y));
        }
        let mut old = [fixture::sentinel(); 8];
        let mut precise = old;
        let budget = BuildBudget {
            output_bytes: 0,
            ..BUDGET
        };
        let a = plan_into(input, &mut old, budget).unwrap_err();
        let b = plan_into_detailed(input, &mut precise, budget).unwrap_err();
        assert_eq!(
            a,
            BuildError::OutputBudget {
                required_bytes: golden.bytes.len(),
                available_bytes: 0
            }
        );
        assert_eq!(a, b.kind);
        assert_eq!(b.context.field, PlanField::OutputBudget);
        unchanged(&old);
        unchanged(&precise);
    }
}
#[test]
fn positive_controlled_success_and_failure_paths_allocate_nothing() {
    COUNT.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    let control = std::hint::black_box(Box::new([7u8; 71]));
    std::hint::black_box(&control);
    drop(control);
    ACTIVE.with(|v| v.set(false));
    assert!(COUNT.with(Cell::get) > 0);
    let good = [tensor(4)];
    let bad = [tensor(u32::MAX as u64 + 1)];
    let mut scratch = [fixture::sentinel(); 4];
    COUNT.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    let success =
        plan_layout_into_detailed(input(&good), &mut scratch, BUDGET).map(|p| p.output_len());
    let failure =
        plan_layout_into_detailed(input(&bad), &mut scratch, BUDGET).map(|p| p.output_len());
    let coarse = plan_layout_into(input(&bad), &mut scratch, BUDGET).map(|p| p.output_len());
    ACTIVE.with(|v| v.set(false));
    let count = COUNT.with(Cell::get);
    assert_eq!(success.unwrap(), 516);
    assert_eq!(failure.unwrap_err().kind, BuildError::OffsetLimit);
    assert_eq!(coarse.unwrap_err(), BuildError::OffsetLimit);
    assert_eq!(count, 0);
}
