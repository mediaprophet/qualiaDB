//! Bounded DICOM encapsulated JPEG Baseline (.50) & Extended (.51) decoder.
//! All framing, dimensions, color/subsampling and budgets are verified before
//! touching the caller output buffer.
use super::PixelError;
use crate::medical::dicom_dataset::{DatasetError, ElementKind, Part10Dataset, WorkBudget};
use jpeg_decoder::{CodingProcess, ColorTransform, Decoder, Error as DecoderError, PixelFormat};
use std::io::{self, Read};

pub const TS_JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
pub const TS_JPEG_EXTENDED: &str = "1.2.840.10008.1.2.4.51";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegError {
    Dataset(DatasetError),
    Pixel(PixelError),
    Unsupported(&'static str),
    OutputTooSmall,
    ByteLimit,
    FrameLimit,
    FragmentLimit,
    DimensionLimit,
    Overflow,
    FragmentCount,
    BasicOffsetTable,
    CorruptStream(&'static str),
    DimensionMismatch,
    ColorOrSubsamplingMismatch,
    BitPrecisionMismatch,
}
impl From<DatasetError> for JpegError { fn from(e: DatasetError) -> Self { Self::Dataset(e) } }
impl From<PixelError> for JpegError { fn from(e: PixelError) -> Self { Self::Pixel(e) } }
impl std::fmt::Display for JpegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "DICOM JPEG: {self:?}") }
}
impl std::error::Error for JpegError {}

#[derive(Clone, Copy, Debug)]
pub struct JpegLimits {
    pub max_decoded_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_fragments: usize,
    pub max_width: u16,
    pub max_height: u16,
}
impl Default for JpegLimits {
    fn default() -> Self {
        Self {
            max_decoded_bytes: 256 * 1024 * 1024, max_compressed_bytes: 64 * 1024 * 1024,
            max_frames: 4096, max_fragments: 16384, max_width: 16384, max_height: 16384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JpegColorFormat {
    Gray8,
    Rgb24,
    /// 12-bit monochrome (`0..=4095`) packed into little-endian `u16` (`2` bytes/sample).
    Gray12,
    /// 12-bit 3-channel RGB (`0..=4095` per channel) packed into little-endian `u16` (`6` bytes/pixel).
    Rgb36,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegDecodedInfo<'a> {
    pub source_blake3: [u8; 32],
    pub transfer_syntax_uid: &'a str,
    pub rows: u16,
    pub columns: u16,
    pub frames: u32,
    pub samples_per_pixel: u16,
    pub bits_stored: u16,
    pub source_photometric: &'a str,
    pub output_format: JpegColorFormat,
    pub bytes: usize,
}

struct FragmentChainReader<'a, 'b> {
    fragments: &'b [&'a [u8]],
    frag_idx: usize,
    offset: usize,
}
impl<'a, 'b> FragmentChainReader<'a, 'b> {
    fn new(fragments: &'b [&'a [u8]]) -> Self {
        Self { fragments, frag_idx: 0, offset: 0 }
    }
}
impl Read for FragmentChainReader<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.frag_idx < self.fragments.len() {
            let frag = self.fragments[self.frag_idx];
            if self.offset < frag.len() {
                let n = buf.len().min(frag.len() - self.offset);
                buf[..n].copy_from_slice(&frag[self.offset..self.offset + n]);
                self.offset += n;
                return Ok(n);
            }
            self.frag_idx += 1;
            self.offset = 0;
        }
        Ok(0)
    }
}

fn read_u8_stream<R: Read>(r: &mut R) -> Result<u8, JpegError> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|_| JpegError::CorruptStream("truncated JPEG header"))?;
    Ok(b[0])
}

fn read_u16_be_stream<R: Read>(r: &mut R) -> Result<u16, JpegError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|_| JpegError::CorruptStream("truncated JPEG marker length"))?;
    Ok(u16::from_be_bytes(b))
}

fn skip_exact<R: Read>(r: &mut R, mut remaining: usize) -> Result<(), JpegError> {
    let mut scratch = [0u8; 256];
    while remaining > 0 {
        let step = remaining.min(scratch.len());
        r.read_exact(&mut scratch[..step]).map_err(|_| JpegError::CorruptStream("truncated JPEG marker payload"))?;
        remaining -= step;
    }
    Ok(())
}

fn bot_entry(bot: &[u8], index: usize) -> Result<u32, JpegError> {
    let start = index.checked_mul(4).ok_or(JpegError::Overflow)?;
    let end = start.checked_add(4).ok_or(JpegError::Overflow)?;
    let b = bot.get(start..end).ok_or(JpegError::BasicOffsetTable)?;
    Ok(u32::from_le_bytes(b.try_into().map_err(|_| JpegError::BasicOffsetTable)?))
}

fn inspect_jpeg_sof(
    fragments: &[&[u8]],
    ts_uid: &str,
    rows: u16,
    cols: u16,
    spp: u16,
    bs: u16,
    photometric: &str,
) -> Result<(), JpegError> {
    let mut r = FragmentChainReader::new(fragments);
    if read_u8_stream(&mut r)? != 0xFF || read_u8_stream(&mut r)? != 0xD8 {
        return Err(JpegError::CorruptStream("missing JPEG SOI marker"));
    }
    loop {
        let mut b = read_u8_stream(&mut r)?;
        while b != 0xFF {
            b = read_u8_stream(&mut r)?;
        }
        let mut marker = read_u8_stream(&mut r)?;
        while marker == 0xFF {
            marker = read_u8_stream(&mut r)?;
        }
        if marker == 0x00 || marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            continue;
        }
        if marker == 0xD9 || marker == 0xDA {
            return Err(JpegError::CorruptStream("JPEG stream missing SOF marker"));
        }
        let seg_len = read_u16_be_stream(&mut r)? as usize;
        if seg_len < 2 {
            return Err(JpegError::CorruptStream("invalid JPEG marker length"));
        }
        let payload_len = seg_len - 2;
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF | 0xF7) {
            if marker == 0xC1 && ts_uid == TS_JPEG_BASELINE {
                return Err(JpegError::Unsupported("JPEG Baseline (.50) rejects Extended SOF1"));
            }
            if !matches!(marker, 0xC0 | 0xC1) {
                return Err(JpegError::Unsupported("JPEG process outside Baseline SOF0 / Extended SOF1"));
            }
            if payload_len < 6 || payload_len > 18 {
                return Err(JpegError::CorruptStream("invalid SOF length"));
            }
            let mut sof = [0u8; 18];
            r.read_exact(&mut sof[..payload_len]).map_err(|_| JpegError::CorruptStream("truncated SOF marker"))?;
            let precision = sof[0];
            let expected_prec = if bs == 12 { 12 } else { 8 };
            if precision == 12 && ts_uid == TS_JPEG_BASELINE {
                return Err(JpegError::Unsupported("JPEG Baseline (.50) rejects 12-bit precision"));
            }
            if precision != expected_prec {
                return Err(JpegError::BitPrecisionMismatch);
            }
            if precision == 12 && marker != 0xC1 {
                return Err(JpegError::Unsupported("12-bit JPEG requires SOF1 Extended Sequential"));
            }
            let h = u16::from_be_bytes([sof[1], sof[2]]);
            let w = u16::from_be_bytes([sof[3], sof[4]]);
            if h != rows || w != cols {
                return Err(JpegError::DimensionMismatch);
            }
            let nf = sof[5];
            if nf as u16 != spp || payload_len != 6 + 3 * nf as usize {
                return Err(JpegError::ColorOrSubsamplingMismatch);
            }
            let mut hv = [(0u8, 0u8); 3];
            for i in 0..nf as usize {
                let s = sof[6 + i * 3 + 1];
                let (hs, vs) = (s >> 4, s & 0x0F);
                if !(1..=4).contains(&hs) || !(1..=4).contains(&vs) {
                    return Err(JpegError::CorruptStream("invalid SOF sampling factor"));
                }
                hv[i] = (hs, vs);
            }
            match photometric {
                "MONOCHROME1" | "MONOCHROME2" if nf == 1 => {}
                "RGB" | "YBR_FULL" if nf == 3 && hv == [(1, 1); 3] => {}
                "YBR_FULL_422" if nf == 3 && hv == [(2, 1), (1, 1), (1, 1)] => {}
                _ => return Err(JpegError::ColorOrSubsamplingMismatch),
            }
            return Ok(());
        }
        skip_exact(&mut r, payload_len)?;
    }
}

fn map_decoder_err(err: DecoderError) -> JpegError {
    match err {
        DecoderError::Unsupported(_) => JpegError::Unsupported("unsupported JPEG feature in jpeg-decoder"),
        DecoderError::Format(_) | DecoderError::Io(_) | DecoderError::Internal(_) => {
            JpegError::CorruptStream("JPEG entropy decode failed")
        }
    }
}

fn validate_metadata<'a>(
    ds: &Part10Dataset<'a>,
    limits: JpegLimits,
    w: &mut WorkBudget,
) -> Result<(u16, u16, u32, u16, u16, &'a str, usize, usize), JpegError> {
    let ts_uid = ds.transfer_syntax_uid();
    if !matches!(ts_uid, TS_JPEG_BASELINE | TS_JPEG_EXTENDED) {
        return Err(JpegError::Unsupported("requires JPEG Baseline (.50) or Extended (.51) transfer syntax"));
    }
    for tag in [0x7fe00008, 0x7fe00009, 0x7fe00001, 0x7fe00002] {
        if ds.find(None, tag, w)?.is_some() {
            return Err(JpegError::Unsupported("float pixel data or Extended Offset Table unsupported"));
        }
    }
    let req = |tag, w: &mut WorkBudget| -> Result<u16, JpegError> {
        ds.unsigned_short(tag, w).map_err(JpegError::from)?.ok_or_else(|| JpegError::Pixel(PixelError::Missing(tag)))
    };
    let (rows, cols) = (req(0x00280010, w)?, req(0x00280011, w)?);
    if rows == 0 || cols == 0 {
        return Err(PixelError::Invalid(0x00280010).into());
    }
    if cols > limits.max_width || rows > limits.max_height {
        return Err(JpegError::DimensionLimit);
    }
    let spp = req(0x00280002, w)?;
    if !matches!(spp, 1 | 3) {
        return Err(JpegError::Unsupported("SamplesPerPixel must be 1 or 3"));
    }
    let photometric = ds.text(None, 0x00280004, *b"CS", w)?.ok_or(PixelError::Missing(0x00280004))?;
    let planar = ds.unsigned_short(0x00280006, w)?;
    if spp == 1 {
        if !matches!(photometric, "MONOCHROME1" | "MONOCHROME2") {
            return Err(JpegError::ColorOrSubsamplingMismatch);
        }
        if planar.is_some() {
            return Err(JpegError::Unsupported("monochrome JPEG requires absent PlanarConfiguration"));
        }
    } else {
        if !matches!(photometric, "RGB" | "YBR_FULL" | "YBR_FULL_422") {
            return Err(JpegError::ColorOrSubsamplingMismatch);
        }
        if planar.is_some_and(|p| p != 0) {
            return Err(JpegError::Unsupported("planar RGB JPEG unsupported"));
        }
    }
    let (ba, bs, hb, rep) = (req(0x00280100, w)?, req(0x00280101, w)?, req(0x00280102, w)?, req(0x00280103, w)?);
    if rep != 0 {
        return Err(JpegError::BitPrecisionMismatch);
    }
    let is_8bit = ba == 8 && bs == 8 && hb == 7;
    let is_12bit = ts_uid == TS_JPEG_EXTENDED && ba == 16 && bs == 12 && hb == 11;
    if !is_8bit && !is_12bit {
        return Err(JpegError::BitPrecisionMismatch);
    }
    let frames = ds.frame_count(w)?;
    if frames == 0 || frames > limits.max_frames {
        return Err(JpegError::FrameLimit);
    }
    let sample_bytes = if is_12bit { 2usize } else { 1usize };
    let frame_bytes = (rows as usize)
        .checked_mul(cols as usize)
        .and_then(|v| v.checked_mul(spp as usize))
        .and_then(|v| v.checked_mul(sample_bytes))
        .ok_or(JpegError::Overflow)?;
    let total_bytes = frame_bytes.checked_mul(frames as usize).ok_or(JpegError::Overflow)?;
    if total_bytes > limits.max_decoded_bytes {
        return Err(JpegError::ByteLimit);
    }
    Ok((rows, cols, frames, spp, bs, photometric, frame_bytes, total_bytes))
}

fn collect_frame_ranges<'a>(
    ds: &Part10Dataset<'a>,
    frames: u32,
    limits: JpegLimits,
    w: &mut WorkBudget,
) -> Result<(Vec<&'a [u8]>, Vec<(usize, usize)>), JpegError> {
    let id = ds.find(None, 0x7fe00010, w)?.ok_or(PixelError::Missing(0x7fe00010))?;
    let elem = ds.element(id)?;
    if elem.kind != ElementKind::EncapsulatedPixels || elem.vr != Some(*b"OB") {
        return Err(PixelError::Invalid(0x7fe00010).into());
    }
    let mut items = ds.sequence_items(id, w)?;
    let bot_id = items.next().ok_or(JpegError::FragmentCount)?;
    let bot = ds.item_bytes(bot_id)?;
    let bot_len = (frames as usize).checked_mul(4).ok_or(JpegError::Overflow)?;
    if !bot.is_empty() && (bot.len() != bot_len || bot_entry(bot, 0)? != 0) {
        return Err(JpegError::BasicOffsetTable);
    }
    let (mut frag_count, mut compressed, mut offset, mut next_frame) = (0usize, 0usize, 0usize, 0usize);
    for item in items {
        frag_count = frag_count.checked_add(1).ok_or(JpegError::Overflow)?;
        if frag_count > limits.max_fragments {
            return Err(JpegError::FragmentLimit);
        }
        let bytes = ds.item_bytes(item)?;
        compressed = compressed.checked_add(bytes.len()).ok_or(JpegError::Overflow)?;
        if compressed > limits.max_compressed_bytes {
            return Err(JpegError::ByteLimit);
        }
        if !bot.is_empty() && next_frame < frames as usize {
            let expected = bot_entry(bot, next_frame)? as usize;
            if offset == expected {
                next_frame += 1;
            } else if offset > expected {
                return Err(JpegError::BasicOffsetTable);
            }
        }
        offset = offset.checked_add(8).and_then(|n| n.checked_add(bytes.len())).ok_or(JpegError::Overflow)?;
    }
    if frag_count == 0 || (!bot.is_empty() && next_frame != frames as usize) {
        return Err(if frag_count == 0 { JpegError::FragmentCount } else { JpegError::BasicOffsetTable });
    }
    if bot.is_empty() && frames > 1 && frag_count != frames as usize {
        return Err(JpegError::BasicOffsetTable);
    }
    let mut frags = Vec::new();
    frags.try_reserve_exact(frag_count).map_err(|_| JpegError::ByteLimit)?;
    let mut ranges = Vec::new();
    ranges.try_reserve_exact(frames as usize).map_err(|_| JpegError::FrameLimit)?;
    let mut starts = Vec::new();
    if !bot.is_empty() {
        starts.try_reserve_exact(frames as usize).map_err(|_| JpegError::FrameLimit)?;
    }
    let mut items2 = ds.sequence_items(id, w)?;
    items2.next();
    let (mut cur_offset, mut f_idx) = (0usize, 0usize);
    for (idx, item) in items2.enumerate() {
        let bytes = ds.item_bytes(item)?;
        if !bot.is_empty() && f_idx < frames as usize && cur_offset == bot_entry(bot, f_idx)? as usize {
            starts.push(idx);
            f_idx += 1;
        }
        cur_offset += 8 + bytes.len();
        frags.push(bytes);
    }
    if !bot.is_empty() {
        for i in 0..frames as usize {
            let end = if i + 1 < frames as usize { starts[i + 1] } else { frags.len() };
            if starts[i] >= end {
                return Err(JpegError::FragmentCount);
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
    Ok((frags, ranges))
}

/// Decode encapsulated DICOM JPEG Baseline (.50) or Extended (.51) frames into
/// `output`, verifying dimensions, coding process, sampling and pixel format
/// across all frames before writing a single byte to `output`.
pub fn decode_jpeg_into<'a>(
    ds: &Part10Dataset<'a>,
    output: &mut [u8],
    limits: JpegLimits,
    w: &mut WorkBudget,
) -> Result<JpegDecodedInfo<'a>, JpegError> {
    let (rows, cols, frames, spp, bs, photometric, frame_bytes, total_bytes) = validate_metadata(ds, limits, w)?;
    if output.len() < total_bytes {
        return Err(JpegError::OutputTooSmall);
    }
    let (frags, ranges) = collect_frame_ranges(ds, frames, limits, w)?;
    let mut staged = Vec::new();
    staged.try_reserve_exact(total_bytes).map_err(|_| JpegError::ByteLimit)?;
    staged.resize(total_bytes, 0);
    let ts_uid = ds.transfer_syntax_uid();
    let expected_pf = if spp == 1 { PixelFormat::L8 } else { PixelFormat::RGB24 };
    for (frame_idx, (start, end)) in ranges.into_iter().enumerate() {
        let slice = &frags[start..end];
        inspect_jpeg_sof(slice, ts_uid, rows, cols, spp, bs, photometric)?;
        let pixels = if bs == 12 {
            super::jpeg_12bit::decode_jpeg_12bit_frame(slice, rows, cols, spp, photometric)?
        } else {
            let mut decoder = Decoder::new(FragmentChainReader::new(slice));
            decoder.set_max_decoding_buffer_size(frame_bytes);
            decoder.read_info().map_err(map_decoder_err)?;
            let info = decoder.info().ok_or(JpegError::CorruptStream("missing JPEG frame info"))?;
            if info.width != cols || info.height != rows { return Err(JpegError::DimensionMismatch); }
            if info.coding_process != CodingProcess::DctSequential { return Err(JpegError::Unsupported("non-sequential JPEG process")); }
            if info.pixel_format != expected_pf { return Err(JpegError::ColorOrSubsamplingMismatch); }
            decoder.set_color_transform(match photometric {
                "MONOCHROME1" | "MONOCHROME2" => ColorTransform::Grayscale,
                "RGB" => ColorTransform::RGB,
                _ => ColorTransform::YCbCr,
            });
            decoder.decode().map_err(map_decoder_err)?
        };
        if pixels.len() != frame_bytes {
            return Err(JpegError::DimensionMismatch);
        }
        let dst = frame_idx * frame_bytes;
        staged[dst..dst + frame_bytes].copy_from_slice(&pixels);
    }
    output[..total_bytes].copy_from_slice(&staged);
    Ok(JpegDecodedInfo {
        source_blake3: ds.source_blake3(), transfer_syntax_uid: ts_uid, rows, columns: cols, frames,
        samples_per_pixel: spp, bits_stored: bs, source_photometric: photometric,
        output_format: match (spp, bs) {
            (1, 12) => JpegColorFormat::Gray12,
            (1, _) => JpegColorFormat::Gray8,
            (_, 12) => JpegColorFormat::Rgb36,
            _ => JpegColorFormat::Rgb24,
        },
        bytes: total_bytes,
    })
}
