use super::super::authorization::{
    self, IdentityAuthority, RevocationAuthority, SensingAction, SensingModality, SensingRequest,
    SensingScopeAuthority, TrustedUnixClock,
};
use crate::governance::consent_contract::{ConsentGrant, ConsentScope};
use core::num::NonZeroU64;
use ed25519_dalek::SigningKey;

use super::*;

fn record(value: &[u8]) -> NfcEvidenceRecord {
    let identifier = VariableRadioIdentifier::new(
        VariableRadioIdentifierKind::NfcUid,
        ProtocolIdentifier::Unknown,
        value,
    )
    .unwrap();
    NfcEvidenceRecord::new(
        17,
        identifier,
        3,
        19,
        Some(5),
        Some(2),
        Some(44),
        RadioObservationTime::Unknown,
        RadioEvidenceReference::Unknown,
        None,
    )
    .unwrap()
}

#[test]
fn weak_signer_key_is_rejected() {
    let observed = record(&[1, 2, 0]);
    let mut scratch = [0u8; NFC_EVIDENCE_RECORD_SIZE];
    let mut receipt = NfcCustodyReceipt::sign(
        observed,
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::Unknown,
        &SigningKey::from_bytes(&[3; 32]),
    )
    .unwrap();
    receipt.signer_key = [0; 32];
    receipt.signer_key[0] = 1;
    receipt.signature = [0; 64];
    receipt.signature[0] = 1;
    assert!(matches!(
        receipt.verify_signature(observed, &mut scratch),
        Err(NfcEvidenceError::InvalidSignerKey | NfcEvidenceError::InvalidSignature)
    ));
}

struct Accept {
    key: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
}

impl NfcReceiptAuthority for Accept {
    fn signer_authorized(&self, source_id: u64, key: &[u8; 32]) -> bool {
        source_id == 3 && key == &self.key
    }
    fn permission_context_valid(&self, record: &NfcEvidenceRecord) -> bool {
        record.permission().is_none()
    }
    fn chain_position_valid(
        &self,
        sequence: u64,
        previous: Option<&[u8; 32]>,
        timestamp: ReceiptTimestamp,
        current: &[u8; 32],
    ) -> bool {
        sequence == self.sequence
            && previous.copied() == self.previous
            && timestamp != ReceiptTimestamp::Unknown
            && current.iter().any(|byte| *byte != 0)
    }
}

struct Identity;
impl IdentityAuthority for Identity {
    fn verify_parties(
        &self,
        _grant: &ConsentGrant,
        _actor: &[u8; 32],
    ) -> Result<(), authorization::AuthorizationError> {
        Ok(())
    }
}
struct Scope;
impl SensingScopeAuthority for Scope {
    fn verify_scope(
        &self,
        _grant: &ConsentGrant,
        _digest: &[u8; 32],
        _request: &SensingRequest,
    ) -> Result<(), authorization::AuthorizationError> {
        Ok(())
    }
}
struct Revocation;
impl RevocationAuthority for Revocation {
    fn is_revoked(
        &self,
        _id: &[u8; 32],
        _digest: &[u8; 32],
        _now: u64,
    ) -> Result<bool, authorization::AuthorizationError> {
        Ok(false)
    }
}
struct Clock(u64);
impl TrustedUnixClock for Clock {
    fn now_unix(&mut self) -> Result<u64, authorization::AuthorizationError> {
        Ok(self.0)
    }
}

fn permission(modality: SensingModality) -> RadioPermissionContext {
    let actor = [8; 32];
    let signer = SigningKey::from_bytes(&[31; 32]);
    let grant = ConsentGrant::new_signed(
        [7; 32],
        &signer,
        actor,
        ConsentScope(ConsentScope::ALL),
        77,
        10,
        100,
        1,
    );
    let request = SensingRequest::new(actor, 3, modality, SensingAction::Acquire, 77).unwrap();
    let initial = authorization::authorize_sensing(
        &grant,
        request,
        &Identity,
        &Scope,
        &Revocation,
        &mut Clock(40),
    )
    .unwrap();
    let revalidated = initial
        .revalidate(&grant, &Identity, &Scope, &Revocation, &mut Clock(50))
        .unwrap();
    RadioPermissionContext::from_revalidated(&revalidated).unwrap()
}

#[test]
fn identifiers_preserve_exact_length_and_trailing_zero_bytes() {
    for len in [4usize, 7, 10, NFC_IDENTIFIER_MAX_LEN] {
        let mut bytes = [0x5au8; NFC_IDENTIFIER_MAX_LEN];
        bytes[len - 1] = 0;
        let id = VariableRadioIdentifier::new(
            VariableRadioIdentifierKind::NfcUid,
            ProtocolIdentifier::Known(NonZeroU64::new(0x123).unwrap()),
            &bytes[..len],
        )
        .unwrap();
        assert_eq!(id.len(), len);
        assert_eq!(id.bytes(), &bytes[..len]);
    }
    assert_eq!(
        VariableRadioIdentifier::new(
            VariableRadioIdentifierKind::NfcUid,
            ProtocolIdentifier::Unknown,
            &[]
        ),
        Err(NfcEvidenceError::InvalidIdentifierLength),
    );
    assert_eq!(
        VariableRadioIdentifier::new(
            VariableRadioIdentifierKind::NfcUid,
            ProtocolIdentifier::Unknown,
            &[1; 33]
        ),
        Err(NfcEvidenceError::InvalidIdentifierLength),
    );
}

#[test]
fn canonical_record_keeps_unknowns_and_has_bounded_atomic_output() {
    let record = record(&[0x04, 0x25, 0, 0]);
    let mut out = [0xa5u8; NFC_EVIDENCE_RECORD_SIZE];
    assert_eq!(
        encode_nfc_evidence_record(record, &mut out[..NFC_EVIDENCE_RECORD_SIZE - 1]),
        Err(NfcEvidenceError::OutputTooSmall {
            required: NFC_EVIDENCE_RECORD_SIZE,
            available: NFC_EVIDENCE_RECORD_SIZE - 1
        }),
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    assert_eq!(
        encode_nfc_evidence_record(record, &mut out),
        Ok(NFC_EVIDENCE_RECORD_SIZE)
    );
    assert_eq!(&out[..4], b"NFC1");
    assert_eq!(u16::from_le_bytes([out[4], out[5]]), NFC_EVIDENCE_VERSION);
    assert_eq!(out[55], 0, "unknown time has explicit tag at time offset");
    assert_eq!(out[84], 1, "NFC UID kind remains distinct");
    assert_eq!(out[94], 4, "raw UID byte length is explicit");
    assert_eq!(&out[95..99], &[0x04, 0x25, 0, 0]);
    assert!(out[99..127].iter().all(|byte| *byte == 0));
    assert!(out[283..].iter().all(|byte| *byte == 0));
}

#[test]
fn signed_receipt_binds_exact_variable_identifier_and_host_trust() {
    let key = SigningKey::from_bytes(&[21; 32]);
    let mut scratch = [0u8; NFC_EVIDENCE_RECORD_SIZE];
    let receipt = NfcCustodyReceipt::sign(
        record(&[1, 2, 3, 4, 5, 0]),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(1_800_000_000),
        &key,
    )
    .unwrap();
    assert_eq!(
        receipt.verify_signature(record(&[1, 2, 3, 4, 5, 0]), &mut scratch),
        Ok(NfcReceiptStatus::SignatureValidUntrusted)
    );
    assert_eq!(
        receipt.verify_with_authority(
            record(&[1, 2, 3, 4, 5, 0]),
            &mut scratch,
            &Accept {
                key: key.verifying_key().to_bytes(),
                sequence: 0,
                previous: None
            }
        ),
        Ok(NfcReceiptStatus::VerifiedByAuthority)
    );
    assert_eq!(
        receipt.verify_signature(record(&[1, 2, 3, 4, 5]), &mut scratch),
        Err(NfcEvidenceError::RecordMismatch)
    );
    assert_eq!(
        receipt.verify_with_authority(
            record(&[1, 2, 3, 4, 5, 0]),
            &mut scratch,
            &Accept {
                key: [0; 32],
                sequence: 0,
                previous: None
            }
        ),
        Err(NfcEvidenceError::SignerNotTrusted)
    );
}

#[test]
fn receipt_requires_consistent_chain_position() {
    let key = SigningKey::from_bytes(&[22; 32]);
    let mut scratch = [0u8; NFC_EVIDENCE_RECORD_SIZE];
    assert_eq!(
        NfcCustodyReceipt::sign(
            record(&[1]),
            &mut scratch,
            0,
            Some([1; 32]),
            ReceiptTimestamp::UnixSeconds(2),
            &key
        ),
        Err(NfcEvidenceError::InvalidChain)
    );
    assert_eq!(
        NfcCustodyReceipt::sign(
            record(&[1]),
            &mut scratch,
            1,
            None,
            ReceiptTimestamp::UnixSeconds(2),
            &key
        ),
        Err(NfcEvidenceError::InvalidChain)
    );
    assert_eq!(
        NfcCustodyReceipt::sign(
            record(&[1]),
            &mut scratch,
            1,
            Some([0; 32]),
            ReceiptTimestamp::UnixSeconds(2),
            &key
        ),
        Err(NfcEvidenceError::InvalidPrevious)
    );
}

#[test]
fn nfc_uid_permission_requires_the_nfc_modality() {
    let identifier = VariableRadioIdentifier::new(
        VariableRadioIdentifierKind::NfcUid,
        ProtocolIdentifier::Unknown,
        &[1, 2, 3, 4],
    )
    .unwrap();
    let wrong = NfcEvidenceRecord::new(
        21,
        identifier,
        3,
        30,
        None,
        None,
        None,
        RadioObservationTime::Unknown,
        RadioEvidenceReference::Unknown,
        Some(permission(SensingModality::Wifi)),
    );
    assert_eq!(wrong, Err(NfcEvidenceError::PermissionModalityMismatch));
    let right = NfcEvidenceRecord::new(
        22,
        identifier,
        3,
        31,
        None,
        None,
        None,
        RadioObservationTime::Unknown,
        RadioEvidenceReference::Unknown,
        Some(permission(SensingModality::Nfc)),
    );
    let right = right.unwrap();
    let mut wire = [0u8; NFC_EVIDENCE_RECORD_SIZE];
    encode_nfc_evidence_record(right, &mut wire).unwrap();
    assert_eq!(
        wire[265], 11,
        "NFC remains a distinct permission modality tag"
    );
}

#[test]
fn nfc_receipt_transport_round_trips_but_parsing_does_not_verify_trust() {
    let key = SigningKey::from_bytes(&[23; 32]);
    let mut scratch = [0u8; NFC_EVIDENCE_RECORD_SIZE];
    let receipt = NfcCustodyReceipt::sign(
        record(&[0x11, 0, 0x22]),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::Unknown,
        &key,
    )
    .unwrap();
    let mut out = [0xa5u8; NFC_RECEIPT_ENCODED_LEN];
    assert_eq!(
        receipt.encode_into(&mut out[..NFC_RECEIPT_ENCODED_LEN - 1]),
        Err(NfcEvidenceError::OutputTooSmall {
            required: NFC_RECEIPT_ENCODED_LEN,
            available: NFC_RECEIPT_ENCODED_LEN - 1
        })
    );
    assert!(out.iter().all(|value| *value == 0xa5));
    assert_eq!(receipt.encode_into(&mut out), Ok(NFC_RECEIPT_ENCODED_LEN));
    let parsed = NfcCustodyReceipt::decode_from(&out).unwrap();
    assert_eq!(parsed, receipt);
    assert_eq!(
        parsed.verify_signature(record(&[0x11, 0, 0x22]), &mut scratch),
        Ok(NfcReceiptStatus::SignatureValidUntrusted)
    );
    assert_eq!(
        parsed.verify_with_authority(
            record(&[0x11, 0, 0x22]),
            &mut scratch,
            &Accept {
                key: key.verifying_key().to_bytes(),
                sequence: 0,
                previous: None
            }
        ),
        Err(NfcEvidenceError::ChainNotTrusted)
    );
    let mut malformed = out;
    malformed[NFC_RECEIPT_ENCODED_LEN - 1] = 1;
    assert_eq!(
        NfcCustodyReceipt::decode_from(&malformed),
        Err(NfcEvidenceError::InvalidTransport)
    );
}
