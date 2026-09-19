//! Scalar Q4_K activation lookup `apply_into` (W3: EOS-030).
//!
//! Evaluates dot products directly from packed GGML Q4_K superblocks without
//! materializing dense f32 weight matrices.
//!
//! T-MAC activation lookup algorithm:
//! 1. Precomputes an activation table once per input:
//!    - Sum $S_x = \sum_{l=0}^{31} x_l$ per 32-element subblock.
//!    - Multiples table $\text{LUT}[l][q] = q \cdot x_l$ for $q \in 0..15$.
//! 2. Inner loop across rows:
//!    - Replaces $32$ per-element weight multiplications with $32$ table lookups.
//!    - Dot product evaluates as:
//!      $\sum_{l=0}^{31} w_l x_l = (d \cdot sc) \sum_{l=0}^{31} \text{LUT}[l][q_l] - (d_{\min} \cdot m) S_x$.

use super::descriptor::{Q4K_SUPERBLOCK_BYTES, Q4K_SUPERBLOCK_ELEMS};
use super::error::OperatorError;
use super::view::OperatorView;
use super::workspace::{MatrixView, MatrixViewMut, OperatorWorkspace};

/// Number of f32 elements needed in workspace per 32-element input chunk (1 sum + 32*16 LUT).
pub const FLOATS_PER_SUBBLOCK_LUT: usize = 513;

/// Computes the number of workspace f32 floats required for Q4_K activation lookup.
pub fn q4k_lookup_workspace_floats(in_features: usize) -> Result<usize, OperatorError> {
    if in_features == 0 || in_features % 32 != 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    let n_subblocks = in_features / 32;
    n_subblocks
        .checked_mul(FLOATS_PER_SUBBLOCK_LUT)
        .ok_or(OperatorError::UnsupportedShape)
}

/// Precomputes the activation lookup table into `table_out`.
///
/// Returns the number of floats written to `table_out`.
pub fn prepare_q4k_activation_table(
    input: &[f32],
    table_out: &mut [f32],
) -> Result<usize, OperatorError> {
    if input.is_empty() || input.len() % 32 != 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    let n_subblocks = input.len() / 32;
    let need = n_subblocks
        .checked_mul(FLOATS_PER_SUBBLOCK_LUT)
        .ok_or(OperatorError::UnsupportedShape)?;
    if table_out.len() < need {
        return Err(OperatorError::WorkspaceTooSmall);
    }

    for s in 0..n_subblocks {
        let in_start = s * 32;
        let out_base = s * FLOATS_PER_SUBBLOCK_LUT;

        let mut sum_x = 0.0f32;
        for l in 0..32 {
            let x = input[in_start + l];
            sum_x += x;
            let lut_start = out_base + 1 + l * 16;
            for v in 0..16 {
                table_out[lut_start + v] = (v as f32) * x;
            }
        }
        table_out[out_base] = sum_x;
    }

    Ok(need)
}

/// Directly evaluate Q4_K matrix-vector multiplication using activation lookups.
pub fn apply_q4k_lookup(
    operator: &OperatorView<'_>,
    inn: usize,
    out_n: usize,
    input: MatrixView<'_>,
    output: &mut MatrixViewMut<'_>,
    workspace: OperatorWorkspace<'_>,
) -> Result<(), OperatorError> {
    if inn % (Q4K_SUPERBLOCK_ELEMS as usize) != 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    if input.rows * input.cols != inn {
        return Err(OperatorError::UnsupportedShape);
    }
    if output.rows * output.cols != out_n {
        return Err(OperatorError::UnsupportedShape);
    }

    let weights = operator
        .payloads
        .first()
        .map(|p| p.bytes)
        .ok_or(OperatorError::InvalidPayload)?;

    let blocks_per_row = inn / (Q4K_SUPERBLOCK_ELEMS as usize);
    let total_blocks = out_n
        .checked_mul(blocks_per_row)
        .ok_or(OperatorError::UnsupportedShape)?;
    let need_bytes = total_blocks
        .checked_mul(Q4K_SUPERBLOCK_BYTES)
        .ok_or(OperatorError::UnsupportedShape)?;
    if weights.len() < need_bytes {
        return Err(OperatorError::InvalidPayload);
    }

    // Prepare activation table in caller-supplied workspace
    prepare_q4k_activation_table(input.data, workspace.numeric)?;

    for o in 0..out_n {
        let mut acc = 0.0f32;
        let row_block_offset = o * blocks_per_row;

        for b in 0..blocks_per_row {
            let b_idx = row_block_offset + b;
            let raw_block = &weights[b_idx * Q4K_SUPERBLOCK_BYTES..(b_idx + 1) * Q4K_SUPERBLOCK_BYTES];

            let d = f16_from_le([raw_block[0], raw_block[1]]);
            let dmin = f16_from_le([raw_block[2], raw_block[3]]);
            let scales: [u8; 12] = [
                raw_block[4], raw_block[5], raw_block[6], raw_block[7],
                raw_block[8], raw_block[9], raw_block[10], raw_block[11],
                raw_block[12], raw_block[13], raw_block[14], raw_block[15],
            ];
            let qs = &raw_block[16..144];

            // 8 subblocks per superblock, handled in 4 pairs of (even, odd)
            for pair in 0..4 {
                let is0 = pair * 2;
                let is1 = is0 + 1;
                let q_off = pair * 32;

                // Even subblock
                {
                    let in_s = b * 8 + is0;
                    let out_base = in_s * FLOATS_PER_SUBBLOCK_LUT;
                    let sum_x = workspace.numeric[out_base];

                    let (sc0, m0) = get_scale_min_k4(is0, &scales);
                    let d1 = d * sc0 as f32;
                    let m1 = dmin * m0 as f32;

                    let mut sum_q = 0.0f32;
                    for l in 0..32 {
                        let q = (qs[q_off + l] & 0xF) as usize;
                        let lut_val = workspace.numeric[out_base + 1 + l * 16 + q];
                        sum_q += lut_val;
                    }
                    acc += d1 * sum_q - m1 * sum_x;
                }

                // Odd subblock
                {
                    let in_s = b * 8 + is1;
                    let out_base = in_s * FLOATS_PER_SUBBLOCK_LUT;
                    let sum_x = workspace.numeric[out_base];

                    let (sc1, m1) = get_scale_min_k4(is1, &scales);
                    let d2 = d * sc1 as f32;
                    let m2 = dmin * m1 as f32;

                    let mut sum_q = 0.0f32;
                    for l in 0..32 {
                        let q = (qs[q_off + l] >> 4) as usize;
                        let lut_val = workspace.numeric[out_base + 1 + l * 16 + q];
                        sum_q += lut_val;
                    }
                    acc += d2 * sum_q - m2 * sum_x;
                }
            }
        }
        output.data[o] = acc;
    }

    Ok(())
}

fn get_scale_min_k4(j: usize, scales: &[u8; 12]) -> (u8, u8) {
    if j < 4 {
        (scales[j] & 63, scales[j + 4] & 63)
    } else {
        (
            (scales[j + 4] & 0xF) | ((scales[j - 4] >> 6) << 4),
            (scales[j + 4] >> 4) | ((scales[j] >> 6) << 4),
        )
    }
}

fn f16_from_le(bytes: [u8; 2]) -> f32 {
    let u = u16::from_le_bytes(bytes);
    let sign = (u >> 15) & 1;
    let exp = (u >> 10) & 0x1f;
    let frac = u & 0x3ff;
    let sign_f = if sign == 0 { 1.0f32 } else { -1.0f32 };
    if exp == 0 {
        if frac == 0 {
            return sign_f * 0.0;
        }
        return sign_f * (frac as f32) * f32::from_bits(0x3380_0000); // 2^-24
    }
    if exp == 31 {
        return if frac == 0 {
            if sign == 0 { f32::INFINITY } else { f32::NEG_INFINITY }
        } else {
            f32::NAN
        };
    }
    let norm = (frac as f32) / 1024.0;
    let power = 2.0f32.powi(exp as i32 - 15);
    sign_f * (1.0 + norm) * power
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::descriptor::{
        AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout,
    };
    use crate::operators::q4k::reconstruct_q4k_into;
    use crate::operators::view::{validate_operator, PayloadView};

    fn make_synthetic_q4k_block(block_idx: u8) -> [u8; Q4K_SUPERBLOCK_BYTES] {
        let mut blk = [0u8; Q4K_SUPERBLOCK_BYTES];
        blk[0] = 0x00; // d = 1.0 (f16)
        blk[1] = 0x3c;
        blk[2] = 0x00; // dmin = 0.1 (f16)
        blk[3] = 0x2e;
        for i in 4..16 {
            blk[i] = ((block_idx + i as u8) & 0x3f) as u8;
        }
        for i in 16..144 {
            blk[i] = block_idx.wrapping_add(i as u8);
        }
        blk
    }

    #[test]
    fn q4k_lookup_matches_reconstructed_dense_gemv() {
        let in_features = 256;
        let out_features = 2;
        let mut weights_raw = Vec::new();
        weights_raw.extend_from_slice(&make_synthetic_q4k_block(1));
        weights_raw.extend_from_slice(&make_synthetic_q4k_block(2));

        // Reconstruct weights into dense f32 to compute golden reference dot products
        let mut dense_weights = vec![0.0f32; out_features * in_features];
        reconstruct_q4k_into(&weights_raw, out_features * in_features, &mut dense_weights)
            .expect("reconstruct weights");

        // Input vector
        let mut input_vec = vec![0.0f32; in_features];
        for (i, v) in input_vec.iter_mut().enumerate() {
            *v = (i as f32 * 0.05).sin();
        }

        // Golden dot product
        let mut golden_output = vec![0.0f32; out_features];
        for o in 0..out_features {
            let mut acc = 0.0f32;
            for i in 0..in_features {
                acc += dense_weights[o * in_features + i] * input_vec[i];
            }
            golden_output[o] = acc;
        }

        // Q4_K lookup apply
        let desc = OperatorDescriptor {
            kind: OperatorKind::Q4KBitPlane,
            in_features: in_features as u32,
            out_features: out_features as u32,
            batch_hint: 1,
            tile_elems: Q4K_SUPERBLOCK_ELEMS,
            scale_layout: ScaleLayout::GgmlQ4K,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        };
        let payloads = [PayloadView { bytes: &weights_raw }];
        let op_view = validate_operator(&desc, &payloads).expect("validate op");

        let mut lookup_output = vec![0.0f32; out_features];
        let mut ws_floats = vec![0.0f32; q4k_lookup_workspace_floats(in_features).unwrap()];
        let mut ws_bytes = [];
        let ws = OperatorWorkspace {
            numeric: &mut ws_floats,
            bytes: &mut ws_bytes,
        };

        apply_q4k_lookup(
            &op_view,
            in_features,
            out_features,
            MatrixView { data: &input_vec, rows: 1, cols: in_features },
            &mut MatrixViewMut { data: &mut lookup_output, rows: 1, cols: out_features },
            ws,
        )
        .expect("lookup apply must succeed");

        // Assert numerical parity
        for o in 0..out_features {
            let diff = (lookup_output[o] - golden_output[o]).abs();
            let max_val = lookup_output[o].abs().max(golden_output[o].abs()).max(1.0);
            let rel_err = diff / max_val;
            assert!(
                rel_err < 1e-4,
                "row {o} mismatch: lookup={} golden={} rel_err={}",
                lookup_output[o],
                golden_output[o],
                rel_err
            );
        }
    }
}
