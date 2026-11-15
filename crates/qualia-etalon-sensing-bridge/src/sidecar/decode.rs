use super::{
    cursor::Reader, encode::encode_parts, SidecarError, UntrustedSidecar, MAGIC, SIDECAR_SIZE,
};
use etalon_sense_core::{
    evidence::{ArtifactDigest, AuthorityKeyRef, DigestAlgorithm, EvidenceOrigin},
    observation::{
        HostPolicyDecision, MeasurementUncertainty, ObservationEnvelope, ObservationKind,
        ObservationTimeAnchor, QuantityUnitIds,
    },
    policy::{DecisionCommitment, EvidenceCommitment, PolicyId, PolicyOutcome, PolicyRequest},
    quality::{
        CalibrationArtifactId, CalibrationRecord, CalibrationState, QualityMetrics, QualityRecord,
        SourceDescriptor, SourceKind,
    },
    time::ClockStamp,
};
use qualia_core_db::sensing::record::{
    decode_measurement_index_record, CalibrationArtifactReference, CalibrationDeclaration,
    MeasurementQuality, ProjectionDeclaration, ProvenanceDeclaration, RecordClockStamp,
    ResolvedFieldPayloadBinding, MEASUREMENT_INDEX_RECORD_SIZE,
};

/// Parse canonical data against a caller-resolved payload binding. This only
/// validates structure; the historical evidence and policy are untrusted.
pub fn decode_sidecar(
    bytes: &[u8],
    binding: ResolvedFieldPayloadBinding,
) -> Result<UntrustedSidecar, SidecarError> {
    if bytes.len() != SIDECAR_SIZE {
        return Err(SidecarError::BufferSize);
    }
    let mut r = Reader::new(bytes);
    if &r.bytes::<4>()? != MAGIC
        || u16::from_le_bytes(r.bytes()?) != 1
        || u16::from_le_bytes(r.bytes()?) as usize != SIDECAR_SIZE
    {
        return Err(SidecarError::InvalidEncoding);
    }
    let encoded_record = r.bytes::<MEASUREMENT_INDEX_RECORD_SIZE>()?;
    let record =
        decode_measurement_index_record(&encoded_record, binding).map_err(SidecarError::Record)?;
    let kind = match r.u8()? {
        1 => ObservationKind::LiveFrame,
        2 => ObservationKind::RecordedFrame,
        3 => ObservationKind::SyntheticFrame,
        4 => ObservationKind::CachedQuery,
        5 => ObservationKind::HostDerived,
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let source_kind = match r.u8()? {
        1 => SourceKind::Synthetic,
        2 => SourceKind::Recorded,
        3 => SourceKind::Live,
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let source = SourceDescriptor::new(r.u64()?, source_kind)
        .map_err(|_| SidecarError::InvalidObservation)?;
    let stream = optional_id(r.u64()?);
    let generation = match r.u32()? {
        0 => None,
        value => Some(value),
    };
    let sequence = read_optional_u64(&mut r)?;
    let has_clock = r.u8()?;
    let clock_id = r.u64()?;
    let epoch = r.u32()?;
    let ticks = r.u64()?;
    let error = r.u64()?;
    let timestamp = match has_clock {
        0 => None,
        1 => Some(
            ClockStamp::new(clock_id, epoch, ticks, error)
                .map_err(|_| SidecarError::InvalidObservation)?,
        ),
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let anchor = match r.u8()? {
        0 => ObservationTimeAnchor::Unknown,
        1 => ObservationTimeAnchor::FirstSample,
        2 => ObservationTimeAnchor::LastSample,
        3 => ObservationTimeAnchor::Midpoint,
        4 => ObservationTimeAnchor::ReadCompletion,
        5 => ObservationTimeAnchor::NativeEvent,
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let units = QuantityUnitIds::new(optional_id(r.u64()?), optional_id(r.u64()?))
        .map_err(|_| SidecarError::InvalidObservation)?;
    let uncertainty_tag = r.u8()?;
    let first = r.u64()?;
    let second = r.u64()?;
    let unit = optional_id(r.u64()?);
    let uncertainty = match uncertainty_tag {
        0 => MeasurementUncertainty::Unknown,
        1 => MeasurementUncertainty::absolute_bound(f64::from_bits(first), unit)
            .map_err(|_| SidecarError::InvalidObservation)?,
        2 => MeasurementUncertainty::relative_bound(f64::from_bits(first))
            .map_err(|_| SidecarError::InvalidObservation)?,
        3 => MeasurementUncertainty::interval(f64::from_bits(first), f64::from_bits(second), unit)
            .map_err(|_| SidecarError::InvalidObservation)?,
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let quality_tag = r.u8()?;
    let confidence = r.u32()?;
    let relative = r.u32()?;
    let quality = match quality_tag {
        0 => QualityRecord::Unknown,
        1 => QualityRecord::Known(
            QualityMetrics::new(f32::from_bits(confidence), f32::from_bits(relative))
                .map_err(|_| SidecarError::InvalidObservation)?,
        ),
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let calibration_tag = r.u8()?;
    let artifact = r.bytes::<32>()?;
    let calibration = match calibration_tag {
        1 => CalibrationState::Missing,
        2 => CalibrationState::Expired,
        3 => CalibrationState::OutOfRange,
        4 => CalibrationState::Uncalibrated,
        5 => CalibrationState::Calibrated {
            artifact_id: CalibrationArtifactId::new(artifact)
                .map_err(|_| SidecarError::InvalidObservation)?,
        },
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let calibration =
        CalibrationRecord::new(calibration).map_err(|_| SidecarError::InvalidObservation)?;
    let evidence_tag = r.u8()?;
    let assertion = r.u64()?;
    let issuer = r.u64()?;
    let key = r.u64()?;
    let signed = read_digest(&mut r)?;
    let signature = read_digest(&mut r)?;
    let evidence = match evidence_tag {
        0 => EvidenceOrigin::Unknown,
        1 => EvidenceOrigin::asserted(source, assertion)
            .map_err(|_| SidecarError::InvalidObservation)?,
        2 | 3 => EvidenceOrigin::signature_presented(
            source,
            AuthorityKeyRef::new(issuer, key).map_err(|_| SidecarError::InvalidObservation)?,
            signed.ok_or(SidecarError::InvalidEncoding)?,
            signature.ok_or(SidecarError::InvalidEncoding)?,
        ),
        _ => return Err(SidecarError::InvalidEncoding),
    };
    let payload_digest = read_digest(&mut r)?;
    let policy = read_policy(&mut r)?;
    let observation = ObservationEnvelope::new(
        kind,
        source,
        stream,
        generation,
        sequence,
        timestamp,
        units,
        uncertainty,
        quality,
        calibration,
        evidence,
        payload_digest,
        policy,
    )
    .and_then(|value| value.with_time_anchor(anchor))
    .map_err(|_| SidecarError::InvalidObservation)?;
    let expected_calibration = match calibration.state() {
        CalibrationState::Missing => CalibrationDeclaration::Missing,
        CalibrationState::Expired => CalibrationDeclaration::Expired,
        CalibrationState::OutOfRange => CalibrationDeclaration::OutOfRange,
        CalibrationState::Uncalibrated => CalibrationDeclaration::Uncalibrated,
        CalibrationState::Calibrated { artifact_id } => CalibrationDeclaration::Calibrated(
            CalibrationArtifactReference::new(artifact_id.bytes()).map_err(SidecarError::Record)?,
        ),
    };
    let expected_timestamp = timestamp
        .map(|stamp| {
            RecordClockStamp::new(
                stamp.clock_id(),
                stamp.restart_epoch(),
                stamp.ticks_ns(),
                stamp.uncertainty_ns(),
            )
        })
        .transpose()
        .map_err(SidecarError::Record)?;
    if record.source_id().value().is_none()
        || record.calibration() != expected_calibration
        || record.timestamp() != expected_timestamp
        || record.provenance() != ProvenanceDeclaration::Unknown
        || record.quality() != MeasurementQuality::Unknown
        || record.projection() != ProjectionDeclaration::Unknown
    {
        return Err(SidecarError::InvalidEncoding);
    }
    let verified_claim = evidence_tag == 3;
    let canonical = encode_parts(&record, &observation, verified_claim)?;
    if canonical != bytes {
        return Err(SidecarError::InvalidEncoding);
    }
    Ok(UntrustedSidecar {
        record,
        observation,
        evidence_verification_claimed: verified_claim,
    })
}

fn optional_id(value: u64) -> Option<u64> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}
fn read_optional_u64(r: &mut Reader<'_>) -> Result<Option<u64>, SidecarError> {
    let flag = r.u8()?;
    let value = r.u64()?;
    match flag {
        0 => Ok(None),
        1 => Ok(Some(value)),
        _ => Err(SidecarError::InvalidEncoding),
    }
}
fn read_digest(r: &mut Reader<'_>) -> Result<Option<ArtifactDigest>, SidecarError> {
    let kind = r.u8()?;
    let bytes = r.bytes::<32>()?;
    match kind {
        0 => Ok(None),
        1 => Ok(Some(ArtifactDigest::new(DigestAlgorithm::Sha256, bytes))),
        2 => Ok(Some(ArtifactDigest::new(DigestAlgorithm::Blake3, bytes))),
        _ => Err(SidecarError::InvalidEncoding),
    }
}
fn id(value: u64) -> Result<PolicyId, SidecarError> {
    PolicyId::new(value).map_err(|_| SidecarError::InvalidObservation)
}
fn read_policy(r: &mut Reader<'_>) -> Result<HostPolicyDecision, SidecarError> {
    let tag = r.u8()?;
    let subject = r.u64()?;
    let actor = r.u64()?;
    let source = r.u64()?;
    let operation = r.u64()?;
    let modality = r.u64()?;
    let context = r.u64()?;
    let purpose = r.u64()?;
    let revocation = r.u64()?;
    let evidence = r.bytes::<32>()?;
    let outcome_tag = r.u8()?;
    let commitment = r.bytes::<32>()?;
    if tag == 0 {
        return Ok(HostPolicyDecision::NotEvaluated);
    }
    if tag != 1 {
        return Err(SidecarError::InvalidEncoding);
    }
    let request = PolicyRequest::new(
        optional_id(subject).map(id).transpose()?,
        optional_id(actor).map(id).transpose()?,
        id(source)?,
        id(operation)?,
        id(modality)?,
        id(context)?,
        id(purpose)?,
        id(revocation)?,
        EvidenceCommitment::new(evidence).map_err(|_| SidecarError::InvalidObservation)?,
    );
    let outcome = match outcome_tag {
        1 => PolicyOutcome::Permit(
            DecisionCommitment::new(commitment).map_err(|_| SidecarError::InvalidObservation)?,
        ),
        2 => PolicyOutcome::Deny,
        3 => PolicyOutcome::Unknown,
        _ => return Err(SidecarError::InvalidEncoding),
    };
    Ok(HostPolicyDecision::Evaluated { request, outcome })
}
