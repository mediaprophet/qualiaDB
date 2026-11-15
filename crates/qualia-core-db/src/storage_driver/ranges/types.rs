use crate::sensing::local_custody::quota::StagingCharge;
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeFile {
    Payload,
    Proof,
    Checkpoint,
}
impl RangeFile {
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Payload => 0,
            Self::Proof => 1,
            Self::Checkpoint => 2,
        }
    }
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Payload => "payload",
            Self::Proof => "proof",
            Self::Checkpoint => "checkpoint",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeLimits {
    pub payload_bytes: u64,
    pub proof_bytes: u64,
    pub checkpoint_bytes: u64,
}
impl RangeLimits {
    pub fn total(self) -> Result<u64, RangeError> {
        let total = self
            .payload_bytes
            .checked_add(self.proof_bytes)
            .and_then(|n| n.checked_add(self.checkpoint_bytes))
            .ok_or(RangeError::Overflow)?;
        if total == 0 {
            return Err(RangeError::InvalidLimits);
        }
        Ok(total)
    }
    pub const fn ceiling(self, file: RangeFile) -> u64 {
        match file {
            RangeFile::Payload => self.payload_bytes,
            RangeFile::Proof => self.proof_bytes,
            RangeFile::Checkpoint => self.checkpoint_bytes,
        }
    }
    pub fn check(self, file: RangeFile, offset: u64, len: usize) -> Result<(), RangeError> {
        let len = u64::try_from(len).map_err(|_| RangeError::Overflow)?;
        let end = offset.checked_add(len).ok_or(RangeError::Overflow)?;
        if end > self.ceiling(file) {
            return Err(RangeError::Bounds);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeError {
    OwnershipMarker,
    InvalidLimits,
    InvalidRoot,
    ScopeMismatch,
    Bounds,
    Overflow,
    Unsupported,
    LockUnavailable,
    Io(io::ErrorKind),
}
impl From<io::Error> for RangeError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.kind())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriterExclusion {
    /// Retained Windows handles deny ordinary open sharing; hostile/raw disk
    /// access and namespace/ancestor protection remain host responsibilities.
    WindowsShareDeny,
    /// A cooperative advisory file lock is not independent-writer exclusion.
    CooperativeLockOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeCapabilities {
    pub writer_exclusion: WriterExclusion,
    /// Always false in this staging profile; no directory power-loss claim.
    pub directory_power_loss_qualified: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileBarrier {
    SyncAllReturned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupObservation {
    Removed,
}

/// Creation failed after quota admission. A retained pending owner must be
/// retried or deliberately handed to host recovery; no quota refund occurs.
#[derive(Debug)]
pub struct RangeCreateError<'root> {
    pub error: RangeError,
    pub cleanup_pending: Option<CleanupPending<'root>>,
}

/// Only internally constructed from one owned staging directory. The path is
/// diagnostic, not an arbitrary-path cleanup capability.
#[derive(Debug)]
pub struct CleanupPending<'root> {
    pub(super) path: PathBuf,
    pub(super) parent: PathBuf,
    pub(super) charge: StagingCharge<'root>,
    pub(super) error: RangeError,
}
impl CleanupPending<'_> {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub const fn error(&self) -> RangeError {
        self.error
    }
    pub fn attempt_id(&self) -> [u8; 32] {
        self.charge.attempt_id()
    }
}
