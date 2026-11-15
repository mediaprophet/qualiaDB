//! P64 complete-entry streaming view over a whole authenticated `.p64` entry.
//! Inner parse is `P64TensorIndex::from_p64_with_integrity`. Bytes are already
//! leaf-authenticated; this is format admit, not a second disclosure grant.
use super::{AuthenticatedRange, FormatError};
use crate::p64_weight::{has_p64_magic, IntegrityMode, P64TensorIndex};
/// Version/profile capability. Complete-entry is the transfer unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P64Profile {
    CompleteEntry,
}
/// Admitted complete `.p64` plus the streaming interpretation that succeeded.
pub struct P64StreamView<'a> {
    range: AuthenticatedRange<'a>,
    index: P64TensorIndex,
    profile: P64Profile,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P64AdmitError {
    Format(FormatError),
    Family,
    Version,
    Extent,
    Integrity,
    Limit,
    Unsupported,
}
impl From<FormatError> for P64AdmitError {
    fn from(e: FormatError) -> Self {
        Self::Format(e)
    }
}
fn map_parse(err: &str) -> P64AdmitError {
    if err.contains("magic") {
        P64AdmitError::Family
    } else if err.contains("version") {
        P64AdmitError::Version
    } else if err.contains("truncated")
        || err.contains("out of bounds")
        || err.contains("overlap")
        || err.contains("unaligned")
    {
        P64AdmitError::Extent
    } else if err.contains("CRC") || err.contains("checksum") {
        P64AdmitError::Integrity
    } else if err.contains("unsupported") {
        P64AdmitError::Unsupported
    } else if err.contains("limit") || err.contains("overflow") {
        P64AdmitError::Limit
    } else {
        P64AdmitError::Integrity
    }
}
impl<'a> P64StreamView<'a> {
    /// Whole selected entry only. Dispatch on magic `p64\0`, never filename.
    /// Partial range is Extent. Metadata integrity is used because the leaf
    /// span already authenticated the bytes (`from_env` is not this path).
    pub fn admit(range: AuthenticatedRange<'a>) -> Result<Self, P64AdmitError> {
        Self::admit_with_integrity(range, IntegrityMode::Metadata)
    }
    pub fn admit_with_integrity(
        range: AuthenticatedRange<'a>,
        integrity: IntegrityMode,
    ) -> Result<Self, P64AdmitError> {
        if range.offset() != 0 || range.bytes().len() as u64 != range.selected().logical_length() {
            return Err(P64AdmitError::Extent);
        }
        if !has_p64_magic(range.bytes()) {
            return Err(P64AdmitError::Family);
        }
        let index = P64TensorIndex::from_p64_with_integrity(range.bytes(), integrity)
            .map_err(|e| map_parse(&e))?;
        Ok(Self {
            range,
            index,
            profile: P64Profile::CompleteEntry,
        })
    }
    pub const fn profile(&self) -> P64Profile {
        self.profile
    }
    pub const fn range(&self) -> AuthenticatedRange<'a> {
        self.range
    }
    pub fn index(&self) -> &P64TensorIndex {
        &self.index
    }
    pub const fn ready_for_plan(&self) -> bool {
        matches!(self.profile, P64Profile::CompleteEntry)
    }
}
/// Planner `present` id. Successful complete-entry admit only.
pub fn p64_present_id(entry_id: u32, view: &P64StreamView<'_>) -> Option<u32> {
    view.ready_for_plan().then_some(entry_id)
}
#[cfg(test)]
#[path = "p64_stream_tests.rs"]
mod p64_stream_tests;
