use super::{binding, observation, Authority};
use crate::{
    bridge_observation,
    read_store::{read_stored_observation, StoredReadAuthority, StoredReadError},
    sidecar::{encode_sidecar, UntrustedSidecar, SIDECAR_SIZE},
};
use ed25519_dalek::SigningKey;
use qualia_core_db::sensing::{
    local_custody::{publish_bundle, CustodyAuthority, CustodyError},
    record::ResolvedFieldPayloadBinding,
    sidecar_attestation::{
        SidecarReceipt, SidecarReceiptAuthority, SidecarReceiptError, SIDECAR_BYTES,
    },
};
use std::{cell::Cell, path::Path};

const BUDGET: usize = 16 + SIDECAR_SIZE + 272 + 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Allow,
    DenyDecoded,
    PanicDecoded,
    RevokeReceipt,
    RevokeRead,
}

struct ReadPolicy {
    mode: Mode,
    decoded_calls: usize,
    receipt_checks: Cell<usize>,
    read_checks: usize,
}

impl ReadPolicy {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            decoded_calls: 0,
            receipt_checks: Cell::new(0),
            read_checks: 0,
        }
    }
}

impl SidecarReceiptAuthority for ReadPolicy {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.receipt_checks.set(self.receipt_checks.get() + 1);
        !(self.mode == Mode::RevokeReceipt && self.decoded_calls != 0)
    }
}

impl CustodyAuthority for ReadPolicy {
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
        self.read_checks += 1;
        !(self.mode == Mode::RevokeRead && self.decoded_calls != 0)
    }
}

impl StoredReadAuthority for ReadPolicy {
    fn permit_decoded(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: &ResolvedFieldPayloadBinding,
        _: &SidecarReceipt,
        decoded: &UntrustedSidecar,
        _: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.decoded_calls += 1;
        assert_eq!(payload, &[5; 16]);
        assert_eq!(decoded.record().raw_payload().length(), 16);
        if self.mode == Mode::PanicDecoded {
            panic!("decoded policy panic");
        }
        self.mode != Mode::DenyDecoded
    }
}

fn canonical_sidecar() -> [u8; SIDECAR_SIZE] {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut sidecar = [0; SIDECAR_SIZE];
    encode_sidecar(&value, &mut sidecar).unwrap();
    sidecar
}

fn fixture(sidecar: &[u8; SIDECAR_SIZE], payload: &[u8]) -> (tempfile::TempDir, [u8; 32]) {
    let root = tempfile::tempdir().unwrap();
    let signer = SigningKey::from_bytes(&[11; 32]);
    let receipt =
        SidecarReceipt::sign(sidecar, payload, 32, [1; 32], [2; 32], 0, None, &signer).unwrap();
    let published = publish_bundle(
        root.path(),
        sidecar,
        payload,
        &receipt,
        BUDGET,
        &mut ReadPolicy::new(Mode::Allow),
    )
    .unwrap();
    (root, published.checkpoint)
}

#[test]
fn stored_read_preserves_payload_and_clears_unused_tail_on_success() {
    let (root, checkpoint) = fixture(&canonical_sidecar(), &[5; 16]);
    let mut policy = ReadPolicy::new(Mode::Allow);
    let mut output = [0xaa; 32];
    let read = read_stored_observation(
        root.path(),
        &checkpoint,
        BUDGET,
        binding(),
        &mut output,
        &mut policy,
    )
    .unwrap();
    assert_eq!(read.payload_bytes, 16);
    assert_eq!(&output[..16], &[5; 16]);
    assert_eq!(&output[16..], &[0; 16]);
    assert_eq!(policy.decoded_calls, 1);
    assert_eq!(policy.receipt_checks.get(), 2);
    assert_eq!(policy.read_checks, 3);
}

#[test]
fn decoded_policy_denial_clears_entire_output_after_verified_read() {
    let (root, checkpoint) = fixture(&canonical_sidecar(), &[5; 16]);
    let mut policy = ReadPolicy::new(Mode::DenyDecoded);
    let mut output = [0xaa; 32];
    assert!(matches!(
        read_stored_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            binding(),
            &mut output,
            &mut policy
        ),
        Err(StoredReadError::NotAuthorized)
    ));
    assert_eq!(policy.receipt_checks.get(), 1);
    assert_eq!(policy.decoded_calls, 1);
    assert_eq!(output, [0; 32]);
}

#[test]
fn decoded_policy_panic_clears_entire_output_on_unwind() {
    let (root, checkpoint) = fixture(&canonical_sidecar(), &[5; 16]);
    let mut policy = ReadPolicy::new(Mode::PanicDecoded);
    let mut output = [0xaa; 32];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_stored_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            binding(),
            &mut output,
            &mut policy,
        )
    }));
    assert!(result.is_err());
    assert_eq!(policy.decoded_calls, 1);
    assert_eq!(output, [0; 32]);
}

#[test]
fn post_decode_receipt_or_read_revocation_clears_entire_output() {
    let (root, checkpoint) = fixture(&canonical_sidecar(), &[5; 16]);
    for mode in [Mode::RevokeReceipt, Mode::RevokeRead] {
        let mut policy = ReadPolicy::new(mode);
        let mut output = [0xaa; 32];
        let result = read_stored_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            binding(),
            &mut output,
            &mut policy,
        );
        match mode {
            Mode::RevokeReceipt => assert!(matches!(
                result,
                Err(StoredReadError::Custody(CustodyError::Receipt(
                    SidecarReceiptError::NotAuthorized
                )))
            )),
            Mode::RevokeRead => assert!(matches!(result, Err(StoredReadError::NotAuthorized))),
            _ => unreachable!(),
        }
        assert_eq!(policy.decoded_calls, 1);
        assert_eq!(policy.receipt_checks.get(), 2);
        assert_eq!(output, [0; 32]);
    }
}

#[test]
fn signed_payload_length_or_resolved_binding_mismatch_clears_output() {
    let sidecar = canonical_sidecar();
    let (root, checkpoint) = fixture(&sidecar, &[5; 15]);
    let mut policy = ReadPolicy::new(Mode::Allow);
    let mut output = [0xaa; 32];
    assert!(matches!(
        read_stored_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            binding(),
            &mut output,
            &mut policy
        ),
        Err(StoredReadError::NotAuthorized)
    ));
    assert_eq!(policy.receipt_checks.get(), 1);
    assert_eq!(policy.decoded_calls, 0);
    assert_eq!(output, [0; 32]);

    // Re-sign a canonical-looking index with a different payload container.
    // Cryptographic byte validity cannot substitute for the resolved binding.
    let mut altered = sidecar;
    altered[86] ^= 1; // Sidecar header (8) + index payload reference (78).
    let (root, checkpoint) = fixture(&altered, &[5; 16]);
    let mut policy = ReadPolicy::new(Mode::Allow);
    output.fill(0xaa);
    assert!(matches!(
        read_stored_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            binding(),
            &mut output,
            &mut policy
        ),
        Err(StoredReadError::Sidecar(_))
    ));
    assert_eq!(policy.receipt_checks.get(), 1);
    assert_eq!(policy.decoded_calls, 0);
    assert_eq!(output, [0; 32]);
}
