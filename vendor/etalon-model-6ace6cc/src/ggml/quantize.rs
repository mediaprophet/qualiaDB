use super::*;

/// Quantize 256 f32 weights into one Q4_K_SOA superblock (160 bytes).
///
/// Layout produced:
/// - `[0..128)`: nibbles (same as Q4_K: 4 groups × 32 bytes, low=even sub, high=odd sub)
/// - `[128..144)`: 8 × f16 effective scale (d_sub[j])
/// - `[144..160)`: 8 × f16 effective min (m_sub[j])
///
/// Dequant formula: `w = d_sub[j] * nibble - m_sub[j]`
/// So: `d_sub[j] = (max_j - min_j) / 15`, `m_sub[j] = -min_j`
/// And: `nibble = round((w + m_sub[j]) / d_sub[j])` clamped to [0, 15].
#[allow(clippy::needless_range_loop)] // Preserve original numerical iteration order.
pub(super) fn quantize_block_f32_to_q4_k_soa(src: &[f32], out: &mut [u8]) {
    debug_assert!(out.len() >= BLOCK_Q4K_SOA_BYTES);
    // Zero qs region so unused nibbles are 0.
    out[..128].fill(0);

    for j in 0..8 {
        let sub_start = j * 32;
        let sub_end = (sub_start + 32).min(src.len());

        // Find min/max for this sub-block.
        let mut min_val = 0.0f32;
        let mut max_val = 0.0f32;
        if sub_start < sub_end {
            min_val = src[sub_start];
            max_val = src[sub_start];
            for i in sub_start + 1..sub_end {
                let w = src[i];
                if w < min_val {
                    min_val = w;
                }
                if w > max_val {
                    max_val = w;
                }
            }
        }

        let scale = if max_val > min_val {
            (max_val - min_val) / 15.0
        } else {
            1.0
        };
        // Dequant: w = d_sub * nibble - m_sub
        //   nibble=0 → w = -m_sub = min_val  →  m_sub = -min_val
        //   nibble=15 → w = d_sub*15 - m_sub = max_val  →  d_sub = (max_val - min_val)/15 = scale
        let d_sub = half::f16::from_f32(scale);
        let m_sub = half::f16::from_f32(-min_val);
        let d_actual = d_sub.to_f32();
        let m_actual = m_sub.to_f32();

        // Store f16 scale/min in SoA region.
        let d_bytes = d_sub.to_le_bytes();
        let m_bytes = m_sub.to_le_bytes();
        out[128 + j * 2] = d_bytes[0];
        out[128 + j * 2 + 1] = d_bytes[1];
        out[144 + j * 2] = m_bytes[0];
        out[144 + j * 2 + 1] = m_bytes[1];

        // Quantize and pack nibbles.
        // Group g = j / 2; low nibble if j even, high if j odd.
        let g = j / 2;
        let q_off = g * 32;
        let is_high = j % 2 == 1;
        for l in 0..32 {
            let nibble: u8 = if sub_start + l < sub_end {
                let w = src[sub_start + l];
                let q = ((w + m_actual) / d_actual).round();
                q.clamp(0.0, 15.0) as u8
            } else {
                0
            };
            let bi = q_off + l;
            if is_high {
                out[bi] = (out[bi] & 0x0F) | (nibble << 4);
            } else {
                out[bi] = (out[bi] & 0xF0) | (nibble & 0x0F);
            }
        }
    }
}

/// Quantize a full f32 weight matrix to Q4_K_SOA layout.
///
/// `src`: row-major f32 weights, `n_row_elems × n_rows` elements.
/// `out`: destination buffer, must be at least `ggml_row_bytes(Q4_K_SOA, n_row_elems) * n_rows` bytes.
pub fn quantize_f32_to_q4_k_soa_tensor(
    src: &[f32],
    n_row_elems: usize,
    n_rows: usize,
    out: &mut [u8],
) -> Result<(), GgmlDequantError> {
    let dst_row =
        ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let need = dst_row
        .checked_mul(n_rows)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    if out.len() < need {
        return Err(GgmlDequantError::BufferTooSmall);
    }
    if src.len() < n_row_elems * n_rows {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let n_blocks = n_row_elems.div_ceil(BLOCK_Q4K_SOA_ELEMS);
    for r in 0..n_rows {
        let src_base = r * n_row_elems;
        let dst_base = r * dst_row;
        for b in 0..n_blocks {
            let block_start = src_base + b * BLOCK_Q4K_SOA_ELEMS;
            let block_end = (block_start + BLOCK_Q4K_SOA_ELEMS).min(src_base + n_row_elems);
            let block_src = &src[block_start..block_end];
            let dst_off = dst_base + b * BLOCK_Q4K_SOA_BYTES;
            quantize_block_f32_to_q4_k_soa(
                block_src,
                &mut out[dst_off..dst_off + BLOCK_Q4K_SOA_BYTES],
            );
        }
    }
    Ok(())
}
