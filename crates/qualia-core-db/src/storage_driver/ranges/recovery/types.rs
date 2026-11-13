use crate::{
    sensing::local_custody::quota::recovery::RecoveryScopeKey, storage_driver::ranges::RangeLimits,
};
pub const MAX_ROOT_BYTES: usize = 4096;
pub const MAX_RUN_NAME_BYTES: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryError {
    InvalidRoot,
    InvalidLimits,
    InvalidAttempt,
    OwnershipMarker,
    NotRegular,
    ScopeMismatch,
    Bounds,
    Overflow,
    Unavailable,
    Busy,
    Ambiguous,
    Incomplete,
    Unsupported,
    Io(std::io::ErrorKind),
}
impl From<std::io::Error> for RecoveryError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.kind())
    }
}
/// Bounded shallow lookup. This is a work ceiling, not authority or quota.
#[derive(Clone, Copy, Debug)]
pub struct LookupLimits {
    pub entries: u32,
    pub io_steps: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookupReport {
    pub entries: u32,
    pub io_steps: u32,
    pub matches: usize,
}
/// Candidate local location issued only by actual-root bounded discovery.
#[derive(Debug)]
pub struct StagingLocator {
    pub(super) name: [u8; MAX_RUN_NAME_BYTES],
    pub(super) len: u8,
    pub(super) attempt: [u8; 32],
    pub(super) scope: RecoveryScopeKey,
}
impl StagingLocator {
    pub const fn attempt_id(&self) -> [u8; 32] {
        self.attempt
    }
    pub(super) fn name(&self) -> Result<&str, RecoveryError> {
        core::str::from_utf8(&self.name[..usize::from(self.len)])
            .map_err(|_| RecoveryError::InvalidRoot)
    }
}
pub(super) fn check_limits(limits: RangeLimits) -> Result<(), RecoveryError> {
    limits.total().map_err(|e| match e {
        crate::storage_driver::ranges::RangeError::Overflow => RecoveryError::Overflow,
        _ => RecoveryError::InvalidLimits,
    })?;
    Ok(())
}
