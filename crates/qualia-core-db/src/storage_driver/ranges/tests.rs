use super::*;
use crate::{
    sensing::local_custody::quota::{
        CustodyQuota, QuotaError, QuotaLimits, QuotaSnapshot, QuotaSnapshotAuthority,
    },
    storage_driver::MmapDriver,
};
use std::{fs, path::Path};

struct Snapshot;
impl QuotaSnapshotAuthority for Snapshot {
    fn admit_usage(&self, _: &Path, _: QuotaLimits, snapshot: QuotaSnapshot) -> bool {
        snapshot.id == [1; 32]
    }
}
fn new_quota<const N: usize>(path: &Path, bytes: u64) -> CustodyQuota<'_, N> {
    CustodyQuota::from_authenticated_usage(
        path,
        QuotaLimits {
            encoded_bytes: bytes,
            entries: N as u64,
        },
        QuotaSnapshot {
            id: [1; 32],
            encoded_bytes: 0,
            entries: 0,
        },
        &Snapshot,
    )
    .unwrap()
}
fn limits() -> RangeLimits {
    RangeLimits {
        payload_bytes: 16,
        proof_bytes: 8,
        checkpoint_bytes: 4,
    }
}
fn children(path: &Path) -> usize {
    fs::read_dir(path).unwrap().count()
}

#[test]
fn canonical_quota_tokens_coexist_and_no_admission_error_mutates_root() {
    let root = tempfile::tempdir().unwrap();
    let mut quota = new_quota::<2>(root.path(), 40);
    assert!(matches!(
        quota.admit_staging([0; 32], 1),
        Err(QuotaError::InvalidLimits)
    ));
    assert!(matches!(
        quota.admit_staging([2; 32], 0),
        Err(QuotaError::InvalidLimits)
    ));
    assert!(matches!(
        quota.admit_staging([2; 32], 41),
        Err(QuotaError::BytesExceeded)
    ));
    assert_eq!(quota.charged_usage(), (0, 0));
    assert_eq!(children(root.path()), 0);
    let first = quota.admit_staging([2; 32], 20).unwrap();
    let second = quota.admit_staging([3; 32], 20).unwrap();
    assert_eq!((first.allowance(), second.allowance()), (20, 20));
    assert_eq!(quota.charged_usage(), (40, 2));
    assert!(matches!(
        quota.admit_staging([2; 32], 1),
        Err(QuotaError::AlreadyCharged)
    ));
    drop((first, second));
    assert_eq!(quota.charged_usage(), (40, 2));
    assert!(matches!(
        quota.admit_staging([4; 32], 1),
        Err(QuotaError::SlotsExhausted)
    ));
}
#[test]
fn limits_and_scope_fail_after_charge_before_filesystem_mutation() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<4>(root.path(), 100);
    let error = driver
        .create_range_staging(quota.admit_staging([2; 32], 27).unwrap(), limits())
        .err()
        .unwrap();
    assert_eq!(error.error, RangeError::InvalidLimits);
    assert!(error.cleanup_pending.is_none());
    let zero = RangeLimits {
        payload_bytes: 0,
        proof_bytes: 0,
        checkpoint_bytes: 0,
    };
    assert_eq!(
        driver
            .create_range_staging(quota.admit_staging([3; 32], 1).unwrap(), zero)
            .err()
            .unwrap()
            .error,
        RangeError::InvalidLimits
    );
    let overflow = RangeLimits {
        payload_bytes: u64::MAX,
        proof_bytes: 1,
        checkpoint_bytes: 0,
    };
    assert_eq!(
        driver
            .create_range_staging(quota.admit_staging([4; 32], 1).unwrap(), overflow)
            .err()
            .unwrap()
            .error,
        RangeError::Overflow
    );
    let mut other_quota = new_quota::<1>(other.path(), 28);
    assert_eq!(
        driver
            .create_range_staging(other_quota.admit_staging([5; 32], 28).unwrap(), limits())
            .err()
            .unwrap()
            .error,
        RangeError::ScopeMismatch
    );
    assert_eq!(quota.charged_usage(), (29, 3));
    assert_eq!(other_quota.charged_usage(), (28, 1));
    assert_eq!(children(root.path()), 0);
    assert_eq!(children(other.path()), 0);
}
#[test]
fn actual_three_file_io_boundaries_holes_sync_and_success_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let mut session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    let path = session.staging_path().to_path_buf();
    assert_eq!(children(&path), 4);
    assert!(!session.capabilities().directory_power_loss_qualified);
    for (file, cap) in [
        (RangeFile::Payload, 16),
        (RangeFile::Proof, 8),
        (RangeFile::Checkpoint, 4),
    ] {
        assert_eq!(
            session.write_exact_at(file, cap, &[1]),
            Err(RangeError::Bounds)
        );
        assert_eq!(
            session.read_exact_at(file, cap, &mut [0]),
            Err(RangeError::Bounds)
        );
        assert_eq!(
            session.write_exact_at(file, u64::MAX, &[1]),
            Err(RangeError::Overflow)
        );
        session.write_exact_at(file, cap, &[]).unwrap();
        assert_eq!(session.observed_file_length(file).unwrap(), 0);
        session.write_exact_at(file, cap - 1, &[7]).unwrap();
        let mut bytes = [9; 16];
        session
            .read_exact_at(file, 0, &mut bytes[..cap as usize])
            .unwrap();
        // A hole returns zeros, but this backend has no verified presence map.
        assert!(bytes[..cap as usize - 1].iter().all(|b| *b == 0));
        assert_eq!(bytes[cap as usize - 1], 7);
        assert_eq!(
            session.sync_file(file).unwrap(),
            FileBarrier::SyncAllReturned
        );
    }
    assert_eq!(session.close().unwrap(), CleanupObservation::Removed);
    assert!(!path.exists());
    assert_eq!(quota.charged_usage(), (28, 1));
}
#[test]
fn actual_eof_is_not_implicitly_materialized_and_drop_keeps_charge() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let mut session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    let path = session.staging_path().to_path_buf();
    assert_eq!(
        session.read_exact_at(RangeFile::Payload, 0, &mut [0]),
        Err(RangeError::Io(std::io::ErrorKind::UnexpectedEof))
    );
    drop(session);
    assert!(!path.exists());
    assert_eq!(quota.charged_usage(), (28, 1));
}
#[test]
fn unwind_cleans_owned_directory_but_cannot_refund_charge() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut session = driver
            .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
            .unwrap();
        session
            .write_exact_at(RangeFile::Payload, 0, &[1, 2])
            .unwrap();
        panic!("deliberate staging unwind");
    }));
    assert!(outcome.is_err());
    assert_eq!(children(root.path()), 0);
    assert_eq!(quota.charged_usage(), (28, 1));
}
#[test]
fn failed_root_preflight_keeps_charge_without_creating_a_staging_directory() {
    let root = tempfile::tempdir().unwrap();
    let child = root.path().join("removed");
    let driver = MmapDriver::new(&child);
    let mut quota = new_quota::<1>(&child, 28);
    let charge = quota.admit_staging([2; 32], 28).unwrap();
    fs::remove_dir(&child).unwrap();
    assert!(matches!(
        driver
            .create_range_staging(charge, limits())
            .err()
            .unwrap()
            .error,
        RangeError::Io(_)
    ));
    assert_eq!(children(root.path()), 0);
    assert_eq!(quota.charged_usage(), (28, 1));
}
#[test]
fn true_u64_quota_overflow_refuses_without_consuming_slot() {
    let root = tempfile::tempdir().unwrap();
    let mut quota = CustodyQuota::<1>::from_authenticated_usage(
        root.path(),
        QuotaLimits {
            encoded_bytes: u64::MAX,
            entries: 2,
        },
        QuotaSnapshot {
            id: [1; 32],
            encoded_bytes: u64::MAX,
            entries: 0,
        },
        &Snapshot,
    )
    .unwrap();
    assert!(matches!(
        quota.admit_staging([2; 32], 1),
        Err(QuotaError::Overflow)
    ));
    assert_eq!(quota.charged_usage(), (u64::MAX, 0));
    assert_eq!(children(root.path()), 0);
}
#[cfg(unix)]
#[test]
fn unix_permissions_and_cooperative_exclusion_are_explicit() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    assert_eq!(
        session.capabilities().writer_exclusion,
        WriterExclusion::CooperativeLockOnly
    );
    assert_eq!(
        fs::metadata(session.staging_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    for file in [RangeFile::Payload, RangeFile::Proof, RangeFile::Checkpoint] {
        assert_eq!(
            fs::metadata(session.staging_path().join(file.name()))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    // Ordinary Unix opens are possible; no independent-writer exclusion claim.
    let _ordinary = fs::OpenOptions::new()
        .write(true)
        .open(session.staging_path().join("payload"))
        .unwrap();
    session.close().unwrap();
}
#[cfg(windows)]
#[test]
fn windows_retained_handles_deny_ordinary_writers() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    assert_eq!(
        session.capabilities().writer_exclusion,
        WriterExclusion::WindowsShareDeny
    );
    assert!(fs::OpenOptions::new()
        .write(true)
        .open(session.staging_path().join("payload"))
        .is_err());
    session.close().unwrap();
}
#[cfg(windows)]
#[test]
fn actual_cleanup_denial_returns_pending_then_retries_after_handle_release() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    let path = session.staging_path().to_path_buf();
    let original_owner = fs::read(path.join(".owner")).unwrap();
    let mut blocker = None;
    let pending = super::session::close_for_test(session, || {
        blocker = Some(
            fs::OpenOptions::new()
                .read(true)
                .share_mode(1)
                .open(path.join("payload"))
                .unwrap(),
        );
    })
    .unwrap_err();
    assert_eq!(pending.attempt_id(), [2; 32]);
    assert!(pending.path().exists());
    assert!(matches!(pending.error(), RangeError::Io(_)));
    assert_eq!(fs::read(path.join(".owner")).unwrap(), original_owner);
    let pending = pending.retry().unwrap_err();
    assert!(matches!(pending.error(), RangeError::Io(_)));
    assert_eq!(fs::read(path.join(".owner")).unwrap(), original_owner);
    drop(blocker);
    assert_eq!(pending.retry().unwrap(), CleanupObservation::Removed);
    assert!(!path.exists());
    assert_eq!(quota.charged_usage(), (28, 1));
}

#[test]
fn partial_creation_error_closes_handles_then_cleans_directory() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let error = super::session::creation_failure_for_test(
        &driver,
        quota.admit_staging([2; 32], 28).unwrap(),
        limits(),
    )
    .err()
    .unwrap();
    assert!(matches!(
        error.error,
        RangeError::Io(std::io::ErrorKind::AlreadyExists)
    ));
    assert!(error.cleanup_pending.is_none());
    assert_eq!(children(root.path()), 0);
    assert_eq!(quota.charged_usage(), (28, 1));
}

#[test]
fn full_u64_ceiling_is_accounted_without_preallocating_file_or_zero_components() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), u64::MAX);
    let limits = RangeLimits {
        payload_bytes: u64::MAX,
        proof_bytes: 0,
        checkpoint_bytes: 0,
    };
    let mut session = driver
        .create_range_staging(quota.admit_staging([2; 32], u64::MAX).unwrap(), limits)
        .unwrap();
    assert_eq!(session.observed_file_length(RangeFile::Payload).unwrap(), 0);
    assert_eq!(
        session.write_exact_at(RangeFile::Proof, 0, &[1]),
        Err(RangeError::Bounds)
    );
    session.write_exact_at(RangeFile::Proof, 0, &[]).unwrap();
    session
        .read_exact_at(RangeFile::Payload, u64::MAX, &mut [])
        .unwrap();
    assert_eq!(
        session.write_exact_at(RangeFile::Payload, u64::MAX, &[1]),
        Err(RangeError::Overflow)
    );
    session.close().unwrap();
    assert_eq!(quota.charged_usage(), (u64::MAX, 1));
}

mod markers;

mod allocation;
