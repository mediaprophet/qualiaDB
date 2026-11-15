use crate::net::{
    peer::runtime::ReservationHandle,
    qdnf::{errors::QdnfError, types::Generation},
};
use crate::sensing::local_custody::quota::recovery::RecoveryScopeKey;
use crate::storage_driver::ranges::{
    recovery::{RangeRecoveryRead, RangeRecoverySource, RecoveryError},
    RangeLimits,
};
pub const MAX_RETAINED_SOURCES: usize = 4;
/// Whole actual fixed source plus a conservative cold path/metadata allowance.
/// OS-internal allocation remains separately qualified; hot reads allocate none.
pub const SOURCE_COLD_PATH_BYTES: usize = 2 * (4096 + 64 + 32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Runtime(QdnfError),
    Backend(RecoveryError),
}
impl From<QdnfError> for SourceError {
    fn from(e: QdnfError) -> Self {
        Self::Runtime(e)
    }
}
impl From<RecoveryError> for SourceError {
    fn from(e: RecoveryError) -> Self {
        Self::Backend(e)
    }
}
#[must_use = "release explicitly through the original TransferRuntime"]
pub struct RetainedRecoverySource<'root> {
    pub(super) owner: usize,
    pub(super) slot: u8,
    pub(super) generation: Generation,
    pub(super) source: RangeRecoverySource<'root>,
}
impl<'root> RetainedRecoverySource<'root> {
    pub const fn charged_bytes() -> usize {
        core::mem::size_of::<Self>() + SOURCE_COLD_PATH_BYTES
    }
    pub fn limits(&self) -> RangeLimits {
        self.source.limits()
    }
    pub fn attempt_id(&self) -> [u8; 32] {
        self.source.attempt_id()
    }
    pub fn checkpoint_available(&self) -> bool {
        self.source.checkpoint_available()
    }
    pub fn matches_canonical_root(&self, path: &std::path::Path) -> bool {
        self.source.matches_canonical_root(path)
    }
    pub(in super::super) fn with_read_view<T>(
        &mut self,
        effect: impl for<'a> FnOnce(RangeRecoveryRead<'a, 'root>) -> T,
    ) -> T {
        effect(self.source.read_view())
    }
}
impl core::fmt::Debug for RetainedRecoverySource<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RetainedRecoverySource")
            .field("slot", &self.slot)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}
#[must_use = "retain the real source and retry release with its original runtime"]
#[derive(Debug)]
pub struct SourceReleaseError<'root> {
    pub error: QdnfError,
    pub source: RetainedRecoverySource<'root>,
}
pub(in super::super) struct RecoverySourceSlot {
    pub(in super::super) generation: Generation,
    pub(in super::super) charge: Option<ReservationHandle>,
    pub(in super::super) scope: Option<RecoveryScopeKey>,
    pub(in super::super) attempt: [u8; 32],
    pub(in super::super) limits: Option<RangeLimits>,
}
impl RecoverySourceSlot {
    pub(in super::super) const fn empty() -> Self {
        Self {
            generation: Generation::ZERO,
            charge: None,
            scope: None,
            attempt: [0; 32],
            limits: None,
        }
    }
}
