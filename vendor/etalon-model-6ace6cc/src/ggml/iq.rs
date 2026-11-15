use super::*;

/// GGML `dequantize_row_iq4_nl`.
///
/// The Qwen3.8 PLE has 160 values per row, so it is five of these compact
/// 18-byte blocks. Each block has one f16 scale and 16 packed codebook pairs.
pub(super) fn dequant_iq4_nl(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 18;
    const VALUES: [f32; 16] = [
        -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0, 1.0, 13.0, 25.0, 38.0, 53.0,
        69.0, 89.0, 113.0,
    ];
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for block_index in 0..n_blocks {
        let base = block_index * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[base], raw[base + 1]]).to_f32();
        let block_start = block_index * BLOCK_ELEMS;
        for lane in 0..16 {
            let packed = raw[base + 2 + lane];
            let lo = block_start + lane;
            if lo < n_elems {
                out[lo] = scale * VALUES[(packed & 0x0f) as usize];
            }
            let hi = lo + 16;
            if hi < n_elems {
                out[hi] = scale * VALUES[(packed >> 4) as usize];
            }
        }
    }
    Ok(n_elems)
}

/// GGML `dequantize_row_iq4_xs`.
///
/// One 136-byte superblock covers 256 values: an f16 master scale, eight
/// packed 6-bit scales and 128 packed non-linear 4-bit codes.  The lane order
/// deliberately mirrors ggml's CPU/CUDA reference: each 32-value group stores
/// four low-nibble values followed by their four high-nibble partners.
pub(super) fn dequant_iq4_xs(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 136;
    const VALUES: [f32; 16] = [
        -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0, 1.0, 13.0, 25.0, 38.0, 53.0,
        69.0, 89.0, 113.0,
    ];
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for block_index in 0..n_blocks {
        let base = block_index * BLOCK_BYTES;
        let block = &raw[base..base + BLOCK_BYTES];
        let master = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let scales_h = u16::from_le_bytes([block[2], block[3]]);
        let scales_l = &block[4..8];
        let qs = &block[8..136];
        let block_start = block_index * BLOCK_ELEMS;
        for group in 0..8 {
            let low = (scales_l[group / 2] >> (4 * (group % 2))) & 0x0f;
            let high = ((scales_h >> (2 * group)) & 0x03) as u8;
            let scale = master * (((low | (high << 4)) as i32 - 32) as f32);
            let q_base = group * 16;
            let y_base = block_start + group * 32;
            for lane in 0..16 {
                let packed = qs[q_base + lane];
                let lo = y_base + lane;
                if lo < n_elems {
                    out[lo] = scale * VALUES[(packed & 0x0f) as usize];
                }
                let hi = lo + 16;
                if hi < n_elems {
                    out[hi] = scale * VALUES[(packed >> 4) as usize];
                }
            }
        }
    }
    Ok(n_elems)
}
