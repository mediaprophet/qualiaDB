//! Checked, bounded access to the object-range BIDX.

use std::io;

use super::super::{
    BIDX_MAGIC, FIELD_RANGE_INDEX_ENTRY_BYTES, FIELD_RANGE_INDEX_HEADER_BYTES,
    FIELD_RANGE_INDEX_MAGIC,
};

const BIDX_HEADER_BYTES: usize = 16;
const BIDX_ENTRY_BYTES: usize = 16;

/// A contiguous half-open block interval selected by an object hash.
///
/// The interval can be much larger than a caller output buffer.  Use
/// [`BidxMatchPage`] to enumerate it without allocating one `Vec` entry per
/// matching block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BidxBlockRange {
    pub start: usize,
    pub end: usize,
}

impl BidxBlockRange {
    #[inline]
    pub fn len(self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// One caller-buffered page from a BIDX match interval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BidxMatchPage {
    /// The complete matching interval, for accounting and range coalescing.
    pub range: BidxBlockRange,
    /// Number of block indices written into the caller buffer.
    pub returned: usize,
    /// Resume with this offset relative to `range.start`; `None` means done.
    pub next_cursor: Option<usize>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn layout(bidx: &[u8]) -> io::Result<usize> {
    if bidx.len() < BIDX_HEADER_BYTES {
        return Err(invalid("BIDX section is shorter than its header"));
    }
    if bidx[0..4] != BIDX_MAGIC {
        return Err(invalid("invalid BIDX magic"));
    }
    let version = u32::from_le_bytes(bidx[4..8].try_into().unwrap());
    if version != 1 {
        return Err(invalid(format!("unsupported BIDX version {version}")));
    }
    let block_count = u32::from_le_bytes(bidx[8..12].try_into().unwrap()) as usize;
    let expected = BIDX_HEADER_BYTES
        .checked_add(
            block_count
                .checked_mul(BIDX_ENTRY_BYTES)
                .ok_or_else(|| invalid("BIDX entry count overflows usize"))?,
        )
        .ok_or_else(|| invalid("BIDX length overflows usize"))?;
    if bidx.len() != expected {
        return Err(invalid(format!(
            "BIDX length {} does not match {block_count} entries ({expected} bytes)",
            bidx.len()
        )));
    }
    Ok(block_count)
}

fn range_at(bidx: &[u8], index: usize) -> (u64, u64) {
    let offset = BIDX_HEADER_BYTES + index * BIDX_ENTRY_BYTES;
    let min = u64::from_le_bytes(bidx[offset..offset + 8].try_into().unwrap());
    let max = u64::from_le_bytes(bidx[offset + 8..offset + 16].try_into().unwrap());
    (min, max)
}

/// Validate the complete BIDX layout and the monotonicity required for binary
/// range lookup.  Equal boundaries are valid: a high-frequency object may span
/// many adjacent SuperBlocks.
pub(crate) fn validate_bidx(bidx: &[u8], expected_blocks: usize) -> io::Result<()> {
    let block_count = layout(bidx)?;
    if block_count != expected_blocks {
        return Err(invalid(format!(
            "BIDX block count {block_count} does not match directory count {expected_blocks}"
        )));
    }

    let mut previous_min = 0u64;
    let mut previous_max = 0u64;
    for index in 0..block_count {
        let (min, max) = range_at(bidx, index);
        if min > max {
            return Err(invalid(format!("BIDX entry {index} has min > max")));
        }
        if index != 0 && (min < previous_min || max < previous_max) {
            return Err(invalid(format!(
                "BIDX entry {index} is not monotonic by min/max object hash"
            )));
        }
        previous_min = min;
        previous_max = max;
    }
    Ok(())
}

pub(crate) fn validate_field_range_index(bytes: &[u8], expected_blocks: usize) -> io::Result<()> {
    let expected = FIELD_RANGE_INDEX_HEADER_BYTES
        .checked_add(
            expected_blocks
                .checked_mul(FIELD_RANGE_INDEX_ENTRY_BYTES)
                .ok_or_else(|| invalid("field-range index length overflows"))?,
        )
        .ok_or_else(|| invalid("field-range index length overflows"))?;
    if bytes.len() != expected {
        return Err(invalid(format!(
            "field-range index is {} bytes, expected {expected}",
            bytes.len()
        )));
    }
    if bytes[0..4] != FIELD_RANGE_INDEX_MAGIC {
        return Err(invalid("field-range index has bad magic"));
    }
    if u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 1 {
        return Err(invalid("unsupported field-range index version"));
    }
    if u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize != expected_blocks {
        return Err(invalid("field-range index block count mismatch"));
    }
    for index in 0..expected_blocks {
        let offset = FIELD_RANGE_INDEX_HEADER_BYTES + index * FIELD_RANGE_INDEX_ENTRY_BYTES;
        for field in 0..3 {
            let min_offset = offset + field * 16;
            let min = u64::from_le_bytes(bytes[min_offset..min_offset + 8].try_into().unwrap());
            let max =
                u64::from_le_bytes(bytes[min_offset + 8..min_offset + 16].try_into().unwrap());
            if min > max {
                return Err(invalid("field-range index has an inverted range"));
            }
        }
    }
    Ok(())
}

/// Return the full contiguous BIDX interval that can contain `object_hash`.
///
/// The caller must use a BIDX already validated by [`validate_bidx`].  This
/// function still validates its header/length so standalone callers fail closed
/// on truncated data.
pub(crate) fn bidx_block_range_for_hash(
    bidx: &[u8],
    object_hash: u64,
) -> io::Result<Option<BidxBlockRange>> {
    let block_count = layout(bidx)?;
    validate_bidx(bidx, block_count)?;
    if block_count == 0 {
        return Ok(None);
    }

    // First block whose maximum can contain the hash.
    let mut lo = 0usize;
    let mut hi = block_count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if range_at(bidx, mid).1 < object_hash {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let start = lo;
    if start == block_count || range_at(bidx, start).0 > object_hash {
        return Ok(None);
    }

    // First block whose minimum is strictly greater than the hash.
    lo = start;
    hi = block_count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if range_at(bidx, mid).0 <= object_hash {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    Ok(Some(BidxBlockRange { start, end: lo }))
}

/// Fill one bounded page of matching BIDX block indices.
pub(crate) fn bidx_blocks_for_hash_into(
    bidx: &[u8],
    object_hash: u64,
    cursor: usize,
    out: &mut [usize],
) -> io::Result<Option<BidxMatchPage>> {
    let Some(range) = bidx_block_range_for_hash(bidx, object_hash)? else {
        return Ok(None);
    };
    if out.is_empty() && cursor < range.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "BIDX page buffer must contain at least one block index",
        ));
    }
    if cursor > range.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "BIDX cursor is beyond the matching interval",
        ));
    }
    let remaining = range.len() - cursor;
    let returned = remaining.min(out.len());
    for (offset, slot) in out.iter_mut().take(returned).enumerate() {
        *slot = range.start + cursor + offset;
    }
    let next = cursor + returned;
    Ok(Some(BidxMatchPage {
        range,
        returned,
        next_cursor: (next < range.len()).then_some(next),
    }))
}
