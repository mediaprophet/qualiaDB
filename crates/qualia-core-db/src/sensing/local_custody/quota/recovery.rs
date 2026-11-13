//! Zero-growth root provenance from the actual admitted quota owner.
//! No marker/scan reconstructs usage, per-run membership or a budget token.
//! Host holds current protected-namespace and aggregate-quota leases throughout.
use super::{CustodyQuota, QuotaError};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::path::Path;
static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);
pub(super) fn next_recovery_owner() -> Result<usize, QuotaError> {
    NEXT_OWNER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| QuotaError::Overflow)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RecoveryScopeKey {
    owner: usize,
    serial: u64,
}
#[derive(Debug)]
pub struct RecoveryRootScope<'root> {
    root: &'root Path,
    snapshot: [u8; 32],
    key: RecoveryScopeKey,
}
impl<'root> RecoveryRootScope<'root> {
    pub const fn snapshot_id(&self) -> [u8; 32] {
        self.snapshot
    }
    pub(crate) const fn root(&self) -> &'root Path {
        self.root
    }
    pub(crate) const fn key(&self) -> RecoveryScopeKey {
        self.key
    }
}
impl<'root, const N: usize> CustodyQuota<'root, N> {
    /// Borrow actual root provenance without retaining a mutable quota borrow.
    /// This does not refund/reissue old usage or authenticate a particular run.
    pub fn recovery_root_scope(&mut self) -> Result<RecoveryRootScope<'root>, QuotaError> {
        let serial = self
            .recovery_scope_serial
            .checked_add(1)
            .ok_or(QuotaError::Overflow)?;
        self.recovery_scope_serial = serial;
        Ok(RecoveryRootScope {
            root: self.root,
            snapshot: self.usage.id,
            key: RecoveryScopeKey {
                owner: self.recovery_owner,
                serial,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensing::local_custody::quota::{
        QuotaLimits, QuotaSnapshot, QuotaSnapshotAuthority,
    };
    struct Snapshot;
    impl QuotaSnapshotAuthority for Snapshot {
        fn admit_usage(&self, _: &Path, _: QuotaLimits, s: QuotaSnapshot) -> bool {
            s.id == [1; 32]
        }
    }
    #[test]
    fn scopes_are_distinct_noncopy_provenance_without_usage_or_mutable_borrow() {
        let root = tempfile::tempdir().unwrap();
        let mut quota = CustodyQuota::<4>::from_authenticated_usage(
            root.path(),
            QuotaLimits {
                encoded_bytes: 100,
                entries: 4,
            },
            QuotaSnapshot {
                id: [1; 32],
                encoded_bytes: 10,
                entries: 1,
            },
            &Snapshot,
        )
        .unwrap();
        let first = quota.recovery_root_scope().unwrap();
        let second = quota.recovery_root_scope().unwrap();
        assert_ne!(first.key(), second.key());
        assert_eq!(first.snapshot_id(), [1; 32]);
        assert_eq!(quota.charged_usage(), (10, 1));
        let token = quota.admit_staging([2; 32], 20).unwrap();
        drop(token);
        assert_eq!(quota.charged_usage(), (30, 2));
        assert_eq!(first.root(), root.path());
        let before = second.key();
        quota.recovery_scope_serial = u64::MAX;
        assert!(matches!(
            quota.recovery_root_scope(),
            Err(QuotaError::Overflow)
        ));
        assert_eq!(second.key(), before);
        assert_eq!(quota.charged_usage(), (30, 2));
    }
}
