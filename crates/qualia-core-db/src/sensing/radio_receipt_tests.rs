use ed25519_dalek::SigningKey;

use super::super::authorization::{
    self, IdentityAuthority, RevocationAuthority, SensingAction, SensingModality, SensingRequest,
    SensingScopeAuthority, TrustedUnixClock,
};
use super::*;
use crate::governance::consent_contract::{ConsentGrant, ConsentScope};
use crate::sensing::radio_evidence::{
    AddressAssignmentKind, RadioAddress48, RadioCaptureReference, RadioEvidenceReference,
    RadioIdentifier, RadioObservationTime,
};

fn record(address_tail: u8) -> RadioEvidenceRecord {
    RadioEvidenceRecord::new(
        7,
        RadioIdentifier::Bssid {
            address: RadioAddress48::new([0x02, 0x11, 0x22, 0x33, 0x44, address_tail]),
            assignment: AddressAssignmentKind::Unknown,
        },
        RadioCaptureReference::new(2, 9, Some(4), Some(1), Some(12)).unwrap(),
        RadioObservationTime::Unknown,
        RadioEvidenceReference::NotAttached,
        None,
    )
    .unwrap()
}

#[test]
fn weak_signer_key_is_rejected() {
    let observed = record(3);
    let mut scratch = [0u8; 256];
    let mut receipt = RadioCustodyReceipt::sign(
        observed,
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::Unknown,
        None,
        &SigningKey::from_bytes(&[3; 32]),
    )
    .unwrap();
    receipt.signer_public_key = [0; 32];
    receipt.signer_public_key[0] = 1;
    receipt.signature = [0; 64];
    receipt.signature[0] = 1;
    assert!(matches!(
        receipt.verify_signature(observed, &mut scratch),
        Err(RadioReceiptError::InvalidSignerKey | RadioReceiptError::InvalidSignature)
    ));
}

fn record_with_permission(
    address_tail: u8,
    permission: RadioPermissionContext,
) -> RadioEvidenceRecord {
    RadioEvidenceRecord::new(
        8,
        RadioIdentifier::Bssid {
            address: RadioAddress48::new([0x02, 0x11, 0x22, 0x33, 0x44, address_tail]),
            assignment: AddressAssignmentKind::Unknown,
        },
        RadioCaptureReference::new(2, 10, None, None, None).unwrap(),
        RadioObservationTime::Unknown,
        RadioEvidenceReference::NotAttached,
        Some(permission),
    )
    .unwrap()
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

fn permission() -> RadioPermissionContext {
    let actor = [8; 32];
    let signer = SigningKey::from_bytes(&[33; 32]);
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
    let request =
        SensingRequest::new(actor, 2, SensingModality::Wifi, SensingAction::Acquire, 77).unwrap();
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

fn policy() -> PolicyDecisionReference {
    PolicyDecisionReference::new([1; 32], [2; 32], [3; 32]).unwrap()
}

struct AcceptAll {
    expected_key: [u8; 32],
    expected_policy: Option<PolicyDecisionReference>,
    expected_sequence: u64,
    expected_previous: Option<[u8; 32]>,
    allow_permission: bool,
}

impl RadioReceiptAuthority for AcceptAll {
    fn signer_authorized(&self, source_id: u64, public_key: &[u8; 32]) -> bool {
        source_id == 2 && public_key == &self.expected_key
    }

    fn permission_context_valid(&self, _record: &RadioEvidenceRecord) -> bool {
        self.allow_permission
    }

    fn policy_decision_valid(
        &self,
        _receipt: &RadioCustodyReceipt,
        _record: &RadioEvidenceRecord,
        policy: PolicyDecisionReference,
    ) -> bool {
        self.expected_policy == Some(policy)
    }

    fn chain_position_valid(
        &self,
        sequence: u64,
        previous_checkpoint: Option<&[u8; 32]>,
        timestamp: ReceiptTimestamp,
        current_checkpoint: &[u8; 32],
    ) -> bool {
        sequence == self.expected_sequence
            && previous_checkpoint.copied() == self.expected_previous
            && timestamp != ReceiptTimestamp::Unknown
            && current_checkpoint.iter().any(|byte| *byte != 0)
    }
}

fn authority(
    key: &SigningKey,
    expected_policy: Option<PolicyDecisionReference>,
    expected_sequence: u64,
    expected_previous: Option<[u8; 32]>,
) -> AcceptAll {
    AcceptAll {
        expected_key: key.verifying_key().to_bytes(),
        expected_policy,
        expected_sequence,
        expected_previous,
        allow_permission: true,
    }
}

#[test]
fn signature_covers_exact_canonical_record_and_returns_untrusted_status() {
    let key = SigningKey::from_bytes(&[11; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        record(0x55),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(1_800_000_000),
        None,
        &key,
    )
    .unwrap();

    assert_eq!(
        receipt.verify_signature(record(0x55), &mut scratch),
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    );
    assert_eq!(
        receipt.verify_with_authority(record(0x55), &mut scratch, &authority(&key, None, 0, None)),
        Ok(ReceiptVerificationStatus::VerifiedByAuthority)
    );
    assert_eq!(
        receipt.verify_signature(record(0x56), &mut scratch),
        Err(RadioReceiptError::RecordMismatch)
    );
}

#[test]
fn authority_checks_signer_policy_and_chain_separately() {
    let key = SigningKey::from_bytes(&[12; 32]);
    let previous = [9; 32];
    let policy = policy();
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        record(0x65),
        &mut scratch,
        4,
        Some(previous),
        ReceiptTimestamp::WindowsUtc100ns(123_456),
        Some(policy),
        &key,
    )
    .unwrap();

    assert_eq!(
        receipt.verify_with_authority(
            record(0x65),
            &mut scratch,
            &authority(&key, Some(policy), 4, Some(previous))
        ),
        Ok(ReceiptVerificationStatus::VerifiedByAuthority)
    );
    assert_eq!(
        receipt.verify_with_authority(
            record(0x65),
            &mut scratch,
            &authority(&key, None, 4, Some(previous))
        ),
        Err(RadioReceiptError::PolicyDecisionNotTrusted)
    );
    assert_eq!(
        receipt.verify_with_authority(
            record(0x65),
            &mut scratch,
            &authority(&key, Some(policy), 5, Some(previous))
        ),
        Err(RadioReceiptError::ChainNotTrusted)
    );
    let other_key = SigningKey::from_bytes(&[13; 32]);
    assert_eq!(
        receipt.verify_with_authority(
            record(0x65),
            &mut scratch,
            &authority(&other_key, Some(policy), 4, Some(previous))
        ),
        Err(RadioReceiptError::SignerNotTrusted)
    );
}

#[test]
fn sequence_and_previous_checkpoint_must_be_paired() {
    let key = SigningKey::from_bytes(&[14; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    assert_eq!(
        RadioCustodyReceipt::sign(
            record(0x75),
            &mut scratch,
            0,
            Some([1; 32]),
            ReceiptTimestamp::UnixSeconds(1),
            None,
            &key
        ),
        Err(RadioReceiptError::InvalidChainPosition),
    );
    assert_eq!(
        RadioCustodyReceipt::sign(
            record(0x75),
            &mut scratch,
            1,
            None,
            ReceiptTimestamp::UnixSeconds(1),
            None,
            &key
        ),
        Err(RadioReceiptError::InvalidChainPosition),
    );
    assert_eq!(
        RadioCustodyReceipt::sign(
            record(0x75),
            &mut scratch,
            1,
            Some([0; 32]),
            ReceiptTimestamp::UnixSeconds(1),
            None,
            &key
        ),
        Err(RadioReceiptError::InvalidPreviousCheckpoint),
    );
}

#[test]
fn receipt_transport_is_fixed_size_atomic_and_not_a_trust_verifier() {
    let key = SigningKey::from_bytes(&[15; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        record(0x85),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::Monotonic(RecordClockStamp::new(5, 1, 99, 2).unwrap()),
        None,
        &key,
    )
    .unwrap();
    let mut out = [0xa5u8; RADIO_RECEIPT_ENCODED_LEN];
    assert_eq!(
        receipt.encode_into(&mut out[..RADIO_RECEIPT_ENCODED_LEN - 1]),
        Err(RadioReceiptError::OutputTooSmall {
            required: RADIO_RECEIPT_ENCODED_LEN,
            available: RADIO_RECEIPT_ENCODED_LEN - 1
        })
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    assert_eq!(receipt.encode_into(&mut out), Ok(RADIO_RECEIPT_ENCODED_LEN));
    assert_eq!(&out[..4], b"RCP1");
    assert_eq!(u16::from_le_bytes([out[4], out[5]]), RADIO_RECEIPT_VERSION);
    assert!(out[327] == 0);
    let parsed = RadioCustodyReceipt::decode_from(&out).unwrap();
    assert_eq!(parsed, receipt);
    assert_eq!(
        parsed.verify_signature(record(0x85), &mut scratch),
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    );
    let mut malformed = out;
    malformed[327] = 1;
    assert_eq!(
        RadioCustodyReceipt::decode_from(&malformed),
        Err(RadioReceiptError::InvalidTransport)
    );
}

#[test]
fn valid_signature_does_not_override_chain_authority_rejection() {
    let key = SigningKey::from_bytes(&[16; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        record(0x95),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(2),
        None,
        &key,
    )
    .unwrap();
    let denied = AcceptAll {
        expected_key: key.verifying_key().to_bytes(),
        expected_policy: None,
        expected_sequence: 1,
        expected_previous: None,
        allow_permission: true,
    };
    assert_eq!(
        receipt.verify_signature(record(0x95), &mut scratch),
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    );
    assert_eq!(
        receipt.verify_with_authority(record(0x95), &mut scratch, &denied),
        Err(RadioReceiptError::ChainNotTrusted)
    );
}

#[test]
fn embedded_permission_is_checked_even_without_receipt_policy_reference() {
    let key = SigningKey::from_bytes(&[17; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let observed = record_with_permission(0xa5, permission());
    let receipt = RadioCustodyReceipt::sign(
        observed,
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(3),
        None,
        &key,
    )
    .unwrap();
    let authority = AcceptAll {
        expected_key: key.verifying_key().to_bytes(),
        expected_policy: None,
        expected_sequence: 0,
        expected_previous: None,
        allow_permission: false,
    };
    assert_eq!(
        receipt.verify_with_authority(observed, &mut scratch, &authority),
        Err(RadioReceiptError::PermissionContextNotTrusted)
    );
    assert_eq!(
        receipt.verify_signature(record_with_permission(0xa5, permission()), &mut scratch),
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    );
}

#[test]
fn optional_receipt_policy_must_match_embedded_grant_digest() {
    let key = SigningKey::from_bytes(&[19; 32]);
    let permission = permission();
    let observed = record_with_permission(0xa7, permission);
    let mismatched = policy();
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        observed,
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(4),
        Some(mismatched),
        &key,
    )
    .unwrap();
    let reject_authority = authority(&key, Some(mismatched), 0, None);
    assert_eq!(
        receipt.verify_with_authority(observed, &mut scratch, &reject_authority),
        Err(RadioReceiptError::PolicyBindingMismatch)
    );

    let matched =
        PolicyDecisionReference::new(permission.grant_digest(), [2; 32], [3; 32]).unwrap();
    let receipt = RadioCustodyReceipt::sign(
        observed,
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::UnixSeconds(4),
        Some(matched),
        &key,
    )
    .unwrap();
    assert_eq!(
        receipt.verify_with_authority(
            observed,
            &mut scratch,
            &authority(&key, Some(matched), 0, None)
        ),
        Ok(ReceiptVerificationStatus::VerifiedByAuthority)
    );
}

#[test]
fn unknown_receipt_time_is_signed_but_host_chain_authority_can_reject_it() {
    let key = SigningKey::from_bytes(&[18; 32]);
    let mut scratch = [0u8; super::super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE];
    let receipt = RadioCustodyReceipt::sign(
        record(0xa6),
        &mut scratch,
        0,
        None,
        ReceiptTimestamp::Unknown,
        None,
        &key,
    )
    .unwrap();
    assert_eq!(
        receipt.verify_signature(record(0xa6), &mut scratch),
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    );
    let mut wire = [0u8; RADIO_RECEIPT_ENCODED_LEN];
    receipt.encode_into(&mut wire).unwrap();
    assert_eq!(RadioCustodyReceipt::decode_from(&wire).unwrap(), receipt);
    assert_eq!(
        receipt.verify_with_authority(record(0xa6), &mut scratch, &authority(&key, None, 0, None)),
        Err(RadioReceiptError::ChainNotTrusted)
    );
}
