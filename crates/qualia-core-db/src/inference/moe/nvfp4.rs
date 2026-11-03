//! Compatibility exports for the canonical Etalon numerical implementation.
pub use qualia_inference_kernel::moe::nvfp4::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fp8_e4m3_conversion() {
        // Bias is 7.
        // 0x38 = 0b00111000 -> sign 0, exp 7 (7-7=0, 2^0=1.0), mantissa 0 -> 1.0
        assert_eq!(fp8_e4m3_to_f32(0x38), 1.0);
        // 0x40 = 0b01000000 -> sign 0, exp 8 (8-7=1, 2^1=2.0), mantissa 0 -> 2.0
        assert_eq!(fp8_e4m3_to_f32(0x40), 2.0);
        // 0xB8 = 0b10111000 -> sign 1 -> -1.0
        assert_eq!(fp8_e4m3_to_f32(0xB8), -1.0);
    }

    #[test]
    fn test_dequantize_nvfp4_block() {
        let mut packed = [0u8; 8];
        // Byte 0: low nibble = 2 (1.0), high nibble = 4 (2.0)
        packed[0] = 0x42;
        // Byte 1: low nibble = 7 (6.0), high nibble = 10 (-1.0)
        packed[1] = 0xA7;

        let scale = 0x38; // 1.0
        let global = 1.0f32;
        let mut out = [0.0f32; 16];

        dequantize_nvfp4_block(&packed, scale, global, &mut out);

        assert_eq!(out[0], 1.0);
        assert_eq!(out[1], 2.0);
        assert_eq!(out[2], 6.0);
        assert_eq!(out[3], -1.0);
        assert_eq!(out[4], 0.0);
    }

    #[test]
    fn test_nvfp4_gemv_zero_heap() {
        // 2 rows, 16 cols (1 block per row)
        let mut packed = [0u8; 16];
        // Row 0, byte 0: low = 2 (1.0), high = 4 (2.0)
        packed[0] = 0x42;
        // Row 1, byte 0: low = 3 (1.5), high = 5 (3.0)
        packed[8] = 0x53;

        let scales = [0x38, 0x38]; // 1.0, 1.0
        let mut input = [0.0f32; 16];
        input[0] = 10.0;
        input[1] = 5.0;

        let mut out = [0.0f32; 2];
        nvfp4_gemv_zero_heap(&input, &packed, &scales, 1.0, 2, 16, &mut out).unwrap();

        // Row 0 dot product: 1.0 * 10.0 + 2.0 * 5.0 = 20.0
        assert_eq!(out[0], 20.0);
        // Row 1 dot product: 1.5 * 10.0 + 3.0 * 5.0 = 30.0
        assert_eq!(out[1], 30.0);
    }
}
