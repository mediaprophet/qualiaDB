//! Explicit, host-authorized Etalon observation to Qualia measurement bridge.
//!
//! This local integration preserves the complete original envelope alongside
//! the derived index. Persistence must retain both: the index alone omits
//! policy, evidence, stream metadata, clock anchor and detailed uncertainty.
//! Host authority must independently validate payload identity, namespace and
//! unit mappings, policy, and any referenced container or artifact.
#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub mod container_custody;
#[cfg(not(target_arch = "wasm32"))]
pub mod local_publish;
pub mod nfc;
#[cfg(not(target_arch = "wasm32"))]
pub mod publish_store;
pub mod radio;
#[cfg(not(target_arch = "wasm32"))]
pub mod read_store;
pub mod sidecar;
pub mod store_receipt;

use etalon_sense_core::{observation::ObservationEnvelope, quality::CalibrationState};
use qualia_core_db::sensing::{
    authorization::{
        AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction, SensingPermit,
        SensingRequest, SensingScopeAuthority, TrustedUnixClock,
    },
    record::{
        CalibrationArtifactReference, CalibrationDeclaration, MeasurementQuality,
        MeasurementRecord, ProjectionDeclaration, ProvenanceDeclaration, RecordClockStamp,
        RecordError, ResolvedFieldPayloadBinding,
    },
    PhysicalFieldSemantics, SemanticId,
};

/// Host validation binds the observation, source ID mapping, chosen field and
/// exact resolved payload. Returning true is an assertion by the host, not
/// verification, attestation or a reusable QualiaDB policy decision.
pub trait BridgeAuthority {
    /// Affirm the declared cross-registry source, quantity and unit mappings.
    fn confirm_mapping(
        &self,
        observation: &ObservationEnvelope,
        qualia_source_id: u64,
        physical: &PhysicalFieldSemantics,
    ) -> bool;

    /// Approve this particular use and the binding to the selected payload.
    fn approve(
        &self,
        observation: &ObservationEnvelope,
        qualia_source_id: u64,
        binding: &ResolvedFieldPayloadBinding,
    ) -> bool;

    /// Confirm that the clock ID in the observation has the same meaning in
    /// the Qualia clock registry, including its restart epoch.
    fn confirm_clock_namespace(&self, _observation: &ObservationEnvelope) -> bool {
        false
    }

    /// Confirm that the calibration artifact ID identifies the same artifact
    /// in Qualia's registry. This does not verify the artifact's validity.
    fn confirm_calibration_namespace(&self, _observation: &ObservationEnvelope) -> bool {
        false
    }
}

/// Resolve the complete consent request against this observation and payload.
/// This includes modality, actor, purpose and any applicable relationships.
pub trait ConsentBridgeAuthority: BridgeAuthority {
    fn confirm_consent_scope(
        &self,
        grant: &qualia_core_db::governance::consent_contract::ConsentGrant,
        request: &SensingRequest,
        observation: &ObservationEnvelope,
        binding: &ResolvedFieldPayloadBinding,
    ) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeError {
    InvalidSource,
    UnmappedSemantics,
    UnmappedClock,
    UnmappedCalibration,
    NotAuthorized,
    PermitScopeMismatch,
    Consent(AuthorizationError),
    InvalidRecord(RecordError),
}

/// Revalidate and consume a QualiaDB process authorization for this source
/// before returning a bridge record. The host authority must still
/// bind the precise payload, semantic namespaces, and any applicable context.
/// This is an optional consent adapter; other policy frameworks may use the
/// generic bridge. Storing, sharing or exporting requires its own authority.
pub fn bridge_observation_with_consent<A: ConsentBridgeAuthority>(
    authority: &A,
    permit: SensingPermit,
    grant: &qualia_core_db::governance::consent_contract::ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    record_id: u64,
    qualia_source_id: u64,
    observation: ObservationEnvelope,
    binding: ResolvedFieldPayloadBinding,
) -> Result<BridgedObservation, BridgeError> {
    let request = permit.request();
    if request.source_id() != qualia_source_id
        || request.action() != SensingAction::Process
        || !authority.confirm_consent_scope(grant, &request, &observation, &binding)
    {
        return Err(BridgeError::PermitScopeMismatch);
    }
    let result = bridge_observation(authority, record_id, qualia_source_id, observation, binding)?;
    let _use = permit
        .revalidate(grant, identity, sensor_scope, revocations, clock)
        .map_err(BridgeError::Consent)?;
    Ok(result)
}

/// A structurally constructed index and its complete source metadata. Neither
/// component authenticates the other, nor does it represent a policy grant.
pub struct BridgedObservation {
    observation: ObservationEnvelope,
    record: MeasurementRecord,
}

impl BridgedObservation {
    pub const fn observation(&self) -> &ObservationEnvelope {
        &self.observation
    }
    pub const fn record(&self) -> &MeasurementRecord {
        &self.record
    }
    pub fn into_parts(self) -> (ObservationEnvelope, MeasurementRecord) {
        (self.observation, self.record)
    }
}

/// The authority decides whether the source and quantity/unit namespaces map
/// to the selected field, and whether the selected bytes actually contain the
/// observation. No equality of unrelated registry IDs is assumed. Timestamp
/// anchor and all uncertainty and policy details remain in the envelope.
/// Quality is left unknown in the index because its millionths representation
/// cannot preserve arbitrary f32 source values exactly.
pub fn bridge_observation<A: BridgeAuthority>(
    authority: &A,
    record_id: u64,
    qualia_source_id: u64,
    observation: ObservationEnvelope,
    binding: ResolvedFieldPayloadBinding,
) -> Result<BridgedObservation, BridgeError> {
    let source = SemanticId::new(qualia_source_id).map_err(|_| BridgeError::InvalidSource)?;
    if !authority.confirm_mapping(&observation, qualia_source_id, &binding.physical()) {
        return Err(BridgeError::UnmappedSemantics);
    }
    if !authority.approve(&observation, qualia_source_id, &binding) {
        return Err(BridgeError::NotAuthorized);
    }
    if observation.timestamp().is_some() && !authority.confirm_clock_namespace(&observation) {
        return Err(BridgeError::UnmappedClock);
    }
    if matches!(
        observation.calibration().state(),
        CalibrationState::Calibrated { .. }
    ) && !authority.confirm_calibration_namespace(&observation)
    {
        return Err(BridgeError::UnmappedCalibration);
    }
    let calibration = match observation.calibration().state() {
        CalibrationState::Missing => CalibrationDeclaration::Missing,
        CalibrationState::Expired => CalibrationDeclaration::Expired,
        CalibrationState::OutOfRange => CalibrationDeclaration::OutOfRange,
        CalibrationState::Uncalibrated => CalibrationDeclaration::Uncalibrated,
        CalibrationState::Calibrated { artifact_id } => CalibrationDeclaration::Calibrated(
            CalibrationArtifactReference::new(artifact_id.bytes())
                .map_err(BridgeError::InvalidRecord)?,
        ),
    };
    let timestamp = observation
        .timestamp()
        .map(|stamp| {
            RecordClockStamp::new(
                stamp.clock_id(),
                stamp.restart_epoch(),
                stamp.ticks_ns(),
                stamp.uncertainty_ns(),
            )
        })
        .transpose()
        .map_err(BridgeError::InvalidRecord)?;
    let record = MeasurementRecord::new(
        record_id,
        source,
        binding,
        ProvenanceDeclaration::Unknown,
        calibration,
        MeasurementQuality::Unknown,
        timestamp,
        ProjectionDeclaration::Unknown,
    )
    .map_err(BridgeError::InvalidRecord)?;
    Ok(BridgedObservation {
        observation,
        record,
    })
}
