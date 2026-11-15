//! Decode one index record only against a separately resolved field payload.
use super::super::SemanticId;
use super::{
    encode_measurement_index_records, CalibrationArtifactReference, CalibrationDeclaration,
    MeasurementQuality, MeasurementRecord, PayloadReference, ProjectionDeclaration,
    ProjectionReference, ProvenanceDeclaration, QualityMetrics, RecordClockStamp, RecordError,
    ResolvedFieldPayloadBinding, MEASUREMENT_INDEX_RECORD_SIZE,
};

/// Decode canonical bytes and bind the encoded physical field and raw payload
/// to a caller-resolved field. This does not authenticate the container,
/// establish source trust, or validate the referenced artifacts.
pub fn decode_measurement_index_record(
    bytes: &[u8],
    binding: ResolvedFieldPayloadBinding,
) -> Result<MeasurementRecord, RecordError> {
    if bytes.len() != MEASUREMENT_INDEX_RECORD_SIZE {
        return Err(RecordError::InvalidEncoding);
    }
    let record_id = u64_at(bytes, 8);
    let source_id = SemanticId::from_optional(u64_at(bytes, 16));
    let provenance = match bytes[114] {
        0 => ProvenanceDeclaration::Unknown,
        1 => ProvenanceDeclaration::NotAttached,
        2 => ProvenanceDeclaration::Reference(payload_at(bytes, 115)?),
        _ => return Err(RecordError::InvalidEncoding),
    };
    let calibration = match bytes[151] {
        0 => CalibrationDeclaration::Unknown,
        1 => CalibrationDeclaration::Missing,
        2 => CalibrationDeclaration::Expired,
        3 => CalibrationDeclaration::OutOfRange,
        4 => CalibrationDeclaration::Uncalibrated,
        5 => {
            let mut artifact = [0; 32];
            artifact.copy_from_slice(&bytes[152..184]);
            CalibrationDeclaration::Calibrated(CalibrationArtifactReference::new(artifact)?)
        }
        _ => return Err(RecordError::InvalidEncoding),
    };
    let quality = match bytes[184] {
        0 => MeasurementQuality::Unknown,
        1 => {
            MeasurementQuality::Known(QualityMetrics::new(u32_at(bytes, 185), u32_at(bytes, 189))?)
        }
        _ => return Err(RecordError::InvalidEncoding),
    };
    let timestamp = match bytes[193] {
        0 => None,
        1 => Some(RecordClockStamp::new(
            u64_at(bytes, 194),
            u32_at(bytes, 202),
            u64_at(bytes, 206),
            u64_at(bytes, 214),
        )?),
        _ => return Err(RecordError::InvalidEncoding),
    };
    let projection = match bytes[222] {
        0 => ProjectionDeclaration::Unknown,
        1 => ProjectionDeclaration::NotProjected,
        2 => ProjectionDeclaration::Reference(ProjectionReference::new(
            u64_at(bytes, 223),
            u64_at(bytes, 231),
            u64_at(bytes, 239),
            u64_at(bytes, 247),
        )?),
        _ => return Err(RecordError::InvalidEncoding),
    };
    let record = MeasurementRecord::new(
        record_id,
        source_id,
        binding,
        provenance,
        calibration,
        quality,
        timestamp,
        projection,
    )?;
    // Canonical re-encoding also compares the caller-resolved physical field,
    // raw payload, magic/version/size, absent-field padding and reserved bytes.
    let mut canonical = [0u8; MEASUREMENT_INDEX_RECORD_SIZE];
    encode_measurement_index_records(core::slice::from_ref(&record), &mut canonical)?;
    if canonical != bytes {
        return Err(RecordError::InvalidEncoding);
    }
    Ok(record)
}

fn payload_at(bytes: &[u8], offset: usize) -> Result<PayloadReference, RecordError> {
    let mut container_id = [0; 16];
    container_id.copy_from_slice(&bytes[offset..offset + 16]);
    PayloadReference::new(
        container_id,
        u32_at(bytes, offset + 16),
        u64_at(bytes, offset + 20),
        u64_at(bytes, offset + 28),
    )
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
