use super::*;

pub(super) fn dequant_q4_0(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 18;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let half = BLOCK_ELEMS / 2;
        for j in 0..half {
            if b * BLOCK_ELEMS + j >= n_elems {
                break;
            }
            let byte = raw[bs + 2 + j];
            let x0 = (byte & 0x0F) as i32 - 8;
            let x1 = ((byte >> 4) & 0x0F) as i32 - 8;
            out[b * BLOCK_ELEMS + j] = x0 as f32 * scale;
            let hi = b * BLOCK_ELEMS + j + half;
            if hi < n_elems {
                out[hi] = x1 as f32 * scale;
            }
        }
    }
    Ok(n_elems)
}

/// `dequantize_row_q5_0` from ggml-quants.c — 5-bit weights, 32 elems per 22-byte block.
#[allow(clippy::needless_range_loop)] // Preserve the original indexed decoder.
pub(super) fn dequant_q5_0(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 22;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let d = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let qh = u32::from_le_bytes([raw[bs + 2], raw[bs + 3], raw[bs + 4], raw[bs + 5]]);
        let qs = &raw[bs + 6..bs + 22];
        let half = BLOCK_ELEMS / 2;
        for j in 0..half {
            let xh_0 = ((qh >> j) << 4) & 0x10;
            let xh_1 = ((qh >> (j + 16)) & 1) << 4;
            let x0 = ((qs[j] & 0x0F) as u32 | xh_0) as i32 - 16;
            let x1 = ((qs[j] >> 4) as u32 | xh_1) as i32 - 16;
            let lo = b * BLOCK_ELEMS + j;
            if lo < n_elems {
                out[lo] = x0 as f32 * d;
            }
            let hi = lo + half;
            if hi < n_elems {
                out[hi] = x1 as f32 * d;
            }
        }
    }
    Ok(n_elems)
}

pub(super) fn dequant_q5_1(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 24;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let d = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let m = half::f16::from_le_bytes([raw[bs + 2], raw[bs + 3]]).to_f32();
        let qh = u32::from_le_bytes([raw[bs + 4], raw[bs + 5], raw[bs + 6], raw[bs + 7]]);
        let qs = &raw[bs + 8..bs + 24];
        let half = BLOCK_ELEMS / 2;
        for j in 0..half {
            let xh_0 = ((qh >> j) << 4) & 0x10;
            let xh_1 = ((qh >> (j + 16)) & 1) << 4;
            let x0 = ((qs[j] & 0x0F) as u32 | xh_0) as f32;
            let x1 = ((qs[j] >> 4) as u32 | xh_1) as f32;
            let lo = b * BLOCK_ELEMS + j;
            if lo < n_elems {
                out[lo] = x0 * d + m;
            }
            let hi = lo + half;
            if hi < n_elems {
                out[hi] = x1 * d + m;
            }
        }
    }
    Ok(n_elems)
}

pub(super) fn dequant_mxfp4(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 17;
    const KVALUES_MXFP4: [f32; 16] = [
        0.0, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 0.0, -1.0, -2.0, -3.0, -4.0, -6.0, -8.0, -12.0,
    ];
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let e = raw[bs];
        let d_bits = if e < 2 {
            0x0020_0000u32 << u32::from(e)
        } else {
            u32::from(e - 1) << 23
        };
        let d = f32::from_bits(d_bits);
        let qs = &raw[bs + 1..bs + 17];
        let half = BLOCK_ELEMS / 2;
        for (j, &byte) in qs.iter().enumerate() {
            let x0 = KVALUES_MXFP4[(byte & 0x0F) as usize];
            let x1 = KVALUES_MXFP4[(byte >> 4) as usize];
            let lo = b * BLOCK_ELEMS + j;
            if lo < n_elems {
                out[lo] = x0 * d;
            }
            let hi = lo + half;
            if hi < n_elems {
                out[hi] = x1 * d;
            }
        }
    }
    Ok(n_elems)
}

pub(super) fn dequant_q8_0(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 34;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let elems = BLOCK_ELEMS.min(n_elems - b * BLOCK_ELEMS);
        for j in 0..elems {
            out[b * BLOCK_ELEMS + j] = raw[bs + 2 + j] as i8 as f32 * scale;
        }
    }
    Ok(n_elems)
}
