use super::*;
use ed25519_dalek::SigningKey;

struct Snapshot;
impl QuotaSnapshotAuthority for Snapshot {
    fn admit_usage(&self, _: &Path, _: QuotaLimits, snapshot: QuotaSnapshot) -> bool {
        snapshot.id == [1; 32]
    }
}

fn snapshot() -> QuotaSnapshot {
    QuotaSnapshot {
        id: [1; 32],
        encoded_bytes: 10,
        entries: 1,
    }
}

#[test]
fn admission_rejects_capacity_duplicate_and_scope_without_mutation() {
    let root = std::env::temp_dir();
    let mut quota = CustodyQuota::<2>::from_authenticated_usage(
        &root,
        QuotaLimits {
            encoded_bytes: 30,
            entries: 3,
        },
        snapshot(),
        &Snapshot,
    )
    .unwrap();
    quota.charge(&root, [2; 32], 10).unwrap();
    assert_eq!(
        quota.charge(&root, [2; 32], 1),
        Err(QuotaError::AlreadyCharged)
    );
    assert_eq!(
        quota.charge(&root.join("other"), [3; 32], 1),
        Err(QuotaError::ScopeMismatch)
    );
    assert_eq!(
        quota.charge(&root, [3; 32], 11),
        Err(QuotaError::BytesExceeded)
    );
    assert_eq!(quota.charged_usage(), (20, 2));
    quota.charge(&root, [3; 32], 10).unwrap();
    assert_eq!(
        quota.charge(&root, [4; 32], 1),
        Err(QuotaError::SlotsExhausted)
    );
}

#[test]
fn actual_u64_byte_and_entry_overflow_leave_totals_and_slots_unchanged() {
    let root = std::env::temp_dir();
    let limits = QuotaLimits {
        encoded_bytes: u64::MAX,
        entries: u64::MAX,
    };
    let mut bytes = CustodyQuota::<1>::from_authenticated_usage(
        &root,
        limits,
        QuotaSnapshot {
            id: [1; 32],
            encoded_bytes: u64::MAX,
            entries: 1,
        },
        &Snapshot,
    )
    .unwrap();
    assert_eq!(bytes.charge(&root, [2; 32], 1), Err(QuotaError::Overflow));
    assert_eq!(bytes.charged_usage(), (u64::MAX, 1));
    assert_eq!(bytes.charged, [None]);
    let mut entries = CustodyQuota::<1>::from_authenticated_usage(
        &root,
        limits,
        QuotaSnapshot {
            id: [1; 32],
            encoded_bytes: 10,
            entries: u64::MAX,
        },
        &Snapshot,
    )
    .unwrap();
    assert_eq!(entries.charge(&root, [2; 32], 1), Err(QuotaError::Overflow));
    assert_eq!(entries.charged_usage(), (10, u64::MAX));
    assert_eq!(entries.charged, [None]);
}

#[test]
fn exhausted_entries_and_untrusted_snapshot_are_rejected() {
    let root = std::env::temp_dir();
    let limits = QuotaLimits {
        encoded_bytes: 30,
        entries: 1,
    };
    let mut quota =
        CustodyQuota::<1>::from_authenticated_usage(&root, limits, snapshot(), &Snapshot).unwrap();
    assert_eq!(
        quota.charge(&root, [2; 32], 1),
        Err(QuotaError::EntriesExceeded)
    );
    assert_eq!(quota.charged_usage(), (10, 1));
    let mut invalid = snapshot();
    invalid.id = [2; 32];
    assert!(matches!(
        CustodyQuota::<1>::from_authenticated_usage(&root, limits, invalid, &Snapshot),
        Err(QuotaError::SnapshotDenied)
    ));
}

struct DenyOnSecondCheck {
    calls: usize,
}
impl super::super::SidecarReceiptAuthority for DenyOnSecondCheck {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for DenyOnSecondCheck {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        self.calls += 1;
        self.calls == 1
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

#[test]
fn late_denial_retains_charge_before_any_storage_creation() {
    let root = std::env::temp_dir();
    let sidecar = [7; SIDECAR_BYTES];
    let payload = b"p";
    let receipt = SidecarReceipt::sign(
        &sidecar,
        payload,
        1,
        [0; 32],
        [3; 32],
        0,
        None,
        &SigningKey::from_bytes(&[4; 32]),
    )
    .unwrap();
    let total = entry_size(payload.len(), 4096).unwrap();
    let mut quota = CustodyQuota::<1>::from_authenticated_usage(
        &root,
        QuotaLimits {
            encoded_bytes: 4096,
            entries: 5,
        },
        snapshot(),
        &Snapshot,
    )
    .unwrap();
    let mut authority = DenyOnSecondCheck { calls: 0 };
    let result = publish_bundle_with_quota(
        &root,
        &sidecar,
        payload,
        &receipt,
        4096,
        &mut quota,
        &mut authority,
    );
    assert!(matches!(
        result,
        Err(QuotaPublicationError::AttemptCharged {
            error: CustodyError::PermissionDenied,
            ..
        })
    ));
    assert_eq!(quota.charged_usage(), (10 + total as u64, 2));
    assert_eq!(authority.calls, 2);
}

struct PanicOnSecondCheck {
    calls: usize,
}
impl super::super::SidecarReceiptAuthority for PanicOnSecondCheck {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for PanicOnSecondCheck {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        self.calls += 1;
        assert_ne!(
            self.calls, 2,
            "injected authority unwind before storage creation"
        );
        true
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

#[test]
fn callback_unwind_retains_exact_charge_and_blocks_retry() {
    let root = std::env::temp_dir();
    let sidecar = [7; SIDECAR_BYTES];
    let payload = b"p";
    let receipt = SidecarReceipt::sign(
        &sidecar,
        payload,
        1,
        [0; 32],
        [3; 32],
        0,
        None,
        &SigningKey::from_bytes(&[4; 32]),
    )
    .unwrap();
    let total = entry_size(payload.len(), 4096).unwrap();
    let mut quota = CustodyQuota::<1>::from_authenticated_usage(
        &root,
        QuotaLimits {
            encoded_bytes: 4096,
            entries: 5,
        },
        snapshot(),
        &Snapshot,
    )
    .unwrap();
    let mut authority = PanicOnSecondCheck { calls: 0 };
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = publish_bundle_with_quota(
            &root,
            &sidecar,
            payload,
            &receipt,
            4096,
            &mut quota,
            &mut authority,
        );
    }));
    assert!(unwound.is_err());
    assert_eq!(authority.calls, 2);
    assert_eq!(quota.charged_usage(), (10 + total as u64, 2));
    assert_eq!(quota.charged, [Some(receipt.checkpoint())]);
    assert_eq!(
        quota.charge(&root, receipt.checkpoint(), total),
        Err(QuotaError::AlreadyCharged)
    );
}

struct UnavailableHead {
    publish_checks: usize,
    head_checks: usize,
}
impl super::super::SidecarReceiptAuthority for UnavailableHead {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for UnavailableHead {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        self.publish_checks += 1;
        true
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}
impl ChainAuthority for UnavailableHead {
    fn authenticated_head(
        &mut self,
        _: &Path,
        _: &[u8; 32],
    ) -> Result<super::super::chain::ChainHead, super::super::chain::ChainAuthorityError> {
        self.head_checks += 1;
        Err(super::super::chain::ChainAuthorityError::Unavailable)
    }
    fn compare_and_commit(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: super::super::chain::ChainHead,
        _: super::super::chain::ChainHead,
        _: &SidecarReceipt,
    ) -> super::super::chain::HeadCommit {
        panic!("unavailable head must stop before publication and commit")
    }
}

#[test]
fn unavailable_chain_head_retains_charge_before_producer_io() {
    let root = std::env::temp_dir();
    let sidecar = [7; SIDECAR_BYTES];
    let payload = b"p";
    let receipt = SidecarReceipt::sign(
        &sidecar,
        payload,
        1,
        [0; 32],
        [3; 32],
        0,
        None,
        &SigningKey::from_bytes(&[4; 32]),
    )
    .unwrap();
    let total = entry_size(payload.len(), 4096).unwrap();
    let mut quota = CustodyQuota::<1>::from_authenticated_usage(
        &root,
        QuotaLimits {
            encoded_bytes: 4096,
            entries: 5,
        },
        snapshot(),
        &Snapshot,
    )
    .unwrap();
    let mut authority = UnavailableHead {
        publish_checks: 0,
        head_checks: 0,
    };
    let result = publish_chain_bundle_with_quota(
        &root,
        &sidecar,
        payload,
        &receipt,
        4096,
        &mut quota,
        &mut authority,
    );
    match result {
        Err(QuotaPublicationError::AttemptCharged {
            checkpoint,
            bytes,
            error:
                ChainPublicationError::Authority(super::super::chain::ChainAuthorityError::Unavailable),
        }) => {
            assert_eq!(checkpoint, receipt.checkpoint());
            assert_eq!(bytes, total);
        }
        other => panic!("expected a retained charge with the exact head error, got {other:?}"),
    }
    assert_eq!((authority.publish_checks, authority.head_checks), (2, 1));
    assert_eq!(quota.charged_usage(), (10 + total as u64, 2));
    assert_eq!(quota.charged, [Some(receipt.checkpoint())]);
}
