//! DICOM RLE Lossless, selected MONOCHROME1/2 integer8/16 profile.
//! Caller owns decoded output and one byte-plane scratch; no allocation occurs.
mod frame;
mod packbits;
use super::{metadata::IntegerImageInfo, NativePixelView, PixelError};
use crate::medical::dicom_dataset::{DatasetError, ElementKind, Part10Dataset, WorkBudget};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RleError {
    Dataset(DatasetError),
    Pixel(PixelError),
    Unsupported(&'static str),
    OutputTooSmall,
    ScratchTooSmall,
    ByteLimit,
    FrameLimit,
    WorkLimit,
    Overflow,
    FragmentCount,
    BasicOffsetTable,
    SegmentHeader,
    SegmentOffsets,
    TruncatedRun,
    RunCrossesRow,
    TrailingSegmentData,
}
impl From<DatasetError> for RleError {
    fn from(e: DatasetError) -> Self {
        Self::Dataset(e)
    }
}
impl From<PixelError> for RleError {
    fn from(e: PixelError) -> Self {
        Self::Pixel(e)
    }
}
impl std::fmt::Display for RleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM RLE: {self:?}")
    }
}
impl std::error::Error for RleError {}
#[derive(Clone, Copy, Debug)]
pub struct RleLimits {
    pub max_decoded_bytes: usize,
    pub max_scratch_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_work: usize,
}
impl Default for RleLimits {
    fn default() -> Self {
        Self {
            max_decoded_bytes: 256 * 1024 * 1024,
            max_scratch_bytes: 16 * 1024 * 1024,
            max_compressed_bytes: 32 * 1024 * 1024,
            max_frames: 4096,
            max_work: 2 * 1024 * 1024 * 1024,
        }
    }
}
fn bot_entry(bot: &[u8], index: usize) -> Result<u32, RleError> {
    let start = index.checked_mul(4).ok_or(RleError::Overflow)?;
    let b = bot
        .get(start..start.checked_add(4).ok_or(RleError::Overflow)?)
        .ok_or(RleError::BasicOffsetTable)?;
    Ok(u32::from_le_bytes(
        b.try_into().map_err(|_| RleError::BasicOffsetTable)?,
    ))
}
/// Validate all metadata/fragments/segments/runs and admit complete resource/work
/// bounds before writing output or scratch. Success returns a native-layout view
/// bound to the ORIGINAL compressed source digest; no source/metadata is changed.
/// Extra caller buffer tails remain untouched. Every RLE frame must occupy exactly
/// one fragment. Empty BOT is supported; nonempty BOT is verified, not trusted.
pub fn decode_rle_lossless_into<'source: 'out, 'out>(
    ds: &Part10Dataset<'source>,
    output: &'out mut [u8],
    scratch: &mut [u8],
    limits: RleLimits,
    w: &mut WorkBudget,
) -> Result<NativePixelView<'out>, RleError> {
    if ds.transfer_syntax_uid() != "1.2.840.10008.1.2.5" {
        return Err(RleError::Unsupported(
            "requires RLE Lossless transfer syntax",
        ));
    }
    let info = IntegerImageInfo::from_dataset(ds, limits.max_decoded_bytes, w)?;
    if !matches!(info.bits_allocated, 8 | 16) {
        return Err(RleError::Unsupported(
            "selected RLE monochrome profile requires BitsAllocated8/16",
        ));
    }
    if ds.find(None, 0x00280006, w)?.is_some() {
        return Err(RleError::Unsupported(
            "monochrome RLE requires absent PlanarConfiguration",
        ));
    }
    if ds.find(None, 0x7fe00001, w)?.is_some() || ds.find(None, 0x7fe00002, w)?.is_some() {
        return Err(RleError::Unsupported(
            "Extended Offset Table profile not selected",
        ));
    }
    if info.frames > limits.max_frames {
        return Err(RleError::FrameLimit);
    }
    if output.len() < info.bytes {
        return Err(RleError::OutputTooSmall);
    }
    if info.samples_per_frame > limits.max_scratch_bytes {
        return Err(RleError::ByteLimit);
    }
    if scratch.len() < info.samples_per_frame {
        return Err(RleError::ScratchTooSmall);
    }
    let id = ds
        .find(None, 0x7fe00010, w)?
        .ok_or(PixelError::Missing(0x7fe00010))?;
    let element = ds.element(id)?;
    if element.kind != ElementKind::EncapsulatedPixels || element.vr != Some(*b"OB") {
        return Err(PixelError::Invalid(0x7fe00010).into());
    }
    let planes = info.bits_allocated as usize / 8;
    let frame_bytes = info
        .samples_per_frame
        .checked_mul(planes)
        .ok_or(RleError::Overflow)?;
    let mut items = ds.sequence_items(id, w)?;
    let table = items.next().ok_or(RleError::FragmentCount)?;
    let bot = ds.item_bytes(table)?;
    let bot_bytes = (info.frames as usize)
        .checked_mul(4)
        .ok_or(RleError::Overflow)?;
    if !bot.is_empty() && (bot.len() != bot_bytes || bot_entry(bot, 0)? != 0) {
        return Err(RleError::BasicOffsetTable);
    }
    let mut count = 0usize;
    let mut offset = 0usize;
    let mut compressed = 0usize;
    let mut work = 0usize;
    // First pass admits bytes/work and validates one-fragment-per-frame/BOT.
    for item in items {
        if count >= info.frames as usize {
            return Err(RleError::FragmentCount);
        }
        let bytes = ds.item_bytes(item)?;
        if !bot.is_empty() && bot_entry(bot, count)? as usize != offset {
            return Err(RleError::BasicOffsetTable);
        }
        compressed = compressed
            .checked_add(bytes.len())
            .ok_or(RleError::Overflow)?;
        if compressed > limits.max_compressed_bytes {
            return Err(RleError::ByteLimit);
        }
        let cost = bytes
            .len()
            .checked_mul(2)
            .and_then(|n| frame_bytes.checked_mul(3).and_then(|m| n.checked_add(m)))
            .and_then(|n| n.checked_add(64))
            .ok_or(RleError::Overflow)?;
        work = work.checked_add(cost).ok_or(RleError::Overflow)?;
        if work > limits.max_work {
            return Err(RleError::WorkLimit);
        }
        offset = offset
            .checked_add(8)
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or(RleError::Overflow)?;
        count += 1;
    }
    if count != info.frames as usize {
        return Err(RleError::FragmentCount);
    }
    let mut validate = ds.sequence_items(id, w)?;
    validate.next().ok_or(RleError::FragmentCount)?;
    for item in validate {
        frame::validate(
            ds.item_bytes(item)?,
            info.rows as usize,
            info.columns as usize,
            planes,
        )?;
    }
    // Acquire the final charged iterator before the first output write. The same
    // immutable source was fully validated, so deterministic decode cannot add a
    // new data/work validation failure after publication into caller buffers.
    let mut decode = ds.sequence_items(id, w)?;
    decode.next().ok_or(RleError::FragmentCount)?;
    for (index, item) in decode.enumerate() {
        let start = index.checked_mul(frame_bytes).ok_or(RleError::Overflow)?;
        let end = start.checked_add(frame_bytes).ok_or(RleError::Overflow)?;
        frame::decode(
            ds.item_bytes(item)?,
            info.rows as usize,
            info.columns as usize,
            planes,
            output.get_mut(start..end).ok_or(RleError::OutputTooSmall)?,
            scratch,
        )?;
    }
    let needed = info.bytes;
    NativePixelView::from_admitted_layout(info, &output[..needed]).map_err(RleError::from)
}
#[cfg(test)]
mod tests;
