//! ISO/IEC 15444-1 JPEG 2000 (.90/.91) & ISO/IEC 15444-15 HTJ2K (.201-.203) decoder.
mod color;
mod dwt;
mod ht_block;
pub mod ht_tables;
mod markers;
mod mq_coder;
mod mq_ebcot;
mod tier2;
mod types;
#[cfg(test)]
mod tests;

pub use ht_block::{HtVlcEntry, HtVlcTables};
pub use ht_tables::{
    annex_c_ht_vlc_tables, decode_ht_cblk_annex_c, encode_ht_cleanup_segment, lookup_vlc_codeword,
    ANNEX_C_HT_VLC_TABLES, ANNEX_C_TBL0, ANNEX_C_TBL1,
};
pub use markers::{
    J2kError, J2kLimits, ProgressionOrder, WaveletTransform, TS_HTJ2K, TS_HTJ2K_LOSSLESS,
    TS_HTJ2K_RPCL, TS_J2K, TS_J2K_LOSSLESS,
};
pub use tier2::{TagTree, TagTreeLevel};
pub use types::CodingStyle;

use super::encapsulated::{collect_part10_dataset_frames, concat_fragments, parse_encapsulated_part10, EncapLimits};
use crate::medical::dicom_dataset::{Part10Dataset, WorkBudget};
use color::{apply_mct_and_level_shift, pack_tile_into_output};
use dwt::{subband_total_bitplanes, synthesize_level_53, synthesize_level_97};
use ht_block::decode_ht_cblk_with_tables;
use markers::{parse_j2k_codestream, ParsedCodestream};
use mq_ebcot::decode_ebcot_cblk;
use tier2::{build_subband_grid, decode_packet, packet_iteration_order, PacketBitReader, SubbandGrid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct J2kFrameInfo<'a> {
    pub source_blake3: [u8; 32],
    pub transfer_syntax_uid: &'a str,
    pub rows: u16,
    pub columns: u16,
    pub frames: u32,
    pub samples_per_pixel: u16,
    pub bits_allocated: u16,
    pub bits_stored: u16,
    pub signed: bool,
    pub source_photometric: &'a str,
    pub transform: WaveletTransform,
    pub is_htj2k: bool,
    pub bytes: usize,
}

fn decode_frame_with_tables(
    codestream: &[u8],
    expected_w: u16,
    expected_h: u16,
    expected_nc: u16,
    expected_ba: u16,
    expected_bs: u16,
    lossless_only: bool,
    limits: J2kLimits,
    ht_tables: Option<&HtVlcTables<'_>>,
    out: &mut [u8],
) -> Result<(WaveletTransform, bool), J2kError> {
    let cs = parse_j2k_codestream(codestream, limits)?;
    if cs.siz.width() != expected_w as u32 || cs.siz.height() != expected_h as u32 {
        return Err(J2kError::DimensionMismatch);
    }
    if cs.siz.components.len() != expected_nc as usize {
        return Err(J2kError::CorruptStream("J2K component count mismatch"));
    }
    for comp in &cs.siz.components {
        if comp.precision as u16 != expected_bs {
            return Err(J2kError::BitPrecisionMismatch);
        }
        if comp.xrsiz != 1 || comp.yrsiz != 1 {
            return Err(J2kError::Unsupported("subsampled J2K components unsupported"));
        }
    }
    let bps = if expected_ba <= 8 { 1usize } else { 2usize };
    let need = (expected_w as usize)
        .checked_mul(expected_h as usize)
        .and_then(|v| v.checked_mul(expected_nc as usize))
        .and_then(|v| v.checked_mul(bps))
        .ok_or(J2kError::Overflow)?;
    if out.len() < need {
        return Err(J2kError::OutputTooSmall);
    }
    let is_htj2k = cs.cap.ccap_htj2k.is_some() || cs.cod.uses_htj2k_only();
    if lossless_only && cs.cod.transform != WaveletTransform::Reversible53 {
        return Err(J2kError::LosslessModeViolation);
    }

    let mut staged = vec![0u8; need];
    decode_all_tiles(&cs, bps, limits.max_compressed_bytes, ht_tables, &mut staged)?;
    out[..need].copy_from_slice(&staged);
    Ok((cs.cod.transform, is_htj2k))
}

fn decode_all_tiles(
    cs: &ParsedCodestream<'_>,
    bps: usize,
    max_compressed_bytes: usize,
    ht_tables: Option<&HtVlcTables<'_>>,
    staged: &mut [u8],
) -> Result<(), J2kError> {
    let nx = cs.siz.num_tiles_x() as usize;
    let ny = cs.siz.num_tiles_y() as usize;
    let nc = cs.siz.components.len();
    let img_w = cs.siz.width() as usize;

    for ty in 0..ny {
        for tx in 0..nx {
            let t_idx = (ty * nx + tx) as u16;
            let tx0 = (cs.siz.xtosiz + tx as u32 * cs.siz.xtsiz).max(cs.siz.xosiz);
            let ty0 = (cs.siz.ytosiz + ty as u32 * cs.siz.ytsiz).max(cs.siz.yosiz);
            let tx1 = (cs.siz.xtosiz + (tx as u32 + 1) * cs.siz.xtsiz).min(cs.siz.xsiz);
            let ty1 = (cs.siz.ytosiz + (ty as u32 + 1) * cs.siz.ytsiz).min(cs.siz.ysiz);

            let mut tile_cod = cs.cod.clone();
            let mut tile_qcd = cs.qcd.clone();
            let mut parts: Vec<&[u8]> = Vec::new();
            for tp in &cs.tile_parts {
                if tp.tile_idx == t_idx {
                    if let Some(ref c) = tp.cod { tile_cod = c.clone(); }
                    if let Some(ref q) = tp.qcd { tile_qcd = q.clone(); }
                    parts.push(tp.bitstream);
                }
            }
            if parts.is_empty() {
                return Err(J2kError::CorruptStream("missing J2K tile-part"));
            }
            let comp_cod = |c: usize| -> &super::j2k::markers::CodingStyle {
                cs.coc.iter().find(|&&(ci, _)| ci as usize == c).map(|(_, v)| v).unwrap_or(&tile_cod)
            };
            let comp_qcd = |c: usize| -> &super::j2k::markers::QuantizationParams {
                cs.qcc.iter().find(|&&(ci, _)| ci as usize == c).map(|(_, v)| v).unwrap_or(&tile_qcd)
            };

            let mut comp_grids: Vec<Vec<Vec<SubbandGrid>>> = Vec::with_capacity(nc);
            for c in 0..nc {
                let ccod = comp_cod(c);
                let mut res_vec = Vec::with_capacity(ccod.decomp_levels as usize + 1);
                for r in 0..=ccod.decomp_levels {
                    let sbs = if r == 0 {
                        vec![build_subband_grid(tx0, ty0, tx1, ty1, r, ccod.decomp_levels, 0, ccod.cblk_w_shift, ccod.cblk_h_shift)]
                    } else {
                        vec![
                            build_subband_grid(tx0, ty0, tx1, ty1, r, ccod.decomp_levels, 1, ccod.cblk_w_shift, ccod.cblk_h_shift),
                            build_subband_grid(tx0, ty0, tx1, ty1, r, ccod.decomp_levels, 2, ccod.cblk_w_shift, ccod.cblk_h_shift),
                            build_subband_grid(tx0, ty0, tx1, ty1, r, ccod.decomp_levels, 3, ccod.cblk_w_shift, ccod.cblk_h_shift),
                        ]
                    };
                    res_vec.push(sbs);
                }
                comp_grids.push(res_vec);
            }

            let joined = concat_fragments(&parts, max_compressed_bytes)?;
            let mut pkt_reader = PacketBitReader::new(joined.as_ref());
            let max_res = (0..nc).map(|c| comp_cod(c).decomp_levels).max().unwrap_or(0);
            for (_layer, res, comp) in packet_iteration_order(tile_cod.progression, tile_cod.layers, max_res, nc) {
                let ccod = comp_cod(comp);
                if res > ccod.decomp_levels {
                    continue;
                }
                let mut refs: Vec<&mut SubbandGrid> = comp_grids[comp][res as usize].iter_mut().collect();
                decode_packet(&mut pkt_reader, ccod, _layer, &mut refs)?;
            }

            let mut planes_f64: Vec<Vec<f64>> = Vec::with_capacity(nc);
            for c in 0..nc {
                let ccod = comp_cod(c);
                let cqcd = comp_qcd(c);
                let nom_bpp = cs.siz.components[c].precision;
                let mut sb_seq = 0usize;
                let mut ll_i32 = Vec::new();
                let mut ll_f64 = Vec::new();
                let (mut ll_w, mut ll_h) = (0usize, 0usize);

                for r in 0..=ccod.decomp_levels {
                    let sbs = &comp_grids[c][r as usize];
                    if r == 0 {
                        let sb = &sbs[0];
                        ll_w = (sb.x1 - sb.x0) as usize;
                        ll_h = (sb.y1 - sb.y0) as usize;
                        let (mb, delta) = subband_total_bitplanes(cqcd, sb_seq, ccod.decomp_levels, nom_bpp);
                        sb_seq += 1;
                        let raw = assemble_subband(sb, 0, mb, ccod, ht_tables)?;
                        if ccod.transform == WaveletTransform::Reversible53 {
                            ll_i32 = raw;
                        } else {
                            ll_f64 = raw.into_iter().map(|v| v as f64 * delta).collect();
                        }
                    } else {
                        let mut hl_i32 = Vec::new();
                        let mut lh_i32 = Vec::new();
                        let mut hh_i32 = Vec::new();
                        let mut hl_f64 = Vec::new();
                        let mut lh_f64 = Vec::new();
                        let mut hh_f64 = Vec::new();
                        let w_hl = (sbs[0].x1 - sbs[0].x0) as usize;
                        let h_lh = (sbs[1].y1 - sbs[1].y0) as usize;
                        for (idx, sb) in sbs.iter().enumerate() {
                            let (mb, delta) = subband_total_bitplanes(cqcd, sb_seq, ccod.decomp_levels, nom_bpp);
                            sb_seq += 1;
                            let raw = assemble_subband(sb, sb.subband_id, mb, ccod, ht_tables)?;
                            if ccod.transform == WaveletTransform::Reversible53 {
                                match idx { 0 => hl_i32 = raw, 1 => lh_i32 = raw, _ => hh_i32 = raw }
                            } else {
                                let scaled: Vec<f64> = raw.into_iter().map(|v| v as f64 * delta).collect();
                                match idx { 0 => hl_f64 = scaled, 1 => lh_f64 = scaled, _ => hh_f64 = scaled }
                            }
                        }
                        let shift = ccod.decomp_levels - r;
                        let rx0 = tx0.div_ceil(1u32 << shift);
                        let ry0 = ty0.div_ceil(1u32 << shift);
                        if ccod.transform == WaveletTransform::Reversible53 {
                            ll_i32 = synthesize_level_53(&ll_i32, &hl_i32, &lh_i32, &hh_i32, ll_w, ll_h, w_hl, h_lh, rx0, ry0);
                        } else {
                            ll_f64 = synthesize_level_97(&ll_f64, &hl_f64, &lh_f64, &hh_f64, ll_w, ll_h, w_hl, h_lh, rx0, ry0);
                        }
                        ll_w += w_hl;
                        ll_h += h_lh;
                    }
                }
                if ccod.transform == WaveletTransform::Reversible53 {
                    planes_f64.push(ll_i32.into_iter().map(|v| v as f64).collect());
                } else {
                    planes_f64.push(ll_f64);
                }
            }
            apply_mct_and_level_shift(&mut planes_f64, tile_cod.mct, tile_cod.transform, &cs.siz.components);
            let tw = (tx1 - tx0) as usize;
            let th = (ty1 - ty0) as usize;
            pack_tile_into_output(&planes_f64, (tx0 - cs.siz.xosiz) as usize, (ty0 - cs.siz.yosiz) as usize, tw, th, img_w, &cs.siz.components, bps, staged);
        }
    }
    Ok(())
}

fn assemble_subband(
    sb: &SubbandGrid,
    sb_id: usize,
    total_bps: u8,
    cod: &super::j2k::markers::CodingStyle,
    ht_tables: Option<&HtVlcTables<'_>>,
) -> Result<Vec<i32>, J2kError> {
    let w = (sb.x1 - sb.x0) as usize;
    let h = (sb.y1 - sb.y0) as usize;
    let mut out = vec![0i32; w * h];
    let use_ht = cod.uses_htj2k_only();
    for cb in &sb.cblks {
        let cb_vals = if use_ht {
            decode_ht_cblk_with_tables(cb, total_bps, ht_tables)?
        } else {
            decode_ebcot_cblk(cb, sb_id, total_bps, cod.cblk_style)?
        };
        let cw = (cb.x1 - cb.x0) as usize;
        let ch = (cb.y1 - cb.y0) as usize;
        let ox = (cb.x0 - sb.x0) as usize;
        let oy = (cb.y0 - sb.y0) as usize;
        for y in 0..ch {
            out[(oy + y) * w + ox..(oy + y) * w + ox + cw].copy_from_slice(&cb_vals[y * cw..(y + 1) * cw]);
        }
    }
    Ok(out)
}

pub fn decode_j2k_codestream_into(
    codestream: &[u8],
    expected_width: u16,
    expected_height: u16,
    expected_components: u16,
    expected_bits_allocated: u16,
    expected_bits_stored: u16,
    lossless_only: bool,
    output: &mut [u8],
) -> Result<WaveletTransform, J2kError> {
    let (tf, _) = decode_frame_with_tables(
        codestream,
        expected_width,
        expected_height,
        expected_components,
        expected_bits_allocated,
        expected_bits_stored,
        lossless_only,
        J2kLimits::default(),
        Some(annex_c_ht_vlc_tables()),
        output,
    )?;
    Ok(tf)
}

pub fn decode_j2k_part10_dataset_into<'a>(
    ds: &Part10Dataset<'a>,
    output: &mut [u8],
    limits: J2kLimits,
    w: &mut WorkBudget,
) -> Result<J2kFrameInfo<'a>, J2kError> {
    let ts_uid = ds.transfer_syntax_uid();
    if !matches!(ts_uid, TS_J2K_LOSSLESS | TS_J2K | TS_HTJ2K_LOSSLESS | TS_HTJ2K_RPCL | TS_HTJ2K) {
        return Err(J2kError::Unsupported("Part10Dataset path supports .90, .91, and .201-.203"));
    }
    for tag in [0x7fe00008, 0x7fe00009, 0x7fe00001, 0x7fe00002] {
        if ds.find(None, tag, w).map_err(super::encapsulated::EncapError::from)?.is_some() {
            return Err(J2kError::Unsupported("float PixelData or Extended Offset Table unsupported"));
        }
    }
    let req = |tag, w: &mut WorkBudget| -> Result<u16, J2kError> {
        ds.unsigned_short(tag, w).map_err(super::encapsulated::EncapError::from)?.ok_or(J2kError::Encap(super::encapsulated::EncapError::Missing(tag)))
    };
    let (rows, cols, spp, ba, bs, hb, rep) = (
        req(0x00280010, w)?, req(0x00280011, w)?, req(0x00280002, w)?,
        req(0x00280100, w)?, req(0x00280101, w)?, req(0x00280102, w)?, req(0x00280103, w)?,
    );
    if !matches!(ba, 8 | 16) || bs == 0 || bs > ba || hb != bs - 1 || rep > 1 {
        return Err(J2kError::BitPrecisionMismatch);
    }
    let photo = ds.text(None, 0x00280004, *b"CS", w).map_err(super::encapsulated::EncapError::from)?.ok_or(J2kError::Encap(super::encapsulated::EncapError::Missing(0x00280004)))?;
    let frames = ds.frame_count(w).map_err(super::encapsulated::EncapError::from)?;
    let (frags, ranges) = collect_part10_dataset_frames(ds, frames, limits.max_fragments, limits.max_compressed_bytes, w)?;
    let bps = if ba <= 8 { 1usize } else { 2usize };
    let frame_bytes = (rows as usize) * (cols as usize) * (spp as usize) * bps;
    let total_bytes = frame_bytes.checked_mul(frames as usize).ok_or(J2kError::Overflow)?;
    if total_bytes > limits.max_decoded_bytes { return Err(J2kError::ByteLimit); }
    if output.len() < total_bytes { return Err(J2kError::OutputTooSmall); }

    let lossless_only = matches!(ts_uid, TS_J2K_LOSSLESS | TS_HTJ2K_LOSSLESS);
    let mut staged = vec![0u8; total_bytes];
    let mut tf_out = WaveletTransform::Reversible53;
    let mut is_ht = false;
    for (f, &(s, e)) in ranges.iter().enumerate() {
        let joined = concat_fragments(&frags[s..e], limits.max_compressed_bytes)?;
        let (tf, ht) = decode_frame_with_tables(joined.as_ref(), cols, rows, spp, ba, bs, lossless_only, limits, Some(annex_c_ht_vlc_tables()), &mut staged[f * frame_bytes..(f + 1) * frame_bytes])?;
        tf_out = tf;
        is_ht |= ht;
    }
    output[..total_bytes].copy_from_slice(&staged);
    Ok(J2kFrameInfo {
        source_blake3: ds.source_blake3(), transfer_syntax_uid: ts_uid, rows, columns: cols, frames,
        samples_per_pixel: spp, bits_allocated: ba, bits_stored: bs, signed: rep == 1,
        source_photometric: photo, transform: tf_out, is_htj2k: is_ht, bytes: total_bytes,
    })
}

/// Decode encapsulated DICOM Part-10 J2K (`.90`/`.91`) and HTJ2K (`.201`-`.203`) using the
/// normative ITU-T T.814 / ISO 15444-15 Annex C `CxtVLC` tables.
pub fn decode_j2k_part10_with_annex_c_into<'a>(
    part10_bytes: &'a [u8],
    output: &mut [u8],
    limits: J2kLimits,
) -> Result<J2kFrameInfo<'a>, J2kError> {
    decode_j2k_part10_into(part10_bytes, output, limits, Some(annex_c_ht_vlc_tables()))
}

pub fn decode_j2k_part10_into<'a>(
    part10_bytes: &'a [u8],
    output: &mut [u8],
    limits: J2kLimits,
    ht_tables: Option<&HtVlcTables<'_>>,
) -> Result<J2kFrameInfo<'a>, J2kError> {
    let encap = parse_encapsulated_part10(
        part10_bytes,
        &[TS_J2K_LOSSLESS, TS_J2K, TS_HTJ2K_LOSSLESS, TS_HTJ2K_RPCL, TS_HTJ2K],
        EncapLimits {
            max_part10_bytes: limits.max_part10_bytes, max_decoded_bytes: limits.max_decoded_bytes,
            max_compressed_bytes: limits.max_compressed_bytes, max_frames: limits.max_frames,
            max_fragments: limits.max_fragments, max_width: limits.max_width, max_height: limits.max_height, max_elements: 8192,
        },
    )?;
    let bps = if encap.bits_allocated <= 8 { 1usize } else { 2usize };
    let frame_bytes = (encap.rows as usize) * (encap.columns as usize) * (encap.samples_per_pixel as usize) * bps;
    let total_bytes = frame_bytes.checked_mul(encap.frames as usize).ok_or(J2kError::Overflow)?;
    if total_bytes > limits.max_decoded_bytes { return Err(J2kError::ByteLimit); }
    if output.len() < total_bytes { return Err(J2kError::OutputTooSmall); }
    let lossless_only = matches!(encap.transfer_syntax_uid, TS_J2K_LOSSLESS | TS_HTJ2K_LOSSLESS);
    let mut staged = vec![0u8; total_bytes];
    let mut tf_out = WaveletTransform::Reversible53;
    let mut is_ht = false;
    for f in 0..encap.frames as usize {
        let fb = encap.frame_bytes(f, limits.max_compressed_bytes)?;
        let (tf, ht) = decode_frame_with_tables(fb.as_ref(), encap.columns, encap.rows, encap.samples_per_pixel, encap.bits_allocated, encap.bits_stored, lossless_only, limits, ht_tables, &mut staged[f * frame_bytes..(f + 1) * frame_bytes])?;
        tf_out = tf;
        is_ht |= ht;
    }
    output[..total_bytes].copy_from_slice(&staged);
    Ok(J2kFrameInfo {
        source_blake3: encap.source_blake3, transfer_syntax_uid: encap.transfer_syntax_uid, rows: encap.rows,
        columns: encap.columns, frames: encap.frames, samples_per_pixel: encap.samples_per_pixel,
        bits_allocated: encap.bits_allocated, bits_stored: encap.bits_stored, signed: encap.pixel_representation == 1,
        source_photometric: encap.photometric, transform: tf_out, is_htj2k: is_ht, bytes: total_bytes,
    })
}
