use super::*;
use ed25519_dalek::SigningKey;

fn signed(sequence: u64, previous: Option<[u8; 32]>) -> SidecarReceipt {
    SidecarReceipt::sign(
        &[1; SIDECAR_BYTES],
        b"payload",
        7,
        [0; 32],
        [4; 32],
        sequence,
        previous,
        &SigningKey::from_bytes(&[3; 32]),
    )
    .unwrap()
}

struct DenyBeforeHead;
impl super::super::SidecarReceiptAuthority for DenyBeforeHead {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for DenyBeforeHead {
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
        false
    }
}
impl ChainAuthority for DenyBeforeHead {
    fn authenticated_head(
        &mut self,
        _: &Path,
        _: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        panic!("unauthorized head lookup")
    }
    fn compare_and_commit(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> HeadCommit {
        panic!("unauthorized commit")
    }
}

#[test]
fn denied_store_never_looks_up_chain_head() {
    let receipt = signed(0, None);
    let result = publish_chain_bundle(
        &std::env::temp_dir(),
        &[1; SIDECAR_BYTES],
        b"payload",
        &receipt,
        super::super::FIXED_BYTES + 7,
        &mut DenyBeforeHead,
    );
    assert!(matches!(
        result,
        Err(ChainPublicationError::Publication(
            CustodyError::PermissionDenied
        ))
    ));
}

#[test]
fn genesis_and_next_head_bind_sequence_and_previous_checkpoint() {
    let genesis = signed(0, None);
    let head = expected_transition(ChainHead::Empty, &genesis).unwrap();
    assert_eq!(
        head,
        ChainHead::Present {
            sequence: 0,
            checkpoint: genesis.checkpoint()
        }
    );
    let next = signed(1, Some(genesis.checkpoint()));
    assert_eq!(
        expected_transition(head, &next).unwrap(),
        ChainHead::Present {
            sequence: 1,
            checkpoint: next.checkpoint()
        }
    );
}

#[test]
fn gaps_stale_heads_and_wrong_predecessors_fail_closed() {
    let genesis = signed(0, None);
    let current = ChainHead::Present {
        sequence: 0,
        checkpoint: genesis.checkpoint(),
    };
    let gap = signed(2, Some(genesis.checkpoint()));
    assert!(matches!(
        expected_transition(current, &gap),
        Err(ChainPublicationError::WrongSequence)
    ));
    let wrong = signed(1, Some([8; 32]));
    assert!(matches!(
        expected_transition(current, &wrong),
        Err(ChainPublicationError::WrongPredecessor)
    ));
    assert!(matches!(
        expected_transition(ChainHead::Empty, &wrong),
        Err(ChainPublicationError::WrongSequence)
    ));
    assert!(matches!(
        expected_transition(current, &genesis),
        Err(ChainPublicationError::WrongSequence)
    ));
}

#[test]
fn invalid_or_exhausted_host_head_fails_closed() {
    let receipt = signed(1, Some([8; 32]));
    assert!(matches!(
        expected_transition(
            ChainHead::Present {
                sequence: 0,
                checkpoint: [0; 32]
            },
            &receipt
        ),
        Err(ChainPublicationError::InvalidHead)
    ));
    assert!(matches!(
        expected_transition(
            ChainHead::Present {
                sequence: u64::MAX,
                checkpoint: [8; 32]
            },
            &receipt
        ),
        Err(ChainPublicationError::SequenceExhausted)
    ));
}
