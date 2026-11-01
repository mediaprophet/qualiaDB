use super::{cursor::Writer, SidecarError, UntrustedSidecar, MAGIC, SIDECAR_SIZE};
use crate::BridgedObservation;
use etalon_sense_core::{
    evidence::{ArtifactDigest, DigestAlgorithm, EvidenceOrigin},
    observation::{
        HostPolicyDecision, MeasurementUncertainty, ObservationEnvelope, ObservationKind,
        ObservationTimeAnchor,
    },
    policy::PolicyOutcome,
    quality::{CalibrationState, QualityRecord, SourceKind},
};
use qualia_core_db::sensing::record::{
    encode_measurement_index_records, MeasurementRecord, MEASUREMENT_INDEX_RECORD_SIZE,
};

// Header, index and the fixed-width observation fields; remaining bytes are zero.
const ENCODED_PREFIX_SIZE: usize = 670;
const _: () = assert!(ENCODED_PREFIX_SIZE <= SIDECAR_SIZE);

/// Write one canonical, fixed-size sidecar; output is unchanged on failure.
/// Storage does not authenticate it or preserve a current authorization.
pub fn encode_sidecar(value: &BridgedObservation, out: &mut [u8]) -> Result<usize, SidecarError> {
    let verified = matches!(value.observation().evidence(), EvidenceOrigin::Verified(_));
    let bytes = encode_parts(value.record(), value.observation(), verified)?;
    if out.len() < SIDECAR_SIZE {
        return Err(SidecarError::BufferSize);
    }
    out[..SIDECAR_SIZE].copy_from_slice(&bytes);
    Ok(SIDECAR_SIZE)
}

/// Re-encode historical declarations without promoting them to trusted state.
pub fn reencode_untrusted_sidecar(
    value: &UntrustedSidecar,
    out: &mut [u8],
) -> Result<usize, SidecarError> {
    let bytes = encode_parts(
        &value.record,
        &value.observation,
        value.evidence_verification_claimed,
    )?;
    if out.len() < SIDECAR_SIZE {
        return Err(SidecarError::BufferSize);
    }
    out[..SIDECAR_SIZE].copy_from_slice(&bytes);
    Ok(SIDECAR_SIZE)
}

pub(super) fn encode_parts(
    record: &MeasurementRecord,
    observation: &ObservationEnvelope,
    verified_claim: bool,
) -> Result<[u8; SIDECAR_SIZE], SidecarError> {
    let mut w = Writer::new();
    w.bytes(MAGIC);
    w.bytes(&1u16.to_le_bytes());
    w.bytes(&(SIDECAR_SIZE as u16).to_le_bytes());
    let mut record_bytes = [0; MEASUREMENT_INDEX_RECORD_SIZE];
    encode_measurement_index_records(core::slice::from_ref(record), &mut record_bytes)
        .map_err(SidecarError::Record)?;
    w.bytes(&record_bytes);
    w.u8(match observation.kind() {
        ObservationKind::LiveFrame => 1,
        ObservationKind::RecordedFrame => 2,
        ObservationKind::SyntheticFrame => 3,
        ObservationKind::CachedQuery => 4,
        ObservationKind::HostDerived => 5,
    });
    let source = observation.source();
    w.u8(match source.kind() {
        SourceKind::Synthetic => 1,
        SourceKind::Recorded => 2,
        SourceKind::Live => 3,
    });
    w.u64(source.source_id());
    w.u64(observation.stream_id().map_or(0, |id| id.get()));
    w.u32(observation.generation().map_or(0, |id| id.get()));
    optional_u64(&mut w, observation.sequence());
    match observation.timestamp() {
        Some(stamp) => {
            w.u8(1);
            w.u64(stamp.clock_id());
            w.u32(stamp.restart_epoch());
            w.u64(stamp.ticks_ns());
            w.u64(stamp.uncertainty_ns());
        }
        None => {
            w.u8(0);
            w.bytes(&[0; 28]);
        }
    }
    w.u8(match observation.time_anchor() {
        ObservationTimeAnchor::Unknown => 0,
        ObservationTimeAnchor::FirstSample => 1,
        ObservationTimeAnchor::LastSample => 2,
        ObservationTimeAnchor::Midpoint => 3,
        ObservationTimeAnchor::ReadCompletion => 4,
        ObservationTimeAnchor::NativeEvent => 5,
    });
    let units = observation.quantity_unit();
    w.u64(units.quantity_id().map_or(0, |id| id.get()));
    w.u64(units.unit_id().map_or(0, |id| id.get()));
    match observation.uncertainty() {
        MeasurementUncertainty::Unknown => {
            w.u8(0);
            w.bytes(&[0; 24]);
        }
        MeasurementUncertainty::AbsoluteBound { magnitude, unit_id } => {
            w.u8(1);
            w.u64(magnitude.to_bits());
            w.u64(0);
            w.u64(unit_id.map_or(0, |v| v.get()));
        }
        MeasurementUncertainty::RelativeBound { fraction } => {
            w.u8(2);
            w.u64(fraction.to_bits());
            w.u64(0);
            w.u64(0);
        }
        MeasurementUncertainty::Interval {
            lower,
            upper,
            unit_id,
        } => {
            w.u8(3);
            w.u64(lower.to_bits());
            w.u64(upper.to_bits());
            w.u64(unit_id.map_or(0, |v| v.get()));
        }
    }
    match observation.quality() {
        QualityRecord::Unknown => {
            w.u8(0);
            w.bytes(&[0; 8]);
        }
        QualityRecord::Known(q) => {
            w.u8(1);
            w.u32(q.confidence().to_bits());
            w.u32(q.relative_uncertainty().to_bits());
        }
    }
    match observation.calibration().state() {
        CalibrationState::Missing => {
            w.u8(1);
            w.bytes(&[0; 32]);
        }
        CalibrationState::Expired => {
            w.u8(2);
            w.bytes(&[0; 32]);
        }
        CalibrationState::OutOfRange => {
            w.u8(3);
            w.bytes(&[0; 32]);
        }
        CalibrationState::Uncalibrated => {
            w.u8(4);
            w.bytes(&[0; 32]);
        }
        CalibrationState::Calibrated { artifact_id } => {
            w.u8(5);
            w.bytes(&artifact_id.bytes());
        }
    }
    let (tag, assertion, authority, signed, signature) = match observation.evidence() {
        EvidenceOrigin::Unknown => (0, 0, None, None, None),
        EvidenceOrigin::Asserted {
            assertion_id: 0, ..
        } => return Err(SidecarError::InvalidObservation),
        EvidenceOrigin::Asserted { assertion_id, .. } => (1, assertion_id, None, None, None),
        EvidenceOrigin::SignaturePresented {
            authority,
            signed_digest,
            signature_digest,
            ..
        } => (
            if verified_claim { 3 } else { 2 },
            0,
            Some(authority),
            Some(signed_digest),
            Some(signature_digest),
        ),
        EvidenceOrigin::Verified(v) => (
            3,
            0,
            Some(v.authority()),
            Some(v.signed_digest()),
            Some(v.signature_digest()),
        ),
    };
    if verified_claim && tag != 3 {
        return Err(SidecarError::InvalidObservation);
    }
    w.u8(tag);
    w.u64(assertion);
    w.u64(authority.map_or(0, |a| a.issuer_id()));
    w.u64(authority.map_or(0, |a| a.key_id()));
    digest(&mut w, signed);
    digest(&mut w, signature);
    digest(&mut w, observation.payload_digest());
    match observation.policy() {
        HostPolicyDecision::NotEvaluated => {
            w.u8(0);
            w.bytes(&[0; 129]);
        }
        HostPolicyDecision::Evaluated { request, outcome } => {
            w.u8(1);
            w.u64(request.subject().map_or(0, |v| v.get()));
            w.u64(request.actor().map_or(0, |v| v.get()));
            w.u64(request.source().get());
            w.u64(request.operation().get());
            w.u64(request.modality().get());
            w.u64(request.context().get());
            w.u64(request.purpose().get());
            w.u64(request.revocation_authority().get());
            w.bytes(request.evidence().as_bytes());
            match outcome {
                PolicyOutcome::Permit(value) => {
                    w.u8(1);
                    w.bytes(value.as_bytes());
                }
                PolicyOutcome::Deny => {
                    w.u8(2);
                    w.bytes(&[0; 32]);
                }
                PolicyOutcome::Unknown => {
                    w.u8(3);
                    w.bytes(&[0; 32]);
                }
            }
        }
    }
    w.finish()
}

fn optional_u64(w: &mut Writer, value: Option<u64>) {
    match value {
        Some(value) => {
            w.u8(1);
            w.u64(value);
        }
        None => {
            w.u8(0);
            w.u64(0);
        }
    }
}
fn digest(w: &mut Writer, value: Option<ArtifactDigest>) {
    match value {
        None => {
            w.u8(0);
            w.bytes(&[0; 32]);
        }
        Some(value) => {
            w.u8(match value.algorithm() {
                DigestAlgorithm::Sha256 => 1,
                DigestAlgorithm::Blake3 => 2,
            });
            w.bytes(&value.bytes());
        }
    }
}
