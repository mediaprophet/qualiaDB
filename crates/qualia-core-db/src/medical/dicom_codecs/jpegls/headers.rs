//! ITU-T T.87 (JPEG-LS) markers, error types, budgets and coding parameters.
use super::super::encapsulated::EncapError;
use super::super::PixelError;

pub const TS_JPEGLS_LOSSLESS: &str = "1.2.840.10008.1.2.4.80";
pub const TS_JPEGLS_NEAR_LOSSLESS: &str = "1.2.840.10008.1.2.4.81";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegLsError {
    Encap(EncapError),
    Pixel(PixelError),
    Unsupported(&'static str),
    OutputTooSmall,
    ByteLimit,
    FrameLimit,
    FragmentLimit,
    DimensionLimit,
    Overflow,
    CorruptStream(&'static str),
    DimensionMismatch,
    BitPrecisionMismatch,
    LosslessModeViolation,
}
impl From<EncapError> for JpegLsError {
    fn from(e: EncapError) -> Self {
        match e {
            EncapError::ByteLimit => Self::ByteLimit,
            EncapError::FrameLimit => Self::FrameLimit,
            EncapError::FragmentLimit => Self::FragmentLimit,
            EncapError::DimensionLimit => Self::DimensionLimit,
            EncapError::Overflow => Self::Overflow,
            EncapError::Unsupported(s) => Self::Unsupported(s),
            other => Self::Encap(other),
        }
    }
}
impl From<PixelError> for JpegLsError {
    fn from(e: PixelError) -> Self {
        Self::Pixel(e)
    }
}
impl std::fmt::Display for JpegLsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM JPEG-LS: {self:?}")
    }
}
impl std::error::Error for JpegLsError {}

#[derive(Clone, Copy, Debug)]
pub struct JpegLsLimits {
    pub max_part10_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_fragments: usize,
    pub max_width: u16,
    pub max_height: u16,
}
impl Default for JpegLsLimits {
    fn default() -> Self {
        Self {
            max_part10_bytes: 128 * 1024 * 1024,
            max_decoded_bytes: 256 * 1024 * 1024,
            max_compressed_bytes: 64 * 1024 * 1024,
            max_frames: 4096,
            max_fragments: 16384,
            max_width: 16384,
            max_height: 16384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegLsFrameInfo<'a> {
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
    pub near_max: u8,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PresetParams {
    pub maxval: u32,
    pub t1: i32,
    pub t2: i32,
    pub t3: i32,
    pub reset: i32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct DerivedCoding {
    pub maxval: i32,
    pub t1: i32,
    pub t2: i32,
    pub t3: i32,
    pub reset: i32,
    pub near: i32,
    pub range: i32,
    pub qbpp: u8,
    pub bpp: u8,
    pub limit: u8,
}

pub fn derive_coding(
    precision: u8,
    near: u8,
    preset: Option<PresetParams>,
) -> Result<DerivedCoding, JpegLsError> {
    let near_i = near as i32;
    let default_max = (1u32 << precision) - 1;
    let (maxval, t1, t2, t3, reset) = if let Some(p) = preset {
        let mv = if p.maxval == 0 { default_max } else { p.maxval };
        if mv == 0 || mv > 65535 || near_i >= mv as i32 {
            return Err(JpegLsError::CorruptStream("invalid LSE MAXVAL/NEAR"));
        }
        let (dt1, dt2, dt3) = default_thresholds(mv as i32, near_i);
        let rt1 = if p.t1 == 0 { dt1 } else { p.t1 };
        let rt2 = if p.t2 == 0 { dt2 } else { p.t2 };
        let rt3 = if p.t3 == 0 { dt3 } else { p.t3 };
        let rrst = if p.reset == 0 { 64 } else { p.reset };
        if rt1 < near_i + 1 || rt1 > rt2 || rt2 > rt3 || rt3 > mv as i32 || rrst < 3 {
            return Err(JpegLsError::CorruptStream("invalid LSE thresholds/RESET"));
        }
        (mv as i32, rt1, rt2, rt3, rrst)
    } else {
        let mv = default_max as i32;
        if near_i >= mv {
            return Err(JpegLsError::CorruptStream("NEAR exceeds MAXVAL"));
        }
        let (dt1, dt2, dt3) = default_thresholds(mv, near_i);
        (mv, dt1, dt2, dt3, 64)
    };
    let range = if near_i == 0 {
        maxval + 1
    } else {
        (maxval + 2 * near_i) / (2 * near_i + 1) + 1
    };
    let bpp = precision.max(2);
    let mut qbpp = 0u8;
    while (1i32 << qbpp) < range {
        qbpp += 1;
    }
    let qbpp = qbpp.max(2);
    let limit = 2 * (bpp + bpp.max(8));
    Ok(DerivedCoding { maxval, t1, t2, t3, reset, near: near_i, range, qbpp, bpp, limit })
}

pub fn default_thresholds(maxval: i32, near: i32) -> (i32, i32, i32) {
    let clamp = |v: i32, lo: i32| v.max(lo).min(maxval);
    if maxval >= 128 {
        let factor = (maxval.min(4095) + 128) / 256;
        let t1 = clamp(factor * (3 - 2) + 2 + 3 * near, near + 1);
        let t2 = clamp(factor * (7 - 3) + 3 + 5 * near, t1);
        let t3 = clamp(factor * (21 - 4) + 4 + 7 * near, t2);
        (t1, t2, t3)
    } else {
        let factor = 256 / (maxval + 1);
        let t1 = clamp(3 / factor + 3 * near, near + 1);
        let t2 = clamp(7 / factor + 5 * near, t1);
        let t3 = clamp(21 / factor + 7 * near, t2);
        (t1, t2, t3)
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ParsedScan<'a> {
    pub comp_indices: Vec<usize>,
    pub near: u8,
    pub ilv: u8,
    pub point_transform: u8,
    pub preset: Option<PresetParams>,
    pub restart_interval: u16,
    pub entropy_and_tail: &'a [u8],
}

#[allow(dead_code)]
pub struct ParsedJpegLs<'a> {
    pub precision: u8,
    pub height: u16,
    pub width: u16,
    pub comp_ids: [u8; 4],
    pub comp_count: u8,
    pub scans: Vec<ParsedScan<'a>>,
}

#[inline]
pub fn be_u16(b: &[u8], at: usize) -> Result<u16, JpegLsError> {
    let s = b.get(at..at + 2).ok_or(JpegLsError::CorruptStream("truncated JPEG-LS marker"))?;
    Ok(u16::from_be_bytes([s[0], s[1]]))
}

pub fn parse_jpegls_frame<'a>(data: &'a [u8]) -> Result<ParsedJpegLs<'a>, JpegLsError> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(JpegLsError::CorruptStream("missing JPEG-LS SOI"));
    }
    let mut pos = 2usize;
    let mut sof: Option<(u8, u16, u16, u8, [u8; 4])> = None;
    let mut preset: Option<PresetParams> = None;
    let mut restart_interval = 0u16;
    let mut scans = Vec::new();
    let mut saw_eoi = false;

    while pos < data.len() {
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        if pos >= data.len() {
            return Err(JpegLsError::CorruptStream("truncated JPEG-LS marker"));
        }
        let marker = data[pos];
        pos += 1;
        if marker == 0xD9 {
            saw_eoi = true;
            break;
        }
        if matches!(marker, 0x00 | 0x01 | 0xD0..=0xD8) {
            continue;
        }
        let seg_len = be_u16(data, pos)? as usize;
        if seg_len < 2 {
            return Err(JpegLsError::CorruptStream("invalid JPEG-LS segment length"));
        }
        let payload_end = pos.checked_add(seg_len).ok_or(JpegLsError::Overflow)?;
        let payload = data.get(pos + 2..payload_end).ok_or(JpegLsError::CorruptStream("truncated JPEG-LS segment"))?;
        pos = payload_end;

        match marker {
            0xF7 => {
                if sof.is_some() {
                    return Err(JpegLsError::CorruptStream("duplicate SOF55"));
                }
                if payload.len() < 6 {
                    return Err(JpegLsError::CorruptStream("invalid SOF55"));
                }
                let prec = payload[0];
                let h = u16::from_be_bytes([payload[1], payload[2]]);
                let w = u16::from_be_bytes([payload[3], payload[4]]);
                let nf = payload[5];
                if !(2..=16).contains(&prec) || !matches!(nf, 1 | 3) || payload.len() != 6 + 3 * nf as usize {
                    return Err(JpegLsError::CorruptStream("unsupported SOF55 frame geometry"));
                }
                let mut ids = [0u8; 4];
                for c in 0..nf as usize {
                    ids[c] = payload[6 + 3 * c];
                    let hv = payload[6 + 3 * c + 1];
                    if hv != 0x11 {
                        return Err(JpegLsError::Unsupported("subsampled JPEG-LS SOF55 unsupported"));
                    }
                }
                sof = Some((prec, h, w, nf, ids));
            }
            0xF8 => {
                if payload.is_empty() {
                    return Err(JpegLsError::CorruptStream("empty LSE marker"));
                }
                match payload[0] {
                    1 => {
                        if payload.len() != 11 {
                            return Err(JpegLsError::CorruptStream("invalid LSE preset parameters"));
                        }
                        preset = Some(PresetParams {
                            maxval: u16::from_be_bytes([payload[1], payload[2]]) as u32,
                            t1: u16::from_be_bytes([payload[3], payload[4]]) as i32,
                            t2: u16::from_be_bytes([payload[5], payload[6]]) as i32,
                            t3: u16::from_be_bytes([payload[7], payload[8]]) as i32,
                            reset: u16::from_be_bytes([payload[9], payload[10]]) as i32,
                        });
                    }
                    2 | 3 => return Err(JpegLsError::Unsupported("JPEG-LS LSE mapping tables unsupported")),
                    _ => return Err(JpegLsError::Unsupported("unsupported JPEG-LS LSE id")),
                }
            }
            0xDD => {
                if payload.len() != 2 {
                    return Err(JpegLsError::CorruptStream("invalid DRI marker"));
                }
                restart_interval = u16::from_be_bytes([payload[0], payload[1]]);
            }
            0xDA => {
                let (_, _, _, nf, ids) = sof.ok_or(JpegLsError::CorruptStream("SOS before SOF55"))?;
                if payload.is_empty() {
                    return Err(JpegLsError::CorruptStream("empty SOS"));
                }
                let ns = payload[0];
                if ns == 0 || ns > nf || payload.len() != 1 + 2 * ns as usize + 3 {
                    return Err(JpegLsError::CorruptStream("invalid SOS length"));
                }
                let mut comp_indices = Vec::with_capacity(ns as usize);
                for s in 0..ns as usize {
                    let cs = payload[1 + 2 * s];
                    let idx = (0..nf as usize)
                        .find(|&c| ids[c] == cs)
                        .ok_or(JpegLsError::CorruptStream("SOS references unknown component"))?;
                    if comp_indices.contains(&idx) {
                        return Err(JpegLsError::CorruptStream("duplicate component in SOS"));
                    }
                    comp_indices.push(idx);
                }
                let tail = &payload[1 + 2 * ns as usize..];
                let near = tail[0];
                let ilv = tail[1];
                let pt = tail[2] & 0x0F;
                if ilv > 2 || (ilv == 0 && ns != 1) || (ilv != 0 && ns != nf) {
                    return Err(JpegLsError::CorruptStream("invalid JPEG-LS ILV/component count"));
                }
                if pt != 0 {
                    return Err(JpegLsError::Unsupported("JPEG-LS point transform unsupported"));
                }
                let entropy_start = pos;
                let scan_end = find_scan_end(data, &mut pos)?;
                scans.push(ParsedScan {
                    comp_indices,
                    near,
                    ilv,
                    point_transform: pt,
                    preset,
                    restart_interval,
                    entropy_and_tail: &data[entropy_start..scan_end],
                });
            }
            0xE0..=0xEF | 0xFE => {}
            _ => return Err(JpegLsError::Unsupported("unsupported JPEG-LS marker segment")),
        }
        if marker != 0xDA && pos < data.len() && data[pos] != 0xFF {
            return Err(JpegLsError::CorruptStream("unexpected byte between JPEG-LS markers"));
        }
    }
    let (prec, h, w, nf, ids) = sof.ok_or(JpegLsError::CorruptStream("missing SOF55"))?;
    if !saw_eoi || scans.is_empty() {
        return Err(JpegLsError::CorruptStream("missing EOI or SOS in JPEG-LS stream"));
    }
    Ok(ParsedJpegLs { precision: prec, height: h, width: w, comp_ids: ids, comp_count: nf, scans })
}

pub fn find_scan_end(data: &[u8], pos: &mut usize) -> Result<usize, JpegLsError> {
    let mut i = *pos;
    while i < data.len() {
        if data[i] == 0xFF {
            if i + 1 >= data.len() {
                return Err(JpegLsError::CorruptStream("truncated JPEG-LS entropy scan"));
            }
            let next = data[i + 1];
            if next <= 0x7F {
                i += 2;
                continue;
            }
            if (0xD0..=0xD7).contains(&next) {
                i += 2;
                continue;
            }
            *pos = i;
            return Ok(i);
        }
        i += 1;
    }
    Err(JpegLsError::CorruptStream("unterminated JPEG-LS scan before EOI"))
}
