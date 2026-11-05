use super::*;
use etalon_inference::ordinary::qwen35::weights::SliceBytes;

fn fixture(kind: u32) -> (Vec<u8>, Projection, Vec<f32>) {
    let cols = 257;
    let rows = 6;
    let mut bytes: Vec<u8> = (0..rows * cols)
        .map(|i| {
            let code = (i * 73) as u8;
            if code & 127 == 127 {
                0
            } else {
                code
            }
        })
        .collect();
    let offset = bytes.len() as u64;
    for scale in [0.0009765625f32, 0.25, 1.0, 1.5, 16.0, 128.0] {
        match kind {
            crate::ggml_quants::GGML_TYPE_F32 => bytes.extend(scale.to_le_bytes()),
            crate::ggml_quants::GGML_TYPE_F16 => {
                bytes.extend(half::f16::from_f32(scale).to_le_bytes())
            }
            crate::ggml_quants::GGML_TYPE_BF16 => {
                bytes.extend(((scale.to_bits() >> 16) as u16).to_le_bytes())
            }
            _ => unreachable!(),
        }
    }
    let projection = Projection {
        weight: TensorInfo {
            dims: [cols as u64, rows as u64, 0, 0],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_FP8_E4M3,
            byte_offset: 0,
        },
        row_scale: Some(TensorInfo {
            dims: [rows as u64, 0, 0, 0],
            n_dims: 1,
            ggml_type: kind,
            byte_offset: offset,
        }),
    };
    let input = (0..cols)
        .map(|i| ((i * 17 % 31) as f32 - 15.0) / 16.0)
        .collect();
    (bytes, projection, input)
}

#[test]
fn fp8_lut_is_bit_exact_for_all_finite_codes_and_scale_encodings() {
    for kind in [
        crate::ggml_quants::GGML_TYPE_F32,
        crate::ggml_quants::GGML_TYPE_F16,
        crate::ggml_quants::GGML_TYPE_BF16,
    ] {
        let (bytes, p, input) = fixture(kind);
        let source = SliceBytes { bytes: &bytes };
        let oracle = CpuByteHost { source: &source };
        let fast = DenseHost { source: &source };
        assert!(oracle.admits(&p));
        assert!(fast.admits(&p));
        let mut expected = [0.0; 6];
        let mut actual = [0.0; 6];
        oracle.gemm(&p, &input, &mut expected).unwrap();
        let guard =
            crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard::begin(
                "qwen_fp8_lut_host",
                true,
            );
        fast.gemm(&p, &input, &mut actual).unwrap();
        guard.check().expect("FP8 host has no hot allocation calls");
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
        assert!(FP8[127].is_nan());
        assert!(FP8[255].is_nan());
        assert_eq!(FP8[128].to_bits(), (-0.0f32).to_bits());
    }
}

#[test]
fn invalid_weight_or_scale_rejects_before_output_write() {
    let (mut bytes, p, input) = fixture(crate::ggml_quants::GGML_TYPE_F32);
    bytes[6 * 257 - 1] = 0xff;
    let mut out = [123.0; 6];
    assert!(DenseHost {
        source: &SliceBytes { bytes: &bytes }
    }
    .gemm(&p, &input, &mut out)
    .is_err());
    assert_eq!(out, [123.0; 6]);
    bytes[6 * 257 - 1] = 0;
    let start = p.row_scale.unwrap().byte_offset as usize;
    bytes[start..start + 4].copy_from_slice(&(-1.0f32).to_le_bytes());
    assert!(!DenseHost {
        source: &SliceBytes { bytes: &bytes }
    }
    .admits(&p));
    bytes[start..start + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(!DenseHost {
        source: &SliceBytes { bytes: &bytes }
    }
    .admits(&p));
}

#[test]
fn overflow_dimensions_and_nonfinite_input_fail_closed() {
    let (bytes, mut p, mut input) = fixture(crate::ggml_quants::GGML_TYPE_F32);
    let source = SliceBytes { bytes: &bytes };
    let host = DenseHost { source: &source };
    input[0] = f32::INFINITY;
    assert!(host.gemm(&p, &input, &mut [0.0; 6]).is_err());
    input[0] = 1.0;
    assert!(host.gemm(&p, &input[..256], &mut [0.0; 6]).is_err());
    p.weight.dims = [u64::MAX, u64::MAX, 0, 0];
    assert!(!host.admits(&p));
}

#[test]
fn finite_scale_overflow_matches_oracle_admission_and_execution() {
    let (mut bytes, p, mut input) = fixture(crate::ggml_quants::GGML_TYPE_F32);
    let start = p.row_scale.unwrap().byte_offset as usize;
    bytes[start..start + 4].copy_from_slice(&3e38f32.to_le_bytes());
    let source = SliceBytes { bytes: &bytes };
    let oracle = CpuByteHost { source: &source };
    let fast = DenseHost { source: &source };
    assert!(oracle.admits(&p));
    assert!(fast.admits(&p));
    for value in [0.0, 1.0] {
        input.fill(value);
        assert!(oracle.gemm(&p, &input, &mut [123.0; 6]).is_err());
        assert!(fast.gemm(&p, &input, &mut [123.0; 6]).is_err());
    }
}
