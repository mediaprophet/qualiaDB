use super::super::session::owner_bytes;
use super::*;
use crate::{
    sensing::local_custody::quota::{
        CustodyQuota, QuotaLimits, QuotaSnapshot, QuotaSnapshotAuthority,
    },
    storage_driver::{
        ranges::{RangeFile, RangeLimits},
        MmapDriver,
    },
};
use std::{fs, path::Path};
struct Snapshot;
impl QuotaSnapshotAuthority for Snapshot {
    fn admit_usage(&self, _: &Path, _: QuotaLimits, s: QuotaSnapshot) -> bool {
        s.id == [1; 32]
    }
}
pub(crate) fn limits() -> RangeLimits {
    RangeLimits {
        payload_bytes: 8,
        proof_bytes: 8,
        checkpoint_bytes: 8,
    }
}
pub(crate) fn quota(root: &Path) -> CustodyQuota<'_, 8> {
    CustodyQuota::from_authenticated_usage(
        root,
        QuotaLimits {
            encoded_bytes: 256,
            entries: 8,
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
pub(crate) fn material(root: &Path, attempt: [u8; 32], proof: bool, cp: bool) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("qualia-range-")
        .tempdir_in(root)
        .unwrap();
    fs::write(dir.path().join(".owner"), owner_bytes(attempt)).unwrap();
    fs::write(dir.path().join("payload"), [1, 2]).unwrap();
    if proof {
        fs::write(dir.path().join("proof"), [3, 4]).unwrap()
    }
    if cp {
        fs::write(dir.path().join("checkpoint"), [5, 6]).unwrap()
    }
    dir
}
pub(crate) fn locator(
    driver: &MmapDriver,
    scope: &crate::sensing::local_custody::quota::recovery::RecoveryRootScope<'_>,
    id: [u8; 32],
) -> StagingLocator {
    let mut out = [None];
    let report = driver
        .lookup_range_sources(
            scope,
            id,
            LookupLimits {
                entries: 64,
                io_steps: 512,
            },
            &mut out,
        )
        .unwrap();
    assert_eq!(report.matches, 1);
    out[0].take().unwrap()
}
#[test]
fn actual_source_reads_bounds_optional_checkpoint_and_no_quota_refund() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let charge = quota.admit_staging([2; 32], 24).unwrap();
    drop(charge);
    let run = material(root.path(), [2; 32], true, false);
    let scope = quota.recovery_root_scope().unwrap();
    let item = locator(&driver, &scope, [2; 32]);
    let before = quota.charged_usage();
    let mut source = driver
        .open_range_recovery_source(&scope, item, limits())
        .unwrap();
    assert!(!source.checkpoint_available());
    assert!(!source.capabilities().directory_power_loss_qualified);
    let mut bytes = [0; 2];
    source
        .read_exact_at(RangeFile::Payload, 0, &mut bytes)
        .unwrap();
    assert_eq!(bytes, [1, 2]);
    let mut absent = [9];
    assert_eq!(
        source.read_exact_at(RangeFile::Checkpoint, 0, &mut absent),
        Err(RecoveryError::Unavailable)
    );
    assert_eq!(absent, [9]);
    assert_eq!(
        source.read_exact_at(RangeFile::Payload, 8, &mut absent),
        Err(RecoveryError::Bounds)
    );
    assert_eq!(
        source.read_exact_at(RangeFile::Payload, u64::MAX, &mut absent),
        Err(RecoveryError::Overflow)
    );
    source.close();
    assert!(run.path().exists());
    assert_eq!(quota.charged_usage(), before);
}
#[test]
fn missing_required_file_scope_mismatch_and_malformed_marker_never_repair() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    drop(quota.admit_staging([2; 32], 24).unwrap());
    let run = material(root.path(), [2; 32], false, false);
    let first = quota.recovery_root_scope().unwrap();
    let item = locator(&driver, &first, [2; 32]);
    let second = quota.recovery_root_scope().unwrap();
    assert!(matches!(
        driver.open_range_recovery_source(&second, item, limits()),
        Err(RecoveryError::ScopeMismatch)
    ));
    let item = locator(&driver, &first, [2; 32]);
    assert!(matches!(
        driver.open_range_recovery_source(&first, item, limits()),
        Err(RecoveryError::Unavailable)
    ));
    assert!(!run.path().join("proof").exists());
    assert!(!run.path().join("checkpoint").exists());
    let item = locator(&driver, &first, [2; 32]);
    fs::write(run.path().join(".owner"), [0; 64]).unwrap();
    assert!(matches!(
        driver.open_range_recovery_source(&first, item, limits()),
        Err(RecoveryError::OwnershipMarker)
    ));
    assert_eq!(fs::read(run.path().join(".owner")).unwrap(), [0; 64]);
    assert_eq!(quota.charged_usage(), (24, 1));
}
#[test]
fn bounded_scan_duplicate_attempt_and_oversized_component_refuse() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let first = material(root.path(), [2; 32], true, true);
    let scope = quota.recovery_root_scope().unwrap();
    let mut out = [None];
    assert_eq!(
        driver.lookup_range_sources(
            &scope,
            [2; 32],
            LookupLimits {
                entries: 0,
                io_steps: 512
            },
            &mut out
        ),
        Err(RecoveryError::InvalidLimits)
    );
    let item = locator(&driver, &scope, [2; 32]);
    fs::write(first.path().join("proof"), [0; 9]).unwrap();
    assert!(matches!(
        driver.open_range_recovery_source(&scope, item, limits()),
        Err(RecoveryError::Bounds)
    ));
    let _duplicate = material(root.path(), [2; 32], true, false);
    assert_eq!(
        driver.lookup_range_sources(
            &scope,
            [2; 32],
            LookupLimits {
                entries: 64,
                io_steps: 512
            },
            &mut out
        ),
        Err(RecoveryError::Ambiguous)
    );
    assert!(out[0].is_none());
}
#[cfg(unix)]
#[test]
fn final_component_symlink_refuses_without_following() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let run = material(root.path(), [2; 32], true, false);
    let scope = quota.recovery_root_scope().unwrap();
    let item = locator(&driver, &scope, [2; 32]);
    fs::remove_file(run.path().join("payload")).unwrap();
    symlink(run.path().join("proof"), run.path().join("payload")).unwrap();
    assert!(driver
        .open_range_recovery_source(&scope, item, limits())
        .is_err());
}
#[test]
fn retained_real_handles_refuse_competing_open_and_hot_reads_allocate_zero() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let _run = material(root.path(), [2; 32], true, false);
    let scope = quota.recovery_root_scope().unwrap();
    let item = locator(&driver, &scope, [2; 32]);
    let copy = StagingLocator {
        name: item.name,
        len: item.len,
        attempt: item.attempt,
        scope: item.scope,
    };
    let mut source = driver
        .open_range_recovery_source(&scope, item, limits())
        .unwrap();
    assert!(driver
        .open_range_recovery_source(&scope, copy, limits())
        .is_err());
    let mut out = [0; 2];
    source
        .read_exact_at(RangeFile::Payload, 0, &mut out)
        .unwrap();
    let guard = AllocGuard::begin("retained-source-real-read", true);
    source
        .read_exact_at(RangeFile::Payload, 0, &mut out)
        .unwrap();
    assert_eq!(
        source.read_exact_at(RangeFile::Checkpoint, 0, &mut out),
        Err(RecoveryError::Unavailable)
    );
    assert_eq!(
        source.read_exact_at(RangeFile::Proof, 8, &mut out),
        Err(RecoveryError::Bounds)
    );
    guard.check().expect("retained reads must not allocate");
    source.close();
}
