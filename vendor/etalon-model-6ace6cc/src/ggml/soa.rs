use super::*;

/// Convert one stock Q4_K superblock (144 B) → SoA superblock (160 B).
///
/// Pre-expands the 8 sub-block `(d·scale, dmin·min)` pairs to f16 so the GPU
/// GEMV never runs `get_scale_min_k4` / shared-header decode.
#[inline]
pub fn q4k_block_to_soa(src: &[u8], dst: &mut [u8]) -> Result<(), GgmlDequantError> {
    if src.len() < 144 || dst.len() < BLOCK_Q4K_SOA_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    // qs first (same nibble layout as stock Q4_K).
    dst[..128].copy_from_slice(&src[16..144]);
    let d = half::f16::from_le_bytes([src[0], src[1]]).to_f32();
    let dmin = half::f16::from_le_bytes([src[2], src[3]]).to_f32();
    let scales: [u8; 12] = src[4..16].try_into().unwrap_or([0; 12]);
    for j in 0..8 {
        let mut sc = 0u8;
        let mut m = 0u8;
        get_scale_min_k4(j, &scales, &mut sc, &mut m);
        let d_bits = half::f16::from_f32(d * sc as f32).to_le_bytes();
        let m_bits = half::f16::from_f32(dmin * m as f32).to_le_bytes();
        let o = 128 + j * 2;
        dst[o] = d_bits[0];
        dst[o + 1] = d_bits[1];
        let om = 144 + j * 2;
        dst[om] = m_bits[0];
        dst[om + 1] = m_bits[1];
    }
    Ok(())
}

/// Expand a full Q4_K tensor blob (row-major superblocks) into SoA layout.
/// `n_row_elems` = dims\[0\] (weights per row). `n_rows` = dims\[1\].
pub fn expand_q4k_tensor_to_soa(
    raw: &[u8],
    n_row_elems: usize,
    n_rows: usize,
    out: &mut [u8],
) -> Result<(), GgmlDequantError> {
    let src_row =
        ggml_row_bytes(GGML_TYPE_Q4_K, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let dst_row =
        ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let need = dst_row
        .checked_mul(n_rows)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    if out.len() < need || raw.len() < src_row.saturating_mul(n_rows) {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let n_blocks = n_row_elems.div_ceil(256);
    for r in 0..n_rows {
        let src_base = r * src_row;
        let dst_base = r * dst_row;
        for b in 0..n_blocks {
            let s = src_base + b * 144;
            let d = dst_base + b * BLOCK_Q4K_SOA_BYTES;
            if s + 144 > raw.len() || d + BLOCK_Q4K_SOA_BYTES > out.len() {
                return Err(GgmlDequantError::TruncatedInput);
            }
            q4k_block_to_soa(&raw[s..s + 144], &mut out[d..d + BLOCK_Q4K_SOA_BYTES])?;
        }
    }
    Ok(())
}

pub(super) fn dequant_q4_k_soa(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let n_blocks = n_elems.div_ceil(BLOCK_Q4K_SOA_ELEMS);
    if raw.len() < n_blocks * BLOCK_Q4K_SOA_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let mut out_idx = 0usize;
    for b in 0..n_blocks {
        let block = &raw[b * BLOCK_Q4K_SOA_BYTES..(b + 1) * BLOCK_Q4K_SOA_BYTES];
        let qs = &block[0..128];
        // Pre-expanded scales.
        let mut d_sub = [0f32; 8];
        let mut m_sub = [0f32; 8];
        for j in 0..8 {
            let o = 128 + j * 2;
            d_sub[j] = half::f16::from_le_bytes([block[o], block[o + 1]]).to_f32();
            let om = 144 + j * 2;
            m_sub[j] = half::f16::from_le_bytes([block[om], block[om + 1]]).to_f32();
        }
        // Same element order as stock Q4_K: for each of 4 groups of 64:
        //   32 low nibbles (sub 2g), then 32 high nibbles (sub 2g+1).
        let mut q_off = 0usize;
        for g in 0..4 {
            let sub0 = g * 2;
            let sub1 = g * 2 + 1;
            for l in 0..32 {
                if out_idx >= n_elems {
                    return Ok(out_idx);
                }
                let nib = (qs[q_off + l] & 0xF) as f32;
                out[out_idx] = d_sub[sub0] * nib - m_sub[sub0];
                out_idx += 1;
            }
            for l in 0..32 {
                if out_idx >= n_elems {
                    return Ok(out_idx);
                }
                let nib = (qs[q_off + l] >> 4) as f32;
                out[out_idx] = d_sub[sub1] * nib - m_sub[sub1];
                out_idx += 1;
            }
            q_off += 32;
        }
    }
    Ok(out_idx.min(n_elems))
}
