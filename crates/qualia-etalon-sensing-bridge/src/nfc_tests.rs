use super::*;
use ed25519_dalek::SigningKey;
use etalon_sense_core::time::ClockStamp;
use etalon_sense_nfc::{HostNfcMetadata, NfcProtocol, NfcProximity, NfcTagKind};
use qualia_core_db::{
    governance::consent_contract::{ConsentGrant, ConsentScope},
    sensing::authorization::{
        self, AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction,
        SensingModality, SensingRequest, SensingScopeAuthority, TrustedUnixClock,
    },
};

struct Authority {
    approve: bool,
    clock: bool,
}
impl NfcBridgeAuthority for Authority {
    fn approve(&self, observation: &NfcObservation, mapping: &NfcMapping) -> bool {
        self.approve
            && observation.metadata().source_id == 41
            && observation.metadata().protocol == NfcProtocol::Iso14443A
            && mapping.record_id == 7
            && mapping.source_id == 123
            && mapping.protocol == ProtocolIdentifier::from_optional(999)
    }
    fn confirm_qualified_clock(&self, _: &NfcObservation, _: &NfcMapping) -> bool {
        self.clock
    }
}

fn observation(timestamp: Option<ClockStamp>) -> NfcObservation {
    let metadata = HostNfcMetadata {
        source_id: 41,
        stream_id: 5,
        generation: 2,
        sequence: 8,
        gap_before: Some(3),
        uid_len: 4,
        protocol: NfcProtocol::Iso14443A,
        tag_kind: NfcTagKind::Type4,
        proximity: NfcProximity::HostReportedPresent,
        timestamp,
    };
    NfcObservation::new(metadata, &[0, 7, 0, 0]).unwrap()
}

fn mapping() -> NfcMapping {
    NfcMapping {
        record_id: 7,
        source_id: 123,
        capture_id: 6,
        stream_id: Some(17),
        generation: Some(3),
        sequence: Some(8),
        protocol: ProtocolIdentifier::from_optional(999),
        evidence: RadioEvidenceReference::Unknown,
        permission: None,
    }
}

#[test]
fn uid_zero_bytes_protocol_and_original_metadata_are_preserved() {
    let source = observation(None);
    let mapped = bridge_nfc_observation(
        &Authority {
            approve: true,
            clock: false,
        },
        source,
        mapping(),
    )
    .unwrap();
    assert_eq!(mapped.record.identifier().bytes(), &[0, 7, 0, 0]);
    assert_eq!(
        mapped.record.identifier().protocol(),
        ProtocolIdentifier::from_optional(999)
    );
    assert_eq!(mapped.record.time(), RadioObservationTime::Unknown);
    assert_eq!(mapped.observation, source);
    assert_eq!(mapped.record.source_id(), 123);
    assert_eq!(mapped.record.stream_id(), Some(17));
}

#[test]
fn source_and_protocol_mapping_are_host_authorized() {
    let input = observation(None);
    let mut other = mapping();
    other.source_id = 124;
    assert!(matches!(
        bridge_nfc_observation(
            &Authority {
                approve: true,
                clock: false
            },
            input,
            other
        ),
        Err(NfcBridgeError::NotAuthorized)
    ));
    other = mapping();
    other.protocol = ProtocolIdentifier::Unknown;
    assert!(matches!(
        bridge_nfc_observation(
            &Authority {
                approve: true,
                clock: false
            },
            input,
            other
        ),
        Err(NfcBridgeError::NotAuthorized)
    ));
    assert!(matches!(
        bridge_nfc_observation(
            &Authority {
                approve: false,
                clock: true
            },
            input,
            mapping()
        ),
        Err(NfcBridgeError::NotAuthorized)
    ));
}

#[test]
fn mapped_clock_requires_explicit_qualification() {
    let stamp = ClockStamp::new(9, 2, 800, 3).unwrap();
    let input = observation(Some(stamp));
    assert!(matches!(
        bridge_nfc_observation(
            &Authority {
                approve: true,
                clock: false
            },
            input,
            mapping()
        ),
        Err(NfcBridgeError::UnmappedClock)
    ));
    let mapped = bridge_nfc_observation(
        &Authority {
            approve: true,
            clock: true,
        },
        input,
        mapping(),
    )
    .unwrap();
    assert_eq!(
        mapped.record.time(),
        RadioObservationTime::QualifiedMonotonic(RecordClockStamp::new(9, 2, 800, 3).unwrap())
    );
}

struct Identity;
impl IdentityAuthority for Identity {
    fn verify_parties(&self, _: &ConsentGrant, _: &[u8; 32]) -> Result<(), AuthorizationError> {
        Ok(())
    }
}
struct Scope;
impl SensingScopeAuthority for Scope {
    fn verify_scope(
        &self,
        _: &ConsentGrant,
        _: &[u8; 32],
        _: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        Ok(())
    }
}
struct Revocations;
impl RevocationAuthority for Revocations {
    fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], _: u64) -> Result<bool, AuthorizationError> {
        Ok(false)
    }
}
struct Clock;
impl TrustedUnixClock for Clock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(50)
    }
}

fn permission(source: u64, modality: SensingModality) -> RadioPermissionContext {
    let grant = ConsentGrant::new_signed(
        [7; 32],
        &SigningKey::from_bytes(&[3; 32]),
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        0x1122,
        10,
        100,
        1,
    );
    let request =
        SensingRequest::new([8; 32], source, modality, SensingAction::Acquire, 0x1122).unwrap();
    let permit = authorization::authorize_sensing(
        &grant,
        request,
        &Identity,
        &Scope,
        &Revocations,
        &mut Clock,
    )
    .unwrap();
    let checked = permit
        .revalidate(&grant, &Identity, &Scope, &Revocations, &mut Clock)
        .unwrap();
    RadioPermissionContext::from_revalidated(&checked).unwrap()
}

#[test]
fn permission_source_and_modality_are_checked() {
    let input = observation(None);
    let authority = Authority {
        approve: true,
        clock: false,
    };
    let mut candidate = mapping();
    candidate.permission = Some(permission(123, SensingModality::Nfc));
    assert!(bridge_nfc_observation(&authority, input, candidate).is_ok());
    candidate.permission = Some(permission(123, SensingModality::Bluetooth));
    assert!(matches!(
        bridge_nfc_observation(&authority, input, candidate),
        Err(NfcBridgeError::Record(
            NfcEvidenceError::PermissionModalityMismatch
        ))
    ));
    candidate.permission = Some(permission(124, SensingModality::Nfc));
    assert!(matches!(
        bridge_nfc_observation(&authority, input, candidate),
        Err(NfcBridgeError::Record(
            NfcEvidenceError::PermissionSourceMismatch
        ))
    ));
}
