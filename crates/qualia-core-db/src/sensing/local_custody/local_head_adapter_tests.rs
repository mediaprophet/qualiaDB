use super::super::chain::{publish_chain_bundle, ChainPublicationError};
use super::*;
use ed25519_dalek::SigningKey;

struct TestPolicy;
impl SidecarReceiptAuthority for TestPolicy {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for TestPolicy {
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
impl LocalHeadPolicy for TestPolicy {
    fn permit_open(&mut self, _: &Path, _: &[u8; 32]) -> bool {
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

#[test]
fn scoped_publication_advances_signed_head() {
    let directory = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let sidecar = [9; SIDECAR_BYTES];
    let payload = b"abc";
    let chain = [4; 32];
    let receipt =
        SidecarReceipt::sign(&sidecar, payload, 3, [0; 32], chain, 0, None, &key).unwrap();
    let mut policy = TestPolicy;
    let mut lease =
        LocalHeadLease::acquire(directory.path(), chain, &key, 176, &mut policy).unwrap();
    let mut authority =
        LocalChainAuthority::new(&mut lease, &mut policy, &receipt, &sidecar, payload, 3).unwrap();
    let result = publish_chain_bundle(
        directory.path(),
        &sidecar,
        payload,
        &receipt,
        1059,
        &mut authority,
    );
    assert!(
        result.is_ok()
            || matches!(
                result,
                Err(ChainPublicationError::PublishedUncommitted {
                    outcome: HeadCommit::Indeterminate,
                    ..
                })
            )
    );
    drop(authority);
    assert_eq!(
        lease.current(&mut policy).unwrap(),
        ChainHead::Present {
            sequence: 0,
            checkpoint: receipt.checkpoint()
        }
    );
}

#[test]
fn receipt_and_payload_are_bound_to_exact_adapter() {
    let directory = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let sidecar = [9; SIDECAR_BYTES];
    let chain = [4; 32];
    let receipt = SidecarReceipt::sign(&sidecar, b"abc", 3, [0; 32], chain, 0, None, &key).unwrap();
    let mut policy = TestPolicy;
    let mut lease =
        LocalHeadLease::acquire(directory.path(), chain, &key, 176, &mut policy).unwrap();
    let mut authority =
        LocalChainAuthority::new(&mut lease, &mut policy, &receipt, &sidecar, b"abc", 3).unwrap();
    assert!(!authority.permit_publish(directory.path(), &receipt, &sidecar, b"xyz", 1059));
    assert_eq!(
        authority.compare_and_commit(
            directory.path(),
            &chain,
            ChainHead::Empty,
            ChainHead::Present {
                sequence: 0,
                checkpoint: [1; 32]
            },
            &receipt
        ),
        HeadCommit::Conflict
    );
}
