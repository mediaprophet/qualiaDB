//! Pure-Rust ITU-T T.81 SOF1 Extended Sequential 12-bit JPEG decoder (`1.2.840.10008.1.2.4.51`).
//! Supports 8-bit (`Pq=0`) & 16-bit (`Pq=1`) DQT, extended DC/AC DHT categories (`0..=15`),
//! `DRI`/`RSTm`, interleaved & non-interleaved sequential scans, IEEE-1180 `f64` separable IDCT,
//! and 12-bit `MONOCHROME1`/`MONOCHROME2`/`RGB`/`YBR_FULL`/`YBR_FULL_422` LE `u16` output.
use super::jpeg::JpegError;
use super::jpeg_12bit_scan::{
    decode_block_into, idct_basis, write_block_to_plane, FragmentCursor, HuffTable, ScanBitReader,
};

#[derive(Clone, Copy)]
struct CompSpec {
    id: u8,
    h: usize,
    v: usize,
    tq: usize,
}

/// Decode one single-frame ITU-T T.81 SOF1 12-bit Extended Sequential JPEG stream
/// into little-endian `u16` bytes (`0..=4095`). Fails before touching external caller buffers.
pub fn decode_jpeg_12bit_frame(
    fragments: &[&[u8]],
    rows: u16,
    cols: u16,
    spp: u16,
    photometric: &str,
) -> Result<Vec<u8>, JpegError> {
    let mut cur = FragmentCursor::new(fragments);
    if cur.next_u8()? != 0xFF || cur.next_u8()? != 0xD8 {
        return Err(JpegError::CorruptStream("missing JPEG SOI marker"));
    }
    let mut qtables: [Option<[u16; 64]>; 4] = [None, None, None, None];
    let mut dc_tables: [Option<HuffTable>; 4] = [None, None, None, None];
    let mut ac_tables: [Option<HuffTable>; 4] = [None, None, None, None];
    let mut restart_interval: usize = 0;
    let mut comps: Vec<CompSpec> = Vec::new();
    let mut planes: Vec<Vec<u16>> = Vec::new();
    let mut decoded_mask: Vec<bool> = Vec::new();
    let mut plane_dims: Vec<(usize, usize)> = Vec::new();
    let (mut h_max, mut v_max) = (1usize, 1usize);
    let basis = idct_basis();
    let mut carry_marker: Option<u8> = None;

    loop {
        let marker = if let Some(m) = carry_marker.take() {
            m
        } else {
            let mut b = cur.next_u8()?;
            while b != 0xFF { b = cur.next_u8()?; }
            let mut m = cur.next_u8()?;
            while m == 0xFF { m = cur.next_u8()?; }
            m
        };
        if marker == 0xD9 {
            break;
        }
        if marker == 0x00 || marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            continue;
        }
        let seg_len = cur.next_u16_be()? as usize;
        if seg_len < 2 {
            return Err(JpegError::CorruptStream("invalid 12-bit JPEG marker length"));
        }
        let mut rem = seg_len - 2;
        match marker {
            0xDB => {
                while rem > 0 {
                    let hdr = cur.next_u8()?;
                    rem -= 1;
                    let (pq, tq) = (hdr >> 4, (hdr & 0x0F) as usize);
                    if pq > 1 || tq >= 4 {
                        return Err(JpegError::CorruptStream("invalid 12-bit DQT header"));
                    }
                    let need = if pq == 0 { 64 } else { 128 };
                    if rem < need {
                        return Err(JpegError::CorruptStream("truncated 12-bit DQT table"));
                    }
                    let mut tbl = [0u16; 64];
                    for entry in &mut tbl {
                        let v = if pq == 0 { cur.next_u8()? as u16 } else { cur.next_u16_be()? };
                        if v == 0 {
                            return Err(JpegError::CorruptStream("zero JPEG quantization entry"));
                        }
                        *entry = v;
                    }
                    qtables[tq] = Some(tbl);
                    rem -= need;
                }
            }
            0xC4 => {
                while rem > 0 {
                    let hdr = cur.next_u8()?;
                    rem -= 1;
                    let (tc, th) = (hdr >> 4, (hdr & 0x0F) as usize);
                    if tc > 1 || th >= 4 || rem < 16 {
                        return Err(JpegError::CorruptStream("invalid 12-bit DHT header"));
                    }
                    let mut bits = [0u8; 16];
                    for b in &mut bits { *b = cur.next_u8()?; }
                    rem -= 16;
                    let total: usize = bits.iter().map(|&b| b as usize).sum();
                    if total > 256 || rem < total {
                        return Err(JpegError::CorruptStream("invalid 12-bit DHT symbol count"));
                    }
                    let mut vals = vec![0u8; total];
                    for v in &mut vals { *v = cur.next_u8()?; }
                    rem -= total;
                    let ht = HuffTable::build(&bits, vals)?;
                    if tc == 0 { dc_tables[th] = Some(ht); } else { ac_tables[th] = Some(ht); }
                }
            }
            0xDD => {
                if rem != 2 {
                    return Err(JpegError::CorruptStream("invalid JPEG DRI length"));
                }
                restart_interval = cur.next_u16_be()? as usize;
            }
            0xC1 => {
                if !comps.is_empty() {
                    return Err(JpegError::CorruptStream("duplicate SOF1 marker"));
                }
                if rem < 6 {
                    return Err(JpegError::CorruptStream("invalid SOF1 length"));
                }
                let prec = cur.next_u8()?;
                let h = cur.next_u16_be()?;
                let w = cur.next_u16_be()?;
                let nf = cur.next_u8()?;
                if prec != 12 {
                    return Err(JpegError::BitPrecisionMismatch);
                }
                if h != rows || w != cols {
                    return Err(JpegError::DimensionMismatch);
                }
                if nf as u16 != spp || rem != 6 + 3 * nf as usize {
                    return Err(JpegError::ColorOrSubsamplingMismatch);
                }
                for _ in 0..nf as usize {
                    let id = cur.next_u8()?;
                    let hv = cur.next_u8()?;
                    let tq = cur.next_u8()? as usize;
                    let (hs, vs) = ((hv >> 4) as usize, (hv & 0x0F) as usize);
                    if !(1..=4).contains(&hs) || !(1..=4).contains(&vs) || tq >= 4 || comps.iter().any(|c| c.id == id) {
                        return Err(JpegError::CorruptStream("invalid 12-bit SOF1 component"));
                    }
                    h_max = h_max.max(hs);
                    v_max = v_max.max(vs);
                    comps.push(CompSpec { id, h: hs, v: vs, tq });
                }
                let hv_vec: Vec<(usize, usize)> = comps.iter().map(|c| (c.h, c.v)).collect();
                match photometric {
                    "MONOCHROME1" | "MONOCHROME2" if comps.len() == 1 => {}
                    "RGB" | "YBR_FULL" if hv_vec == [(1, 1); 3] => {}
                    "YBR_FULL_422" if hv_vec == [(2, 1), (1, 1), (1, 1)] => {}
                    _ => return Err(JpegError::ColorOrSubsamplingMismatch),
                }
                for c in &comps {
                    let pw = (cols as usize * c.h).div_ceil(h_max);
                    let ph = (rows as usize * c.v).div_ceil(v_max);
                    plane_dims.push((pw, ph));
                    planes.push(vec![0u16; pw * ph]);
                    decoded_mask.push(false);
                }
            }
            0xC0 | 0xC2..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF | 0xF7 => {
                return Err(JpegError::Unsupported("12-bit frame requires SOF1 Extended Sequential"));
            }
            0xDA => {
                if comps.is_empty() || rem < 4 {
                    return Err(JpegError::CorruptStream("SOS before SOF1 or truncated"));
                }
                let ns = cur.next_u8()? as usize;
                if ns == 0 || (ns != 1 && ns != comps.len()) || rem != 4 + 2 * ns {
                    return Err(JpegError::CorruptStream("invalid 12-bit SOS component count"));
                }
                let mut scan_comps = Vec::with_capacity(ns);
                for _ in 0..ns {
                    let cs = cur.next_u8()?;
                    let td_ta = cur.next_u8()?;
                    let (td, ta) = ((td_ta >> 4) as usize, (td_ta & 0x0F) as usize);
                    let cidx = comps.iter().position(|c| c.id == cs).ok_or(JpegError::CorruptStream("unknown SOS component"))?;
                    if td >= 4 || ta >= 4 || scan_comps.iter().any(|&(idx, _, _)| idx == cidx) {
                        return Err(JpegError::CorruptStream("invalid SOS selector"));
                    }
                    scan_comps.push((cidx, td, ta));
                }
                let (ss, se, ah_al) = (cur.next_u8()?, cur.next_u8()?, cur.next_u8()?);
                if ss != 0 || se != 63 || ah_al != 0 {
                    return Err(JpegError::Unsupported("12-bit JPEG must be sequential DCT"));
                }
                let mcu_cols = if ns == 1 {
                    plane_dims[scan_comps[0].0].0.div_ceil(8)
                } else {
                    (cols as usize).div_ceil(h_max * 8)
                };
                let mcu_rows = if ns == 1 {
                    plane_dims[scan_comps[0].0].1.div_ceil(8)
                } else {
                    (rows as usize).div_ceil(v_max * 8)
                };
                let mut rdr = ScanBitReader::new(&mut cur);
                let mut dc_pred = vec![0i32; comps.len()];
                let mut mcu_idx = 0usize;
                let total_mcus = mcu_cols * mcu_rows;
                for my in 0..mcu_rows {
                    for mx in 0..mcu_cols {
                        if restart_interval > 0 && mcu_idx > 0 && mcu_idx % restart_interval == 0 && mcu_idx < total_mcus {
                            rdr.consume_restart((mcu_idx / restart_interval) - 1)?;
                            dc_pred.fill(0);
                        }
                        for &(cidx, td, ta) in &scan_comps {
                            let dc_tbl = dc_tables[td].as_ref().ok_or(JpegError::CorruptStream("missing 12-bit DC Huffman table"))?;
                            let ac_tbl = ac_tables[ta].as_ref().ok_or(JpegError::CorruptStream("missing 12-bit AC Huffman table"))?;
                            let qtbl = qtables[comps[cidx].tq].as_ref().ok_or(JpegError::CorruptStream("missing 12-bit quantization table"))?;
                            let (ch, cv) = if ns == 1 { (1, 1) } else { (comps[cidx].h, comps[cidx].v) };
                            let (pw, ph) = plane_dims[cidx];
                            for vy in 0..cv {
                                for hx in 0..ch {
                                    let blk = decode_block_into(&mut rdr, dc_tbl, ac_tbl, qtbl, &mut dc_pred[cidx], &basis)?;
                                    write_block_to_plane(&mut planes[cidx], pw, ph, mx * ch + hx, my * cv + vy, &blk);
                                }
                            }
                            decoded_mask[cidx] = true;
                        }
                        mcu_idx += 1;
                    }
                }
                carry_marker = rdr.finish_scan()?;
            }
            _ => cur.skip_exact(rem)?,
        }
    }

    if comps.is_empty() || decoded_mask.iter().any(|&ok| !ok) {
        return Err(JpegError::CorruptStream("12-bit JPEG missing SOF1 or component scan"));
    }
    let out_len = (rows as usize) * (cols as usize) * (spp as usize) * 2;
    let mut out = Vec::with_capacity(out_len);
    for y in 0..rows as usize {
        for x in 0..cols as usize {
            if spp == 1 {
                let v = planes[0][y * plane_dims[0].0 + x];
                out.extend_from_slice(&v.to_le_bytes());
            } else if photometric == "RGB" {
                for c in 0..3 {
                    let v = planes[c][y * plane_dims[c].0 + x];
                    out.extend_from_slice(&v.to_le_bytes());
                }
            } else {
                let yv = planes[0][y * plane_dims[0].0 + x] as f64;
                let cx = if photometric == "YBR_FULL_422" { x >> 1 } else { x };
                let cb = planes[1][y * plane_dims[1].0 + cx] as f64 - 2048.0;
                let cr = planes[2][y * plane_dims[2].0 + cx] as f64 - 2048.0;
                let r = (yv + 1.402 * cr).round().clamp(0.0, 4095.0) as u16;
                let g = (yv - 0.344_136_286_201_022_14 * cb - 0.714_136_286_201_022_1 * cr)
                    .round().clamp(0.0, 4095.0) as u16;
                let b = (yv + 1.772 * cb).round().clamp(0.0, 4095.0) as u16;
                out.extend_from_slice(&r.to_le_bytes());
                out.extend_from_slice(&g.to_le_bytes());
                out.extend_from_slice(&b.to_le_bytes());
            }
        }
    }
    Ok(out)
}
