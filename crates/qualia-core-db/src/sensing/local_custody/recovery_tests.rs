use super::*;
use ed25519_dalek::SigningKey;

fn receipt(sequence: u64) -> SidecarReceipt {
    SidecarReceipt::sign(
        &[9; SIDECAR_BYTES],
        b"data",
        4,
        [0; 32],
        [7; 32],
        sequence,
        if sequence == 0 { None } else { Some([8; 32]) },
        &SigningKey::from_bytes(&[3; 32]),
    )
    .unwrap()
}

#[test]
fn current_head_is_distinct_from_signed_but_unproven_bundle() {
    let record = receipt(0);
    let head = ChainHead::Present {
        sequence: 0,
        checkpoint: record.checkpoint(),
    };
    assert_eq!(
        classify(head, &record, MembershipAssertion::Unknown).unwrap(),
        RecoveryDisposition::CommittedAtHead
    );
    assert_eq!(
        classify(ChainHead::Empty, &record, MembershipAssertion::Unknown).unwrap(),
        RecoveryDisposition::Uncertain
    );
    assert_eq!(
        classify(ChainHead::Empty, &record, MembershipAssertion::Excluded).unwrap(),
        RecoveryDisposition::PublishedUncommitted
    );
}

#[test]
fn ancestry_needs_host_proof_and_strictly_earlier_sequence() {
    let record = receipt(1);
    let later = ChainHead::Present {
        sequence: 2,
        checkpoint: [2; 32],
    };
    assert_eq!(
        classify(later, &record, MembershipAssertion::Unknown).unwrap(),
        RecoveryDisposition::Uncertain
    );
    assert_eq!(
        classify(later, &record, MembershipAssertion::CommittedAncestor).unwrap(),
        RecoveryDisposition::CommittedAncestor
    );
    assert!(matches!(
        classify(
            ChainHead::Empty,
            &record,
            MembershipAssertion::CommittedAncestor
        ),
        Err(RecoveryError::InvalidMembership)
    ));
    assert!(matches!(
        classify(
            ChainHead::Present {
                sequence: 1,
                checkpoint: [2; 32]
            },
            &record,
            MembershipAssertion::CommittedAncestor
        ),
        Err(RecoveryError::InvalidMembership)
    ));
    assert!(matches!(
        classify(
            ChainHead::Present {
                sequence: 2,
                checkpoint: [0; 32]
            },
            &record,
            MembershipAssertion::Unknown
        ),
        Err(RecoveryError::InvalidHead)
    ));
}

struct Deny;
impl super::super::SidecarReceiptAuthority for Deny {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        panic!("unexpected verification")
    }
}
impl CustodyAuthority for Deny {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        false
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        panic!("unexpected read permission")
    }
}
impl RecoveryAuthority for Deny {
    fn permit_recovery(&mut self, _: &Path, _: &[u8; 32], _: &[u8; 32], _: usize) -> bool {
        false
    }
    fn recovery_head(&mut self, _: &Path, _: &[u8; 32]) -> Result<ChainHead, ChainAuthorityError> {
        panic!("unauthorized head lookup")
    }
    fn membership(
        &mut self,
        _: &Path,
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError> {
        panic!("unexpected membership lookup")
    }
}

#[test]
fn denied_recovery_never_looks_up_head_and_clears_output() {
    let mut sidecar = [0xff; SIDECAR_BYTES];
    let mut payload = [0xff; 8];
    let result = inspect_recovery_bundle(
        &std::env::temp_dir(),
        &[7; 32],
        &[1; 32],
        super::super::FIXED_BYTES + 8,
        &mut sidecar,
        &mut payload,
        &mut Deny,
    );
    assert!(matches!(
        result,
        Err(RecoveryError::Custody(CustodyError::PermissionDenied))
    ));
    assert_eq!(sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(payload, [0; 8]);
}

#[derive(Clone, Copy)]
enum LateFailure {
    HeadChanged,
    ReadDenied,
    RecoveryDenied,
    MembershipPanic,
}

struct RecoveryFixture {
    failure: LateFailure,
    read_calls: usize,
    recovery_calls: usize,
    head_calls: usize,
    verified: core::cell::Cell<usize>,
}

impl super::super::SidecarReceiptAuthority for RecoveryFixture {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.verified.set(self.verified.get() + 1);
        true
    }
}

impl CustodyAuthority for RecoveryFixture {
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
        self.read_calls += 1;
        !(matches!(self.failure, LateFailure::ReadDenied) && self.read_calls == 4)
    }
}

impl RecoveryAuthority for RecoveryFixture {
    fn permit_recovery(&mut self, _: &Path, _: &[u8; 32], _: &[u8; 32], _: usize) -> bool {
        self.recovery_calls += 1;
        !(matches!(self.failure, LateFailure::RecoveryDenied) && self.recovery_calls == 2)
    }
    fn recovery_head(&mut self, _: &Path, _: &[u8; 32]) -> Result<ChainHead, ChainAuthorityError> {
        self.head_calls += 1;
        let sequence = if matches!(self.failure, LateFailure::HeadChanged) && self.head_calls == 2 {
            3
        } else {
            2
        };
        Ok(ChainHead::Present {
            sequence,
            checkpoint: [5; 32],
        })
    }
    fn membership(
        &mut self,
        _: &Path,
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError> {
        if matches!(self.failure, LateFailure::MembershipPanic) {
            panic!("membership authority failed after read");
        }
        Ok(MembershipAssertion::CommittedAncestor)
    }
}

fn published_fixture(failure: LateFailure) -> (tempfile::TempDir, [u8; 32], RecoveryFixture) {
    let dir = tempfile::tempdir().unwrap();
    let record = receipt(0);
    let mut authority = RecoveryFixture {
        failure,
        read_calls: 0,
        recovery_calls: 0,
        head_calls: 0,
        verified: core::cell::Cell::new(0),
    };
    super::super::publish_bundle(
        dir.path(),
        &[9; SIDECAR_BYTES],
        b"data",
        &record,
        super::super::FIXED_BYTES + 4,
        &mut authority,
    )
    .unwrap();
    authority.verified.set(0);
    (dir, record.checkpoint(), authority)
}

#[test]
fn head_change_after_verified_read_clears_both_outputs() {
    let (dir, checkpoint, mut authority) = published_fixture(LateFailure::HeadChanged);
    let mut sidecar = [0xff; SIDECAR_BYTES];
    let mut payload = [0xff; 8];
    let result = inspect_recovery_bundle(
        dir.path(),
        &[7; 32],
        &checkpoint,
        super::super::FIXED_BYTES + 4,
        &mut sidecar,
        &mut payload,
        &mut authority,
    );
    assert!(matches!(result, Err(RecoveryError::HeadChanged)));
    assert_eq!(authority.head_calls, 2);
    assert_eq!(authority.verified.get(), 1);
    assert_eq!(sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(payload, [0; 8]);
}

#[test]
fn late_read_and_recovery_denial_clear_successfully_read_bytes() {
    for failure in [LateFailure::ReadDenied, LateFailure::RecoveryDenied] {
        let (dir, checkpoint, mut authority) = published_fixture(failure);
        let mut sidecar = [0xff; SIDECAR_BYTES];
        let mut payload = [0xff; 8];
        let result = inspect_recovery_bundle(
            dir.path(),
            &[7; 32],
            &checkpoint,
            super::super::FIXED_BYTES + 4,
            &mut sidecar,
            &mut payload,
            &mut authority,
        );
        assert!(matches!(
            result,
            Err(RecoveryError::Custody(CustodyError::PermissionDenied))
        ));
        assert_eq!(authority.verified.get(), 2);
        assert_eq!(authority.recovery_calls, 2);
        assert_eq!(sidecar, [0; SIDECAR_BYTES]);
        assert_eq!(payload, [0; 8]);
    }
}

#[test]
fn membership_unwind_clears_bytes_after_successful_bundle_read() {
    let (dir, checkpoint, mut authority) = published_fixture(LateFailure::MembershipPanic);
    let mut sidecar = [0xff; SIDECAR_BYTES];
    let mut payload = [0xff; 8];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        inspect_recovery_bundle(
            dir.path(),
            &[7; 32],
            &checkpoint,
            super::super::FIXED_BYTES + 4,
            &mut sidecar,
            &mut payload,
            &mut authority,
        )
    }));
    assert!(result.is_err());
    assert_eq!(authority.verified.get(), 1);
    assert_eq!(sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(payload, [0; 8]);
}
