//! Nonallocating scans and caller-scratch global directory checks.
use super::cbor::{checked, decode_row};
use super::{CheckedHeader, EntryIter, EntryView, IndexError, IndexReadLimits};
use crate::bundle::BundleEntry;
use minicbor::Decoder;

#[derive(Clone, Copy, Debug)]
pub struct EntryScratch<'a> {
    key: &'a str,
    offset: u64,
    end: u64,
    length: u64,
}
impl EntryScratch<'_> {
    pub const EMPTY: Self = Self {
        key: "",
        offset: 0,
        end: 0,
        length: 0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexSizing {
    entry_count: usize,
    retained_bytes: usize,
    scratch_bytes: usize,
    aggregate_bytes: usize,
}
impl IndexSizing {
    pub const fn entry_count(self) -> usize {
        self.entry_count
    }
    pub const fn retained_bytes(self) -> usize {
        self.retained_bytes
    }
    pub const fn scratch_bytes(self) -> usize {
        self.scratch_bytes
    }
    pub const fn aggregate_bytes(self) -> usize {
        self.aggregate_bytes
    }
}

/// Structurally admitted original bytes; no payload/proof/authority assertion.
#[derive(Debug)]
pub struct AdmittedIndex<'a> {
    header: CheckedHeader,
    bytes: &'a [u8],
    sizing: IndexSizing,
    limits: IndexReadLimits,
}
impl<'a> AdmittedIndex<'a> {
    pub const fn header(&self) -> CheckedHeader {
        self.header
    }
    pub const fn encoded_bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn sizing(&self) -> IndexSizing {
        self.sizing
    }
    pub fn entries(&self) -> EntryIter<'a> {
        EntryIter::new(self.bytes, self.sizing.entry_count, self.limits)
    }
}

fn scan<'a>(
    h: &CheckedHeader,
    bytes: &'a [u8],
    l: IndexReadLimits,
    mut visit: impl FnMut(usize, EntryView<'a>, u64) -> Result<(), IndexError>,
) -> Result<IndexSizing, IndexError> {
    h.check_limits(l)?;
    if u64::try_from(bytes.len()).map_err(|_| IndexError::Overflow)? != h.index_length() {
        return Err(IndexError::Footer);
    }
    let mut d = Decoder::new(bytes);
    let count = checked(d.array())?.ok_or(IndexError::Malformed)?;
    if count != u64::from(h.entry_count()) {
        return Err(IndexError::Count);
    }
    let count = usize::try_from(count).map_err(|_| IndexError::Overflow)?;
    let scratch_bytes = count
        .checked_mul(core::mem::size_of::<EntryScratch<'_>>())
        .ok_or(IndexError::Overflow)?;
    let mut retained_bytes = count
        .checked_mul(core::mem::size_of::<BundleEntry>())
        .ok_or(IndexError::Overflow)?;
    let aggregate = |n: usize| {
        n.checked_mul(4)
            .and_then(|n| n.checked_add(scratch_bytes))
            .ok_or(IndexError::Overflow)
    };
    if aggregate(retained_bytes)? > l.decoded_index_bytes {
        return Err(IndexError::ResidentLimit);
    }
    for i in 0..count {
        let row = decode_row(&mut d, l)?;
        let end = row
            .offset
            .checked_add(row.length)
            .ok_or(IndexError::Overflow)?;
        if row.offset < 64 || end > h.index_offset() {
            return Err(IndexError::EntryBounds);
        }
        if row.offset % h.alignment() != 0 {
            return Err(IndexError::Alignment);
        }
        retained_bytes = retained_bytes
            .checked_add(row.retained_bytes()?)
            .ok_or(IndexError::Overflow)?;
        if aggregate(retained_bytes)? > l.decoded_index_bytes {
            return Err(IndexError::ResidentLimit);
        }
        visit(i, row, end)?;
    }
    if d.position() != bytes.len() {
        return Err(IndexError::Malformed);
    }
    Ok(IndexSizing {
        entry_count: count,
        retained_bytes,
        scratch_bytes,
        aggregate_bytes: aggregate(retained_bytes)?,
    })
}

pub fn measure_index(
    h: &CheckedHeader,
    bytes: &[u8],
    l: IndexReadLimits,
) -> Result<IndexSizing, IndexError> {
    scan(h, bytes, l, |_, _, _| Ok(()))
}

pub fn admit_index<'a>(
    h: &CheckedHeader,
    bytes: &'a [u8],
    l: IndexReadLimits,
    scratch: &mut [EntryScratch<'a>],
) -> Result<AdmittedIndex<'a>, IndexError> {
    let count = usize::try_from(h.entry_count()).map_err(|_| IndexError::Overflow)?;
    if scratch.len() < count {
        return Err(IndexError::Scratch);
    }
    let sizing = scan(h, bytes, l, |i, row, end| {
        scratch[i] = EntryScratch {
            key: row.key,
            offset: row.offset,
            end,
            length: row.length,
        };
        Ok(())
    })?;
    let rows = &mut scratch[..count];
    rows.sort_unstable_by(|a, b| a.key.cmp(b.key));
    if rows.windows(2).any(|p| p[0].key == p[1].key) {
        return Err(IndexError::DuplicateKey);
    }
    rows.sort_unstable_by_key(|r| (r.offset, r.end));
    let mut occupied_end = 64;
    for row in rows {
        // Empty half-open ranges occupy no bytes and never reset the frontier.
        if row.length == 0 {
            continue;
        }
        if row.offset < occupied_end {
            return Err(IndexError::Overlap);
        }
        occupied_end = row.end;
    }
    Ok(AdmittedIndex {
        header: *h,
        bytes,
        sizing,
        limits: l,
    })
}
