//! ISO/IEC 18181-2 / DICOM `.111` (`1.2.840.10008.1.2.4.111`) byte-exact legacy JPEG
//! bitstream reconstruction from ISOBMFF `jbrd` boxes and quantized 8x8 DCT coefficients.
use super::container::{extract_jbrd_payload, JxlError, JxlLimits};

const JBRD_MAGIC: &[u8; 5] = b"JBRD\x01";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JbrdExtractedCoeffs {
    pub width: u16,
    pub height: u16,
    pub precision: u8,
    pub num_components: u8,
    pub restart_interval: u16,
    pub source_blake3: [u8; 32],
    pub blocks: Vec<[i16; 64]>,
}

#[derive(Clone)]
struct HuffTable {
    codes: [(u16, u8); 256],
    dec_entries: Vec<(u8, u16, u8)>,
}
impl Default for HuffTable { fn default() -> Self { Self { codes: [(0, 0); 256], dec_entries: Vec::new() } } }

impl HuffTable {
    fn from_dht_segment(counts: &[u8; 16], syms: &[u8]) -> Result<Self, JxlError> {
        let (mut tbl, mut code, mut pos) = (Self::default(), 0u32, 0usize);
        for (i, &cnt) in counts.iter().enumerate() {
            let len = (i + 1) as u8;
            for _ in 0..cnt as usize {
                let sym = *syms.get(pos).ok_or(JxlError::CorruptStream("truncated DHT symbol list"))?;
                pos += 1;
                if code >= (1u32 << len) {
                    return Err(JxlError::CorruptStream("oversubscribed JPEG DHT table"));
                }
                tbl.codes[sym as usize] = (code as u16, len);
                tbl.dec_entries.push((len, code as u16, sym));
                code += 1;
            }
            code <<= 1;
        }
        Ok(tbl)
    }
}

struct JpegParsedFraming {
    precision: u8,
    width: u16,
    height: u16,
    comps: Vec<(u8, u8, u8)>,
    scan_comps: Vec<(usize, usize, usize)>,
    dc_tables: [Option<HuffTable>; 4],
    ac_tables: [Option<HuffTable>; 4],
    restart_interval: u16,
    total_mcus: usize,
    blocks_per_mcu: usize,
    prefix_end: usize,
    scan_end: usize,
}

fn parse_jpeg_framing(jpeg: &[u8]) -> Result<JpegParsedFraming, JxlError> {
    if jpeg.len() < 4 || jpeg[0..2] != [0xFF, 0xD8] {
        return Err(JxlError::CorruptStream("missing JPEG SOI marker"));
    }
    let (mut pos, mut precision, mut width, mut height, mut restart_interval) = (2usize, 0u8, 0u16, 0u16, 0u16);
    let (mut comps, mut scan_comps) = (Vec::new(), Vec::new());
    let mut dc_tables: [Option<HuffTable>; 4] = [None, None, None, None];
    let mut ac_tables: [Option<HuffTable>; 4] = [None, None, None, None];

    while pos + 2 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            return Err(JxlError::CorruptStream("expected JPEG marker 0xFF"));
        }
        while pos < jpeg.len() && jpeg[pos] == 0xFF {
            pos += 1;
        }
        let marker = *jpeg.get(pos).ok_or(JxlError::CorruptStream("truncated JPEG marker"))?;
        pos += 1;
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }
        if marker == 0xD9 || pos + 2 > jpeg.len() {
            return Err(JxlError::CorruptStream("JPEG EOI before SOS or truncated marker length"));
        }
        let seg_len = u16::from_be_bytes([jpeg[pos], jpeg[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > jpeg.len() {
            return Err(JxlError::CorruptStream("invalid JPEG marker segment length"));
        }
        let seg = &jpeg[pos + 2..pos + seg_len];
        pos += seg_len;

        match marker {
            0xC0 | 0xC1 => {
                if seg.len() < 6 {
                    return Err(JxlError::CorruptStream("truncated SOF0/SOF1 header"));
                }
                precision = seg[0];
                height = u16::from_be_bytes([seg[1], seg[2]]);
                width = u16::from_be_bytes([seg[3], seg[4]]);
                let nc = seg[5] as usize;
                if !matches!(precision, 8 | 12) || width == 0 || height == 0 || !matches!(nc, 1..=4) || seg.len() != 6 + 3 * nc {
                    return Err(JxlError::CorruptStream("unsupported SOF geometry or precision"));
                }
                comps.clear();
                for i in 0..nc {
                    let (id, hv) = (seg[6 + 3 * i], seg[7 + 3 * i]);
                    let (h_s, v_s) = (hv >> 4, hv & 0x0F);
                    if !matches!(h_s, 1..=4) || !matches!(v_s, 1..=4) {
                        return Err(JxlError::CorruptStream("invalid JPEG sampling factor"));
                    }
                    comps.push((id, h_s, v_s));
                }
            }
            0xC4 => {
                let mut dpos = 0usize;
                while dpos < seg.len() {
                    if dpos + 17 > seg.len() {
                        return Err(JxlError::CorruptStream("truncated DHT header"));
                    }
                    let (tc, th) = (seg[dpos] >> 4, (seg[dpos] & 0x0F) as usize);
                    if tc > 1 || th >= 4 {
                        return Err(JxlError::CorruptStream("invalid DHT table class/id"));
                    }
                    let counts: [u8; 16] = seg[dpos + 1..dpos + 17].try_into().unwrap();
                    let total_syms: usize = counts.iter().map(|&c| c as usize).sum();
                    dpos += 17;
                    if dpos + total_syms > seg.len() {
                        return Err(JxlError::CorruptStream("truncated DHT symbols"));
                    }
                    let tbl = HuffTable::from_dht_segment(&counts, &seg[dpos..dpos + total_syms])?;
                    dpos += total_syms;
                    if tc == 0 { dc_tables[th] = Some(tbl); } else { ac_tables[th] = Some(tbl); }
                }
            }
            0xDD => {
                if seg.len() != 2 {
                    return Err(JxlError::CorruptStream("invalid DRI length"));
                }
                restart_interval = u16::from_be_bytes([seg[0], seg[1]]);
            }
            0xDA => {
                let ns = *seg.first().ok_or(JxlError::CorruptStream("truncated SOS"))? as usize;
                if comps.is_empty() || ns != comps.len() || seg.len() != 1 + 2 * ns + 3 {
                    return Err(JxlError::CorruptStream("invalid SOS component count"));
                }
                for i in 0..ns {
                    let (cs, td_ta) = (seg[1 + 2 * i], seg[2 + 2 * i]);
                    let (td, ta) = ((td_ta >> 4) as usize, (td_ta & 0x0F) as usize);
                    let comp_idx = comps.iter().position(|c| c.0 == cs).ok_or(JxlError::CorruptStream("unknown SOS component selector"))?;
                    if td >= 4 || ta >= 4 || dc_tables[td].is_none() || ac_tables[ta].is_none() {
                        return Err(JxlError::CorruptStream("missing referenced DHT table"));
                    }
                    scan_comps.push((comp_idx, td, ta));
                }
                let prefix_end = pos;
                let (mut spos, mut scan_end) = (prefix_end, None);
                while spos + 1 < jpeg.len() {
                    if jpeg[spos] == 0xFF {
                        if jpeg[spos + 1] == 0xD9 {
                            scan_end = Some(spos);
                            break;
                        }
                        spos += 2;
                    } else {
                        spos += 1;
                    }
                }
                let scan_end = scan_end.ok_or(JxlError::CorruptStream("missing JPEG EOI marker"))?;
                let max_h = comps.iter().map(|c| c.1 as usize).max().unwrap_or(1);
                let max_v = comps.iter().map(|c| c.2 as usize).max().unwrap_or(1);
                let total_mcus = (width as usize).div_ceil(max_h * 8) * (height as usize).div_ceil(max_v * 8);
                let blocks_per_mcu: usize = comps.iter().map(|c| (c.1 as usize) * (c.2 as usize)).sum();
                return Ok(JpegParsedFraming {
                    precision, width, height, comps, scan_comps, dc_tables, ac_tables,
                    restart_interval, total_mcus, blocks_per_mcu, prefix_end, scan_end,
                });
            }
            _ => {}
        }
    }
    Err(JxlError::CorruptStream("JPEG missing SOS marker"))
}

struct ScanIntervalReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
}

impl<'a> ScanIntervalReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, bits: 0 }
    }
    fn read_bit(&mut self) -> Result<u16, JxlError> {
        if self.bits == 0 {
            let b = *self.data.get(self.pos).ok_or(JxlError::CorruptStream("truncated JPEG scan interval"))?;
            self.pos += 1;
            if b == 0xFF {
                if *self.data.get(self.pos).ok_or(JxlError::CorruptStream("truncated 0xFF00 byte stuffing"))? != 0x00 {
                    return Err(JxlError::CorruptStream("unexpected marker inside JPEG scan interval"));
                }
                self.pos += 1;
            }
            self.acc = b as u32;
            self.bits = 8;
        }
        self.bits -= 1;
        Ok(((self.acc >> self.bits) & 1) as u16)
    }
    fn read_bits(&mut self, n: u8) -> Result<u16, JxlError> {
        let mut v = 0u16;
        for _ in 0..n {
            v = (v << 1) | self.read_bit()?;
        }
        Ok(v)
    }
    fn decode_huff(&mut self, tbl: &HuffTable) -> Result<u8, JxlError> {
        let mut code = 0u16;
        for len in 1..=16u8 {
            code = (code << 1) | self.read_bit()?;
            for &(l, c, sym) in &tbl.dec_entries {
                if l == len && c == code {
                    return Ok(sym);
                }
            }
        }
        Err(JxlError::CorruptStream("invalid Huffman code in JPEG scan"))
    }
    fn finish_padding(self) -> Result<(u8, u8), JxlError> {
        if self.pos != self.data.len() {
            return Err(JxlError::CorruptStream("unconsumed trailing bytes in JPEG scan interval"));
        }
        let pad_bits = if self.bits > 0 { (self.acc & ((1u32 << self.bits) - 1)) as u8 } else { 0 };
        Ok((self.bits, pad_bits))
    }
}

fn decode_magnitude(ssss: u8, bits: u16) -> i16 {
    if ssss == 0 { 0 } else if (bits & (1u16 << (ssss - 1))) != 0 { bits as i16 } else { (bits as i16) - ((1i16 << ssss) - 1) }
}

fn encode_magnitude(v: i16) -> Result<(u8, u16), JxlError> {
    if v == 0 {
        return Ok((0, 0));
    }
    let abs_v = v.unsigned_abs();
    let ssss = (16 - abs_v.leading_zeros()) as u8;
    if ssss > 15 {
        return Err(JxlError::CorruptStream("DCT coefficient magnitude exceeds category 15"));
    }
    let bits = if v > 0 { v as u16 } else { ((1u16 << ssss) - 1).wrapping_sub(abs_v) };
    Ok((ssss, bits))
}

struct ScanBitWriter {
    buf: Vec<u8>,
    acc: u8,
    bits: u8,
}

impl ScanBitWriter {
    fn new() -> Self {
        Self { buf: Vec::new(), acc: 0, bits: 0 }
    }
    fn put_bits(&mut self, val: u16, n: u8) {
        for s in (0..n).rev() {
            self.acc = (self.acc << 1) | (((val >> s) & 1) as u8);
            self.bits += 1;
            if self.bits == 8 {
                self.buf.push(self.acc);
                if self.acc == 0xFF {
                    self.buf.push(0x00);
                }
                self.acc = 0;
                self.bits = 0;
            }
        }
    }
    fn put_huff(&mut self, tbl: &HuffTable, sym: u8) -> Result<(), JxlError> {
        let (code, len) = tbl.codes[sym as usize];
        if len == 0 {
            return Err(JxlError::CorruptStream("symbol missing from JPEG DHT table"));
        }
        self.put_bits(code, len);
        Ok(())
    }
}

/// Extract an ISOBMFF `jbrd` box payload and quantized 8x8 zigzag DCT coefficient blocks
/// from a legacy 8-bit (`SOF0`/`SOF1`) or 12-bit (`SOF1`) JPEG bitstream.
pub fn encode_jbrd_box_from_jpeg(jpeg_bytes: &[u8]) -> Result<(Vec<u8>, JbrdExtractedCoeffs), JxlError> {
    let framing = parse_jpeg_framing(jpeg_bytes)?;
    let scan_slice = &jpeg_bytes[framing.prefix_end..framing.scan_end];
    let (mut intervals, mut start, mut i, mut expected_rst) = (Vec::new(), 0usize, 0usize, 0u8);
    while i < scan_slice.len() {
        if scan_slice[i] == 0xFF {
            let next = *scan_slice.get(i + 1).ok_or(JxlError::CorruptStream("truncated 0xFF in JPEG scan"))?;
            if next == 0x00 {
                i += 2;
                continue;
            }
            if (0xD0..=0xD7).contains(&next) && (next & 7) == expected_rst {
                expected_rst = (expected_rst + 1) & 7;
                intervals.push(&scan_slice[start..i]);
                i += 2;
                start = i;
                continue;
            }
            return Err(JxlError::CorruptStream("unexpected or out-of-order marker inside JPEG scan"));
        }
        i += 1;
    }
    intervals.push(&scan_slice[start..]);

    let mcus_per_interval = if framing.restart_interval == 0 { framing.total_mcus } else { framing.restart_interval as usize };
    if intervals.len() != framing.total_mcus.div_ceil(mcus_per_interval.max(1)) {
        return Err(JxlError::CorruptStream("JPEG restart interval count mismatch"));
    }

    let mut blocks = Vec::with_capacity(framing.total_mcus * framing.blocks_per_mcu);
    let mut pad_records = Vec::with_capacity(intervals.len());
    let mut mcu_done = 0usize;
    for int_bytes in &intervals {
        let mut rdr = ScanIntervalReader::new(int_bytes);
        let mut prev_dc = [0i16; 4];
        let count_mcus = (framing.total_mcus - mcu_done).min(mcus_per_interval);
        for _ in 0..count_mcus {
            for &(comp_idx, td, ta) in &framing.scan_comps {
                let (_, h_s, v_s) = framing.comps[comp_idx];
                let dc_tbl = framing.dc_tables[td].as_ref().unwrap();
                let ac_tbl = framing.ac_tables[ta].as_ref().unwrap();
                for _ in 0..(h_s as usize * v_s as usize) {
                    let mut blk = [0i16; 64];
                    let ssss = rdr.decode_huff(dc_tbl)?;
                    if ssss > 15 {
                        return Err(JxlError::CorruptStream("DC category > 15"));
                    }
                    let diff = decode_magnitude(ssss, rdr.read_bits(ssss)?);
                    prev_dc[comp_idx] = prev_dc[comp_idx].wrapping_add(diff);
                    blk[0] = prev_dc[comp_idx];
                    let mut k = 1usize;
                    while k < 64 {
                        let rs = rdr.decode_huff(ac_tbl)?;
                        if rs == 0x00 { break; }
                        if rs == 0xF0 {
                            k += 16;
                            if k > 64 { return Err(JxlError::CorruptStream("AC ZRL run exceeds block")); }
                            continue;
                        }
                        k += (rs >> 4) as usize;
                        let ac_s = rs & 0x0F;
                        if k >= 64 { return Err(JxlError::CorruptStream("AC run exceeds 64 coefficients")); }
                        blk[k] = decode_magnitude(ac_s, rdr.read_bits(ac_s)?);
                        k += 1;
                    }
                    blocks.push(blk);
                }
            }
        }
        mcu_done += count_mcus;
        pad_records.push(rdr.finish_padding()?);
    }

    let source_blake3 = *blake3::hash(jpeg_bytes).as_bytes();
    let (prefix, suffix) = (&jpeg_bytes[..framing.prefix_end], &jpeg_bytes[framing.scan_end..]);
    let mut jbrd = Vec::with_capacity(64 + prefix.len() + suffix.len() + pad_records.len() * 2);
    jbrd.extend_from_slice(JBRD_MAGIC);
    jbrd.extend_from_slice(&(jpeg_bytes.len() as u32).to_be_bytes());
    jbrd.extend_from_slice(&source_blake3);
    for slice in [prefix, suffix] {
        jbrd.extend_from_slice(&(slice.len() as u32).to_be_bytes());
        jbrd.extend_from_slice(slice);
    }
    jbrd.extend_from_slice(&(pad_records.len() as u32).to_be_bytes());
    for (plen, pbits) in pad_records {
        jbrd.extend_from_slice(&[plen, pbits]);
    }
    Ok((
        jbrd,
        JbrdExtractedCoeffs {
            width: framing.width, height: framing.height, precision: framing.precision,
            num_components: framing.comps.len() as u8, restart_interval: framing.restart_interval,
            source_blake3, blocks,
        },
    ))
}

/// Reconstruct a byte-exact legacy JPEG file (`FF D8 .. FF D9`) from a `jbrd` box payload
/// and quantized 8x8 zigzag DCT coefficient blocks, verifying the BLAKE3 custody digest.
pub fn reconstruct_jpeg_from_jbrd(
    jbrd_payload: &[u8],
    dct_blocks: &[[i16; 64]],
    limits: JxlLimits,
) -> Result<Vec<u8>, JxlError> {
    if jbrd_payload.len() < 49 || &jbrd_payload[..5] != JBRD_MAGIC {
        return Err(JxlError::CorruptStream("invalid or truncated jbrd header"));
    }
    let orig_len = u32::from_be_bytes(jbrd_payload[5..9].try_into().unwrap()) as usize;
    if orig_len > limits.max_decoded_bytes || orig_len < 4 {
        return Err(JxlError::ByteLimit);
    }
    let expected_blake3: [u8; 32] = jbrd_payload[9..41].try_into().unwrap();
    let mut pos = 41usize;
    let read_u32 = |buf: &[u8], p: &mut usize| -> Result<usize, JxlError> {
        let s = buf.get(*p..*p + 4).ok_or(JxlError::CorruptStream("truncated jbrd length field"))?;
        *p += 4;
        Ok(u32::from_be_bytes(s.try_into().unwrap()) as usize)
    };
    let prefix_len = read_u32(jbrd_payload, &mut pos)?;
    let prefix = jbrd_payload.get(pos..pos + prefix_len).ok_or(JxlError::CorruptStream("truncated jbrd prefix"))?;
    pos += prefix_len;
    let suffix_len = read_u32(jbrd_payload, &mut pos)?;
    let suffix = jbrd_payload.get(pos..pos + suffix_len).filter(|s| s.len() >= 2).ok_or(JxlError::CorruptStream("truncated jbrd suffix"))?;
    pos += suffix_len;
    let num_intervals = read_u32(jbrd_payload, &mut pos)?;
    if pos + num_intervals * 2 != jbrd_payload.len() {
        return Err(JxlError::CorruptStream("corrupt jbrd interval padding table"));
    }
    let mut pads = Vec::with_capacity(num_intervals);
    for _ in 0..num_intervals {
        let (plen, pbits) = (jbrd_payload[pos], jbrd_payload[pos + 1]);
        if plen > 7 { return Err(JxlError::CorruptStream("invalid jbrd pad bit length > 7")); }
        pads.push((plen, pbits));
        pos += 2;
    }

    let mut header_probe = Vec::with_capacity(prefix.len() + 2);
    header_probe.extend_from_slice(prefix);
    header_probe.extend_from_slice(&[0xFF, 0xD9]);
    let framing = parse_jpeg_framing(&header_probe)?;
    if dct_blocks.len() != framing.total_mcus * framing.blocks_per_mcu {
        return Err(JxlError::CorruptStream("DCT block count mismatches JPEG SOF geometry"));
    }
    let mcus_per_interval = if framing.restart_interval == 0 { framing.total_mcus } else { framing.restart_interval as usize };
    if pads.len() != framing.total_mcus.div_ceil(mcus_per_interval.max(1)) {
        return Err(JxlError::CorruptStream("jbrd interval count mismatches DRI"));
    }

    let mut out = Vec::with_capacity(orig_len);
    out.extend_from_slice(prefix);
    let (mut blk_idx, mut mcu_done) = (0usize, 0usize);
    for (int_idx, &(plen, pbits)) in pads.iter().enumerate() {
        let (mut bw, mut prev_dc) = (ScanBitWriter::new(), [0i16; 4]);
        let count_mcus = (framing.total_mcus - mcu_done).min(mcus_per_interval);
        for _ in 0..count_mcus {
            for &(comp_idx, td, ta) in &framing.scan_comps {
                let (_, h_s, v_s) = framing.comps[comp_idx];
                let (dc_tbl, ac_tbl) = (framing.dc_tables[td].as_ref().unwrap(), framing.ac_tables[ta].as_ref().unwrap());
                for _ in 0..(h_s as usize * v_s as usize) {
                    let blk = &dct_blocks[blk_idx];
                    blk_idx += 1;
                    let diff = blk[0].wrapping_sub(prev_dc[comp_idx]);
                    prev_dc[comp_idx] = blk[0];
                    let (ssss, dc_bits) = encode_magnitude(diff)?;
                    bw.put_huff(dc_tbl, ssss)?;
                    if ssss > 0 { bw.put_bits(dc_bits, ssss); }
                    let last_nz = (1..64).rev().find(|&k| blk[k] != 0).unwrap_or(0);
                    let mut run = 0usize;
                    for k in 1..=last_nz {
                        if blk[k] == 0 {
                            run += 1;
                        } else {
                            while run >= 16 { bw.put_huff(ac_tbl, 0xF0)?; run -= 16; }
                            let (ac_s, ac_bits) = encode_magnitude(blk[k])?;
                            bw.put_huff(ac_tbl, ((run as u8) << 4) | ac_s)?;
                            bw.put_bits(ac_bits, ac_s);
                            run = 0;
                        }
                    }
                    if last_nz < 63 { bw.put_huff(ac_tbl, 0x00)?; }
                }
            }
        }
        mcu_done += count_mcus;
        if plen > 0 { bw.put_bits(pbits as u16, plen); }
        if bw.bits != 0 { return Err(JxlError::CorruptStream("jbrd interval padding did not byte-align scan")); }
        out.extend_from_slice(&bw.buf);
        if int_idx + 1 < pads.len() { out.extend_from_slice(&[0xFF, 0xD0 + ((int_idx & 7) as u8)]); }
    }
    out.extend_from_slice(suffix);
    if out.len() != orig_len || !out.starts_with(&[0xFF, 0xD8]) || !out.ends_with(&[0xFF, 0xD9]) {
        return Err(JxlError::CorruptStream("reconstructed JPEG framing or length mismatch"));
    }
    if blake3::hash(&out).as_bytes() != &expected_blake3 {
        return Err(JxlError::CorruptStream("reconstructed JPEG BLAKE3 digest mismatch"));
    }
    Ok(out)
}

/// Extract the `jbrd` box from an ISOBMFF `JXL ` container (`.111`) and reconstruct the
/// byte-exact legacy JPEG file from `dct_blocks`.
pub fn reconstruct_jpeg_from_jxl_container(
    container_bytes: &[u8],
    dct_blocks: &[[i16; 64]],
    limits: JxlLimits,
) -> Result<Vec<u8>, JxlError> {
    let jbrd = extract_jbrd_payload(container_bytes, limits)?;
    reconstruct_jpeg_from_jbrd(jbrd, dct_blocks, limits)
}
