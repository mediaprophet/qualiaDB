use etalon_model::p64::{IntegrityMode, P64Error, P64OverflowContext, P64View};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
#[path = "support/p64_fixture.rs"]
mod fixture;

struct CountingAllocator;
thread_local! {
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn count_allocation() {
    let _ = MEASURING.try_with(|active| {
        if active.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}
// This test-only allocator forwards unchanged layouts to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
const MODES: [(IntegrityMode, u8); 3] = [
    (IntegrityMode::Full, 1),
    (IntegrityMode::Metadata, 2),
    (IntegrityMode::Structure, 4),
];

#[test]
fn independent_crc_has_standard_check_value() {
    assert_eq!(fixture::crc(b"123456789"), 0xe306_9283);
    assert_eq!(fixture::crc(b""), 0);
}

#[test]
fn checked_range_errors_retain_context_for_the_target_pointer_width() {
    let mut bytes = fixture::canonical();
    fixture::u32_at(&mut bytes, 64, u32::MAX);
    assert_eq!(
        P64View::parse(&bytes, IntegrityMode::Structure).unwrap_err(),
        P64Error::ArithmeticOverflow(P64OverflowContext::ManifoldCount)
    );
    for (offset, value, overflow, wide_error) in [
        (
            20,
            u32::MAX,
            P64OverflowContext::HyperparameterOffset,
            P64Error::HyperparametersOutOfBounds,
        ),
        (
            36,
            u32::MAX,
            P64OverflowContext::TensorTable,
            P64Error::MetadataBounds,
        ),
        (
            64,
            0x0400_0000,
            P64OverflowContext::ManifoldTable,
            P64Error::MetadataBounds,
        ),
        (
            28,
            u32::MAX,
            P64OverflowContext::ChecksumTable,
            P64Error::MetadataBounds,
        ),
        (
            192,
            u32::MAX,
            P64OverflowContext::TensorName,
            P64Error::InvalidName(0),
        ),
        (
            228,
            u32::MAX,
            P64OverflowContext::TensorBlob,
            P64Error::InvalidBlob(0),
        ),
    ] {
        let mut bytes = fixture::canonical();
        fixture::u32_at(&mut bytes, offset, value);
        let expected = if usize::BITS == 32 {
            P64Error::ArithmeticOverflow(overflow)
        } else {
            wide_error
        };
        assert_eq!(
            P64View::parse(&bytes, IntegrityMode::Structure).unwrap_err(),
            expected,
            "field at {offset}"
        );
    }
}

#[test]
fn complete_container_views_preserve_every_descriptor_and_borrowed_payload() {
    let bytes = fixture::canonical();
    for (mode, _) in MODES {
        let view = P64View::parse(&bytes, mode).unwrap();
        assert_eq!(view.integrity(), mode);
        assert_eq!(view.tensor_count(), 3);
        assert_eq!(view.bytes().as_ptr(), bytes.as_ptr());
        assert_eq!(view.bytes(), bytes);
        let h = view.header();
        assert_eq!(h.magic, *b"p64\0");
        assert_eq!(h.version, 4);
        assert_eq!(h.flags, 0x71);
        assert_eq!(
            [
                h.role_table_offset,
                h.tensor_table_offset,
                h.tokenizer_offset,
                h.hparams_offset,
                h.string_table_offset,
                h.checksum_offset,
                h.manifold_table_offset,
                h.tensor_count,
                h.page_size
            ],
            [128, 192, 640, 64, 384, 768, 512, 3, 512]
        );
        let p = view.hparams();
        assert_eq!(
            [
                p.n_layer,
                p.n_embd,
                p.n_head,
                p.n_kv_head,
                p.vocab_size,
                p.head_dim,
                p.head_dim_swa,
                p.sliding_window,
                p.shared_kv_layers,
                p.architecture,
                p.arch_flags
            ],
            [1, 4, 2, 1, 2, 2, 2, 128, 1, 7, 3]
        );
        assert_eq!(
            [p.rope_freq_base, p.rope_scale, p.logit_softcap],
            [10000., 1., 30.]
        );
        for (i, name) in fixture::NAMES.into_iter().enumerate() {
            let tensor = view.tensor(i).unwrap();
            let entry = tensor.entry();
            assert_eq!(tensor.name_bytes(), name);
            assert_eq!(entry.role_id, [2, 0, 0xfffe][i]);
            assert_eq!(entry.dtype, [0, 0, 1][i]);
            assert_eq!(entry.rank, 1);
            assert_eq!(entry.dimensions, [2, 1, 1, 1]);
            assert_eq!(entry.manifold_idx, if i == 2 { 1 } else { 0 });
            assert_eq!(entry.source_offset, 0x1000 + i as u64 * 0x100);
            assert_eq!(entry.source_name_hash, 0x1122_3344_5566_7786 + i as u64);
            assert_eq!(entry.alt_dtype, 0xffff);
            assert_eq!(entry.precision_views_mask, 1 << i);
            assert_eq!(entry.alt_blob_offset, 0xf000_0000 + i as u32);
            let (start, len) = fixture::BLOBS[i];
            assert_eq!(entry.blob_offset, start as u32);
            assert_eq!(entry.blob_size, len as u32);
            assert_eq!(tensor.blob(), &bytes[start..start + len]);
            assert_eq!(tensor.blob().as_ptr(), bytes[start..].as_ptr());
            assert_eq!(
                tensor.name_bytes().as_ptr(),
                bytes[384 + entry.name_offset as usize..].as_ptr()
            );
        }
        assert_eq!(view.tokenizer_bytes(), &bytes[640..768]);
        assert_eq!(view.tokenizer_bytes().as_ptr(), bytes[640..].as_ptr());
        assert_eq!(view.schedule_bytes().unwrap(), &bytes[128..192]);
        for index in 0..2 {
            let r = view.manifold_record(index).unwrap();
            let start = 512 + index as usize * 64;
            assert_eq!(r.bytes(), &bytes[start..start + 64]);
            assert_eq!(r.bytes().as_ptr(), bytes[start..].as_ptr());
            for (component, value) in r.values_le().into_iter().enumerate() {
                assert_eq!(value, component as f32 / 8. + index as f32);
            }
        }
        assert!(view.tensor(3).is_err());
        assert!(view.tensor(usize::MAX).is_err());
        assert!(view.manifold_record(2).is_err());
        assert!(view.manifold_record(u32::MAX).is_err());
    }
}

#[test]
fn frozen_valid_invalid_and_integrity_mode_matrix() {
    for case in fixture::cases() {
        for (mode, bit) in MODES {
            assert_eq!(
                P64View::parse(&case.bytes, mode).is_ok(),
                case.accepted_modes & bit != 0,
                "{} {mode:?}",
                case.name
            );
        }
    }
}

#[test]
fn unknown_metadata_names_reserved_and_float_bits_are_not_normalized() {
    for case in fixture::cases()
        .into_iter()
        .filter(|case| case.accepted_modes == 7)
    {
        let view = P64View::parse(&case.bytes, IntegrityMode::Full).unwrap();
        assert_eq!(view.bytes(), case.bytes);
        assert_eq!(view.header().reserved, case.bytes[44..64]);
        // Legacy decoded hparams discard their reserved field; the original bytes remain accessible.
        assert_eq!(view.hparams().reserved, [0; 8]);
        for index in 0..2 {
            let record = view.manifold_record(index).unwrap();
            let start = 512 + index as usize * 64;
            for (component, value) in record.values_le().into_iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    fixture::get_u32(&case.bytes, start + component * 4)
                );
            }
            assert_eq!(record.bytes(), &case.bytes[start..start + 64]);
        }
    }
}

#[test]
fn schedule_accessor_checks_alignment_and_ownership_without_tightening_parse() {
    let base = fixture::canonical();
    for offset in [0, 64, 129, 192, 512, 1024, u32::MAX] {
        let mut bytes = base;
        fixture::u32_at(&mut bytes, 8, offset);
        fixture::seal_metadata(&mut bytes);
        for (mode, _) in MODES {
            let view = P64View::parse(&bytes, mode).unwrap();
            assert!(view.schedule_bytes().is_err(), "offset={offset}");
        }
    }
    let mut bytes = base;
    fixture::u16_at(&mut bytes, 6, 0x31);
    fixture::u32_at(&mut bytes, 8, u32::MAX);
    fixture::seal_metadata(&mut bytes);
    assert_eq!(
        P64View::parse(&bytes, IntegrityMode::Full)
            .unwrap()
            .schedule_bytes()
            .unwrap(),
        &[]
    );
}

#[test]
fn all_fixture_success_and_rejection_paths_allocate_zero_after_positive_control() {
    let cases = fixture::cases();
    MEASURING.with(|v| v.set(true));
    ALLOCATIONS.with(|v| v.set(0));
    let control = std::hint::black_box(Box::new([37u8; 64]));
    std::hint::black_box(&*control);
    drop(control);
    MEASURING.with(|v| v.set(false));
    assert!(ALLOCATIONS.with(Cell::get) > 0);
    ALLOCATIONS.with(|v| v.set(0));
    MEASURING.with(|v| v.set(true));
    for _ in 0..32 {
        for case in &cases {
            for (mode, _) in MODES {
                match P64View::parse(std::hint::black_box(&case.bytes), mode) {
                    Ok(view) => {
                        std::hint::black_box(view.bytes());
                        std::hint::black_box(view.header());
                        std::hint::black_box(view.hparams());
                        std::hint::black_box(view.tokenizer_bytes());
                        let _ = std::hint::black_box(view.schedule_bytes());
                        for index in 0..view.tensor_count() {
                            let t = view.tensor(index).unwrap();
                            std::hint::black_box(t.entry());
                            std::hint::black_box(t.name_bytes());
                            std::hint::black_box(t.blob());
                        }
                        let _ = std::hint::black_box(view.tensor(usize::MAX));
                        let _ = std::hint::black_box(view.manifold_record(u32::MAX));
                        std::hint::black_box(view.manifold_record(0).unwrap().values_le());
                    }
                    Err(error) => {
                        std::hint::black_box(error);
                    }
                }
            }
        }
    }
    MEASURING.with(|v| v.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
