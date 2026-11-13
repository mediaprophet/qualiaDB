//! Retained read-only source ownership in the existing canonical parent ledger.
//! Drop closes files but never releases the retained canonical charge/table row.
mod admission;
mod types;
pub(super) use types::RecoverySourceSlot;
pub use types::{
    RetainedRecoverySource, SourceError, SourceReleaseError, MAX_RETAINED_SOURCES,
    SOURCE_COLD_PATH_BYTES,
};
#[cfg(test)]
mod tests;
