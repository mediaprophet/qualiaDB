//! Exact original Part-10 custody, independent of pixel extraction/decoding.
//!
//! Callers must exclusively control the storage root during publication. Paths
//! are checked for containment and symlinks, but this is not an OS sandbox against
//! an adversary concurrently replacing directory entries. Publication is write-once;
//! retained objects are not a multi-file transaction with the existing pixel/WAL store.
mod acquisition;
mod association;
mod ingest;
mod storage;

pub use acquisition::{AcquisitionView, RetainedDicom};
pub use association::source_for_record;
pub use ingest::{split_ingest_sync_with_source, DicomIngestWithSource};
pub use storage::{
    archive_part10_source, DicomSourceArtifact, DicomSourceBudget, DicomSourceError,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod acquisition_tests;
