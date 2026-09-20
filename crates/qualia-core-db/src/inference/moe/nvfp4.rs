//! NVFP4 (E2M1) unpacking and dequantization engine (Work Package F9).
//!
//! Implements zero-heap unpacking for NVIDIA NVFP4 weights (E2M1 floating point,
//! packed 2 per byte) with FP8 block scales (per 16 elements) and global tensor scales.
//! Designed for Ampere (Compute Capability 8.6, RTX A2000) execution without requiring
//! native FP4 tensor cores.

/// NVFP4 block size: each scale factor governs 16 weights (8 packed bytes).
pub const NVFP4_BLOCK_SIZE: usize = 16;
pub const NVFP4_BYTES_PER_BLOCK: usize = 8;

/// Error conditions during NVFP4 unpacking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nvfp4Error {
    BufferTooSmall,
    DimensionMismatch,
    InvalidBlockAlignment,
}

impl std::fmt::Display for Nvfp4Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "NVFP4 output buffer is too small"),
            Self::DimensionMismatch => write!(f, "Dimension mismatch in NVFP4 matrix operation"),
            Self::InvalidBlockAlignment => {
                write!(f, "Data length is not aligned to NVFP4 16-element blocks")
            }
        }
    }
}

impl std::error::Error for Nvfp4Error {}

/// Standard E2M1 FP4 lookup table (16 entries: 1 sign, 2 exp, 1 mantissa).
pub const E2M1_TABLE: [f32; 16] = [
    0.0,  // 0000
    0.5,  // 0001
    1.0,  // 0010
    1.5,  // 0011
    2.0,  // 0100
    3.0,  // 0101
    4.0,  // 0110
    6.0,  // 0111
    -0.0, // 1000
    -0.5, // 1001
    -1.0, // 1010
    -1.5, // 1011
    -2.0, // 1100
    -3.0, // 1101
    -4.0, // 1110
    -6.0, // 1111
];

/// Convert an FP8 E4M3 byte to f32.
///
/// Format: 1 sign bit, 4 exponent bits (bias 7), 3 mantissa bits.
#[inline(always)]
pub fn fp8_e4m3_to_f32(byte: u8) -> f32 {
    let sign = if (byte & 0x80) != 0 { -1.0f32 } else { 1.0f32 };
    let exp = ((byte >> 3) & 0x0F) as i32;
    let mant = (byte & 0x07) as f32;

    if exp == 0 {
        // Subnormal: (-1)^s * 2^(-6) * (m / 8)
        sign * (1.0 / 64.0) * (mant / 8.0)
    } else if exp == 15 && (byte & 0x07) == 0x07 {
        // NaN / Inf in E4M3
        0.0
    } else {
        // Normal: (-1)^s * 2^(exp - 7) * (1 + m / 8)
        let scale = 2.0f32.powi(exp - 7);
        sign * scale * (1.0 + mant / 8.0)
    }
}

/// Dequantize a single 16-element block of NVFP4 weights.
///
/// `packed`: 8 bytes containing 16 packed 4-bit nibbles.
/// `fp8_scale`: FP8 E4M3 scale factor for this block.
/// `global_scale`: Multiplier for the entire tensor / weight group.
/// `out`: Output array of 16 f32 values.
#[inline(always)]
pub fn dequantize_nvfp4_block(
    packed: &[u8; NVFP4_BYTES_PER_BLOCK],
    fp8_scale: u8,
    global_scale: f32,
    out: &mut [f32; NVFP4_BLOCK_SIZE],
) {
    let effective_scale = fp8_e4m3_to_f32(fp8_scale) * global_scale;

    for i in 0..NVFP4_BYTES_PER_BLOCK {
        let b = packed[i];
        let low_nibble = (b & 0x0F) as usize;
        let high_nibble = ((b >> 4) & 0x0F) as usize;

        out[i * 2] = E2M1_TABLE[low_nibble] * effective_scale;
        out[i * 2 + 1] = E2M1_TABLE[high_nibble] * effective_scale;
    }
}

/// Dequantize a row of packed NVFP4 weights into caller-supplied `out_row`.
///
/// Zero-heap: operates directly on caller buffers.
pub fn dequantize_nvfp4_row(
    packed_row: &[u8],
    block_scales: &[u8],
    global_scale: f32,
    out_row: &mut [f32],
) -> Result<(), Nvfp4Error> {
    let num_elements = packed_row.len() * 2;
    if num_elements % NVFP4_BLOCK_SIZE != 0 {
        return Err(Nvfp4Error::InvalidBlockAlignment);
    }
    let num_blocks = num_elements / NVFP4_BLOCK_SIZE;

    if block_scales.len() < num_blocks || out_row.len() < num_elements {
        return Err(Nvfp4Error::BufferTooSmall);
    }

    for block_idx in 0..num_blocks {
        let in_offset = block_idx * NVFP4_BYTES_PER_BLOCK;
        let out_offset = block_idx * NVFP4_BLOCK_SIZE;

        let mut block_in = [0u8; NVFP4_BYTES_PER_BLOCK];
        block_in.copy_from_slice(&packed_row[in_offset..in_offset + NVFP4_BYTES_PER_BLOCK]);

        let mut block_out = [0.0f32; NVFP4_BLOCK_SIZE];
        dequantize_nvfp4_block(
            &block_in,
            block_scales[block_idx],
            global_scale,
            &mut block_out,
        );

        out_row[out_offset..out_offset + NVFP4_BLOCK_SIZE].copy_from_slice(&block_out);
    }

    Ok(())
}

/// Compute Matrix-Vector multiplication $y = A \cdot x$ where $A$ is in NVFP4 format.
///
/// Zero-heap hot path: unpacks weights into stack scratch `[f32; NVFP4_BLOCK_SIZE]` per block,
/// computing the dot product incrementally without heap allocation.
pub fn nvfp4_gemv_zero_heap(
    input: &[f32],
    packed_matrix: &[u8],
    block_scales: &[u8],
    global_scale: f32,
    rows: usize,
    cols: usize,
    out: &mut [f32],
) -> Result<(), Nvfp4Error> {
    if cols % NVFP4_BLOCK_SIZE != 0 {
        return Err(Nvfp4Error::InvalidBlockAlignment);
    }
    if input.len() < cols || out.len() < rows {
        return Err(Nvfp4Error::BufferTooSmall);
    }

    let bytes_per_row = cols / 2;
    let blocks_per_row = cols / NVFP4_BLOCK_SIZE;

    if packed_matrix.len() < rows * bytes_per_row || block_scales.len() < rows * blocks_per_row {
        return Err(Nvfp4Error::DimensionMismatch);
    }

    // Zero-heap stack buffer for block dequantization
    let mut block_scratch = [0.0f32; NVFP4_BLOCK_SIZE];
    let mut block_bytes = [0u8; NVFP4_BYTES_PER_BLOCK];

    for r in 0..rows {
        let row_bytes_start = r * bytes_per_row;
        let row_scales_start = r * blocks_per_row;
        let mut acc = 0.0f32;

        for b in 0..blocks_per_row {
            let b_byte_start = row_bytes_start + b * NVFP4_BYTES_PER_BLOCK;
            let scale_byte = block_scales[row_scales_start + b];

            block_bytes.copy_from_slice(
                &packed_matrix[b_byte_start..b_byte_start + NVFP4_BYTES_PER_BLOCK],
            );
            dequantize_nvfp4_block(&block_bytes, scale_byte, global_scale, &mut block_scratch);

            let in_offset = b * NVFP4_BLOCK_SIZE;
            for i in 0..NVFP4_BLOCK_SIZE {
                acc += block_scratch[i] * input[in_offset + i];
            }
        }

        out[r] = acc;
    }

    Ok(())
}

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
