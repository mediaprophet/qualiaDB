use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[path = "tests/quota_publication.rs"]
mod quota_publication;

#[cfg(not(target_arch = "wasm32"))]
#[path = "container_custody_tests.rs"]
mod container_custody;
#[cfg(not(target_arch = "wasm32"))]
#[path = "read_store_tests.rs"]
mod stored_reads;
use crate::store_receipt::{prepare_store_receipt, StoreBindingAuthority, StoreReceiptError};
use crate::{bridge_observation, BridgeAuthority};
use crate::{bridge_observation_with_consent, BridgeError, ConsentBridgeAuthority};
use ed25519_dalek::SigningKey;
use etalon_sense_core::{
    evidence::{ArtifactDigest, AuthorityKeyRef, DigestAlgorithm, EvidenceOrigin},
    observation::{
        HostPolicyDecision, MeasurementUncertainty, ObservationEnvelope, ObservationKind,
        ObservationTimeAnchor, QuantityUnitIds,
    },
    policy::{DecisionCommitment, EvidenceCommitment, PolicyId, PolicyOutcome, PolicyRequest},
    quality::{
        CalibrationRecord, CalibrationState, QualityMetrics, QualityRecord, SourceDescriptor,
        SourceKind,
    },
    time::ClockStamp,
};
use qualia_core_db::governance::consent_contract::{ConsentGrant, ConsentScope};
use qualia_core_db::sensing::authorization::{
    self, AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction,
    SensingModality, SensingRequest, SensingScopeAuthority, TrustedUnixClock,
};
use qualia_core_db::{
    container_10d::{
        self as c,
        field_section::{
            FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind,
        },
    },
    sensing::{
        record::ResolvedFieldPayloadBinding, MeasurementModality, PhysicalFieldSemantics,
        SemanticId,
    },
};

struct Authority;
impl BridgeAuthority for Authority {
    fn confirm_mapping(&self, _: &ObservationEnvelope, _: u64, _: &PhysicalFieldSemantics) -> bool {
        true
    }
    fn approve(&self, _: &ObservationEnvelope, _: u64, _: &ResolvedFieldPayloadBinding) -> bool {
        true
    }
    fn confirm_clock_namespace(&self, _: &ObservationEnvelope) -> bool {
        true
    }
    fn confirm_calibration_namespace(&self, _: &ObservationEnvelope) -> bool {
        true
    }
}

struct ConsentAuthority(bool);
impl BridgeAuthority for ConsentAuthority {
    fn confirm_mapping(&self, _: &ObservationEnvelope, _: u64, _: &PhysicalFieldSemantics) -> bool {
        true
    }
    fn approve(&self, _: &ObservationEnvelope, _: u64, _: &ResolvedFieldPayloadBinding) -> bool {
        true
    }
    fn confirm_clock_namespace(&self, _: &ObservationEnvelope) -> bool {
        true
    }
}
impl ConsentBridgeAuthority for ConsentAuthority {
    fn confirm_consent_scope(
        &self,
        grant: &ConsentGrant,
        request: &SensingRequest,
        observation: &ObservationEnvelope,
        binding: &ResolvedFieldPayloadBinding,
    ) -> bool {
        self.0
            && grant.compute_digest() == signed_grant().compute_digest()
            && request.actor_id() == [8; 32]
            && request.source_id() == 17
            && request.modality() == SensingModality::Sdr
            && request.action() == SensingAction::Process
            && request.purpose_hash() == 0x1122
            && observation.source().source_id() == 41
            && *binding == self::binding()
    }
}
struct Identity;
impl IdentityAuthority for Identity {
    fn verify_parties(&self, _: &ConsentGrant, _: &[u8; 32]) -> Result<(), AuthorizationError> {
        Ok(())
    }
}
struct SensorScope(bool);
impl SensingScopeAuthority for SensorScope {
    fn verify_scope(
        &self,
        _: &ConsentGrant,
        _: &[u8; 32],
        _: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        if self.0 {
            Ok(())
        } else {
            Err(AuthorizationError::UnsupportedSensorScope)
        }
    }
}
struct Revocations(bool);
impl RevocationAuthority for Revocations {
    fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], _: u64) -> Result<bool, AuthorizationError> {
        Ok(self.0)
    }
}
struct Clock(u64);
impl TrustedUnixClock for Clock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(self.0)
    }
}

fn signed_grant() -> ConsentGrant {
    ConsentGrant::new_signed(
        [7; 32],
        &SigningKey::from_bytes(&[3; 32]),
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        0x1122,
        10,
        100,
        1,
    )
}

fn permit(
    grant: &ConsentGrant,
    action: SensingAction,
    source: u64,
) -> authorization::SensingPermit {
    permit_with_modality(grant, action, source, SensingModality::Sdr)
}
fn permit_with_modality(
    grant: &ConsentGrant,
    action: SensingAction,
    source: u64,
    modality: SensingModality,
) -> authorization::SensingPermit {
    let request = SensingRequest::new([8; 32], source, modality, action, 0x1122).unwrap();
    authorization::authorize_sensing(
        grant,
        request,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(50),
    )
    .unwrap()
}

#[test]
fn consent_bridge_requires_process_source_and_complete_scope() {
    let grant = signed_grant();
    let bridge = |permit, source, authority: &ConsentAuthority| {
        bridge_observation_with_consent(
            authority,
            permit,
            &grant,
            &Identity,
            &SensorScope(true),
            &Revocations(false),
            &mut Clock(60),
            21,
            source,
            observation(),
            binding(),
        )
    };
    assert!(bridge(
        permit(&grant, SensingAction::Process, 17),
        17,
        &ConsentAuthority(true)
    )
    .is_ok());
    assert!(matches!(
        bridge(
            permit(&grant, SensingAction::Acquire, 17),
            17,
            &ConsentAuthority(true)
        ),
        Err(BridgeError::PermitScopeMismatch)
    ));
    assert!(matches!(
        bridge(
            permit(&grant, SensingAction::Process, 18),
            17,
            &ConsentAuthority(true)
        ),
        Err(BridgeError::PermitScopeMismatch)
    ));
    assert!(matches!(
        bridge(
            permit(&grant, SensingAction::Process, 17),
            17,
            &ConsentAuthority(false)
        ),
        Err(BridgeError::PermitScopeMismatch)
    ));
    assert!(matches!(
        bridge(
            permit_with_modality(
                &grant,
                SensingAction::Process,
                17,
                SensingModality::Bluetooth
            ),
            17,
            &ConsentAuthority(true)
        ),
        Err(BridgeError::PermitScopeMismatch)
    ));

    let other = ConsentGrant::new_signed(
        [6; 32],
        &SigningKey::from_bytes(&[3; 32]),
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        0x1122,
        10,
        100,
        1,
    );
    assert!(matches!(
        bridge_observation_with_consent(
            &ConsentAuthority(true),
            permit(&other, SensingAction::Process, 17),
            &other,
            &Identity,
            &SensorScope(true),
            &Revocations(false),
            &mut Clock(60),
            21,
            17,
            observation(),
            binding()
        ),
        Err(BridgeError::PermitScopeMismatch)
    ));
}

#[test]
fn consent_bridge_revalidates_expiry_revocation_and_scope_before_return() {
    let grant = signed_grant();
    let bridge = |revoked, time, scope| {
        bridge_observation_with_consent(
            &ConsentAuthority(true),
            permit(&grant, SensingAction::Process, 17),
            &grant,
            &Identity,
            &SensorScope(scope),
            &Revocations(revoked),
            &mut Clock(time),
            21,
            17,
            observation(),
            binding(),
        )
    };
    assert!(matches!(
        bridge(false, 100, true),
        Err(BridgeError::Consent(AuthorizationError::GrantExpired))
    ));
    assert!(matches!(
        bridge(true, 60, true),
        Err(BridgeError::Consent(AuthorizationError::Revoked))
    ));
    assert!(matches!(
        bridge(false, 60, false),
        Err(BridgeError::Consent(
            AuthorizationError::UnsupportedSensorScope
        ))
    ));
}

fn binding() -> ResolvedFieldPayloadBinding {
    let data = [5u8; 16];
    let section = c::SectionInput {
        section_type: c::SectionType::FieldSidecar,
        alignment_tier: c::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &data,
    };
    let typed = c::TypedFieldRecord {
        field_id: 9,
        kind: c::FIELD_KIND_SCALAR,
        flags: 0,
        unit_id: 11,
        payload_section_id: c::SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: 16,
    };
    let identity = c::ManifoldIdentityV2 {
        entity_id: c::StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut manifest = [0; c::MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let mut sections = [section; 2];
    let mut bytes = [0; 4096];
    let len = c::encode_container_with_manifold_identity(
        &c::Container10dHeader::proposed(),
        &[section],
        &identity,
        &[typed],
        &mut manifest,
        &mut sections,
        &mut bytes,
    )
    .unwrap();
    c::seal_whole_file_crc32c(&mut bytes[..len]);
    let mut scratch = [c::TypedFieldRecord::default(); 1];
    let mut fields = [c::TypedFieldRecord::default(); 1];
    let resolver =
        c::typed_payloads::TypedPayloadResolver::new(&bytes[..len], &[], &mut scratch, &mut fields)
            .unwrap();
    let field = FieldDescriptor {
        name_hash: 22,
        unit_hash: 33,
        type_kind: FieldTypeKind::Spectral as u8,
        support: FieldSupportKind::Stream as u8,
        representation: FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let physical = PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(13).unwrap(),
    )
    .unwrap();
    ResolvedFieldPayloadBinding::new(physical, &resolver, [1; 16]).unwrap()
}

fn observation() -> ObservationEnvelope {
    let source = SourceDescriptor::new(41, SourceKind::Live).unwrap();
    let evidence = EvidenceOrigin::signature_presented(
        source,
        AuthorityKeyRef::new(8, 9).unwrap(),
        ArtifactDigest::new(DigestAlgorithm::Sha256, [2; 32]),
        ArtifactDigest::new(DigestAlgorithm::Blake3, [3; 32]),
    );
    let id = |value| PolicyId::new(value).unwrap();
    let request = PolicyRequest::new(
        Some(id(1)),
        None,
        id(2),
        id(3),
        id(4),
        id(5),
        id(6),
        id(7),
        EvidenceCommitment::new([8; 32]).unwrap(),
    );
    ObservationEnvelope::new(
        ObservationKind::LiveFrame,
        source,
        Some(12),
        Some(4),
        Some(0),
        Some(ClockStamp::new(2, 3, 100, 1).unwrap()),
        QuantityUnitIds::new(Some(13), Some(11)).unwrap(),
        MeasurementUncertainty::interval(-1.0, 2.0, Some(11)).unwrap(),
        QualityRecord::Known(QualityMetrics::new(0.3, 0.1).unwrap()),
        CalibrationRecord::new(CalibrationState::Missing).unwrap(),
        evidence,
        Some(ArtifactDigest::new(DigestAlgorithm::Blake3, [4; 32])),
        HostPolicyDecision::Evaluated {
            request,
            outcome: PolicyOutcome::Permit(DecisionCommitment::new([9; 32]).unwrap()),
        },
    )
    .unwrap()
    .with_time_anchor(ObservationTimeAnchor::FirstSample)
    .unwrap()
}

struct StoreAuthority(bool);
struct ZeroStoreAuthority;
impl StoreBindingAuthority for ZeroStoreAuthority {
    fn authorize_binding(
        &self,
        _: &ConsentGrant,
        _: &SensingRequest,
        _: &crate::BridgedObservation,
        _: &[u8; 768],
        _: &[u8],
        _: &[u8; 32],
        _: &[u8; 32],
        _: u64,
        _: Option<&[u8; 32]>,
    ) -> Option<[u8; 32]> {
        Some([0; 32])
    }
}
impl StoreBindingAuthority for StoreAuthority {
    fn authorize_binding(
        &self,
        grant: &ConsentGrant,
        request: &SensingRequest,
        observation: &crate::BridgedObservation,
        sidecar: &[u8; 768],
        payload: &[u8],
        signer: &[u8; 32],
        chain: &[u8; 32],
        sequence: u64,
        previous: Option<&[u8; 32]>,
    ) -> Option<[u8; 32]> {
        if self.0
            && grant.compute_digest() == signed_grant().compute_digest()
            && request.source_id() == 17
            && request.action() == SensingAction::Store
            && request.modality() == SensingModality::Sdr
            && request.purpose_hash() == 0x1122
            && observation.record().raw_payload().length() == 16
            && sidecar[..4] == *b"QES1"
            && payload == [5; 16]
            && *signer == SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes()
            && *chain == [1; 32]
            && sequence <= 1
            && previous.is_none()
        {
            Some([6; 32])
        } else {
            None
        }
    }
}

#[test]
fn store_receipt_requires_distinct_fresh_store_permission_and_exact_binding() {
    use qualia_core_db::sensing::sidecar_attestation::SIDECAR_RECEIPT_BYTES;
    let grant = signed_grant();
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let key = SigningKey::from_bytes(&[11; 32]);
    let mut sidecar = [0xa5; SIDECAR_SIZE + 3];
    let mut receipt = [0xa5; SIDECAR_RECEIPT_BYTES + 3];
    let result = prepare_store_receipt(
        &StoreAuthority(true),
        permit(&grant, SensingAction::Store, 17),
        &grant,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(60),
        &bridged,
        &[5; 16],
        16,
        [1; 32],
        0,
        None,
        &key,
        &mut sidecar,
        &mut receipt,
    )
    .unwrap();
    let exact_sidecar: &[u8; 768] = sidecar[..SIDECAR_SIZE].try_into().unwrap();
    assert_eq!(result.verify_signature(exact_sidecar, &[5; 16], 16), Ok(()));
    assert_eq!(*result.policy_commitment(), [6; 32]);
    assert_eq!(&sidecar[SIDECAR_SIZE..], &[0xa5; 3]);
    assert_eq!(&receipt[SIDECAR_RECEIPT_BYTES..], &[0xa5; 3]);

    sidecar.fill(0xa5);
    receipt.fill(0xa5);
    let error = prepare_store_receipt(
        &StoreAuthority(true),
        permit(&grant, SensingAction::Process, 17),
        &grant,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(60),
        &bridged,
        &[5; 16],
        16,
        [1; 32],
        0,
        None,
        &key,
        &mut sidecar,
        &mut receipt,
    );
    assert!(matches!(error, Err(StoreReceiptError::ScopeMismatch)));
    assert!(sidecar.iter().chain(receipt.iter()).all(|&b| b == 0xa5));
}


#[path = "tests_publication_quins.rs"]
mod tests_publication_quins;
