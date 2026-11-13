use etalon_model::p64::build::*;
use std::cell::Cell;
#[path = "support/p64_stream_fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/p64_writer_fixture.rs"]
mod legacy;
use fixture::*;

struct CountingAllocator;
thread_local! {
    static MEASURING: Cell<bool> = const {Cell::new(false)};
    static ALLOCATIONS: Cell<usize> = const {Cell::new(0)};
}
fn count_allocation() {
    let _ = MEASURING.try_with(|active| {
        if active.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        count_allocation();
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        count_allocation();
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { std::alloc::System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn library_and_fixture_callbacks_allocate_nothing_on_success_and_refusal() {
    ALLOCATIONS.with(|count| count.set(0));
    MEASURING.with(|active| active.set(true));
    let control = std::hint::black_box(Box::new([7u8; 32]));
    std::hint::black_box(&control);
    drop(control);
    MEASURING.with(|active| active.set(false));
    assert!(ALLOCATIONS.with(Cell::get) > 0);
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|tensor| tensor.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let mut output = [0u8; 4096];
    let mut source = Source::new(&blobs);
    let mut sink = Sink::new(&mut output);
    ALLOCATIONS.with(|count| count.set(0));
    MEASURING.with(|active| active.set(true));
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    let success = run(&plan, &mut source, &mut sink, unlimited(), &|| false);
    let refused = run(
        &plan,
        &mut source,
        &mut sink,
        BuildBudget {
            output_bytes: usize::MAX,
            scratch_bytes: 0,
        },
        &|| false,
    );
    MEASURING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(success.is_ok());
    assert!(refused.is_err());
    assert_eq!(&sink.bytes[..golden.bytes.len()], golden.bytes);
}

#[test]
fn all_nine_legacy_profiles_and_ffn_stream_byte_exactly() {
    for (golden, ffn) in legacy::GOLDENS
        .iter()
        .map(|g| (g, false))
        .chain([(&FFN, true)])
    {
        let descriptors = legacy::descriptors(golden);
        let tensors = sized(&descriptors);
        let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
        let mut entries = [legacy::sentinel(); 8];
        let plan =
            plan_layout_into(input(golden, &tensors, ffn), &mut entries, unlimited()).unwrap();
        assert_eq!(plan.output_len(), golden.bytes.len());
        let mut output = vec![0xa5; plan.output_len() + 17];
        let mut source = Source::new(&blobs);
        let mut sink = Sink::new(&mut output);
        let report = run(&plan, &mut source, &mut sink, unlimited(), &|| false).unwrap();
        assert_eq!(report.bytes_written, golden.bytes.len());
        assert_eq!(report.tensors_written, tensors.len());
        assert_eq!(
            report.scratch_bytes,
            tensors.len() * (64 + size_of::<WorkspaceRequirements>() + 4) + 32 + 16 * 4
        );
        assert_eq!(sink.confirmed, golden.bytes.len());
        assert_eq!(
            &sink.bytes[..golden.bytes.len()],
            golden.bytes,
            "{} complete legacy bytes",
            golden.name
        );
        assert!(sink.bytes[golden.bytes.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        for index in 0..tensors.len() {
            assert_eq!(source.requirements_calls[index].get(), 1);
            assert_eq!(source.emits[index], 2);
        }
    }
}

#[test]
fn budget_requirement_errors_and_shortage_precede_all_output_and_payload_calls() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    let required = tensors.len() * (64 + size_of::<WorkspaceRequirements>() + 4) + 32 + 16 * 4;
    for variant in 0..4 {
        let mut output = vec![0xa5; plan.output_len()];
        let mut sink = Sink::new(&mut output);
        let mut source = Source::new(&blobs);
        let mut budget = unlimited();
        match variant {
            0 => budget.scratch_bytes = required - 1,
            1 => budget.output_bytes = plan.output_len() - 1,
            2 => source.requirement_error = Some(1),
            _ => source.oversized = true,
        }
        let error = run(&plan, &mut source, &mut sink, budget, &|| false).unwrap_err();
        assert_eq!(error.phase, StreamPhase::Preflight);
        assert_eq!(error.confirmed_bytes, 0);
        assert_eq!(sink.calls, 0);
        assert!(sink.bytes.iter().all(|byte| *byte == 0xa5));
        assert_eq!(source.emits, [0; 8]);
        if variant < 2 {
            assert!(source
                .requirements_calls
                .iter()
                .all(|count| count.get() == 0));
        }
        if variant == 0 {
            assert!(
                matches!(error.kind,StreamFailureKind::Build(BuildError::ScratchBudget{required_bytes,..}) if required_bytes==required)
            );
        }
        if variant == 2 {
            assert!(matches!(error.kind, StreamFailureKind::Source(31)));
        }
    }
}

#[test]
fn short_long_and_ignored_overrun_fail_in_both_passes() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    for pass in [1, 2] {
        for fault in [
            Fault::Short(pass),
            Fault::Long(pass),
            Fault::IgnoreOverrun(pass),
        ] {
            let mut output = vec![0xa5; plan.output_len()];
            let mut sink = Sink::new(&mut output);
            let mut source = Source::new(&blobs);
            source.fault = fault;
            let error = run(&plan, &mut source, &mut sink, unlimited(), &|| false).unwrap_err();
            assert_eq!(
                error.phase,
                if pass == 1 {
                    StreamPhase::Checksum
                } else {
                    StreamPhase::Payload
                }
            );
            assert_eq!(error.tensor_index, Some(0));
            assert!(matches!(error.kind, StreamFailureKind::Length { .. }));
            assert_eq!(error.confirmed_bytes, sink.confirmed);
            if pass == 1 {
                assert_eq!(sink.calls, 0);
            } else {
                assert!(sink.confirmed < plan.output_len());
            }
        }
    }
}

#[test]
fn replay_crc_and_source_errors_report_incomplete_phase_and_index() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    for fault in [Fault::ChangedReplay, Fault::Source(1), Fault::Source(2)] {
        let mut output = vec![0xa5; plan.output_len()];
        let mut sink = Sink::new(&mut output);
        let mut source = Source::new(&blobs);
        source.fault = fault;
        let error = run(&plan, &mut source, &mut sink, unlimited(), &|| false).unwrap_err();
        assert_eq!(error.tensor_index, Some(0));
        assert_eq!(error.confirmed_bytes, sink.confirmed);
        match fault {
            Fault::ChangedReplay => assert!(
                matches!(error.kind,StreamFailureKind::ReplayChecksum{expected,actual} if expected!=actual)
            ),
            _ => assert!(matches!(error.kind, StreamFailureKind::Source(42))),
        }
        assert!(sink.confirmed < plan.output_len());
    }
}

#[test]
fn partial_sink_errors_are_sticky_and_count_only_successful_calls() {
    let golden = &legacy::GOLDENS[3];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    for (at, phase) in [
        (0, StreamPhase::Metadata),
        (plan.entries()[0].blob_offset as usize, StreamPhase::Payload),
        (
            (plan.entries()[0].blob_offset + plan.entries()[0].blob_size) as usize,
            StreamPhase::Padding,
        ),
    ] {
        let mut output = vec![0xa5; plan.output_len() + 1];
        let mut sink = Sink::new(&mut output);
        sink.fail_at = Some(at);
        let mut source = Source::new(&blobs);
        source.fault = Fault::IgnoreRefusal(2);
        let error = run(&plan, &mut source, &mut sink, unlimited(), &|| false).unwrap_err();
        assert!(matches!(error.kind, StreamFailureKind::Sink(77)));
        assert_eq!(error.phase, phase);
        assert_eq!(error.confirmed_bytes, sink.confirmed);
        assert_eq!(sink.actual, sink.confirmed + 1);
    }
}

#[test]
fn cancellation_before_output_and_between_chunks_never_completes() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    for scenario in 0..4 {
        let cancelled = Cell::new(scenario == 0);
        let mut source = Source::new(&blobs);
        if scenario == 1 {
            source.cancel_after_chunk = Some(&cancelled);
        }
        let mut output = vec![0xa5; plan.output_len()];
        let mut sink = Sink::new(&mut output);
        if scenario == 2 {
            sink.cancel_after = Some((&cancelled, plan.entries()[0].blob_offset as usize + 1));
        }
        if scenario == 3 {
            sink.cancel_after = Some((&cancelled, plan.output_len()));
        }
        let error = run(&plan, &mut source, &mut sink, unlimited(), &|| {
            cancelled.get()
        })
        .unwrap_err();
        assert!(matches!(error.kind, StreamFailureKind::Cancelled));
        assert_eq!(error.confirmed_bytes, sink.confirmed);
        if scenario < 2 {
            assert_eq!(sink.calls, 0);
        } else if scenario == 2 {
            assert!(sink.confirmed < plan.output_len());
        } else {
            assert_eq!(sink.confirmed, plan.output_len());
            assert_eq!(error.phase, StreamPhase::Padding);
        }
    }
}

#[test]
fn provider_unwind_never_returns_completion_and_caller_retains_ownership() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|t| t.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    for pass in [1, 2] {
        let mut source = Source::new(&blobs);
        source.fault = Fault::Panic(pass);
        let mut output = vec![0xa5; plan.output_len()];
        let mut sink = Sink::new(&mut output);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&plan, &mut source, &mut sink, unlimited(), &|| false)
        }));
        assert!(result.is_err());
        assert!(sink.confirmed < plan.output_len());
        // Generic sinks do not promise rollback; caller can discard its own buffer.
        sink.bytes.fill(0);
        assert!(sink.bytes.iter().all(|byte| *byte == 0));
    }
}

#[test]
fn ffn_raw_schedule_is_preserved_and_zero_payloads_work() {
    let descriptors = legacy::descriptors(&FFN);
    let mut tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|_| &[][..]).collect();
    for tensor in &mut tensors {
        tensor.blob_len = 0;
    }
    let mut layout = input(&FFN, &tensors, true);
    let schedule = [0x5a; 128];
    layout.schedule_bytes = &schedule;
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(layout, &mut entries, unlimited()).unwrap();
    let mut output = vec![0; plan.output_len()];
    let mut sink = Sink::new(&mut output);
    let mut source = Source::new(&blobs);
    let report = run(&plan, &mut source, &mut sink, unlimited(), &|| false).unwrap();
    assert_eq!(report.tensors_written, tensors.len());
    assert_eq!(&sink.bytes[128..256], &schedule);
    assert!(plan.entries().iter().all(|entry| entry.blob_size == 0));
}

#[test]
fn layout_only_large_payload_uses_metadata_without_payload_allocation() {
    let golden = &legacy::GOLDENS[3];
    let descriptors = legacy::descriptors(golden);
    let mut tensors = sized(&descriptors[..1]);
    tensors[0].blob_len = 1 << 30;
    let mut entries = [legacy::sentinel(); 1];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    assert_eq!(plan.entries()[0].blob_size, 1 << 30);
    assert!(plan.output_len() > 1 << 30);
    tensors[0].blob_len = u64::MAX;
    assert!(plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).is_err());
}

#[test]
fn only_used_metadata_prefix_and_admitted_workspace_slices_are_touched() {
    let golden = &legacy::GOLDENS[0];
    let descriptors = legacy::descriptors(golden);
    let tensors = sized(&descriptors);
    let blobs: Vec<_> = descriptors.iter().map(|tensor| tensor.blob).collect();
    let mut entries = [legacy::sentinel(); 8];
    let plan = plan_layout_into(input(golden, &tensors, false), &mut entries, unlimited()).unwrap();
    let sentinel = WorkspaceRequirements {
        byte_count: 999,
        f32_count: 888,
    };
    let mut requirements = [sentinel; 8];
    let mut checksums = [0xdeadbeef; 8];
    let mut bytes = [0xa5; 32];
    let mut floats = [123.; 16];
    let mut output = vec![0; plan.output_len()];
    let mut sink = Sink::new(&mut output);
    let mut source = Source::new(&blobs);
    stream_into(
        &plan,
        &mut source,
        &mut sink,
        StreamScratch {
            requirements: &mut requirements,
            checksums: &mut checksums,
            bytes: &mut bytes,
            floats: &mut floats,
        },
        unlimited(),
        &|| false,
    )
    .unwrap();
    assert!(requirements[tensors.len()..]
        .iter()
        .all(|value| *value == sentinel));
    assert!(checksums[tensors.len()..]
        .iter()
        .all(|value| *value == 0xdeadbeef));
    for (index, blob) in blobs.iter().enumerate() {
        assert_eq!(checksums[index], legacy::crc(blob));
    }
    assert!(bytes[4..].iter().all(|byte| *byte == 0xa5));
    assert!(floats[2..].iter().all(|value| *value == 123.));
}
