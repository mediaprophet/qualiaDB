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

use etalon_sense_core::{
    capability::Modality, observation::ObservationEnvelope, quality::CalibrationState,
    spectrum::SpectralCoverage,
};
use qualia_core_db::{
    frame_layout::{INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER, INLINE_VALUE_MASK},
    net::qdnf::{authority::Plane, fabric::InstrumentKind},
    q_hash,
    sensing::{
        authorization::{
            AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction,
            SensingPermit, SensingRequest, SensingScopeAuthority, TrustedUnixClock,
        },
        quin_index::proposed_predicates,
        record::{
            CalibrationArtifactReference, CalibrationDeclaration, MeasurementQuality,
            MeasurementRecord, ProjectionDeclaration, ProvenanceDeclaration, RecordClockStamp,
            RecordError, ResolvedFieldPayloadBinding,
        },
        PhysicalFieldSemantics, SemanticId,
    },
    NQuin,
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
    spectral_coverage: Option<SpectralCoverage>,
}

impl BridgedObservation {
    pub const fn observation(&self) -> &ObservationEnvelope {
        &self.observation
    }
    pub const fn record(&self) -> &MeasurementRecord {
        &self.record
    }
    pub const fn spectral_coverage(&self) -> Option<&SpectralCoverage> {
        self.spectral_coverage.as_ref()
    }
    pub fn with_spectral_coverage(mut self, coverage: SpectralCoverage) -> Self {
        self.spectral_coverage = Some(coverage);
        self
    }
    pub fn into_parts(self) -> (ObservationEnvelope, MeasurementRecord) {
        (self.observation, self.record)
    }

    /// Emit up to 8 canonical 48-byte `NQuin`s (all with 5-field XOR parity) combining the 5
    /// sensing index `NQuin`s with 3 Identifier Fabric plane-separation `NQuin`s:
    /// - `out[0..5]`: canonical sensing record index `NQuin`s (`MeasurementRecord::enumerate_index_quins`)
    /// - `out[5]`: sensor source tagged as `InstrumentKind::SensorId` on `Plane::Instrument`
    /// - `out[6]`: spatial/temporal observation window tagged as `EnvironmentPredicateShape` on `Plane::Handle`
    /// - `out[7]`: declared `SpectralCoverage` distinguishing the 9 EMF modalities (`is_electromagnetic() == true`)
    ///   from mechanical `AcousticBand` (`is_electromagnetic() == false`), never equating sensor evidence with a `NaturalAgent`.
    pub fn enumerate_fabric_quins(&self, out: &mut [NQuin; 8]) -> Result<usize, BridgeError> {
        let mk_quin = |s: u64, p: u64, o: u64, c: u64, m: u64| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity: NQuin::calculate_parity(s, p, o, c, m),
        };
        let rec_id = self.record.record_id();
        let src_id = self.record.source_id().value().unwrap_or(0);
        let field_id = self.record.physical().field_id();
        let field_kind = u64::from(self.record.physical().typed_field_kind());
        let claim_ctx = q_hash("urn:qualia:idf:plane:Claim");

        out[0] = mk_quin(
            rec_id,
            proposed_predicates::RECORD_ID,
            INLINE_TAG_INTEGER | (rec_id & INLINE_VALUE_MASK),
            claim_ctx,
            0,
        );
        out[1] = mk_quin(
            rec_id,
            proposed_predicates::FIELD_ID,
            INLINE_TAG_INTEGER | (field_id & INLINE_VALUE_MASK),
            claim_ctx,
            0,
        );
        out[2] = mk_quin(
            rec_id,
            proposed_predicates::FIELD_KIND,
            INLINE_TAG_INTEGER | (field_kind & INLINE_VALUE_MASK),
            claim_ctx,
            0,
        );
        out[3] = mk_quin(
            rec_id,
            proposed_predicates::SOURCE_KNOWN,
            INLINE_TAG_BOOLEAN | u64::from(self.record.source_id().value().is_some()),
            claim_ctx,
            0,
        );
        out[4] = mk_quin(
            rec_id,
            proposed_predicates::SOURCE_ID,
            INLINE_TAG_INTEGER | (src_id & INLINE_VALUE_MASK),
            claim_ctx,
            0,
        );

        out[5] = mk_quin(
            src_id,
            q_hash("urn:qualia:idf:instrumentKind"),
            q_hash("urn:qualia:idf:instrument:SensorId"),
            q_hash("urn:qualia:idf:plane:Instrument"),
            ((Plane::Instrument as u64) << 56) | (InstrumentKind::SensorId as u64),
        );
        out[6] = mk_quin(
            rec_id,
            q_hash("urn:qualia:idf:spatiotemporalWindow"),
            q_hash("urn:qualia:idf:shape:EnvironmentPredicateShape"),
            q_hash("urn:qualia:idf:plane:Handle"),
            self.observation
                .timestamp()
                .map(|stamp| stamp.ticks_ns())
                .unwrap_or(0),
        );

        let (coverage_obj, coverage_meta) = match &self.spectral_coverage {
            Some(cov) => {
                let is_emf = matches!(
                    cov.modality(),
                    Modality::Wifi
                        | Modality::Bluetooth
                        | Modality::Cellular
                        | Modality::Sdr
                        | Modality::Radar
                        | Modality::FieldInstrument
                        | Modality::VisibleCamera
                        | Modality::NearInfraredCamera
                        | Modality::ThermalCamera
                );
                let uri_hash = if cov.acoustic().is_some() || cov.modality() == Modality::Microphone
                {
                    q_hash("urn:qualia:etalon:spectrum:AcousticBand")
                } else {
                    q_hash("urn:qualia:etalon:spectrum:Electromagnetic")
                };
                let (lower_bits, upper_bits) = if let Some(ac) = cov.acoustic() {
                    (
                        (ac.lower_hz() as f32).to_bits() as u64,
                        ((ac.upper_hz() as f32).to_bits() as u64) & 0x7FFF_FFFF,
                    )
                } else if let Some(band) = cov
                    .carrier()
                    .or_else(|| cov.baseband())
                    .or_else(|| cov.modulation())
                    .or_else(|| cov.wavelength())
                {
                    (
                        (band.lower() as f32).to_bits() as u64,
                        ((band.upper() as f32).to_bits() as u64) & 0x7FFF_FFFF,
                    )
                } else {
                    (0u64, 0u64)
                };
                (
                    uri_hash,
                    ((is_emf as u64) << 63) | (upper_bits << 32) | lower_bits,
                )
            }
            None => (q_hash("urn:qualia:etalon:spectrum:unclassified"), 0u64),
        };
        out[7] = mk_quin(
            rec_id,
            q_hash("urn:qualia:etalon:spectrum:coverage"),
            coverage_obj,
            claim_ctx,
            coverage_meta,
        );

        Ok(8)
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
        spectral_coverage: None,
    })
}
