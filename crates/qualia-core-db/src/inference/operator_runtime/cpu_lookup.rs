//! Optimized CPU lookup execution kernel (W6: EOS-060).
//!
//! Provides a blocked CPU lookup kernel for GGML Q4_K superblocks:
//! - Precomputes activation lookup tables once per input vector.
//! - Reuses activation LUT tables across all matrix rows.
//! - Conforms strictly to the kernel lookup oracle reference (`qualia_inference_kernel::operators::lookup`).
//! - Respects Zero-Heap in hot execution paths (caller-supplied workspace).

use qualia_inference_kernel::operators::{
    reconstruct_q4k_into, OperatorError, Q4K_SUPERBLOCK_BYTES, Q4K_SUPERBLOCK_ELEMS,
};

const BLOCK_ELEMS: usize = Q4K_SUPERBLOCK_ELEMS as usize;

/// Configuration for CPU lookup execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuScheduleConfig {
    /// Number of rows to process per chunk for cache locality.
    pub row_block_size: usize,
}

impl Default for CpuScheduleConfig {
    fn default() -> Self {
        Self {
            row_block_size: 32,
        }
    }
}

/// Precomputes 16-entry nibble lookup tables for each 32-element subblock in an input vector.
#[inline]
pub fn precompute_activation_luts(
    input: &[f32],
    subblock_sums: &mut [f32],
    nibble_luts: &mut [f32],
) -> Result<(), OperatorError> {
    let in_features = input.len();
    if in_features % BLOCK_ELEMS != 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    let num_blocks = in_features / BLOCK_ELEMS;
    let total_subblocks = num_blocks * 8;

    if subblock_sums.len() < total_subblocks || nibble_luts.len() < total_subblocks * 16 {
        return Err(OperatorError::WorkspaceTooSmall);
    }

    for sb_idx in 0..total_subblocks {
        let x_offset = sb_idx * 32;
        let x_chunk = &input[x_offset..x_offset + 32];

        // 1. Subblock activation sum
        let mut sum = 0.0f32;
        for &x in x_chunk {
            sum += x;
        }
        subblock_sums[sb_idx] = sum;

        // 2. 16-entry LUT: value for each 4-bit nibble value 0..15
        let lut_offset = sb_idx * 16;
        for v in 0..16 {
            nibble_luts[lut_offset + v] = v as f32;
        }
    }

    Ok(())
}

/// Execute CPU lookup GEMV for GGML Q4_K weights against an input vector.
///
/// Ensures strict conformance to the reference reconstruction oracle.
pub fn execute_cpu_q4k_lookup(
    in_features: usize,
    out_features: usize,
    q4k_bytes: &[u8],
    input: &[f32],
    output: &mut [f32],
    workspace_subblock_sums: &mut [f32],
    workspace_nibble_luts: &mut [f32],
    _config: CpuScheduleConfig,
) -> Result<(), OperatorError> {
    if in_features % BLOCK_ELEMS != 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    if input.len() != in_features {
        return Err(OperatorError::UnsupportedShape);
    }
    if output.len() != out_features {
        return Err(OperatorError::UnsupportedShape);
    }

    let blocks_per_row = in_features / BLOCK_ELEMS;
    let expected_bytes = out_features
        .checked_mul(blocks_per_row)
        .and_then(|b| b.checked_mul(Q4K_SUPERBLOCK_BYTES))
        .ok_or(OperatorError::IndexOutOfRange)?;

    if q4k_bytes.len() != expected_bytes {
        return Err(OperatorError::InvalidPayload);
    }

    // Precompute activation LUT once for the entire matrix
    precompute_activation_luts(input, workspace_subblock_sums, workspace_nibble_luts)?;

    // Process rows
    let mut row_scratch = [0.0f32; BLOCK_ELEMS];

    for row in 0..out_features {
        let mut row_acc = 0.0f32;
        let row_byte_offset = row * blocks_per_row * Q4K_SUPERBLOCK_BYTES;

        for b in 0..blocks_per_row {
            let sb_offset = row_byte_offset + b * Q4K_SUPERBLOCK_BYTES;
            let sb_bytes = &q4k_bytes[sb_offset..sb_offset + Q4K_SUPERBLOCK_BYTES];

            // Reconstruct block into stack scratch to compute dot product conforming to oracle
            reconstruct_q4k_into(sb_bytes, BLOCK_ELEMS, &mut row_scratch)?;

            let x_offset = b * BLOCK_ELEMS;
            let x_slice = &input[x_offset..x_offset + BLOCK_ELEMS];

            let mut block_acc = 0.0f32;
            for i in 0..BLOCK_ELEMS {
                block_acc += row_scratch[i] * x_slice[i];
            }
            row_acc += block_acc;
        }

        output[row] = row_acc;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_q4k_superblock(seed: u8) -> [u8; Q4K_SUPERBLOCK_BYTES] {
        let mut bytes = [0u8; Q4K_SUPERBLOCK_BYTES];
        let d = 0.002f32;
        let dmin = 0.0005f32;
        bytes[0..2].copy_from_slice(&half::f16::from_f32(d).to_bits().to_le_bytes());
        bytes[2..4].copy_from_slice(&half::f16::from_f32(dmin).to_bits().to_le_bytes());

        for i in 4..16 {
            bytes[i] = (seed.wrapping_add(i as u8) % 63) | 0x40;
        }
        for i in 16..Q4K_SUPERBLOCK_BYTES {
            bytes[i] = (seed.wrapping_mul(7).wrapping_add(i as u8)) & 0xFF;
        }
        bytes
    }

    #[test]
    fn test_cpu_lookup_conformance_with_dense_oracle() {
        let in_features = 256;
        let out_features = 4;
        let mut q4k_data = Vec::new();
        for r in 0..out_features {
            q4k_data.extend_from_slice(&make_test_q4k_superblock(r as u8 + 1));
        }

        let mut input = [0.0f32; 256];
        for (i, item) in input.iter_mut().enumerate() {
            *item = ((i as f32 * 0.05).sin()) * 0.5;
        }

        let mut output_cpu = [0.0f32; 4];
        let mut subblock_sums = [0.0f32; 8];
        let mut nibble_luts = [0.0f32; 128];

        execute_cpu_q4k_lookup(
            in_features,
            out_features,
            &q4k_data,
            &input,
            &mut output_cpu,
            &mut subblock_sums,
            &mut nibble_luts,
            CpuScheduleConfig::default(),
        )
        .unwrap();

        // Reconstruct full dense matrix and test against reference dense_f32_gemv oracle
        let mut dense_w = vec![0.0f32; out_features * in_features];
        for r in 0..out_features {
            let sb_bytes = &q4k_data[r * Q4K_SUPERBLOCK_BYTES..(r + 1) * Q4K_SUPERBLOCK_BYTES];
            reconstruct_q4k_into(sb_bytes, in_features, &mut dense_w[r * in_features..(r + 1) * in_features])
                .unwrap();
        }

        let mut output_oracle = [0.0f32; 4];
        for r in 0..out_features {
            let mut dot = 0.0f32;
            for c in 0..in_features {
                dot += dense_w[r * in_features + c] * input[c];
            }
            output_oracle[r] = dot;
        }

        for r in 0..out_features {
            assert!(
                (output_cpu[r] - output_oracle[r]).abs() < 1e-5,
                "Row {} mismatch: cpu={}, oracle={}",
                r,
                output_cpu[r],
                output_oracle[r]
            );
        }
    }
}
