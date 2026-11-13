//! Bounded source interval construction and a separate exact partition validator.
use super::ProtectionError as Error;
use crate::bundle::stream::descriptor::MAX_ENTRIES;
use crate::bundle::stream::index::AdmittedIndex;
use crate::crypto::network::object_stream::{MAX_METADATA, MAX_OBJECT_BYTES, MAX_OBJECTS};
const _: () = assert!(MAX_ENTRIES == 4096 && MAX_OBJECTS == 2 * MAX_ENTRIES + 1);

/// Private original index geometry; not a public descriptor row.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntryRange {
    offset: u64,
    length: u64,
}
impl EntryRange {
    pub(crate) const EMPTY: Self = Self {
        offset: 0,
        length: 0,
    };
    pub(super) const fn new(offset: u64, length: u64) -> Self {
        Self { offset, length }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectKind {
    Entry(u32),
    Gap,
}
/// This contains private offsets/ordinals. It intentionally has no Debug implementation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceObject {
    kind: ObjectKind,
    range: EntryRange,
}
impl SourceObject {
    pub(crate) const EMPTY: Self = Self {
        kind: ObjectKind::Gap,
        range: EntryRange::EMPTY,
    };
    pub(super) const fn entry(ordinal: u32, range: EntryRange) -> Self {
        Self {
            kind: ObjectKind::Entry(ordinal),
            range,
        }
    }
    pub(super) const fn gap(offset: u64, length: u64) -> Self {
        Self {
            kind: ObjectKind::Gap,
            range: EntryRange::new(offset, length),
        }
    }
    pub(crate) const fn kind(&self) -> ObjectKind {
        self.kind
    }
    pub(crate) const fn offset(&self) -> u64 {
        self.range.offset
    }
    pub(crate) const fn length(&self) -> u64 {
        self.range.length
    }
}

/// Fixed private metadata framing; checked before any metadata/key allocation.
pub(super) fn metadata_length(objects: usize, index_length: u64) -> Result<usize, Error> {
    if objects > MAX_OBJECTS {
        return Err(Error::Limit);
    }
    let rows = u64::try_from(objects).map_err(|_| Error::Overflow)?;
    let length = rows
        .checked_mul(108)
        .and_then(|n| n.checked_add(288 + 64))
        .and_then(|n| n.checked_add(index_length))
        .ok_or(Error::Overflow)?;
    if length > MAX_METADATA {
        return Err(Error::Limit);
    }
    usize::try_from(length).map_err(|_| Error::Overflow)
}

/// Object order is original index order followed by ascending maximal gaps.
/// Later opaque-ID assignment must retain this private association.
pub(super) fn prepare(
    index: &AdmittedIndex<'_>,
    entries: &mut [EntryRange],
    objects: &mut [SourceObject],
    order: &mut [u32],
) -> Result<usize, Error> {
    let count = index.sizing().entry_count();
    if count > MAX_ENTRIES {
        return Err(Error::Limit);
    }
    let entries = entries.get_mut(..count).ok_or(Error::Capacity)?;
    let sorted = order.get_mut(..count).ok_or(Error::Capacity)?;
    let mut found = 0;
    for (ordinal, row) in index.entries().enumerate() {
        let row = row.map_err(|_| Error::Source)?;
        let out = entries.get_mut(ordinal).ok_or(Error::Coverage)?;
        *out = EntryRange::new(row.offset(), row.length());
        sorted[ordinal] = u32::try_from(ordinal).map_err(|_| Error::Overflow)?;
        found += 1;
    }
    if found != count {
        return Err(Error::Coverage);
    }
    sorted.sort_unstable_by_key(|i| (entries[*i as usize].offset, entries[*i as usize].length, *i));
    let end = index.header().index_offset();
    let mut frontier = 64;
    let mut gaps = 0usize;
    for ordinal in sorted.iter() {
        let row = entries[*ordinal as usize];
        if row.length == 0 {
            continue;
        }
        if row.offset < frontier {
            return Err(Error::Coverage);
        }
        if row.offset > frontier {
            gaps += 1;
        }
        frontier = row.offset.checked_add(row.length).ok_or(Error::Overflow)?;
        if frontier > end {
            return Err(Error::Coverage);
        }
    }
    if frontier < end {
        gaps += 1;
    }
    let total = count.checked_add(gaps).ok_or(Error::Overflow)?;
    if total > MAX_OBJECTS {
        return Err(Error::Limit);
    }
    if order.len() < total || objects.len() < total {
        return Err(Error::Capacity);
    }
    // Reborrow after capacity checks: sort order still contains the source ordinals.
    for (ordinal, row) in entries.iter().enumerate() {
        objects[ordinal] = SourceObject::entry(ordinal as u32, *row);
    }
    frontier = 64;
    let mut next = count;
    for ordinal in &order[..count] {
        let row = entries[*ordinal as usize];
        if row.length == 0 {
            continue;
        }
        if row.offset > frontier {
            objects[next] = SourceObject::gap(frontier, row.offset - frontier);
            next += 1;
        }
        frontier = row.offset.checked_add(row.length).ok_or(Error::Overflow)?;
    }
    if frontier < end {
        objects[next] = SourceObject::gap(frontier, end - frontier);
        next += 1;
    }
    if next != total {
        return Err(Error::Coverage);
    }
    validate(end, entries, &objects[..total], order)?;
    Ok(total)
}

/// Independently checks a proposed mapping against original admitted ordinal ranges.
/// Sorting uses caller scratch, with a fixed 4096-bit ordinal set; no recursion/heap.
pub(super) fn validate(
    index_offset: u64,
    entries: &[EntryRange],
    objects: &[SourceObject],
    order: &mut [u32],
) -> Result<(), Error> {
    if entries.len() > MAX_ENTRIES || objects.len() > MAX_OBJECTS || index_offset > MAX_OBJECT_BYTES
    {
        return Err(Error::Limit);
    }
    if index_offset < 64 {
        return Err(Error::Coverage);
    }
    let order = order.get_mut(..objects.len()).ok_or(Error::Capacity)?;
    let mut seen = [0u64; MAX_ENTRIES / 64];
    let mut entry_count = 0;
    for (i, row) in objects.iter().enumerate() {
        let end = row
            .offset()
            .checked_add(row.length())
            .ok_or(Error::Overflow)?;
        if row.offset() < 64 || end > index_offset {
            return Err(Error::Coverage);
        }
        match row.kind {
            ObjectKind::Entry(ordinal) => {
                let ordinal = usize::try_from(ordinal).map_err(|_| Error::Overflow)?;
                let expected = entries.get(ordinal).ok_or(Error::Coverage)?;
                let bit = 1u64 << (ordinal % 64);
                if row.range != *expected || seen[ordinal / 64] & bit != 0 {
                    return Err(Error::Coverage);
                }
                seen[ordinal / 64] |= bit;
                entry_count += 1;
            }
            ObjectKind::Gap if row.length() == 0 => return Err(Error::Coverage),
            ObjectKind::Gap => {}
        }
        order[i] = i as u32;
    }
    if entry_count != entries.len() {
        return Err(Error::Coverage);
    }
    order.sort_unstable_by_key(|i| {
        (
            objects[*i as usize].offset(),
            objects[*i as usize].length(),
            *i,
        )
    });
    let mut frontier = 64;
    let mut previous_gap = false;
    for i in order {
        let row = objects[*i as usize];
        // Empty entries may sit inside occupied intervals or gaps. Neither changes
        // the frontier nor separates two adjacent positive gap objects.
        if row.length() == 0 {
            continue;
        }
        if row.offset() != frontier {
            return Err(Error::Coverage);
        }
        let gap = row.kind == ObjectKind::Gap;
        if gap && previous_gap {
            return Err(Error::Coverage);
        }
        previous_gap = gap;
        frontier = frontier.checked_add(row.length()).ok_or(Error::Overflow)?;
    }
    if frontier != index_offset {
        return Err(Error::Coverage);
    }
    Ok(())
}
