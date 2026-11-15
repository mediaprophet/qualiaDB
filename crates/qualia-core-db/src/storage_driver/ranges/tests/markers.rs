use super::*;

#[test]
fn missing_changed_torn_trailing_or_replaced_owner_never_deletes_directory() {
    for mutation in 0..7 {
        let root = tempfile::tempdir().unwrap();
        let driver = MmapDriver::new(root.path());
        let mut quota = new_quota::<1>(root.path(), 28);
        let session = driver
            .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
            .unwrap();
        let path = session.staging_path().to_path_buf();
        let owner = fs::read(path.join(".owner")).unwrap();
        assert_eq!(owner.len(), 64);
        let pending = super::session::close_for_test(session, || {
            let mut bytes = [0; 65];
            bytes[..64].copy_from_slice(&owner);
            match mutation {
                0 => fs::remove_file(path.join(".owner")).unwrap(),
                1 => {
                    bytes[16] ^= 1;
                    fs::write(path.join(".owner"), &bytes[..64]).unwrap();
                }
                2 => {
                    bytes[0] ^= 1;
                    fs::write(path.join(".owner"), &bytes[..64]).unwrap();
                }
                3 => fs::write(path.join(".owner"), &bytes[..63]).unwrap(),
                4 => fs::write(path.join(".owner"), bytes).unwrap(),
                5 => {
                    bytes[63] = 1;
                    fs::write(path.join(".owner"), &bytes[..64]).unwrap();
                }
                _ => {
                    // Explicit fixture replacement stays in the resolved child.
                    let parent = fs::canonicalize(root.path()).unwrap();
                    assert_eq!(path.parent(), Some(parent.as_path()));
                    assert_eq!(fs::canonicalize(&path).unwrap(), path);
                    fs::remove_dir_all(&path).unwrap();
                    fs::create_dir(&path).unwrap();
                    fs::write(path.join("replacement-sentinel"), [9]).unwrap();
                }
            }
        })
        .unwrap_err();
        assert_eq!(pending.error(), RangeError::OwnershipMarker);
        let pending = pending.retry().unwrap_err();
        assert!(path.exists());
        if mutation == 6 {
            assert!(path.join("replacement-sentinel").exists());
        } else {
            assert!(path.join("payload").exists());
        }
        // Host fixture repairs exact ownership explicitly; retry then succeeds.
        fs::write(path.join(".owner"), &owner).unwrap();
        assert_eq!(pending.retry().unwrap(), CleanupObservation::Removed);
        assert!(!path.exists());
        assert_eq!(quota.charged_usage(), (28, 1));
    }
}
#[cfg(unix)]
#[test]
fn symlink_owner_is_rejected_without_following_or_removing_target() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    let path = session.staging_path().to_path_buf();
    let owner = fs::read(path.join(".owner")).unwrap();
    let target = root.path().join("outside-owner");
    fs::write(&target, &owner).unwrap();
    let pending = super::session::close_for_test(session, || {
        fs::remove_file(path.join(".owner")).unwrap();
        symlink(&target, path.join(".owner")).unwrap();
    })
    .unwrap_err();
    assert_eq!(pending.error(), RangeError::OwnershipMarker);
    assert!(target.exists());
    fs::remove_file(path.join(".owner")).unwrap();
    fs::write(path.join(".owner"), owner).unwrap();
    pending.retry().unwrap();
    assert!(target.exists());
}

#[test]
fn automatic_drop_checks_marker_and_preserves_foreign_replacement() {
    for mutation in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let driver = MmapDriver::new(root.path());
        let mut quota = new_quota::<1>(root.path(), 28);
        let session = driver
            .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
            .unwrap();
        let path = session.staging_path().to_path_buf();
        super::super::session::drop_for_test(session, || match mutation {
            0 => fs::remove_file(path.join(".owner")).unwrap(),
            1 => fs::write(path.join(".owner"), [0; 64]).unwrap(),
            _ => {
                let parent = fs::canonicalize(root.path()).unwrap();
                assert_eq!(path.parent(), Some(parent.as_path()));
                assert_eq!(fs::canonicalize(&path).unwrap(), path);
                fs::remove_dir_all(&path).unwrap();
                fs::create_dir(&path).unwrap();
                fs::write(path.join("foreign-sentinel"), [9]).unwrap();
            }
        });
        assert!(path.exists());
        if mutation == 2 {
            assert!(path.join("foreign-sentinel").exists());
        } else {
            assert!(path.join("payload").exists());
        }
        assert_eq!(quota.charged_usage(), (28, 1));
    }
}
#[test]
fn unwind_with_invalid_marker_does_not_bypass_guard() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let mut path = None;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let session = driver
            .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
            .unwrap();
        path = Some(session.staging_path().to_path_buf());
        fs::remove_file(session.staging_path().join(".owner")).unwrap();
        panic!("deliberate tampered-marker unwind");
    }));
    assert!(outcome.is_err());
    assert!(path.unwrap().exists());
    assert_eq!(quota.charged_usage(), (28, 1));
}

#[test]
fn partial_cleanup_restore_never_overwrites_foreign_existing_marker() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    let path = session.staging_path().to_path_buf();
    let mut foreign = fs::read(path.join(".owner")).unwrap();
    foreign[16] ^= 1;
    let parent = fs::canonicalize(root.path()).unwrap();
    let pending = super::super::session::close_for_test(session, || {
        assert_eq!(path.parent(), Some(parent.as_path()));
        assert_eq!(fs::canonicalize(&path).unwrap(), path);
        // Deliberately violate the host namespace-stability precondition:
        // a same-path replacement with an existing different ownership marker.
        fs::remove_dir_all(&path).unwrap();
        fs::create_dir(&path).unwrap();
        fs::write(path.join(".owner"), &foreign).unwrap();
        fs::write(path.join("foreign-sentinel"), [9]).unwrap();
        assert_eq!(
            super::super::session::restore_owner_for_test(&path, &parent, [2; 32]),
            Err(RangeError::OwnershipMarker)
        );
        assert_eq!(fs::read(path.join(".owner")).unwrap(), foreign);
    })
    .unwrap_err();
    assert_eq!(pending.error(), RangeError::OwnershipMarker);
    let pending = pending.retry().unwrap_err();
    assert_eq!(pending.error(), RangeError::OwnershipMarker);
    assert_eq!(fs::read(path.join(".owner")).unwrap(), foreign);
    assert!(path.join("foreign-sentinel").exists());
    assert_eq!(quota.charged_usage(), (28, 1));
}
