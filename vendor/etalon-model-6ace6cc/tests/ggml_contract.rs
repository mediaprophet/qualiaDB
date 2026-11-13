#![cfg(feature = "ggml")]
use etalon_model::ggml::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
#[path = "support/ggml_fixture.rs"]
mod fixture;
use fixture::*;

struct Counter;
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static ALLOCS:Cell<usize>=const{Cell::new(0)};}
fn tick() {
    if ACTIVE.try_with(Cell::get).unwrap_or(false) {
        ALLOCS.with(|v| v.set(v.get() + 1));
    }
}
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        tick();
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn measured(f: impl FnOnce()) -> usize {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            ACTIVE.with(|v| v.set(false));
        }
    }
    ALLOCS.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    let stop = Stop;
    f();
    drop(stop);
    ALLOCS.with(Cell::get)
}

// Frozen by compiling actual Qualia ggml_quants.rs SHA256 6b7421fc4db3cbffe5d99ec80cdaf9f5fef0be75ff43ea13eb5340554873c716.
// Reference uses actual fp8_e4m3_to_f32 from nvfp4.rs; no substitute codec.
// Reproduction: compile that original module and FP8 helper as an rlib exposing
// ggml::*; compile this fixture with --extern etalon_model=<original rlib>.
// Fingerprints cover result plus every output bit, including failure state/tails.
#[test]
fn thirteen_actual_original_codecs_match_full_output_bits_and_failures() {
    let expected = [
        0x6b7f26ce43c9d020,
        0xba9687dbf9e4da8d,
        0x1e62a45a2e00b0d3,
        0xcea4d0c7e04ed92f,
        0x09060853b31555a8,
        0x6809a207283869d8,
        0x22664debb5196d99,
        0x0b8dbb5b7f275542,
        0xa93f66e2ba28d795,
        0x50b08fd083c4699d,
        0x13648bac4bd98300,
        0xa382963f2df81619,
        0x3ecd072953c4cde0,
    ];
    for ((dtype, block, bytes), expected) in TYPES.into_iter().zip(expected) {
        assert_eq!(
            row_fingerprint(dtype, block, bytes),
            expected,
            "dtype {dtype}"
        );
    }
}

#[test]
fn every_fp8_byte_matches_independent_exact_binary_value() {
    let raw = std::array::from_fn::<_, 256, _>(|i| i as u8);
    let mut out = [SENTINEL; 258];
    assert_eq!(dequantize_row_into(&raw, 40, 256, &mut out), Ok(256));
    for (b, actual) in out[..256].iter().enumerate() {
        let exponent = (b >> 3) & 15;
        let mantissa = b & 7;
        let magnitude = if exponent == 0 {
            (mantissa as f32) / 512.0
        } else {
            ((8 + mantissa) as f32) * 2.0_f32.powi(exponent as i32 - 10)
        };
        let expected = if b & 127 == 127 {
            0.0
        } else if b & 128 != 0 {
            -magnitude
        } else {
            magnitude
        };
        assert_eq!(actual.to_bits(), expected.to_bits(), "byte {b}");
    }
    assert!(out[256..].iter().all(|v| v.to_bits() == SENTINEL.to_bits()));
}

#[test]
fn scalar_nonfinite_bits_and_refusal_precedence_are_preserved() {
    let bits = [
        0, 0x80000000, 0x7f800000, 0xff800000, 0x7fc12345, 0xffc54321,
    ];
    let raw: Vec<u8> = bits.iter().flat_map(|v: &u32| v.to_le_bytes()).collect();
    let mut out = [SENTINEL; 8];
    assert_eq!(dequantize_row_into(&raw, 0, 6, &mut out), Ok(6));
    for (actual, bits) in out.iter().zip(bits) {
        assert_eq!(actual.to_bits(), bits);
    }
    assert_eq!(
        dequantize_row_into(&[], 158, 1, &mut []),
        Err(GgmlDequantError::BufferTooSmall)
    );
    assert_eq!(
        dequantize_row_into(&[], 158, 0, &mut out),
        Err(GgmlDequantError::UnsupportedType)
    );
    assert_eq!(ggml_row_bytes(158, 0), None);
    assert_eq!(ggml_row_bytes(0, usize::MAX), None);
    assert_eq!(ggml_row_bytes(1, usize::MAX), None);
    for (dtype, _, _) in TYPES {
        let mut out = [SENTINEL; 3];
        assert_eq!(
            dequantize_row_into(&[], dtype, 4, &mut out),
            Err(GgmlDequantError::BufferTooSmall)
        );
        assert!(out.iter().all(|v| v.to_bits() == SENTINEL.to_bits()));
    }
}

#[test]
fn q4_conversion_quantization_and_partial_rows_match_actual_original_bytes() {
    assert_eq!(conversion_fingerprint(), 0xeffe3ccf0d703ecc);
    assert_eq!(ggml_row_bytes(12, 256), Some(144));
    assert_eq!(ggml_row_bytes(112, 256), Some(160));
    let raw = Raw::new();
    let mut out = [0xa5; 320];
    assert_eq!(
        q4k_block_to_soa(&raw.0[..143], &mut out),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(
        q4k_block_to_soa(&raw.0[..144], &mut out[..159]),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(
        expand_q4k_tensor_to_soa(&raw.0[..287], 256, 2, &mut out),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(
        quantize_f32_to_q4_k_soa_tensor(&[], 256, 2, &mut out[..319]),
        Err(GgmlDequantError::BufferTooSmall)
    );
    assert_eq!(
        quantize_f32_to_q4_k_soa_tensor(&[], 256, 2, &mut out),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(out, [0xa5; 320]);
}

#[test]
fn q6_original_alignment_precondition_is_explicit_not_silently_claimed_safe() {
    assert_eq!(std::mem::size_of::<BlockQ6K>(), 210);
    assert_eq!(std::mem::align_of::<BlockQ6K>(), 2);
    let raw = Raw::new();
    let mut out = [0.0; 256];
    assert_eq!((raw.0.as_ptr() as usize) % 2, 0);
    assert_eq!(
        dequantize_row_into(&raw.0[..210], 14, 256, &mut out),
        Ok(256)
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dequantize_row_into(&raw.0[1..211], 14, 256, &mut out)
    }));
    assert!(result.is_err());
}

#[test]
fn successful_and_refused_hot_codecs_allocate_zero_with_positive_control() {
    assert!(
        measured(|| {
            let v = vec![7u8; 1024];
            std::hint::black_box(&v);
        }) > 0
    );
    let raw = Raw::new();
    let mut out = [0.0; 520];
    let mut packed = [0; 1300];
    let src = [0.25; 1026];
    let allocations = measured(|| {
        for (dtype, block, bytes) in TYPES {
            let size = 513_usize.div_ceil(block) * bytes;
            assert_eq!(
                dequantize_row_into(&raw.0[..size], dtype, 513, &mut out),
                Ok(513)
            );
            assert_eq!(
                dequantize_row_into(&[], dtype, 513, &mut out),
                Err(GgmlDequantError::TruncatedInput)
            );
            assert_eq!(
                dequantize_row_into(&[], dtype, 513, &mut out[..1]),
                Err(GgmlDequantError::BufferTooSmall)
            );
        }
        q4k_block_to_soa(&raw.0, &mut packed).unwrap();
        expand_q4k_tensor_to_soa(&raw.0, 513, 2, &mut packed).unwrap();
        quantize_f32_to_q4_k_soa_tensor(&src, 513, 2, &mut packed).unwrap();
        assert!(quantize_f32_to_q4_k_soa_tensor(&[], 513, 2, &mut packed).is_err());
    });
    assert_eq!(allocations, 0);
}
