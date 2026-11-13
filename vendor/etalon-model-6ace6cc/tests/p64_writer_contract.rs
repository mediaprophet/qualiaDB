use etalon_model::p64::build::{
    plan_into, write_into, BuildBudget, BuildError, BuildInput, BuildProfile,
};
use etalon_model::p64::{IntegrityMode, P64View};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
#[path = "support/p64_writer_fixture.rs"]
mod fixture;

struct CountingAllocator;
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
fn count() {
    let _ = ACTIVE.try_with(|a| {
        if a.get() {
            let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        }
    });
}
// Test-only counter forwards unchanged allocation contracts to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn unlimited() -> BuildBudget {
    BuildBudget {
        output_bytes: usize::MAX,
        scratch_bytes: usize::MAX,
    }
}
fn unchanged(scratch: &[etalon_model::p64::P64TensorEntry]) {
    for entry in scratch {
        assert_eq!(
            fixture::entry_bytes(*entry),
            fixture::entry_bytes(fixture::sentinel())
        );
    }
}

#[test]
fn nine_actual_legacy_outputs_are_byte_exact_and_spare_buffers_untouched() {
    assert_eq!(fixture::crc(b"123456789"), 0xe306_9283);
    for golden in &fixture::GOLDENS {
        let descriptors = fixture::descriptors(golden);
        let input = fixture::input(golden, &descriptors);
        let mut scratch = [fixture::sentinel(); 8];
        let plan = plan_into(
            input,
            &mut scratch,
            BuildBudget {
                output_bytes: golden.bytes.len(),
                scratch_bytes: descriptors.len() * 64,
            },
        )
        .unwrap();
        assert_eq!(
            plan.output_len(),
            golden.bytes.len(),
            "{} exact final padding",
            golden.name
        );
        assert_eq!(plan.header().flags, fixture::u16le(golden.bytes, 6));
        let table = fixture::u32le(golden.bytes, 12) as usize;
        for (i, entry) in plan.entries().iter().enumerate() {
            assert_eq!(
                fixture::entry_bytes(*entry),
                golden.bytes[table + i * 64..table + (i + 1) * 64],
                "{} entry{i}",
                golden.name
            );
        }
        let mut output = vec![0xa5; golden.bytes.len() + 31];
        let written = write_into(&plan, &mut output).unwrap();
        assert_eq!(written, golden.bytes.len());
        assert_eq!(
            &output[..written],
            golden.bytes,
            "{} whole legacy output (independently verified SHA256 {})",
            golden.name,
            golden.sha256
        );
        assert!(output[written..].iter().all(|b| *b == 0xa5));
        let crc_start = fixture::u32le(&output, 28) as usize;
        assert_eq!(
            fixture::crc(&output[..crc_start]),
            fixture::u32le(&output, crc_start)
        );
        for (i, entry) in plan.entries().iter().enumerate() {
            let start = entry.blob_offset as usize;
            let end = start + entry.blob_size as usize;
            assert_eq!(
                fixture::crc(&output[start..end]),
                fixture::u32le(&output, crc_start + 4 + i * 4)
            );
        }
        assert_eq!(
            P64View::parse(&output[..written], IntegrityMode::Full)
                .unwrap()
                .tensor_count(),
            descriptors.len()
        );
        unchanged(&scratch[descriptors.len()..]);
    }
}

#[test]
fn capacity_and_byte_budgets_reject_before_any_caller_mutation() {
    for golden in &fixture::GOLDENS {
        let descriptors = fixture::descriptors(golden);
        let input = fixture::input(golden, &descriptors);
        let count = descriptors.len();
        for (len, budget, expected) in [
            (
                count - 1,
                unlimited(),
                BuildError::ScratchCapacity {
                    required_entries: count,
                    available_entries: count - 1,
                },
            ),
            (
                8,
                BuildBudget {
                    output_bytes: usize::MAX,
                    scratch_bytes: count * 64 - 1,
                },
                BuildError::ScratchBudget {
                    required_bytes: count * 64,
                    available_bytes: count * 64 - 1,
                },
            ),
            (
                8,
                BuildBudget {
                    output_bytes: golden.bytes.len() - 1,
                    scratch_bytes: usize::MAX,
                },
                BuildError::OutputBudget {
                    required_bytes: golden.bytes.len(),
                    available_bytes: golden.bytes.len() - 1,
                },
            ),
        ] {
            let mut scratch = [fixture::sentinel(); 8];
            assert_eq!(
                plan_into(input, &mut scratch[..len], budget).unwrap_err(),
                expected
            );
            unchanged(&scratch);
        }
        let mut scratch = [fixture::sentinel(); 8];
        let plan = plan_into(input, &mut scratch, unlimited()).unwrap();
        let mut output = vec![0x5a; plan.output_len() - 1];
        assert_eq!(
            write_into(&plan, &mut output).unwrap_err(),
            BuildError::OutputCapacity {
                required_bytes: golden.bytes.len(),
                available_bytes: golden.bytes.len() - 1
            }
        );
        assert!(output.iter().all(|b| *b == 0x5a));
    }
}

#[test]
fn invalid_layout_profiles_and_fields_never_partially_fill_scratch() {
    let golden = &fixture::GOLDENS[0];
    let descriptors = fixture::descriptors(golden);
    let base = fixture::input(golden, &descriptors);
    let mut hp = *base.hparams_bytes;
    hp[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    let invalid = [
        BuildInput {
            tensors: &[],
            ..base
        },
        BuildInput {
            page_log2: 7,
            ..base
        },
        BuildInput {
            page_log2: 31,
            ..base
        },
        BuildInput {
            page_log2: u16::MAX,
            ..base
        },
        BuildInput {
            flags: base.flags ^ 0x20,
            ..base
        },
        BuildInput {
            flags: base.flags ^ 1,
            ..base
        },
        BuildInput {
            manifold_bytes: &base.manifold_bytes[..64],
            ..base
        },
        BuildInput {
            hparams_bytes: &hp,
            ..base
        },
        BuildInput {
            profile: BuildProfile::Vision,
            flags: 1,
            ..base
        },
        BuildInput {
            profile: BuildProfile::Safetensor,
            flags: 3,
            ..base
        },
    ];
    for input in invalid {
        let mut scratch = [fixture::sentinel(); 8];
        assert!(plan_into(input, &mut scratch, unlimited()).is_err());
        unchanged(&scratch);
    }
    for change in 0..3 {
        let mut descriptors = descriptors.clone();
        match change {
            0 => descriptors[4].rank = 0,
            1 => descriptors[4].rank = 5,
            _ => descriptors[4].manifold_idx = 3,
        };
        let mut scratch = [fixture::sentinel(); 8];
        assert!(plan_into(
            BuildInput {
                tensors: &descriptors,
                ..base
            },
            &mut scratch,
            unlimited()
        )
        .is_err());
        unchanged(&scratch);
    }
}

#[test]
fn offset_overflow_is_preflighted_without_allocating_a_large_container() {
    let golden = &fixture::GOLDENS[0];
    let descriptors = fixture::descriptors(golden);
    let base = fixture::input(golden, &descriptors);
    let mut tensors = [descriptors[0]; 8];
    for (i, t) in tensors.iter_mut().enumerate() {
        t.manifold_idx = i as u32;
    }
    let mut hp = *base.hparams_bytes;
    hp[..4].copy_from_slice(&7u32.to_le_bytes());
    let manifold = [0u8; 512];
    let mut scratch = [fixture::sentinel(); 8];
    let error = plan_into(
        BuildInput {
            page_log2: 30,
            hparams_bytes: &hp,
            manifold_bytes: &manifold,
            tensors: &tensors,
            ..base
        },
        &mut scratch,
        unlimited(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        BuildError::OffsetLimit | BuildError::ArithmeticOverflow
    ));
    unchanged(&scratch);
}

#[test]
fn raw_sections_and_opaque_descriptor_fields_are_preserved_without_inference() {
    let golden = &fixture::GOLDENS[0];
    let mut tensors = fixture::descriptors(golden);
    let base = fixture::input(golden, &[]);
    tensors[0].name = b"before\0after";
    tensors[1].name = b"";
    tensors[0].role_id = 0xdddd;
    tensors[0].dtype = 0xeeee;
    tensors[0].dimensions = [0, 17, u32::MAX, 9];
    tensors[0].alt_dtype = 0xabab;
    tensors[0].precision_views_mask = 0xffff;
    tensors[0].alt_blob_offset = 0xfefe_fefe;
    let mut hp = *base.hparams_bytes;
    hp[56..].fill(0x88);
    let mut manifold = base.manifold_bytes.to_vec();
    manifold[0..4].copy_from_slice(&0x7fc0_1234u32.to_le_bytes());
    manifold[40..64].fill(0x99);
    let tokenizer = [0x42, 0, 0xff, 0x17, 0x58];
    let reserved = [0x77; 20];
    let mut scratch = [fixture::sentinel(); 8];
    let plan = plan_into(
        BuildInput {
            tensors: &tensors,
            flags: base.flags | 0x8000,
            header_reserved: reserved,
            hparams_bytes: &hp,
            manifold_bytes: &manifold,
            tokenizer_bytes: &tokenizer,
            ..base
        },
        &mut scratch,
        unlimited(),
    )
    .unwrap();
    let mut out = vec![0xcc; plan.output_len()];
    write_into(&plan, &mut out).unwrap();
    let h = plan.header();
    assert_eq!(&out[44..64], reserved);
    assert_eq!(&out[64..128], hp);
    assert_eq!(h.flags, 0x8071);
    assert_eq!(
        &out[h.manifold_table_offset as usize..h.tokenizer_offset as usize],
        manifold
    );
    assert_eq!(
        &out[h.tokenizer_offset as usize..h.tokenizer_offset as usize + 5],
        tokenizer
    );
    let strings = h.string_table_offset as usize;
    let first = plan.entries()[0];
    let name = strings + first.name_offset as usize;
    assert_eq!(&out[name..name + 13], b"before\0after\0");
    assert_eq!(out[strings + plan.entries()[1].name_offset as usize], 0);
    assert_eq!(
        (
            first.role_id,
            first.dtype,
            first.dimensions,
            first.alt_dtype,
            first.precision_views_mask,
            first.alt_blob_offset
        ),
        (
            0xdddd,
            0xeeee,
            [0, 17, u32::MAX, 9],
            0xabab,
            0xffff,
            0xfefe_fefe
        )
    );
    assert!(P64View::parse(&out, IntegrityMode::Full).is_ok());
}

#[test]
fn zero_length_blob_and_default_page_size_retain_explicit_policy() {
    let golden = &fixture::GOLDENS[3];
    let mut tensors = fixture::descriptors(golden);
    tensors[0].blob = &[];
    let base = fixture::input(golden, &[]);
    let mut scratch = [fixture::sentinel(); 8];
    let plan = plan_into(
        BuildInput {
            tensors: &tensors,
            page_log2: 0,
            ..base
        },
        &mut scratch,
        unlimited(),
    )
    .unwrap();
    assert_eq!(plan.header().page_size, 16384);
    assert_eq!(plan.entries()[0].blob_size, 0);
    let mut out = vec![0; plan.output_len()];
    write_into(&plan, &mut out).unwrap();
    assert!(P64View::parse(&out, IntegrityMode::Full).is_ok());
}

#[test]
fn planning_writing_and_refusals_allocate_zero_with_positive_control() {
    let golden = &fixture::GOLDENS[0];
    let tensors = fixture::descriptors(golden);
    let input = fixture::input(golden, &tensors);
    COUNT.with(|c| c.set(0));
    ACTIVE.with(|a| a.set(true));
    let control = std::hint::black_box(Box::new([1u8; 64]));
    std::hint::black_box(&*control);
    drop(control);
    ACTIVE.with(|a| a.set(false));
    assert!(COUNT.with(Cell::get) > 0);
    COUNT.with(|c| c.set(0));
    ACTIVE.with(|a| a.set(true));
    for _ in 0..64 {
        let mut scratch = [fixture::sentinel(); 8];
        let mut output = [0xa5u8; 3000];
        let plan = plan_into(std::hint::black_box(input), &mut scratch, unlimited()).unwrap();
        std::hint::black_box(write_into(&plan, &mut output).unwrap());
        std::hint::black_box(write_into(&plan, &mut output[..1]).unwrap_err());
        let mut scratch = [fixture::sentinel(); 8];
        std::hint::black_box(
            plan_into(
                input,
                &mut scratch,
                BuildBudget {
                    output_bytes: 0,
                    scratch_bytes: usize::MAX,
                },
            )
            .unwrap_err(),
        );
        std::hint::black_box(
            plan_into(
                BuildInput {
                    page_log2: 31,
                    ..input
                },
                &mut scratch,
                unlimited(),
            )
            .unwrap_err(),
        );
    }
    ACTIVE.with(|a| a.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
