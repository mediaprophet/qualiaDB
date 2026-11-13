use super::super::{GGML_TYPE_MXFP4, GgmlDequantError, dequantize_row_into, ggml_row_bytes};

// Natural E2M1 values and an f64 power-of-two scale form an independent oracle.
const E2M1: [f64; 16] = [
    0., 0.5, 1., 1.5, 2., 3., 4., 6., 0., -0.5, -1., -1.5, -2., -3., -4., -6.,
];

#[test]
fn mxfp4_all_nibbles_and_exponent_edges_match_f64_oracle() {
    for exponent in [0u8, 1, 2, 126, 127, 128, 254, 255] {
        let mut raw = [0u8; 17];
        raw[0] = exponent;
        for j in 0..16 {
            raw[1 + j] = j as u8 | ((15 - j) as u8) << 4;
        }
        let mut out = [f32::NAN; 32];
        assert_eq!(
            dequantize_row_into(&raw, GGML_TYPE_MXFP4, 32, &mut out),
            Ok(32)
        );
        let scale = 2f64.powi(i32::from(exponent) - 127);
        for lane in 0..32 {
            let code = if lane < 16 { lane } else { 31 - lane };
            let expected = (E2M1[code] * scale) as f32;
            assert_eq!(
                out[lane].to_bits(),
                expected.to_bits(),
                "exponent {exponent}, lane {lane}"
            );
        }
    }
}

#[test]
fn mxfp4_block_order_partial_tail_and_bounds() {
    assert_eq!(ggml_row_bytes(GGML_TYPE_MXFP4, 32), Some(17));
    assert_eq!(ggml_row_bytes(GGML_TYPE_MXFP4, 64), Some(34));
    let mut raw = [0u8; 34];
    raw[0] = 127;
    raw[1..17].fill(0xD2);
    raw[17] = 128;
    raw[18..].fill(0x1E);
    let mut out = [999.; 65];
    assert_eq!(
        dequantize_row_into(&raw, GGML_TYPE_MXFP4, 64, &mut out),
        Ok(64)
    );
    assert_eq!(&out[..16], &[1.; 16]);
    assert_eq!(&out[16..32], &[-3.; 16]);
    assert_eq!(&out[32..48], &[-8.; 16]);
    assert_eq!(&out[48..64], &[1.; 16]);
    assert_eq!(out[64], 999.);
    out.fill(999.);
    assert_eq!(
        dequantize_row_into(&raw, GGML_TYPE_MXFP4, 33, &mut out),
        Ok(33)
    );
    assert_eq!(out[32], -8.);
    assert!(out[33..].iter().all(|v| *v == 999.));
    assert_eq!(
        dequantize_row_into(&raw[..33], GGML_TYPE_MXFP4, 64, &mut out),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(
        dequantize_row_into(&raw, GGML_TYPE_MXFP4, 64, &mut out[..63]),
        Err(GgmlDequantError::BufferTooSmall)
    );
}
