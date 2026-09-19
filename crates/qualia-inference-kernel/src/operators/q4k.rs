//! GGML Q4_K superblock reconstruction (144 bytes / 256 weights).
//!
//! Copies `ggml_quants::dequant_q4_k` / `get_scale_min_k4`. This is **not**
//! `directml_bridge::dequantize_q4_k_block` (20 bytes / 32 weights).

use super::descriptor::{Q4K_SUPERBLOCK_BYTES, Q4K_SUPERBLOCK_ELEMS};
use super::error::OperatorError;

/// Reconstruct f32 weights from packed GGML Q4_K superblocks into `out`.
///
/// `n_elems` may be a partial last tile. Returns the number of values written.
pub fn reconstruct_q4k_into(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, OperatorError> {
    if out.len() < n_elems {
        return Err(OperatorError::UnsupportedShape);
    }
    let n_blocks = n_elems.div_ceil(Q4K_SUPERBLOCK_ELEMS as usize);
    let need = n_blocks
        .checked_mul(Q4K_SUPERBLOCK_BYTES)
        .ok_or(OperatorError::UnsupportedShape)?;
    if raw.len() < need {
        return Err(OperatorError::InvalidPayload);
    }

    let mut out_idx = 0usize;
    for b in 0..n_blocks {
        let block = &raw[b * Q4K_SUPERBLOCK_BYTES..b * Q4K_SUPERBLOCK_BYTES + Q4K_SUPERBLOCK_BYTES];
        let d = f16_from_le([block[0], block[1]]);
        let dmin = f16_from_le([block[2], block[3]]);
        let scales: [u8; 12] = [
            block[4], block[5], block[6], block[7], block[8], block[9], block[10], block[11],
            block[12], block[13], block[14], block[15],
        ];
        let qs = &block[16..144];

        let block_elems = (Q4K_SUPERBLOCK_ELEMS as usize).min(n_elems - b * Q4K_SUPERBLOCK_ELEMS as usize);
        let mut q_off = 0usize;
        let mut is = 0usize;
        let mut j = 0usize;
        while j < block_elems && out_idx < n_elems {
            let (sc0, m0) = get_scale_min_k4(is, &scales);
            let d1 = d * sc0 as f32;
            let m1 = dmin * m0 as f32;
            let (sc1, m1b) = get_scale_min_k4(is + 1, &scales);
            let d2 = d * sc1 as f32;
            let m2 = dmin * m1b as f32;

            for l in 0..32 {
                if out_idx >= n_elems || j >= block_elems {
                    break;
                }
                out[out_idx] = d1 * (qs[q_off + l] & 0xF) as f32 - m1;
                out_idx += 1;
                j += 1;
            }
            for l in 0..32 {
                if out_idx >= n_elems || j >= block_elems {
                    break;
                }
                out[out_idx] = d2 * (qs[q_off + l] >> 4) as f32 - m2;
                out_idx += 1;
                j += 1;
            }
            q_off += 32;
            is += 2;
        }
    }
    Ok(out_idx.min(n_elems))
}

/// GGML `get_scale_min_k4`: 6-bit packed sub-scale and min.
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

/// IEEE-754 binary16 → f32. No `half` crate (kernel is dependency-free).
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
            sign_f * f32::INFINITY
        } else {
            f32::NAN
        };
    }
    let f32_exp = (exp as u32) + (127 - 15);
    let f32_frac = (frac as u32) << 13;
    f32::from_bits((sign as u32) << 31 | f32_exp << 23 | f32_frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Matches `ggml_quants` tests: d=1.0, dmin=0.1, sc=1, m=0, qs=0x53.
    fn synthetic_superblock() -> [u8; 144] {
        let mut aos = [0u8; 144];
        // f16 1.0 = 0x3C00, f16 0.1 ≈ 0x2E66
        aos[0..2].copy_from_slice(&0x3C00u16.to_le_bytes());
        aos[2..4].copy_from_slice(&0x2E66u16.to_le_bytes());
        // j<4: sc in scales[j], m in scales[j+4]
        // j>=4: sc low nibble in scales[j+4] (= scales[8..12])
        for i in 0..4 {
            aos[4 + i] = 1;
            aos[8 + i] = 0;
            aos[12 + i] = 1;
        }
        for i in 16..144 {
            aos[i] = 0x53;
        }
        aos
    }

    #[test]
    fn f16_one_and_tenth() {
        assert!((f16_from_le(0x3C00u16.to_le_bytes()) - 1.0).abs() < 1e-6);
        let tenth = f16_from_le(0x2E66u16.to_le_bytes());
        assert!((tenth - 0.1).abs() < 0.002, "got {tenth}");
    }

    #[test]
    fn reconstructs_synthetic_superblock() {
        let aos = synthetic_superblock();
        let mut out = [0f32; 256];
        let n = reconstruct_q4k_into(&aos, 256, &mut out).unwrap();
        assert_eq!(n, 256);
        // low nibble 3, high nibble 5; sc=1 m=0 → 3.0 and 5.0 in 32-weight groups
        for i in 0..256 {
            let group = (i / 32) % 2;
            let expect = if group == 0 { 3.0 } else { 5.0 };
            assert!(
                (out[i] - expect).abs() < 0.02,
                "elem {i}: {} vs {expect}",
                out[i]
            );
        }
    }

    #[test]
    fn partial_tile_writes_n_elems_only() {
        let aos = synthetic_superblock();
        let mut out = [0f32; 16];
        let n = reconstruct_q4k_into(&aos, 10, &mut out).unwrap();
        assert_eq!(n, 10);
        for i in 0..10 {
            assert!((out[i] - 3.0).abs() < 0.02);
        }
    }

    #[test]
    fn truncated_payload_rejected() {
        let aos = [0u8; 80];
        let mut out = [0f32; 256];
        assert_eq!(
            reconstruct_q4k_into(&aos, 256, &mut out).err(),
            Some(OperatorError::InvalidPayload)
        );
    }
}
