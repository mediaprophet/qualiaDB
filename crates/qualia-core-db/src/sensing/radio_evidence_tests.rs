use super::*;
use crate::{
    foundation::frame_layout::object_tag,
    governance::consent_contract::{ConsentGrant, ConsentScope},
};
use ed25519_dalek::SigningKey;

fn sample_record(time: RadioObservationTime) -> RadioEvidenceRecord {
    RadioEvidenceRecord::new(
        91,
        RadioIdentifier::Bluetooth {
            address: RadioAddress48::new([0x10, 0x20, 0x30, 0x40, 0x50, 0x60]),
            assignment: AddressAssignmentKind::Random,
        },
        RadioCaptureReference::new(71, 81, Some(82), Some(3), Some(0)).unwrap(),
        time,
        RadioEvidenceReference::Unknown,
        None,
    )
    .unwrap()
}

#[test]
fn raw_identifier_time_source_and_capture_are_preserved_without_identity_claims() {
    let record = sample_record(RadioObservationTime::WindowsUtc100ns(1_337));
    assert_eq!(
        record.identifier().address().octets_msb_first(),
        [0x10, 0x20, 0x30, 0x40, 0x50, 0x60]
    );
    assert_eq!(record.identifier().address().value_u64(), 0x1020_3040_5060);
    assert_eq!(record.capture().source_id(), 71);
    assert_eq!(record.capture().capture_id(), 81);
    assert_eq!(record.capture().stream_id(), Some(82));
    assert_eq!(record.capture().generation(), Some(3));
    assert_eq!(record.capture().sequence(), Some(0));
    assert_eq!(record.time(), RadioObservationTime::WindowsUtc100ns(1_337));
    assert_eq!(record.permission(), None);
}

#[test]
fn unknown_time_and_native_clock_values_are_explicit_and_not_converted() {
    assert_eq!(
        sample_record(RadioObservationTime::Unknown).time(),
        RadioObservationTime::Unknown
    );
    let native = RadioObservationTime::native_ticks(55, None, None).unwrap();
    assert_eq!(
        native,
        RadioObservationTime::NativeTicks {
            ticks: 55,
            clock_domain_id: None,
            unit_id: None
        }
    );
    assert_eq!(
        RadioObservationTime::native_ticks(55, Some(0), None),
        Err(RadioEvidenceError::InvalidIdentifier),
    );
}

#[test]
fn proposed_quins_use_inline_integer_tags_zero_metadata_and_canonical_parity() {
    let record = sample_record(RadioObservationTime::Unknown);
    let mapping = RadioQuinMapping::new(91, 0x1234, 0x5678).unwrap();
    let mut quins = [NQuin::default(); 3];
    assert_eq!(radio_record_to_quins(record, mapping, &mut quins), Ok(3));
    assert_eq!(PROPOSED_RADIO_VOCABULARY_STATUS, "proposed; not registered");
    assert_eq!(
        [quins[0].predicate, quins[1].predicate, quins[2].predicate],
        [
            PROPOSED_RADIO_ADDRESS48_PREDICATE,
            PROPOSED_RADIO_PROTOCOL_PREDICATE,
            PROPOSED_RADIO_ASSIGNMENT_PREDICATE
        ],
    );
    assert_eq!(object_tag(quins[0].object), INLINE_TAG_INTEGER);
    assert_eq!(quins[0].object & INLINE_VALUE_MASK, 0x1020_3040_5060);
    assert_eq!(quins[1].object & INLINE_VALUE_MASK, 3); // Bluetooth protocol tag.
    assert_eq!(quins[2].object & INLINE_VALUE_MASK, 3); // Host-declared Random.
    for quin in quins {
        assert_eq!(quin.subject, 0x1234);
        assert_eq!(quin.context, 0x5678);
        assert_eq!(quin.metadata, 0);
        assert!(frame_layout::parity_valid(&quin));
    }
    let mismatched = RadioQuinMapping::new(92, 0x1234, 0x5678).unwrap();
    assert_eq!(
        radio_record_to_quins(record, mismatched, &mut quins),
        Err(RadioEvidenceError::RecordMappingMismatch),
    );
}

#[test]
fn fixed_record_encoding_is_bounded_and_keeps_raw_bytes() {
    let record = sample_record(RadioObservationTime::WindowsUtc100ns(1_337));
    let mut bytes = [0xAA; RADIO_EVIDENCE_RECORD_SIZE];
    assert_eq!(
        encode_radio_evidence_records(&[record], &mut bytes),
        Ok(RADIO_EVIDENCE_RECORD_SIZE)
    );
    assert_eq!(&bytes[..4], b"RDE1");
    assert_eq!(
        u16::from_le_bytes([bytes[4], bytes[5]]),
        RADIO_EVIDENCE_VERSION
    );
    assert_eq!(
        u16::from_le_bytes([bytes[6], bytes[7]]),
        RADIO_EVIDENCE_RECORD_SIZE as u16
    );
    assert_eq!(&bytes[57..63], &[0x10, 0x20, 0x30, 0x40, 0x50, 0x60]);
    assert_eq!(bytes[63], 2); // Windows UTC ticks tag; raw ticks stay in side record.
    assert_eq!(bytes[92], 0); // Unknown evidence is not reported absent or verified.
    assert!(bytes[248..].iter().all(|byte| *byte == 0));

    let mut too_small = [0xA5; RADIO_EVIDENCE_RECORD_SIZE - 1];
    assert_eq!(
        encode_radio_evidence_records(&[record], &mut too_small),
        Err(RadioEvidenceError::OutputTooSmall {
            required: RADIO_EVIDENCE_RECORD_SIZE,
            available: RADIO_EVIDENCE_RECORD_SIZE - 1
        }),
    );
    assert!(too_small.iter().all(|byte| *byte == 0xA5));
}

#[test]
fn permission_context_can_only_be_copied_from_fresh_qdb_revalidation() {
    const ACTOR: [u8; 32] = [8; 32];
    const PURPOSE: u64 = 0x1122;
    let grant = ConsentGrant::new_signed(
        [7; 32],
        &SigningKey::from_bytes(&[3; 32]),
        ACTOR,
        ConsentScope(ConsentScope::ALL),
        PURPOSE,
        10,
        100,
        1,
    );
    let request = super::super::authorization::SensingRequest::new(
        ACTOR,
        71,
        SensingModality::Bluetooth,
        SensingAction::Acquire,
        PURPOSE,
    )
    .unwrap();
    let initial = super::super::authorization::authorize_sensing(
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
    let permission = RadioPermissionContext::from_revalidated(&revalidated).unwrap();
    assert_eq!(
        RadioEvidenceRecord::new(
            93,
            RadioIdentifier::Bssid {
                address: RadioAddress48::new([1, 2, 3, 4, 5, 6]),
                assignment: AddressAssignmentKind::Unknown,
            },
            RadioCaptureReference::new(71, 83, None, None, None).unwrap(),
            RadioObservationTime::Unknown,
            RadioEvidenceReference::NotAttached,
            Some(permission),
        ),
        Err(RadioEvidenceError::PermissionModalityMismatch),
    );
    let record = RadioEvidenceRecord::new(
        92,
        RadioIdentifier::Bluetooth {
            address: RadioAddress48::new([1, 2, 3, 4, 5, 6]),
            assignment: AddressAssignmentKind::Unknown,
        },
        RadioCaptureReference::new(71, 82, None, None, None).unwrap(),
        RadioObservationTime::Unknown,
        RadioEvidenceReference::NotAttached,
        Some(permission),
    )
    .unwrap();
    assert_eq!(record.permission().unwrap().actor_id(), ACTOR);
    assert_eq!(record.permission().unwrap().source_id(), 71);
    assert_eq!(record.permission().unwrap().checked_at_unix(), 50);
}

struct Identity;
impl super::super::authorization::IdentityAuthority for Identity {
    fn verify_parties(
        &self,
        _grant: &ConsentGrant,
        _actor_id: &[u8; 32],
    ) -> Result<(), super::super::authorization::AuthorizationError> {
        Ok(())
    }
}

struct Scope;
impl super::super::authorization::SensingScopeAuthority for Scope {
    fn verify_scope(
        &self,
        _grant: &ConsentGrant,
        _grant_digest: &[u8; 32],
        _request: &super::super::authorization::SensingRequest,
    ) -> Result<(), super::super::authorization::AuthorizationError> {
        Ok(())
    }
}

struct Revocation;
impl super::super::authorization::RevocationAuthority for Revocation {
    fn is_revoked(
        &self,
        _grant_id: &[u8; 32],
        _grant_digest: &[u8; 32],
        _current_time_unix: u64,
    ) -> Result<bool, super::super::authorization::AuthorizationError> {
        Ok(false)
    }
}

struct Clock(u64);
impl super::super::authorization::TrustedUnixClock for Clock {
    fn now_unix(&mut self) -> Result<u64, super::super::authorization::AuthorizationError> {
        Ok(self.0)
    }
}
