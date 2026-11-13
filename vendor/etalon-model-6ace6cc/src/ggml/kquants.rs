use super::*;

/// `get_scale_min_k4` from ggml-quants.c — unpack 6-bit scale/min pairs.
#[inline]
pub(super) fn get_scale_min_k4(j: usize, scales: &[u8; 12], sc: &mut u8, m: &mut u8) {
    if j < 4 {
        *sc = scales[j] & 63;
        *m = scales[j + 4] & 63;
    } else {
        *sc = (scales[j + 4] & 0xF) | ((scales[j - 4] >> 6) << 4);
        *m = (scales[j + 4] >> 4) | ((scales[j] >> 6) << 4);
    }
}

pub(super) fn dequant_q4_k(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 144;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let mut out_idx = 0usize;
    for b in 0..n_blocks {
        let block = &raw[b * BLOCK_BYTES..b * BLOCK_BYTES + BLOCK_BYTES];
        let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
        let scales: [u8; 12] = block[4..16].try_into().unwrap_or([0; 12]);
        let qs = &block[16..144];

        let block_elems = BLOCK_ELEMS.min(n_elems - b * BLOCK_ELEMS);
        let mut q_off = 0usize;
        let mut is = 0usize;
        let mut j = 0usize;
        while j < block_elems && out_idx < n_elems {
            let mut sc = 0u8;
            let mut m = 0u8;
            get_scale_min_k4(is, &scales, &mut sc, &mut m);
            let d1 = d * sc as f32;
            let m1 = dmin * m as f32;
            get_scale_min_k4(is + 1, &scales, &mut sc, &mut m);
            let d2 = d * sc as f32;
            let m2 = dmin * m as f32;

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

/// GGML `dequantize_row_q5_K`.
///
/// `Q5_K` keeps Q4_K's eight scale/min pairs and adds a 32-byte bitplane.
/// The bitplane is indexed as `[bit-plane][lane]`: each successive group of
/// 32 unpacked values consumes the next bit of the same 32 source bytes.
pub(super) fn dequant_q5_k(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 176;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let mut out_idx = 0usize;
    for block_index in 0..n_blocks {
        let block = &raw[block_index * BLOCK_BYTES..(block_index + 1) * BLOCK_BYTES];
        let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
        let scales: [u8; 12] = block[4..16].try_into().unwrap_or([0; 12]);
        let qh = &block[16..48];
        let qs = &block[48..176];
        let block_elems = BLOCK_ELEMS.min(n_elems - block_index * BLOCK_ELEMS);
        let mut q_off = 0usize;
        let mut scale_index = 0usize;
        let mut element = 0usize;
        while element < block_elems {
            let mut scale = 0u8;
            let mut minimum = 0u8;
            get_scale_min_k4(scale_index, &scales, &mut scale, &mut minimum);
            let d0 = d * scale as f32;
            let m0 = dmin * minimum as f32;
            get_scale_min_k4(scale_index + 1, &scales, &mut scale, &mut minimum);
            let d1 = d * scale as f32;
            let m1 = dmin * minimum as f32;
            for lane in 0..32 {
                if element >= block_elems {
                    break;
                }
                let high = ((qh[lane] >> (element / 32)) & 1) << 4;
                out[out_idx] = d0 * ((qs[q_off + lane] & 0x0f) | high) as f32 - m0;
                out_idx += 1;
                element += 1;
            }
            for lane in 0..32 {
                if element >= block_elems {
                    break;
                }
                let high = ((qh[lane] >> (element / 32)) & 1) << 4;
                out[out_idx] = d1 * ((qs[q_off + lane] >> 4) | high) as f32 - m1;
                out_idx += 1;
                element += 1;
            }
            q_off += 32;
            scale_index += 2;
        }
    }
    Ok(out_idx)
}

#[allow(clippy::identity_op)] // Keep the original explicit bit-plane shifts.
pub(super) fn dequant_q6_k_block(block: &[u8; 210], out: &mut [f32]) {
    let blk = bytemuck::from_bytes::<BlockQ6K>(block);
    let d = half::f16::from_bits(blk.d).to_f32();
    let mut ql_off = 0usize;
    let mut qh_off = 0usize;
    let mut sc_off = 0usize;
    let mut y_off = 0usize;

    for _ in 0..2 {
        for l in 0..32 {
            let is = l / 16;
            let q1 =
                ((blk.ql[ql_off + l] & 0xF) | (((blk.qh[qh_off + l] >> 0) & 3) << 4)) as i8 - 32;
            let q2 = ((blk.ql[ql_off + l + 32] & 0xF) | (((blk.qh[qh_off + l] >> 2) & 3) << 4))
                as i8
                - 32;
            let q3 =
                ((blk.ql[ql_off + l] >> 4) | (((blk.qh[qh_off + l] >> 4) & 3) << 4)) as i8 - 32;
            let q4 = ((blk.ql[ql_off + l + 32] >> 4) | (((blk.qh[qh_off + l] >> 6) & 3) << 4))
                as i8
                - 32;
            let sc = &blk.scales[sc_off..sc_off + 8];
            out[y_off + l] = d * sc[is] as f32 * q1 as f32;
            out[y_off + l + 32] = d * sc[is + 2] as f32 * q2 as f32;
            out[y_off + l + 64] = d * sc[is + 4] as f32 * q3 as f32;
            out[y_off + l + 96] = d * sc[is + 6] as f32 * q4 as f32;
        }
        y_off += 128;
        ql_off += 64;
        qh_off += 32;
        sc_off += 8;
    }
}

pub(super) fn dequant_q6_k(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 210;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let mut written = 0usize;
    for b in 0..n_blocks {
        let block: &[u8; 210] = raw[b * BLOCK_BYTES..b * BLOCK_BYTES + BLOCK_BYTES]
            .try_into()
            .map_err(|_| GgmlDequantError::TruncatedInput)?;
        let elems = BLOCK_ELEMS.min(n_elems - written);
        let mut block_out = [0f32; BLOCK_ELEMS];
        dequant_q6_k_block(block, &mut block_out);
        out[written..written + elems].copy_from_slice(&block_out[..elems]);
        written += elems;
    }
    Ok(written)
}
