//! ISO/IEC 15444-1 (JPEG 2000) & ISO/IEC 15444-15 (HTJ2K) codestream/JP2 marker parser.
pub use super::types::{
    CapHeader, CodingStyle, ComponentSiz, J2kError, J2kLimits, ParsedCodestream, ProgressionOrder,
    QuantizationParams, SizHeader, TilePart, WaveletTransform, TS_HTJ2K, TS_HTJ2K_LOSSLESS,
    TS_HTJ2K_RPCL, TS_J2K, TS_J2K_LOSSLESS,
};

#[inline]
fn be_u16(b: &[u8], at: usize) -> Result<u16, J2kError> {
    let s = b.get(at..at + 2).ok_or(J2kError::CorruptStream("truncated J2K u16"))?;
    Ok(u16::from_be_bytes([s[0], s[1]]))
}
#[inline]
fn be_u32(b: &[u8], at: usize) -> Result<u32, J2kError> {
    let s = b.get(at..at + 4).ok_or(J2kError::CorruptStream("truncated J2K u32"))?;
    Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// Extract bare J2K codestream from a JP2 file box wrapper if present.
pub fn unwrap_jp2_if_present(input: &[u8]) -> Result<&[u8], J2kError> {
    const JP2_SIG: [u8; 12] = [0, 0, 0, 0x0C, b'j', b'P', b' ', b' ', 0x0D, 0x0A, 0x87, 0x0A];
    if input.len() >= 12 && input[..12] == JP2_SIG {
        let mut cur = 0usize;
        while cur + 8 <= input.len() {
            let lbox = be_u32(input, cur)? as usize;
            let tbox = &input[cur + 4..cur + 8];
            let (hdr_len, box_end) = if lbox == 0 {
                (8usize, input.len())
            } else if lbox == 1 {
                if cur + 16 > input.len() {
                    return Err(J2kError::CorruptStream("truncated JP2 XLBox"));
                }
                let hi = be_u32(input, cur + 8)? as usize;
                let lo = be_u32(input, cur + 12)? as usize;
                let xl = hi.checked_shl(32).unwrap_or(usize::MAX) | lo;
                let end = cur.checked_add(xl).ok_or(J2kError::Overflow)?;
                (16usize, end)
            } else if lbox >= 8 {
                let end = cur.checked_add(lbox).ok_or(J2kError::Overflow)?;
                (8usize, end)
            } else {
                return Err(J2kError::CorruptStream("invalid JP2 box length"));
            };
            if box_end > input.len() || cur + hdr_len > box_end {
                return Err(J2kError::CorruptStream("truncated JP2 box"));
            }
            if tbox == b"jp2c" {
                return Ok(&input[cur + hdr_len..box_end]);
            }
            cur = box_end;
        }
        return Err(J2kError::CorruptStream("JP2 file missing jp2c codestream box"));
    }
    Ok(input)
}

fn parse_siz(p: &[u8], limits: J2kLimits) -> Result<SizHeader, J2kError> {
    if p.len() < 39 {
        return Err(J2kError::CorruptStream("invalid SIZ length"));
    }
    let xsiz = be_u32(p, 2)?;
    let ysiz = be_u32(p, 6)?;
    let xosiz = be_u32(p, 10)?;
    let yosiz = be_u32(p, 14)?;
    let xtsiz = be_u32(p, 18)?;
    let ytsiz = be_u32(p, 22)?;
    let xtosiz = be_u32(p, 26)?;
    let ytosiz = be_u32(p, 30)?;
    let csiz = be_u16(p, 34)? as usize;
    if csiz == 0 || csiz > 16384 || p.len() != 36 + 3 * csiz {
        return Err(J2kError::CorruptStream("invalid SIZ component count"));
    }
    if xtsiz == 0 || ytsiz == 0 || xsiz <= xosiz || ysiz <= yosiz || xtosiz > xosiz || ytosiz > yosiz || xosiz >= xtosiz.saturating_add(xtsiz) || yosiz >= ytosiz.saturating_add(ytsiz) {
        return Err(J2kError::CorruptStream("invalid SIZ tile/image coordinates"));
    }
    let w = xsiz - xosiz;
    let h = ysiz - yosiz;
    if w > limits.max_width as u32 || h > limits.max_height as u32 {
        return Err(J2kError::DimensionLimit);
    }
    let nx = (xsiz - xtosiz).div_ceil(xtsiz) as usize;
    let ny = (ysiz - ytosiz).div_ceil(ytsiz) as usize;
    if nx.checked_mul(ny).is_none_or(|t| t > limits.max_tiles) {
        return Err(J2kError::TileLimit);
    }
    let mut comps = Vec::with_capacity(csiz);
    for i in 0..csiz {
        let ssiz = p[36 + 3 * i];
        let xrsiz = p[36 + 3 * i + 1];
        let yrsiz = p[36 + 3 * i + 2];
        if xrsiz == 0 || yrsiz == 0 {
            return Err(J2kError::CorruptStream("zero SIZ subsampling"));
        }
        comps.push(ComponentSiz {
            precision: (ssiz & 0x7F) + 1,
            signed: (ssiz & 0x80) != 0,
            xrsiz,
            yrsiz,
        });
    }
    Ok(SizHeader { xsiz, ysiz, xosiz, yosiz, xtsiz, ytsiz, xtosiz, ytosiz, components: comps })
}

fn parse_coding_style_body(p: &[u8], decomp_at: usize, has_precincts: bool, limits: J2kLimits, sop: bool, eph: bool, prog: ProgressionOrder, layers: u16, mct: u8) -> Result<CodingStyle, J2kError> {
    if p.len() < decomp_at + 5 {
        return Err(J2kError::CorruptStream("truncated COD/COC parameters"));
    }
    let decomp = p[decomp_at];
    if decomp > limits.max_decomp_levels {
        return Err(J2kError::Unsupported("decomposition levels exceed limit"));
    }
    let cw = p[decomp_at + 1] & 0x0F;
    let ch = p[decomp_at + 2] & 0x0F;
    if cw + ch > 8 || cw > 8 || ch > 8 {
        return Err(J2kError::CorruptStream("code-block exponent exceeds 1024 area"));
    }
    let cblk_style = p[decomp_at + 3];
    let transform = match p[decomp_at + 4] {
        0 => WaveletTransform::Irreversible97,
        1 => WaveletTransform::Reversible53,
        _ => return Err(J2kError::CorruptStream("unknown J2K wavelet transform")),
    };
    let mut precinct_sizes = Vec::new();
    let tail = &p[decomp_at + 5..];
    if has_precincts {
        if tail.len() != decomp as usize + 1 {
            return Err(J2kError::CorruptStream("invalid precinct parameter length"));
        }
        for &b in tail {
            precinct_sizes.push((b & 0x0F, b >> 4));
        }
    } else if !tail.is_empty() {
        return Err(J2kError::CorruptStream("unexpected trailing COD/COC bytes"));
    }
    Ok(CodingStyle {
        sop_markers: sop,
        eph_markers: eph,
        progression: prog,
        layers,
        mct,
        decomp_levels: decomp,
        cblk_w_shift: cw,
        cblk_h_shift: ch,
        cblk_style,
        transform,
        precinct_sizes,
    })
}

fn parse_cod(p: &[u8], limits: J2kLimits) -> Result<CodingStyle, J2kError> {
    if p.len() < 10 {
        return Err(J2kError::CorruptStream("invalid COD length"));
    }
    let scod = p[0];
    let prog = match p[1] {
        0 => ProgressionOrder::Lrcp,
        1 => ProgressionOrder::Rlcp,
        2 => ProgressionOrder::Rpcl,
        3 => ProgressionOrder::Pcrl,
        4 => ProgressionOrder::Cprl,
        _ => return Err(J2kError::CorruptStream("unknown progression order")),
    };
    let layers = u16::from_be_bytes([p[2], p[3]]);
    if layers == 0 {
        return Err(J2kError::CorruptStream("zero layers in COD"));
    }
    let mct = p[4];
    parse_coding_style_body(p, 5, (scod & 1) != 0, limits, (scod & 2) != 0, (scod & 4) != 0, prog, layers, mct)
}

fn parse_qcd(p: &[u8]) -> Result<QuantizationParams, J2kError> {
    if p.is_empty() {
        return Err(J2kError::CorruptStream("empty QCD"));
    }
    let style = p[0] & 0x1F;
    let guard_bits = p[0] >> 5;
    let body = &p[1..];
    let mut steps = Vec::new();
    match style {
        0 => {
            for &b in body {
                steps.push((b >> 3, 0));
            }
        }
        1 | 2 => {
            if body.len() % 2 != 0 {
                return Err(J2kError::CorruptStream("odd QCD step table"));
            }
            for ch in body.chunks_exact(2) {
                let v = u16::from_be_bytes([ch[0], ch[1]]);
                steps.push(((v >> 11) as u8, v & 0x07FF));
            }
        }
        _ => return Err(J2kError::CorruptStream("unknown QCD quantization style")),
    }
    Ok(QuantizationParams { style, guard_bits, steps })
}

pub fn parse_j2k_codestream<'a>(raw: &'a [u8], limits: J2kLimits) -> Result<ParsedCodestream<'a>, J2kError> {
    let data = unwrap_jp2_if_present(raw)?;
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0x4F {
        return Err(J2kError::CorruptStream("missing J2K SOC marker"));
    }
    if data.get(2..4) != Some(&[0xFF, 0x51]) {
        return Err(J2kError::CorruptStream("SIZ must immediately follow SOC"));
    }
    let mut pos = 2usize;
    let mut siz: Option<SizHeader> = None;
    let mut cod: Option<CodingStyle> = None;
    let mut coc = Vec::new();
    let mut qcd: Option<QuantizationParams> = None;
    let mut qcc = Vec::new();
    let mut cap = CapHeader::default();
    let mut tile_parts: Vec<TilePart<'a>> = Vec::new();
    let mut in_tile = false;
    let mut cur_tile = (0u16, 0u8, 0u8, None::<CodingStyle>, None::<QuantizationParams>);
    let mut cur_tile_end: Option<usize> = None;
    let mut saw_eoc = false;

    while pos + 2 <= data.len() {
        let m0 = data[pos];
        let m1 = data[pos + 1];
        if m0 != 0xFF {
            return Err(J2kError::CorruptStream("expected J2K marker"));
        }
        pos += 2;
        if m1 == 0xD9 {
            saw_eoc = true;
            break;
        }
        if m1 == 0x93 {
            if !in_tile {
                return Err(J2kError::CorruptStream("SOD outside SOT"));
            }
            let bs_end = cur_tile_end.unwrap_or(data.len().saturating_sub(2)).min(data.len());
            if pos > bs_end {
                return Err(J2kError::CorruptStream("SOD past Psot tile bound"));
            }
            tile_parts.push(TilePart {
                tile_idx: cur_tile.0,
                part_idx: cur_tile.1,
                num_parts: cur_tile.2,
                cod: cur_tile.3.take(),
                qcd: cur_tile.4.take(),
                bitstream: &data[pos..bs_end],
            });
            pos = bs_end;
            in_tile = false;
            cur_tile_end = None;
            continue;
        }
        let seg_len = be_u16(data, pos)? as usize;
        if seg_len < 2 {
            return Err(J2kError::CorruptStream("invalid J2K marker segment length"));
        }
        let seg_end = pos.checked_add(seg_len).ok_or(J2kError::Overflow)?;
        let p = data.get(pos + 2..seg_end).ok_or(J2kError::CorruptStream("truncated J2K marker segment"))?;
        let seg_start = pos - 2;
        pos = seg_end;

        match m1 {
            0x51 => siz = Some(parse_siz(p, limits)?),
            0x52 => {
                let parsed = parse_cod(p, limits)?;
                if in_tile { cur_tile.3 = Some(parsed); } else { cod = Some(parsed); }
            }
            0x53 => {
                let base_cod = cod.as_ref().ok_or(J2kError::CorruptStream("COC before COD"))?;
                let comp_count = siz.as_ref().map(|s| s.components.len()).unwrap_or(0);
                let (cidx, off) = if comp_count < 257 {
                    (*p.first().ok_or(J2kError::CorruptStream("empty COC"))? as u16, 1usize)
                } else {
                    (be_u16(p, 0)?, 2usize)
                };
                if p.len() <= off {
                    return Err(J2kError::CorruptStream("truncated COC"));
                }
                let scoc = p[off];
                let parsed = parse_coding_style_body(p, off + 1, (scoc & 1) != 0, limits, base_cod.sop_markers, base_cod.eph_markers, base_cod.progression, base_cod.layers, base_cod.mct)?;
                coc.push((cidx, parsed));
            }
            0x5C => {
                let parsed = parse_qcd(p)?;
                if in_tile { cur_tile.4 = Some(parsed); } else { qcd = Some(parsed); }
            }
            0x5D => {
                let comp_count = siz.as_ref().map(|s| s.components.len()).unwrap_or(0);
                let (cidx, off) = if comp_count < 257 {
                    (*p.first().ok_or(J2kError::CorruptStream("empty QCC"))? as u16, 1usize)
                } else {
                    (be_u16(p, 0)?, 2usize)
                };
                qcc.push((cidx, parse_qcd(&p[off..])?));
            }
            0x50 => {
                if p.len() < 4 {
                    return Err(J2kError::CorruptStream("invalid CAP marker"));
                }
                cap.pcap = be_u32(p, 0)?;
                if (cap.pcap & (1u32 << (31 - 14))) != 0 {
                    // Part 15 bit (bit 14 -> 17th bit MSB-1 / bit 17 of u32): find corresponding Ccap.
                    let active = (cap.pcap & 0xFFFC_0000).count_ones() as usize;
                    let idx_15 = (cap.pcap & 0xFFFE_0000).count_ones() as usize;
                    let _ = active;
                    let ccap_off = 4 + idx_15.saturating_sub(1) * 2;
                    if let Some(c) = p.get(ccap_off..ccap_off + 2) {
                        cap.ccap_htj2k = Some(u16::from_be_bytes([c[0], c[1]]));
                    }
                }
            }
            0x90 => {
                if p.len() != 8 {
                    return Err(J2kError::CorruptStream("invalid SOT marker length"));
                }
                if in_tile {
                    return Err(J2kError::CorruptStream("unexpected SOT before SOD"));
                }
                let siz_ref = siz.as_ref().ok_or(J2kError::CorruptStream("SOT before SIZ"))?;
                let num_tiles = (siz_ref.num_tiles_x() as usize) * (siz_ref.num_tiles_y() as usize);
                let isot = u16::from_be_bytes([p[0], p[1]]);
                if (isot as usize) >= num_tiles {
                    return Err(J2kError::CorruptStream("SOT tile index out of range"));
                }
                let psot = u32::from_be_bytes([p[2], p[3], p[4], p[5]]) as usize;
                let tpsot = p[6];
                let tnsot = p[7];
                if tnsot > 0 && tpsot >= tnsot {
                    return Err(J2kError::CorruptStream("TPsot exceeds TNsot"));
                }
                if tile_parts.len() >= limits.max_fragments {
                    return Err(J2kError::TileLimit);
                }
                in_tile = true;
                cur_tile = (isot, tpsot, tnsot, None, None);
                cur_tile_end = if psot == 0 {
                    None
                } else {
                    if psot < 14 {
                        return Err(J2kError::CorruptStream("Psot smaller than SOT+SOD"));
                    }
                    let end = seg_start.checked_add(psot).ok_or(J2kError::Overflow)?;
                    if end > data.len() {
                        return Err(J2kError::CorruptStream("Psot past codestream EOF"));
                    }
                    Some(end)
                };
            }
            0x59 | 0x64 | 0x55 | 0x56 | 0x57 | 0x58 | 0x60 => {}
            _ => {}
        }
    }
    let siz = siz.ok_or(J2kError::CorruptStream("missing SIZ"))?;
    let cod = cod.ok_or(J2kError::CorruptStream("missing COD"))?;
    let qcd = qcd.ok_or(J2kError::CorruptStream("missing QCD"))?;
    if !saw_eoc || in_tile || tile_parts.is_empty() {
        return Err(J2kError::CorruptStream("missing EOC or incomplete tile-part in J2K codestream"));
    }
    validate_and_order_tile_parts(&mut tile_parts)?;
    Ok(ParsedCodestream { siz, cod, coc, qcd, qcc, cap, tile_parts })
}

fn validate_and_order_tile_parts(tile_parts: &mut Vec<TilePart<'_>>) -> Result<(), J2kError> {
    tile_parts.sort_by_key(|tp| (tp.tile_idx, tp.part_idx));
    let mut i = 0usize;
    while i < tile_parts.len() {
        let tid = tile_parts[i].tile_idx;
        let mut expected_total: Option<u8> = None;
        let mut seq = 0usize;
        while i < tile_parts.len() && tile_parts[i].tile_idx == tid {
            let tp = &tile_parts[i];
            if tp.part_idx as usize != seq {
                return Err(J2kError::CorruptStream("duplicate or non-contiguous TPsot tile-part index"));
            }
            if tp.num_parts > 0 {
                if let Some(prev) = expected_total {
                    if prev != tp.num_parts {
                        return Err(J2kError::CorruptStream("conflicting TNsot values across tile-parts"));
                    }
                } else {
                    expected_total = Some(tp.num_parts);
                }
            }
            seq += 1;
            i += 1;
        }
        if let Some(total) = expected_total {
            if seq != total as usize {
                return Err(J2kError::CorruptStream("tile-part count does not match TNsot"));
            }
        }
    }
    Ok(())
}
