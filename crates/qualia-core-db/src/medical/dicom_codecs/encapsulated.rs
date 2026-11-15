//! Shared bounded DICOM Part-10 Explicit Little Endian encapsulated reader.
//! Serves JPEG-LS (.80/.81), JPEG 2000 (.90/.91), HTJ2K (.201-.203) and JPEG-XL
//! (.110-.112) without altering `dicom_dataset` TransferSyntax admissions.
use crate::medical::dicom_dataset::{DatasetError, ElementKind, Part10Dataset, WorkBudget};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncapError {
    NotPart10,
    UnsupportedSyntax,
    Truncated,
    MalformedHeader,
    Missing(u32),
    Invalid(u32),
    ByteLimit,
    FrameLimit,
    FragmentLimit,
    DimensionLimit,
    Overflow,
    FragmentCount,
    BasicOffsetTable,
    Unsupported(&'static str),
}
impl From<DatasetError> for EncapError {
    fn from(e: DatasetError) -> Self {
        match e {
            DatasetError::NotPart10 => Self::NotPart10,
            DatasetError::Truncated => Self::Truncated,
            DatasetError::Overflow => Self::Overflow,
            DatasetError::ByteLimit | DatasetError::IndexLimit => Self::ByteLimit,
            DatasetError::Missing(t) => Self::Missing(t),
            DatasetError::Invalid(t) => Self::Invalid(t),
            DatasetError::UnsupportedSyntax => Self::UnsupportedSyntax,
            _ => Self::MalformedHeader,
        }
    }
}
impl std::fmt::Display for EncapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM encapsulated framing: {self:?}")
    }
}
impl std::error::Error for EncapError {}

#[derive(Clone, Copy, Debug)]
pub struct EncapLimits {
    pub max_part10_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_fragments: usize,
    pub max_width: u16,
    pub max_height: u16,
    pub max_elements: usize,
}
impl Default for EncapLimits {
    fn default() -> Self {
        Self {
            max_part10_bytes: 128 * 1024 * 1024,
            max_decoded_bytes: 256 * 1024 * 1024,
            max_compressed_bytes: 64 * 1024 * 1024,
            max_frames: 4096,
            max_fragments: 16384,
            max_width: 16384,
            max_height: 16384,
            max_elements: 8192,
        }
    }
}

pub struct EncapsulatedPart10<'a> {
    pub source_blake3: [u8; 32],
    pub transfer_syntax_uid: &'a str,
    pub rows: u16,
    pub columns: u16,
    pub frames: u32,
    pub samples_per_pixel: u16,
    pub photometric: &'a str,
    pub planar_configuration: Option<u16>,
    pub bits_allocated: u16,
    pub bits_stored: u16,
    pub high_bit: u16,
    pub pixel_representation: u16,
    pub fragments: Vec<&'a [u8]>,
    pub frame_ranges: Vec<(usize, usize)>,
}

impl<'a> EncapsulatedPart10<'a> {
    pub fn frame_bytes(&self, frame_idx: usize, max_bytes: usize) -> Result<Cow<'a, [u8]>, EncapError> {
        let &(start, end) = self.frame_ranges.get(frame_idx).ok_or(EncapError::FragmentCount)?;
        concat_fragments(&self.fragments[start..end], max_bytes)
    }
}

pub fn concat_fragments<'a>(
    fragments: &[&'a [u8]],
    max_bytes: usize,
) -> Result<Cow<'a, [u8]>, EncapError> {
    if fragments.is_empty() {
        return Err(EncapError::FragmentCount);
    }
    if fragments.len() == 1 {
        if fragments[0].len() > max_bytes {
            return Err(EncapError::ByteLimit);
        }
        return Ok(Cow::Borrowed(fragments[0]));
    }
    let mut total = 0usize;
    for frag in fragments {
        total = total.checked_add(frag.len()).ok_or(EncapError::Overflow)?;
        if total > max_bytes {
            return Err(EncapError::ByteLimit);
        }
    }
    let mut buf = Vec::new();
    buf.try_reserve_exact(total).map_err(|_| EncapError::ByteLimit)?;
    for frag in fragments {
        buf.extend_from_slice(frag);
    }
    Ok(Cow::Owned(buf))
}

#[inline]
fn u16le(b: &[u8], at: usize) -> Result<u16, EncapError> {
    let s = b.get(at..at.checked_add(2).ok_or(EncapError::Overflow)?).ok_or(EncapError::Truncated)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}
#[inline]
fn u32le(b: &[u8], at: usize) -> Result<u32, EncapError> {
    let s = b.get(at..at.checked_add(4).ok_or(EncapError::Overflow)?).ok_or(EncapError::Truncated)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
fn is_long_vr(vr: &[u8; 2]) -> bool {
    matches!(vr, b"OB" | b"OD" | b"OF" | b"OL" | b"OV" | b"OW" | b"SQ" | b"SV" | b"UC" | b"UN" | b"UR" | b"UT" | b"UV")
}
fn read_explicit_header(b: &[u8], at: usize) -> Result<(u32, [u8; 2], u32, usize), EncapError> {
    let tag = ((u16le(b, at)? as u32) << 16) | u16le(b, at + 2)? as u32;
    let vr = [
        *b.get(at + 4).ok_or(EncapError::Truncated)?,
        *b.get(at + 5).ok_or(EncapError::Truncated)?,
    ];
    if is_long_vr(&vr) {
        let reserved = u16le(b, at + 6)?;
        if reserved != 0 {
            return Err(EncapError::MalformedHeader);
        }
        let len = u32le(b, at + 8)?;
        Ok((tag, vr, len, at + 12))
    } else {
        let len = u16le(b, at + 6)? as u32;
        Ok((tag, vr, len, at + 8))
    }
}
fn skip_undefined_sequence(b: &[u8], mut cur: usize, depth: usize, budget: &mut usize) -> Result<usize, EncapError> {
    if depth > 16 {
        return Err(EncapError::MalformedHeader);
    }
    while cur < b.len() {
        *budget = budget.checked_sub(1).ok_or(EncapError::MalformedHeader)?;
        let tag = ((u16le(b, cur)? as u32) << 16) | u16le(b, cur + 2)? as u32;
        let len = u32le(b, cur + 4)?;
        cur += 8;
        if tag == 0xFFFEE0DD && len == 0 {
            return Ok(cur);
        }
        if tag != 0xFFFEE000 {
            return Err(EncapError::MalformedHeader);
        }
        if len != u32::MAX {
            cur = cur.checked_add(len as usize).ok_or(EncapError::Overflow)?;
            if cur > b.len() {
                return Err(EncapError::Truncated);
            }
        } else {
            while cur < b.len() {
                *budget = budget.checked_sub(1).ok_or(EncapError::MalformedHeader)?;
                let itag = ((u16le(b, cur)? as u32) << 16) | u16le(b, cur + 2)? as u32;
                if itag == 0xFFFEE00D {
                    if u32le(b, cur + 4)? != 0 {
                        return Err(EncapError::MalformedHeader);
                    }
                    cur += 8;
                    break;
                }
                let (_, _, elen, val_at) = read_explicit_header(b, cur)?;
                cur = if elen == u32::MAX {
                    skip_undefined_sequence(b, val_at, depth + 1, budget)?
                } else {
                    val_at.checked_add(elen as usize).ok_or(EncapError::Overflow)?
                };
                if cur > b.len() {
                    return Err(EncapError::Truncated);
                }
            }
        }
    }
    Err(EncapError::Truncated)
}
fn bot_entry(bot: &[u8], index: usize) -> Result<u32, EncapError> {
    let start = index.checked_mul(4).ok_or(EncapError::Overflow)?;
    let end = start.checked_add(4).ok_or(EncapError::Overflow)?;
    let slice = bot.get(start..end).ok_or(EncapError::BasicOffsetTable)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
fn parse_ascii_trimmed<'a>(raw: &'a [u8], tag: u32) -> Result<&'a str, EncapError> {
    let mut end = raw.len();
    while end > 0 && matches!(raw[end - 1], 0 | b' ') {
        end -= 1;
    }
    let mut start = 0;
    while start < end && raw[start] == b' ' {
        start += 1;
    }
    std::str::from_utf8(&raw[start..end]).map_err(|_| EncapError::Invalid(tag))
}

pub fn parse_encapsulated_part10<'a>(
    bytes: &'a [u8],
    allowed_uids: &[&str],
    limits: EncapLimits,
) -> Result<EncapsulatedPart10<'a>, EncapError> {
    if bytes.len() > limits.max_part10_bytes {
        return Err(EncapError::ByteLimit);
    }
    if bytes.len() < 132 || &bytes[128..132] != b"DICM" {
        return Err(EncapError::NotPart10);
    }
    let mut cur = 132usize;
    let mut budget = limits.max_elements;
    let mut ts_uid: Option<&'a str> = None;
    let mut last_tag: Option<u32> = None;
    while cur < bytes.len() && u16le(bytes, cur)? == 2 {
        budget = budget.checked_sub(1).ok_or(EncapError::MalformedHeader)?;
        let (tag, _vr, len, val_at) = read_explicit_header(bytes, cur)?;
        if len == u32::MAX || len % 2 != 0 || last_tag.is_some_and(|t| tag <= t) {
            return Err(EncapError::MalformedHeader);
        }
        last_tag = Some(tag);
        let end = val_at.checked_add(len as usize).ok_or(EncapError::Overflow)?;
        let val = bytes.get(val_at..end).ok_or(EncapError::Truncated)?;
        if tag == 0x00020010 {
            ts_uid = Some(parse_ascii_trimmed(val, tag)?);
        }
        cur = end;
    }
    let ts_uid = ts_uid.ok_or(EncapError::Missing(0x00020010))?;
    if !allowed_uids.contains(&ts_uid) {
        return Err(EncapError::UnsupportedSyntax);
    }
    let (mut rows, mut cols, mut frames, mut spp) = (None, None, 1u32, None);
    let (mut planar, mut ba, mut bs, mut hb, mut rep) = (None, None, None, None, None);
    let mut photometric: Option<&'a str> = None;
    let mut pixel_at: Option<usize> = None;
    last_tag = None;
    while cur < bytes.len() {
        budget = budget.checked_sub(1).ok_or(EncapError::MalformedHeader)?;
        let (tag, vr, len, val_at) = read_explicit_header(bytes, cur)?;
        if last_tag.is_some_and(|t| tag <= t) {
            return Err(EncapError::MalformedHeader);
        }
        last_tag = Some(tag);
        if matches!(tag, 0x7FE00008 | 0x7FE00009 | 0x7FE00001 | 0x7FE00002) {
            return Err(EncapError::Unsupported("float PixelData or Extended Offset Table unsupported"));
        }
        if tag == 0x7FE00010 {
            if len != u32::MAX || !matches!(&vr, b"OB" | b"OW") {
                return Err(EncapError::Invalid(0x7FE00010));
            }
            pixel_at = Some(val_at);
            break;
        }
        if len == u32::MAX {
            cur = skip_undefined_sequence(bytes, val_at, 1, &mut budget)?;
            continue;
        }
        if len % 2 != 0 {
            return Err(EncapError::MalformedHeader);
        }
        let end = val_at.checked_add(len as usize).ok_or(EncapError::Overflow)?;
        let val = bytes.get(val_at..end).ok_or(EncapError::Truncated)?;
        match tag {
            0x00280002 => spp = Some(u16le(val, 0)?),
            0x00280004 => photometric = Some(parse_ascii_trimmed(val, tag)?),
            0x00280006 => planar = Some(u16le(val, 0)?),
            0x00280008 => {
                let s = parse_ascii_trimmed(val, tag)?;
                frames = s.parse::<u32>().map_err(|_| EncapError::Invalid(tag))?;
            }
            0x00280010 => rows = Some(u16le(val, 0)?),
            0x00280011 => cols = Some(u16le(val, 0)?),
            0x00280100 => ba = Some(u16le(val, 0)?),
            0x00280101 => bs = Some(u16le(val, 0)?),
            0x00280102 => hb = Some(u16le(val, 0)?),
            0x00280103 => rep = Some(u16le(val, 0)?),
            _ => {}
        }
        cur = end;
    }
    let (rows, cols) = (rows.ok_or(EncapError::Missing(0x00280010))?, cols.ok_or(EncapError::Missing(0x00280011))?);
    if rows == 0 || cols == 0 {
        return Err(EncapError::Invalid(0x00280010));
    }
    if cols > limits.max_width || rows > limits.max_height {
        return Err(EncapError::DimensionLimit);
    }
    if frames == 0 || frames > limits.max_frames {
        return Err(EncapError::FrameLimit);
    }
    let spp = spp.ok_or(EncapError::Missing(0x00280002))?;
    let photometric = photometric.ok_or(EncapError::Missing(0x00280004))?;
    let ba = ba.ok_or(EncapError::Missing(0x00280100))?;
    let bs = bs.ok_or(EncapError::Missing(0x00280101))?;
    let hb = hb.ok_or(EncapError::Missing(0x00280102))?;
    let rep = rep.ok_or(EncapError::Missing(0x00280103))?;
    if bs == 0 || bs > ba || hb != bs - 1 || rep > 1 {
        return Err(EncapError::Invalid(0x00280101));
    }
    let mut pcur = pixel_at.ok_or(EncapError::Missing(0x7FE00010))?;
    let bot_tag = ((u16le(bytes, pcur)? as u32) << 16) | u16le(bytes, pcur + 2)? as u32;
    let bot_len = u32le(bytes, pcur + 4)?;
    if bot_tag != 0xFFFEE000 || bot_len == u32::MAX || bot_len % 4 != 0 {
        return Err(EncapError::BasicOffsetTable);
    }
    let bot_start = pcur + 8;
    let bot_end = bot_start.checked_add(bot_len as usize).ok_or(EncapError::Overflow)?;
    let bot = bytes.get(bot_start..bot_end).ok_or(EncapError::Truncated)?;
    let expected_bot = (frames as usize).checked_mul(4).ok_or(EncapError::Overflow)?;
    if !bot.is_empty() && (bot.len() != expected_bot || bot_entry(bot, 0)? != 0) {
        return Err(EncapError::BasicOffsetTable);
    }
    pcur = bot_end;
    let (mut frags, mut starts) = (Vec::new(), Vec::new());
    let (mut compressed, mut offset, mut next_frame, mut found_eod) = (0usize, 0usize, 0usize, false);
    while pcur < bytes.len() {
        let itag = ((u16le(bytes, pcur)? as u32) << 16) | u16le(bytes, pcur + 2)? as u32;
        let ilen = u32le(bytes, pcur + 4)?;
        pcur += 8;
        if itag == 0xFFFEE0DD && ilen == 0 {
            found_eod = true;
            break;
        }
        if itag != 0xFFFEE000 || ilen == u32::MAX {
            return Err(EncapError::MalformedHeader);
        }
        if frags.len() >= limits.max_fragments {
            return Err(EncapError::FragmentLimit);
        }
        compressed = compressed.checked_add(ilen as usize).ok_or(EncapError::Overflow)?;
        if compressed > limits.max_compressed_bytes {
            return Err(EncapError::ByteLimit);
        }
        let fend = pcur.checked_add(ilen as usize).ok_or(EncapError::Overflow)?;
        let frag = bytes.get(pcur..fend).ok_or(EncapError::Truncated)?;
        if !bot.is_empty() && next_frame < frames as usize {
            let expected = bot_entry(bot, next_frame)? as usize;
            if offset == expected {
                starts.push(frags.len());
                next_frame += 1;
            } else if offset > expected {
                return Err(EncapError::BasicOffsetTable);
            }
        }
        offset = offset.checked_add(8).and_then(|v| v.checked_add(frag.len())).ok_or(EncapError::Overflow)?;
        frags.push(frag);
        pcur = fend;
    }
    if !found_eod || frags.is_empty() || (!bot.is_empty() && next_frame != frames as usize) {
        return Err(if frags.is_empty() { EncapError::FragmentCount } else if !found_eod { EncapError::Truncated } else { EncapError::BasicOffsetTable });
    }
    if bot.is_empty() && frames > 1 && frags.len() != frames as usize {
        return Err(EncapError::BasicOffsetTable);
    }
    let frame_ranges = build_frame_ranges(frames, &starts, &frags, bot.is_empty())?;
    Ok(EncapsulatedPart10 {
        source_blake3: *blake3::hash(bytes).as_bytes(),
        transfer_syntax_uid: ts_uid,
        rows,
        columns: cols,
        frames,
        samples_per_pixel: spp,
        photometric,
        planar_configuration: planar,
        bits_allocated: ba,
        bits_stored: bs,
        high_bit: hb,
        pixel_representation: rep,
        fragments: frags,
        frame_ranges,
    })
}

pub fn collect_part10_dataset_frames<'a>(
    ds: &Part10Dataset<'a>,
    frames: u32,
    max_fragments: usize,
    max_compressed_bytes: usize,
    w: &mut WorkBudget,
) -> Result<(Vec<&'a [u8]>, Vec<(usize, usize)>), EncapError> {
    let id = ds.find(None, 0x7FE00010, w)?.ok_or(EncapError::Missing(0x7FE00010))?;
    let elem = ds.element(id)?;
    if elem.kind != ElementKind::EncapsulatedPixels || elem.vr != Some(*b"OB") {
        return Err(EncapError::Invalid(0x7FE00010));
    }
    let mut items = ds.sequence_items(id, w)?;
    let bot = ds.item_bytes(items.next().ok_or(EncapError::FragmentCount)?)?;
    let bot_len = (frames as usize).checked_mul(4).ok_or(EncapError::Overflow)?;
    if !bot.is_empty() && (bot.len() != bot_len || bot_entry(bot, 0)? != 0) {
        return Err(EncapError::BasicOffsetTable);
    }
    let (mut frags, mut starts) = (Vec::new(), Vec::new());
    let (mut compressed, mut offset, mut next_frame) = (0usize, 0usize, 0usize);
    for item in items {
        if frags.len() >= max_fragments {
            return Err(EncapError::FragmentLimit);
        }
        let frag = ds.item_bytes(item)?;
        compressed = compressed.checked_add(frag.len()).ok_or(EncapError::Overflow)?;
        if compressed > max_compressed_bytes {
            return Err(EncapError::ByteLimit);
        }
        if !bot.is_empty() && next_frame < frames as usize {
            let expected = bot_entry(bot, next_frame)? as usize;
            if offset == expected {
                starts.push(frags.len());
                next_frame += 1;
            } else if offset > expected {
                return Err(EncapError::BasicOffsetTable);
            }
        }
        offset = offset.checked_add(8).and_then(|v| v.checked_add(frag.len())).ok_or(EncapError::Overflow)?;
        frags.push(frag);
    }
    if frags.is_empty() || (!bot.is_empty() && next_frame != frames as usize) {
        return Err(if frags.is_empty() { EncapError::FragmentCount } else { EncapError::BasicOffsetTable });
    }
    if bot.is_empty() && frames > 1 && frags.len() != frames as usize {
        return Err(EncapError::BasicOffsetTable);
    }
    let ranges = build_frame_ranges(frames, &starts, &frags, bot.is_empty())?;
    Ok((frags, ranges))
}

fn build_frame_ranges(frames: u32, starts: &[usize], frags: &[&[u8]], empty_bot: bool) -> Result<Vec<(usize, usize)>, EncapError> {
    let mut ranges = Vec::new();
    ranges.try_reserve_exact(frames as usize).map_err(|_| EncapError::FrameLimit)?;
    if !empty_bot {
        for i in 0..frames as usize {
            let end = if i + 1 < frames as usize { starts[i + 1] } else { frags.len() };
            if starts[i] >= end {
                return Err(EncapError::FragmentCount);
            }
            ranges.push((starts[i], end));
        }
    } else if frames == 1 {
        ranges.push((0, frags.len()));
    } else {
        for i in 0..frames as usize {
            ranges.push((i, i + 1));
        }
    }
    Ok(ranges)
}
