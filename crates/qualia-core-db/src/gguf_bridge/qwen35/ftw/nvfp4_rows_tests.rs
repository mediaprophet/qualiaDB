use super::*;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
use qualia_inference_kernel::moe::nvfp4::{dequantize_nvfp4_row, fp8_e4m3_to_f32};

#[test]
fn every_finite_scale_and_nibble_matches_reference_bits() {
    let mut input = [0.0; 32];
    for (i, x) in input.iter_mut().enumerate() {
        *x = (i as f32 - 14.25) / 13.0;
    }
    let mut packed = [0u8; 16];
    for (i, byte) in packed.iter_mut().enumerate() {
        *byte = i as u8 | (((15 - i) as u8) << 4);
    }
    for code in 0..=255u8 {
        if code & 0x7f == 0x7f {
            continue;
        }
        assert_eq!(
            SCALES[code as usize].to_bits(),
            fp8_e4m3_to_f32(code).to_bits()
        );
        for global in [0.125, 1.0, 1.3, 512.0] {
            let scales = [code, code ^ 0x80];
            let mut decoded = [0.0; 32];
            dequantize_nvfp4_row(&packed, &scales, global, &mut decoded).unwrap();
            let reference: f32 = input.iter().zip(decoded).map(|(x, w)| x * w).sum();
            assert_eq!(
                row_dot(&input, &packed, &scales, global).unwrap().to_bits(),
                reference.to_bits(),
                "code={code:#x} global={global}"
            );
        }
    }
}

#[test]
fn signed_zero_accumulation_matches_iterator_sum() {
    for input in [[0.0; 16], [-0.0; 16], [1.0; 16], [-1.0; 16]] {
        for packed in [[0x00; 8], [0x88; 8]] {
            for scale in [0x00, 0x80, 0x38, 0xb8] {
                let mut decoded = [0.0; 16];
                dequantize_nvfp4_row(&packed, &[scale], 1.0, &mut decoded).unwrap();
                let expected: f32 = input.iter().zip(decoded).map(|(x, w)| x * w).sum();
                assert_eq!(
                    row_dot(&input, &packed, &[scale], 1.0).unwrap().to_bits(),
                    expected.to_bits()
                );
            }
        }
    }
}

#[test]
fn malformed_sources_and_overflow_are_rejected() {
    for code in [0x7f, 0xff] {
        assert_eq!(
            row_dot(&[1.0; 16], &[0x22; 8], &[code], 1.0),
            Err(RowError::NonFinite)
        );
    }
    assert_eq!(
        row_dot(&[1.0; 16], &[0x22; 7], &[0x38], 1.0),
        Err(RowError::Shape)
    );
    assert_eq!(
        row_dot(&[f32::NAN; 16], &[0x22; 8], &[0x38], 1.0),
        Err(RowError::NonFinite)
    );
    assert_eq!(
        row_dot(&[f32::MAX; 16], &[0x77; 8], &[0x38], 1.0),
        Err(RowError::NonFinite)
    );
    assert_eq!(
        row_dot(&[1.0; 16], &[0x22; 8], &[0x7e], f32::MAX),
        Err(RowError::NonFinite)
    );
    assert_eq!(
        row_dot(&[1.0; 16], &[0x22; 8], &[0x38], 0.0),
        Err(RowError::NonFinite)
    );
}

#[test]
fn per_row_f16_globals_and_argmax_are_exact_and_zero_alloc() {
    let mut globals = [0u8; 6];
    for (dst, value) in globals.chunks_exact_mut(2).zip([1.0, 3.0, 3.0]) {
        dst.copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
    }
    let mut out = [0.0; 3];
    assert_zero_alloc("qwen35_nvfp4_fused_rows", || {
        gemv(&[1.0; 16], &[0x22; 24], &[0x38; 3], &globals, &mut out).unwrap();
        assert_eq!(out, [16.0, 48.0, 48.0]);
        assert_eq!(
            argmax(&[1.0; 16], &[0x22; 24], &[0x38; 3], &globals, 3).unwrap(),
            (1, 48.0)
        );
    });
    assert_eq!(
        gemv(&[1.0; 16], &[0x22; 24], &[0x38; 3], &globals[..4], &mut out),
        Err(RowError::Shape)
    );
}
