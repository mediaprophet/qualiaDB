use crate::bundle::stream::{
    proof::{ProofError, VerifiedLeafSpan},
    selection::SelectedPlainEntry,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatError {
    Bounds,
    Capacity,
    Limit,
    Identity,
    Unsupported,
    Geometry,
    Integrity,
    Proof(ProofError),
}
impl From<ProofError> for FormatError {
    fn from(e: ProofError) -> Self {
        Self::Proof(e)
    }
}
/// Exact entry-relative bytes from a fully verified covering leaf span. This
/// retains the signed selection association but has no current policy authority.
#[derive(Clone, Copy)]
pub struct AuthenticatedRange<'a> {
    selected: SelectedPlainEntry<'a>,
    offset: u64,
    bytes: &'a [u8],
}
impl<'a> AuthenticatedRange<'a> {
    pub fn new(
        selected: SelectedPlainEntry<'a>,
        span: &'a VerifiedLeafSpan<'_>,
        offset: u64,
        length: usize,
    ) -> Result<Self, FormatError> {
        if selected.tree_spec().map_err(|_| FormatError::Identity)? != *span.tree_spec()
            || selected.root() != span.root()
        {
            return Err(FormatError::Identity);
        }
        let bytes = span.plaintext_range(offset, length)?;
        Ok(Self {
            selected,
            offset,
            bytes,
        })
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn offset(&self) -> u64 {
        self.offset
    }
    pub const fn selected(&self) -> SelectedPlainEntry<'a> {
        self.selected
    }
    pub(super) fn same_entry(&self, other: &Self) -> bool {
        self.selected.candidate_identity() == other.selected.candidate_identity()
            && self.selected.entry_id() == other.selected.entry_id()
            && self.selected.ordinal() == other.selected.ordinal()
            && self.selected.root() == other.selected.root()
            && self.selected.logical_length() == other.selected.logical_length()
    }
}
/// Numeric limits supplement, never replace, the enclosing runtime reservation.
#[derive(Clone, Copy)]
pub struct FormatLimits {
    pub max_entry_bytes: u64,
    pub max_rows: usize,
    pub max_metadata_bytes: usize,
    pub max_chunk_bytes: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PayloadInterval {
    pub(crate) start: u64,
    pub(crate) end: u64,
}
pub(super) fn extent(start: u64, length: u64, whole: u64) -> Result<u64, FormatError> {
    let end = start.checked_add(length).ok_or(FormatError::Bounds)?;
    if end > whole {
        return Err(FormatError::Bounds);
    }
    Ok(end)
}
pub(super) fn overlap(a: u64, b: u64, c: u64, d: u64) -> bool {
    a < d && c < b
}
pub(super) fn nonoverlap(rows: &mut [PayloadInterval]) -> Result<(), FormatError> {
    // Iterative heapsort: bounded O(n log n), no allocator and no recursion.
    fn sift(rows: &mut [PayloadInterval], mut root: usize, end: usize) {
        while root < end / 2 {
            let mut child = root * 2 + 1;
            if child + 1 < end && rows[child].start < rows[child + 1].start {
                child += 1;
            }
            if rows[root].start >= rows[child].start {
                break;
            }
            rows.swap(root, child);
            root = child;
        }
    }
    for root in (0..rows.len() / 2).rev() {
        sift(rows, root, rows.len());
    }
    for end in (1..rows.len()).rev() {
        rows.swap(0, end);
        sift(rows, 0, end);
    }
    if rows.windows(2).any(|w| w[0].end > w[1].start) {
        return Err(FormatError::Geometry);
    }
    Ok(())
}
