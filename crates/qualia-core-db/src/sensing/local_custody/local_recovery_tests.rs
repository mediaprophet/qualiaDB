use super::super::{publish_bundle, FIXED_BYTES};
use super::*;
use ed25519_dalek::SigningKey;

struct Policy {
    recovery: bool,
    open_calls: usize,
}
impl SidecarReceiptAuthority for Policy {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl LocalHeadPolicy for Policy {
    fn permit_open(&mut self, _: &Path, _: &[u8; 32]) -> bool {
        self.open_calls += 1;
        true
    }
    fn admit_current(&mut self, _: &Path, _: &[u8; 32], _: ChainHead) -> bool {
        true
    }
    fn permit_commit(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: ChainHead,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
    ) -> bool {
        true
    }
}
impl CustodyAuthority for Policy {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        true
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        true
    }
}
impl LocalRecoveryPolicy for Policy {
    fn permit_recovery_read(&mut self, _: &Path, _: &[u8; 32], _: &[u8; 32], _: usize) -> bool {
        self.recovery
    }
    fn verify_recovery_membership(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError> {
        Ok(MembershipAssertion::Unknown)
    }
}

#[test]
fn denial_precedes_signed_head_lookup_and_clears_outputs() {
    let root = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let chain = [2; 32];
    let mut policy = Policy {
        recovery: false,
        open_calls: 0,
    };
    let mut lease = LocalHeadLease::acquire(root.path(), chain, &key, 176, &mut policy).unwrap();
    assert_eq!(policy.open_calls, 1);
    let mut sidecar = [0xff; SIDECAR_BYTES];
    let mut payload = [0xff; 3];
    let result = inspect_local_recovery(
        root.path(),
        &chain,
        &[3; 32],
        FIXED_BYTES + 3,
        &mut sidecar,
        &mut payload,
        &mut lease,
        &mut policy,
    );
    assert!(matches!(
        result,
        Err(RecoveryError::Custody(
            super::super::CustodyError::PermissionDenied
        ))
    ));
    assert_eq!(policy.open_calls, 1);
    assert_eq!(sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(payload, [0; 3]);
}

#[test]
fn signed_local_head_drives_exact_checkpoint_recovery() {
    let root = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let chain = [2; 32];
    let sidecar = [9; SIDECAR_BYTES];
    let payload = b"xyz";
    let receipt =
        SidecarReceipt::sign(&sidecar, payload, 3, [0; 32], chain, 0, None, &key).unwrap();
    let mut policy = Policy {
        recovery: true,
        open_calls: 0,
    };
    let mut lease = LocalHeadLease::acquire(root.path(), chain, &key, 176, &mut policy).unwrap();
    publish_bundle(
        root.path(),
        &sidecar,
        payload,
        &receipt,
        FIXED_BYTES + 3,
        &mut policy,
    )
    .unwrap();
    // Windows may report Indeterminate for directory durability. Recovery reads
    // actual signed state under the fixture's explicitly permissive authority.
    let _outcome = lease
        .compare_and_commit(
            ChainHead::Empty,
            &receipt,
            &sidecar,
            payload,
            3,
            &mut policy,
        )
        .unwrap();
    let mut read_sidecar = [0; SIDECAR_BYTES];
    let mut read_payload = [0; 3];
    let report = inspect_local_recovery(
        root.path(),
        &chain,
        &receipt.checkpoint(),
        FIXED_BYTES + 3,
        &mut read_sidecar,
        &mut read_payload,
        &mut lease,
        &mut policy,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        super::super::recovery::RecoveryDisposition::CommittedAtHead
    );
    assert_eq!(read_sidecar, sidecar);
    assert_eq!(&read_payload, payload);
}
