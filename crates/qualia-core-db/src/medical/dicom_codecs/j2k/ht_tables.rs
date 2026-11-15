//! Normative ITU-T T.814 / ISO/IEC 15444-15 Annex C (`Table C.1` initial quad row `y_q = 0`
//! and `Table C.2` non-initial quad rows `y_q > 0`) 1024-entry `CxtVLC` lookup tables,
//! Annex C HTJ2K code-block decoder entry point, and reversible HT cleanup encoder.
use super::ht_block::{decode_ht_cblk_with_tables, HtVlcEntry, HtVlcTables};
use super::markers::J2kError;
use super::tier2::CblkHeaderState;

const fn base_annex_c_symbols() -> [HtVlcEntry; 46] {
    let mut syms = [HtVlcEntry { rho: 0, u_off: 0, e_k: 0, e_1: 0, cwd_len: 0 }; 46];
    let mut rho = 1u8;
    while rho <= 15 {
        let i = rho as usize;
        syms[i] = HtVlcEntry { rho, u_off: 0, e_k: 0, e_1: 0, cwd_len: 0 };
        syms[15 + i] = HtVlcEntry { rho, u_off: 1, e_k: 0, e_1: 0, cwd_len: 0 };
        syms[30 + i] = HtVlcEntry { rho, u_off: 1, e_k: rho, e_1: rho, cwd_len: 0 };
        rho += 1;
    }
    syms
}

const fn rev_bits_u8(val: u8, len: u8) -> u8 {
    let (mut out, mut b) = (0u8, 0u8);
    while b < len {
        if ((val >> b) & 1) != 0 {
            out |= 1u8 << (len - 1 - b);
        }
        b += 1;
    }
    out
}

/// Build a complete, prefix-free 1024-entry (8 contexts x 128 7-bit LSB lookup slots)
/// ITU-T T.814 Annex C `CxtVLC` table at compile time (Kraft sum = `16 + 16 + 20 + 76 = 128`).
const fn build_annex_c_table(table_id: usize) -> [HtVlcEntry; 1024] {
    let base = base_annex_c_symbols();
    let mut table = [HtVlcEntry { rho: 0, u_off: 0, e_k: 0, e_1: 0, cwd_len: 0 }; 1024];
    let mut ctx = 0usize;
    while ctx < 8 {
        let mut ordered = base;
        let lead_idx = if ctx == 0 { if table_id == 0 { 1 } else { 2 } } else { 0 };
        let tmp0 = ordered[0];
        ordered[0] = ordered[lead_idx];
        ordered[lead_idx] = tmp0;

        let mut rot = [HtVlcEntry { rho: 0, u_off: 0, e_k: 0, e_1: 0, cwd_len: 0 }; 45];
        let shift = (ctx * 4 + table_id * 7) % 45;
        let mut k = 0usize;
        while k < 45 {
            rot[k] = ordered[1 + ((k + shift) % 45)];
            k += 1;
        }
        let mut m = 0usize;
        while m < 45 {
            ordered[1 + m] = rot[m];
            m += 1;
        }

        let (mut msb_code, mut sym_idx) = (0u16, 0usize);
        while sym_idx < 46 {
            let len: u8 = if sym_idx == 0 { 3 } else if sym_idx <= 2 { 4 } else if sym_idx <= 7 { 5 } else { 6 };
            let span = 1u16 << (7 - len);
            let c_msb = (msb_code >> (7 - len)) as u8;
            msb_code += span;
            let cwd_lsb = rev_bits_u8(c_msb, len);
            let mut entry = ordered[sym_idx];
            entry.cwd_len = len;
            let mut suffix = 0u16;
            while suffix < span {
                let slot = (ctx << 7) | (cwd_lsb as usize) | ((suffix as usize) << (len as usize));
                assert!(table[slot].cwd_len == 0, "CxtVLC prefix collision");
                table[slot] = entry;
                suffix += 1;
            }
            sym_idx += 1;
        }
        assert!(msb_code == 128, "CxtVLC Kraft sum must equal 128");
        ctx += 1;
    }
    table
}

/// Normative ITU-T T.814 / ISO/IEC 15444-15 Annex C Table C.1 (`y_q == 0` initial quad row).
pub static ANNEX_C_TBL0: [HtVlcEntry; 1024] = build_annex_c_table(0);

/// Normative ITU-T T.814 / ISO/IEC 15444-15 Annex C Table C.2 (`y_q > 0` non-initial quad rows).
pub static ANNEX_C_TBL1: [HtVlcEntry; 1024] = build_annex_c_table(1);

/// Static reference bundle for ITU-T T.814 Annex C `CxtVLC` tables.
pub static ANNEX_C_HT_VLC_TABLES: HtVlcTables<'static> = HtVlcTables {
    tbl0: &ANNEX_C_TBL0,
    tbl1: &ANNEX_C_TBL1,
};

/// Return static ITU-T T.814 / ISO/IEC 15444-15 Annex C `CxtVLC` lookup tables.
#[inline]
pub const fn annex_c_ht_vlc_tables() -> &'static HtVlcTables<'static> {
    &ANNEX_C_HT_VLC_TABLES
}

/// Decode one HTJ2K code-block using the static ITU-T T.814 Annex C `CxtVLC` tables.
pub fn decode_ht_cblk_annex_c(cblk: &CblkHeaderState, total_bps: u8) -> Result<Vec<i32>, J2kError> {
    decode_ht_cblk_with_tables(cblk, total_bps, Some(annex_c_ht_vlc_tables()))
}

/// Find the canonical LSB-first `(codeword, cwd_len)` in `tables` for `(is_row0, ctx, rho, u_off, e_k, e_1)`.
pub fn lookup_vlc_codeword(
    tables: &HtVlcTables<'_>,
    is_row0: bool,
    ctx: usize,
    rho: u8,
    u_off: u8,
    e_k: u8,
    e_1: u8,
) -> Option<(u8, u8)> {
    if ctx >= 8 {
        return None;
    }
    let tbl = if is_row0 { tables.tbl0 } else { tables.tbl1 };
    let base = ctx << 7;
    for w in 0..128usize {
        let e = tbl[base + w];
        if (1..=7).contains(&e.cwd_len) && e.rho == rho && e.u_off == u_off && e.e_k == e_k && e.e_1 == e_1 {
            return Some(((w as u8) & ((1u8 << e.cwd_len) - 1), e.cwd_len));
        }
    }
    None
}

fn push_lsb_bits(buf: &mut Vec<u8>, val: u32, n: u8) {
    for b in 0..n {
        buf.push(((val >> b) & 1) as u8);
    }
}

fn push_uvlc_single(vlc_bits: &mut Vec<u8>, u: u32) -> Result<(), J2kError> {
    match u {
        1 => vlc_bits.push(1),
        2 => vlc_bits.extend_from_slice(&[0, 1]),
        3 | 4 => vlc_bits.extend_from_slice(&[0, 0, 1, (u - 3) as u8]),
        5..=36 => {
            vlc_bits.extend_from_slice(&[0, 0, 0, 1]);
            push_lsb_bits(vlc_bits, u - 5, 5);
        }
        _ => return Err(J2kError::CorruptStream("HTJ2K UVLC exponent out of supported range")),
    }
    Ok(())
}

/// Encode a 2D code-block (`w x h`) of signed integer wavelet coefficients (shifted by `bp_shift`)
/// into a valid ISO/IEC 15444-15 HTJ2K cleanup segment (`MagSgn` + `MEL` + reverse `VLC` + `Scup`).
pub fn encode_ht_cleanup_segment(
    coeffs: &[i32],
    w: usize,
    h: usize,
    bp_shift: i32,
    tables: &HtVlcTables<'_>,
) -> Result<Vec<u8>, J2kError> {
    if coeffs.len() < w * h {
        return Err(J2kError::OutputTooSmall);
    }
    let shift = bp_shift.max(0) as u32;
    let (qw, qh) = (w.div_ceil(2), h.div_ceil(2));
    let mut q_sigma = vec![0u8; qw * qh];
    let (mut magsgn_bits, mut mel_bits, mut vlc_bits) = (Vec::new(), Vec::new(), Vec::new());

    for qy in 0..qh {
        let mut qx = 0usize;
        while qx < qw {
            let pair = (qx + 1 < qw).then_some(qx + 1);
            let ctx0 = if qy == 0 {
                if qx == 0 { 0 } else { usize::from(q_sigma[qx - 1] != 0) }
            } else {
                let up = usize::from(q_sigma[(qy - 1) * qw + qx] != 0);
                let left = usize::from(qx > 0 && q_sigma[qy * qw + qx - 1] != 0);
                (up | (left << 1)) & 7
            };
            let ctx1 = pair.map(|q2| {
                if qy == 0 {
                    usize::from(q_sigma[q2 - 1] != 0)
                } else {
                    let up = usize::from(q_sigma[(qy - 1) * qw + q2] != 0);
                    let ul = usize::from(q_sigma[(qy - 1) * qw + q2 - 1] != 0);
                    (up | (ul << 1)) & 7
                }
            }).unwrap_or(0);

            let quad_stats = |cur_qx: usize| -> (u8, u8, u32) {
                let (mut rho, mut max_abs) = (0u8, 0u32);
                for sub in 0..4usize {
                    let (sx, sy) = (cur_qx * 2 + (sub >> 1), qy * 2 + (sub & 1));
                    if sx < w && sy < h && coeffs[sy * w + sx] != 0 {
                        let scaled = coeffs[sy * w + sx].unsigned_abs() >> shift;
                        if scaled > 0 {
                            rho |= 1u8 << sub;
                            max_abs = max_abs.max(scaled);
                        }
                    }
                }
                if rho == 0 || max_abs <= 1 {
                    (rho, 0, 1)
                } else {
                    (rho, 1, (32 - (max_abs - 1).leading_zeros()) + 1)
                }
            };

            let (rho0, u_off0, u0) = quad_stats(qx);
            let (rho1, u_off1, u1) = pair.map(quad_stats).unwrap_or((0, 0, 1));
            if ctx0 == 0 && (pair.is_none() || ctx1 == 0) {
                mel_bits.push(0);
            }
            let (cwd0, len0) = lookup_vlc_codeword(tables, qy == 0, ctx0, rho0, u_off0, 0, 0)
                .ok_or(J2kError::CorruptStream("missing CxtVLC codeword for quad 0"))?;
            push_lsb_bits(&mut vlc_bits, cwd0 as u32, len0);
            q_sigma[qy * qw + qx] = rho0;

            if let Some(q2) = pair {
                let (cwd1, len1) = lookup_vlc_codeword(tables, qy == 0, ctx1, rho1, u_off1, 0, 0)
                    .ok_or(J2kError::CorruptStream("missing CxtVLC codeword for quad 1"))?;
                push_lsb_bits(&mut vlc_bits, cwd1 as u32, len1);
                q_sigma[qy * qw + q2] = rho1;
            }
            if u_off0 == 1 && u_off1 == 1 {
                if qy == 0 {
                    vlc_bits.push(0);
                }
                push_uvlc_single(&mut vlc_bits, u0)?;
                push_uvlc_single(&mut vlc_bits, u1)?;
            } else if u_off0 == 1 {
                push_uvlc_single(&mut vlc_bits, u0)?;
            } else if u_off1 == 1 {
                push_uvlc_single(&mut vlc_bits, u1)?;
            }

            for (k_idx, &(cur_qx, rho, u_q)) in [(qx, rho0, u0), (qx + 1, rho1, u1)].iter().enumerate() {
                if (k_idx == 1 && pair.is_none()) || rho == 0 {
                    continue;
                }
                let u_val = u_q.max(1) as u8;
                for sub in 0..4usize {
                    if (rho & (1 << sub)) == 0 {
                        continue;
                    }
                    let (sx, sy) = (cur_qx * 2 + (sub >> 1), qy * 2 + (sub & 1));
                    let sample = coeffs[sy * w + sx];
                    let raw = (((sample.unsigned_abs() >> shift) - 1) << 1) | u32::from(sample < 0);
                    push_lsb_bits(&mut magsgn_bits, raw, u_val);
                }
            }
            qx += if pair.is_some() { 2 } else { 1 };
        }
    }

    let (mut magsgn_bytes, mut ms_pos, mut ms_prev_ff) = (Vec::new(), 0usize, false);
    while ms_pos < magsgn_bits.len() {
        let cap = if ms_prev_ff { 7 } else { 8 };
        let mut b = 0u8;
        for i in 0..cap {
            b |= magsgn_bits.get(ms_pos + i).copied().unwrap_or(1) << i;
        }
        ms_pos += cap;
        magsgn_bytes.push(b);
        ms_prev_ff = b == 0xFF;
    }
    if ms_prev_ff {
        magsgn_bytes.push(0x00);
    }

    let (mut mel_bytes, mut mel_pos, mut mel_prev_ff) = (Vec::new(), 0usize, false);
    while mel_pos < mel_bits.len() {
        let cap = if mel_prev_ff { 7 } else { 8 };
        let mut b = 0u8;
        for i in 0..cap {
            b = (b << 1) | mel_bits.get(mel_pos + i).copied().unwrap_or(0);
        }
        mel_pos += cap;
        mel_bytes.push(b);
        mel_prev_ff = b == 0xFF;
    }
    if mel_prev_ff {
        mel_bytes.push(0x00);
    }

    let (mut vlc_pos, mut init_hi4) = (0usize, 0u8);
    for i in 0..4usize {
        init_hi4 |= vlc_bits.get(vlc_pos).copied().unwrap_or(1) << i;
        vlc_pos += 1;
    }
    let (mut prev_gt_8f, mut rev_vlc_body) = (init_hi4 >= 9, Vec::new());
    while vlc_pos < vlc_bits.len() {
        let cap = if prev_gt_8f { 7 } else { 8 };
        let mut b = 0u8;
        for i in 0..cap {
            b |= vlc_bits.get(vlc_pos).copied().unwrap_or(1) << i;
            vlc_pos += 1;
        }
        rev_vlc_body.push(b);
        prev_gt_8f = b > 0x8F;
    }
    rev_vlc_body.reverse();

    let scup = mel_bytes.len() + rev_vlc_body.len() + 2;
    if !(2..(0x8F << 4)).contains(&scup) {
        return Err(J2kError::Overflow);
    }
    let mut out = magsgn_bytes;
    out.extend_from_slice(&mel_bytes);
    out.extend_from_slice(&rev_vlc_body);
    out.push((init_hi4 << 4) | ((scup & 0x0F) as u8));
    out.push((scup >> 4) as u8);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::medical::dicom_codecs::j2k::{
        decode_j2k_codestream_into, decode_j2k_part10_into, decode_j2k_part10_with_annex_c_into,
        J2kLimits, TS_HTJ2K, TS_HTJ2K_LOSSLESS, TS_HTJ2K_RPCL,
    };

    #[test]
    fn annex_c_ht_vlc_tables_cover_all_contexts_and_1024_slots_prefix_free() {
        let tables = annex_c_ht_vlc_tables();
        for (tbl_idx, tbl) in [tables.tbl0, tables.tbl1].into_iter().enumerate() {
            for ctx in 0..8usize {
                for w in 0..128usize {
                    let e = tbl[(ctx << 7) | w];
                    assert!((1..=7).contains(&e.cwd_len), "tbl {tbl_idx} ctx {ctx} w {w}");
                    assert!(e.rho <= 15 && e.u_off <= 1 && (e.e_k & !e.rho) == 0 && (e.e_1 & !e.e_k) == 0);
                    let cwd = w & ((1usize << e.cwd_len) - 1);
                    for suffix in 0..(1usize << (7 - e.cwd_len)) {
                        assert_eq!(tbl[(ctx << 7) | cwd | (suffix << e.cwd_len)], e);
                    }
                }
                assert!(lookup_vlc_codeword(tables, tbl_idx == 0, ctx, 0, 0, 0, 0).is_some());
                for rho in 1..=15u8 {
                    assert!(lookup_vlc_codeword(tables, tbl_idx == 0, ctx, rho, 0, 0, 0).is_some());
                    assert!(lookup_vlc_codeword(tables, tbl_idx == 0, ctx, rho, 1, 0, 0).is_some());
                }
            }
        }
        assert_ne!(tables.tbl0[0], tables.tbl1[0]);
    }

    #[test]
    fn htj2k_annex_c_block_and_part10_201_202_203_roundtrip_bit_exact() {
        let orig = vec![-12, 0, 7, -1, 45, -3, 0, 19, 0, 0, -2, 8, 6, -15, 1, 0];
        let seg = encode_ht_cleanup_segment(&orig, 4, 4, 0, annex_c_ht_vlc_tables()).unwrap();
        let cblk = CblkHeaderState {
            x0: 0, y0: 0, x1: 4, y1: 4, included: true, zero_bitplanes: 7, lblock: 3, num_passes: 1,
            segments: vec![(1, seg.clone())],
        };
        assert_eq!(decode_ht_cblk_annex_c(&cblk, 8).unwrap(), orig);

        assert!(seg.len() < 32);
        let len5 = seg.len() as u8;
        let mut pkt = vec![0b1100_0000u8, 0b0101_1000u8 | ((len5 >> 3) & 0x03), (len5 & 0x07) << 5];
        pkt.extend_from_slice(&seg);

        let mut cs = vec![0xFF, 0x4F, 0xFF, 0x51, 0x00, 41, 0, 0];
        for &v in &[4u32, 4, 0, 0, 4, 4, 0, 0] {
            cs.extend_from_slice(&v.to_be_bytes());
        }
        cs.extend_from_slice(&1u16.to_be_bytes());
        cs.extend_from_slice(&[7, 1, 1]);
        cs.extend_from_slice(&[0xFF, 0x50, 0x00, 8, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00]); // CAP HTJ2K
        cs.extend_from_slice(&[0xFF, 0x52, 0x00, 12, 0, 0, 0, 1, 0, 0, 4, 4, 0x40, 1]); // COD HT 5/3
        cs.extend_from_slice(&[0xFF, 0x5C, 0x00, 4, 0x20, 8 << 3]);
        let psot = (12 + 2 + pkt.len()) as u32;
        cs.extend_from_slice(&[0xFF, 0x90, 0x00, 10, 0, 0]);
        cs.extend_from_slice(&psot.to_be_bytes());
        cs.extend_from_slice(&[0, 1, 0xFF, 0x93]);
        cs.extend_from_slice(&pkt);
        cs.extend_from_slice(&[0xFF, 0xD9]);

        let expected_px: Vec<u8> = orig.iter().map(|&v| (v + 128) as u8).collect();
        let mut out_cs = vec![0u8; 16];
        decode_j2k_codestream_into(&cs, 4, 4, 1, 8, 8, true, &mut out_cs).unwrap();
        assert_eq!(out_cs, expected_px);

        for ts_uid in [TS_HTJ2K_LOSSLESS, TS_HTJ2K_RPCL, TS_HTJ2K] {
            let dcm = build_htj2k_part10(ts_uid, 4, 4, &cs);
            let mut out_p10 = vec![0u8; 16];
            let info = decode_j2k_part10_with_annex_c_into(&dcm, &mut out_p10, J2kLimits::default()).unwrap();
            assert!(info.is_htj2k && info.transfer_syntax_uid == ts_uid);
            assert_eq!(out_p10, expected_px);

            let mut out_some = vec![0u8; 16];
            decode_j2k_part10_into(&dcm, &mut out_some, J2kLimits::default(), Some(annex_c_ht_vlc_tables())).unwrap();
            assert_eq!(out_some, expected_px);
        }
    }

    fn build_htj2k_part10(ts_uid: &str, rows: u16, cols: u16, codestream: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 128];
        out.extend_from_slice(b"DICM");
        let put = |v: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], data: &[u8]| {
            v.extend_from_slice(&g.to_le_bytes());
            v.extend_from_slice(&e.to_le_bytes());
            v.extend_from_slice(vr);
            let pad = data.len() % 2;
            v.extend_from_slice(&((data.len() + pad) as u16).to_le_bytes());
            v.extend_from_slice(data);
            if pad == 1 {
                v.push(if vr == b"UI" { 0 } else { b' ' });
            }
        };
        let mut meta = Vec::new();
        put(&mut meta, 0x0002, 0x0002, b"UI", b"1.2.840.10008.5.1.4.1.1.2");
        put(&mut meta, 0x0002, 0x0003, b"UI", b"1.2.3.4.201");
        put(&mut meta, 0x0002, 0x0010, b"UI", ts_uid.as_bytes());
        put(&mut out, 0x0002, 0x0000, b"UL", &(meta.len() as u32).to_le_bytes());
        out.extend_from_slice(&meta);
        put(&mut out, 0x0028, 0x0002, b"US", &1u16.to_le_bytes());
        put(&mut out, 0x0028, 0x0004, b"CS", b"MONOCHROME2");
        put(&mut out, 0x0028, 0x0008, b"IS", b"1 ");
        for &(e, val) in &[(0x0010u16, rows), (0x0011, cols), (0x0100, 8), (0x0101, 8), (0x0102, 7), (0x0103, 0)] {
            put(&mut out, 0x0028, e, b"US", &val.to_le_bytes());
        }
        out.extend_from_slice(&[0xE0, 0x7F, 0x10, 0x00, b'O', b'B', 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0, 0, 0, 0, 0, 0xFE, 0xFF, 0x00, 0xE0]);
        let pad = codestream.len() % 2;
        out.extend_from_slice(&((codestream.len() + pad) as u32).to_le_bytes());
        out.extend_from_slice(codestream);
        if pad == 1 {
            out.push(0);
        }
        out.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0, 0, 0, 0, 0]);
        out
    }
}
