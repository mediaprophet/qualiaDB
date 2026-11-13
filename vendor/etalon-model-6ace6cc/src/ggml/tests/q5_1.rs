use super::super::{GGML_TYPE_Q5_1, GgmlDequantError, dequantize_row_into};

#[test]
fn q5_1_each_high_bit_selects_its_exact_logical_lane() {
    for lane in 0..32 {
        let mut block = [0u8; 24];
        block[..2].copy_from_slice(&half::f16::ONE.to_le_bytes());
        block[4..8].copy_from_slice(&(1u32 << lane).to_le_bytes());
        let mut out = [f32::NAN; 32];
        assert_eq!(
            dequantize_row_into(&block, GGML_TYPE_Q5_1, 32, &mut out),
            Ok(32)
        );
        for (i, value) in out.into_iter().enumerate() {
            assert_eq!(
                value,
                if i == lane { 16.0 } else { 0.0 },
                "high bit {lane}, lane {i}"
            );
        }
    }
}

#[test]
fn q5_1_split_nibbles_scale_offset_and_second_block() {
    let mut bytes = [0u8; 48];
    let high = [0xA581_136Fu32, 0x5A7E_EC90u32];
    for (block, raw) in bytes.chunks_exact_mut(24).enumerate() {
        raw[..2].copy_from_slice(&half::f16::from_f32(0.5).to_le_bytes());
        raw[2..4].copy_from_slice(&half::f16::from_f32(-2.0).to_le_bytes());
        raw[4..8].copy_from_slice(&high[block].to_le_bytes());
        for j in 0..16 {
            raw[8 + j] = (j as u8) | (((15 - j) as u8) << 4);
        }
    }
    let mut out = [0.; 64];
    assert_eq!(
        dequantize_row_into(&bytes, GGML_TYPE_Q5_1, 64, &mut out),
        Ok(64)
    );
    for block in 0..2 {
        for lane in 0..32 {
            let low = if lane < 16 { lane } else { 31 - lane };
            let quant = low
                + if high[block] & (1 << lane) != 0 {
                    16
                } else {
                    0
                };
            assert_eq!(out[32 * block + lane], quant as f32 * 0.5 - 2.0);
        }
    }
    assert_eq!(
        dequantize_row_into(&bytes[..47], GGML_TYPE_Q5_1, 64, &mut out),
        Err(GgmlDequantError::TruncatedInput)
    );
    assert_eq!(
        dequantize_row_into(&bytes, GGML_TYPE_Q5_1, 64, &mut out[..63]),
        Err(GgmlDequantError::BufferTooSmall)
    );
}

#[test]
fn q5_0_each_high_bit_preserves_signed_logical_lane() {
    use super::super::GGML_TYPE_Q5_0;
    for lane in 0..32 {
        let mut block = [0u8; 22];
        block[..2].copy_from_slice(&half::f16::ONE.to_le_bytes());
        block[2..6].copy_from_slice(&(1u32 << lane).to_le_bytes());
        let mut out = [f32::NAN; 32];
        assert_eq!(
            dequantize_row_into(&block, GGML_TYPE_Q5_0, 32, &mut out),
            Ok(32)
        );
        for (i, value) in out.into_iter().enumerate() {
            assert_eq!(
                value,
                if i == lane { 0. } else { -16. },
                "high bit {lane}, lane {i}"
            );
        }
    }
}
