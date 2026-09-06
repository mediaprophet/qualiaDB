//! Import job status and promote result.

use super::error::ImportError;
use std::path::PathBuf;

/// Lifecycle status for a bounded import job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    Running,
    Cancelled,
    Failed(String),
    Succeeded {
        bytes_read: u64,
        accepted: u64,
        quarantined: u64,
        output_bytes: u64,
    },
}

impl ImportStatus {
    pub fn is_succeeded(&self) -> bool {
        matches!(self, Self::Succeeded { .. })
    }
}

/// Progress returned by one `feed_chunk` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedProgress {
    /// Bytes consumed from the immutable raw scratch copy in this call.
    pub bytes_this_chunk: u64,
    /// Cumulative bytes read from raw so far.
    pub bytes_read_total: u64,
    /// True when the raw stream is exhausted after this call.
    pub eof: bool,
}

/// Per-chunk accept / quarantine counts from the caller callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChunkOutcome {
    pub accepted: u64,
    pub quarantined: u64,
}

/// Artifact moved out of scratch on successful promote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedArtifact {
    pub path: PathBuf,
    pub sha256: [u8; 32],
    pub bytes: u64,
    pub accepted: u64,
    pub quarantined: u64,
}

impl ImportStatus {
    pub(crate) fn require_succeeded(&self) -> Result<(u64, u64, u64, u64), ImportError> {
        match self {
            Self::Succeeded {
                bytes_read,
                accepted,
                quarantined,
                output_bytes,
            } => Ok((*bytes_read, *accepted, *quarantined, *output_bytes)),
            Self::Cancelled => Err(ImportError::Cancelled),
            _ => Err(ImportError::NotSucceeded),
        }
    }
}
