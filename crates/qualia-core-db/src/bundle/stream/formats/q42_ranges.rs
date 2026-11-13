//! Q42 byte access limited to caller-supplied, entry-root-authenticated spans.
//!
//! This adapter authenticates bytes through `AuthenticatedRange`; it does not
//! validate the complete Q42 indexes or establish query completeness.
use std::io;

use crate::q42::q42_volume::{Q42ByteRange, Q42RangeSource};

use super::range::{AuthenticatedRange, FormatError};
use crate::bundle::stream::selection::SelectedPlainEntry;

/// A Q42 range source over a bounded, ordered set of already authenticated
/// bytes from one selected plaintext entry. Gaps are permitted in the
/// collection, but reads crossing a gap fail before touching the output.
pub struct Q42AuthenticatedRangeSource<'r, 'a> {
    selected: SelectedPlainEntry<'a>,
    ranges: &'r [AuthenticatedRange<'a>],
    length: u64,
}

impl<'r, 'a> Q42AuthenticatedRangeSource<'r, 'a> {
    /// Bind a caller-bounded collection to the selected entry and its exact
    /// logical length. Ranges must be ordered, nonempty, nonoverlapping, and
    /// within that entry. `max_ranges` is a caller policy bound, not a memory
    /// reservation.
    pub fn new(
        selected: SelectedPlainEntry<'a>,
        ranges: &'r [AuthenticatedRange<'a>],
        declared_length: u64,
        max_ranges: usize,
    ) -> Result<Self, FormatError> {
        if declared_length != selected.logical_length() {
            return Err(FormatError::Identity);
        }
        if ranges.len() > max_ranges {
            return Err(FormatError::Limit);
        }
        let mut previous_end = 0u64;
        for (index, range) in ranges.iter().enumerate() {
            let other = range.selected();
            if other.candidate_identity() != selected.candidate_identity()
                || other.entry_id() != selected.entry_id()
                || other.ordinal() != selected.ordinal()
                || other.root() != selected.root()
                || other.logical_length() != selected.logical_length()
            {
                return Err(FormatError::Identity);
            }
            let start = range.offset();
            let size = u64::try_from(range.bytes().len()).map_err(|_| FormatError::Bounds)?;
            let end = start.checked_add(size).ok_or(FormatError::Bounds)?;
            if size == 0 || end > declared_length || (index != 0 && start < previous_end) {
                return Err(FormatError::Geometry);
            }
            previous_end = end;
        }
        Ok(Self {
            selected,
            ranges,
            length: declared_length,
        })
    }

    pub const fn selected(&self) -> SelectedPlainEntry<'a> {
        self.selected
    }

    /// Exact-fit read helper. It first proves full request coverage and only
    /// then copies, so a missing span cannot leave a partially written buffer.
    pub fn read_exact_at(&self, offset: u64, out: &mut [u8]) -> io::Result<()> {
        self.read_range_into(
            Q42ByteRange {
                offset,
                length: out.len(),
            },
            out,
        )
    }

    fn preflight(&self, start: u64, end: u64) -> io::Result<()> {
        let mut cursor = start;
        for range in self.ranges {
            let range_start = range.offset();
            let range_end = range_start + range.bytes().len() as u64;
            if range_end <= cursor {
                continue;
            }
            if range_start > cursor {
                break;
            }
            cursor = cursor.max(range_end);
            if cursor >= end {
                return Ok(());
            }
        }
        if start == end {
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::UnexpectedEof))
        }
    }
}

impl Q42RangeSource for Q42AuthenticatedRangeSource<'_, '_> {
    fn length(&self) -> io::Result<u64> {
        Ok(self.length)
    }

    fn read_range_into(&self, range: Q42ByteRange, out: &mut [u8]) -> io::Result<()> {
        if out.len() != range.length {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        let end = range
            .offset
            .checked_add(
                u64::try_from(range.length)
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?,
            )
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        if end > self.length {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        self.preflight(range.offset, end)?;

        let mut cursor = range.offset;
        let mut written = 0usize;
        for span in self.ranges {
            let span_start = span.offset();
            let span_end = span_start + span.bytes().len() as u64;
            if span_end <= cursor {
                continue;
            }
            if cursor == end {
                break;
            }
            let copy_start = cursor.max(span_start);
            let copy_end = end.min(span_end);
            if copy_start >= copy_end {
                continue;
            }
            let within = usize::try_from(copy_start - span_start)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            let count = usize::try_from(copy_end - copy_start)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            out[written..written + count].copy_from_slice(&span.bytes()[within..within + count]);
            written += count;
            cursor = copy_end;
        }
        debug_assert_eq!(written, out.len());
        Ok(())
    }
}
