// Fixed-size caller-buffered serialization for measurement index records.

use super::super::MeasurementModality;
use super::{
    CalibrationDeclaration, MeasurementQuality, MeasurementRecord, PayloadReference,
    ProjectionDeclaration, ProvenanceDeclaration, RecordClockStamp, RecordError,
    MAX_MEASUREMENT_INDEX_RECORDS, MEASUREMENT_INDEX_RECORD_SIZE, MEASUREMENT_INDEX_VERSION,
};

const RECORD_MAGIC: [u8; 4] = *b"SMR1";

/// Serialize records into a caller-owned byte slice as fixed 264-byte records.
///
/// The encoding has a per-record magic/version/size header and explicit
/// little-endian fields. Required references are copied verbatim; absent
/// optional references use explicit presence bytes plus zero padding. The
/// function checks all capacity and duplicate-ID failures before writing.
pub fn encode_measurement_index_records(
    records: &[MeasurementRecord],
    out: &mut [u8],
) -> Result<usize, RecordError> {
    if records.len() > MAX_MEASUREMENT_INDEX_RECORDS {
        return Err(RecordError::TooManyRecords);
    }
    for i in 0..records.len() {
        for j in (i + 1)..records.len() {
            if records[i].record_id == records[j].record_id {
                return Err(RecordError::DuplicateRecordId);
            }
        }
    }
    let needed = records
        .len()
        .checked_mul(MEASUREMENT_INDEX_RECORD_SIZE)
        .ok_or(RecordError::RangeOverflow)?;
    if out.len() < needed {
        return Err(RecordError::OutputTooSmall);
    }

    for (index, record) in records.iter().enumerate() {
        let base = index * MEASUREMENT_INDEX_RECORD_SIZE;
        encode_one(record, &mut out[base..base + MEASUREMENT_INDEX_RECORD_SIZE]);
    }
    Ok(needed)
}

fn encode_one(record: &MeasurementRecord, out: &mut [u8]) {
    let mut cursor = 0usize;
    write_bytes(out, &mut cursor, &RECORD_MAGIC);
    write_u16(out, &mut cursor, MEASUREMENT_INDEX_VERSION);
    write_u16(out, &mut cursor, MEASUREMENT_INDEX_RECORD_SIZE as u16);
    write_u64(out, &mut cursor, record.record_id);
    write_u64(out, &mut cursor, record.source_id.value().unwrap_or(0));
    write_u64(out, &mut cursor, record.physical.field_id());
    write_u64(out, &mut cursor, record.physical.field_name_hash());
    write_u16(out, &mut cursor, record.physical.typed_field_kind());
    write_u8(out, &mut cursor, record.physical.field_type() as u8);
    write_u8(out, &mut cursor, record.physical.support() as u8);
    write_u8(out, &mut cursor, record.physical.representation() as u8);
    let (modality_tag, modality_id) = modality_parts(record.physical.modality());
    write_u8(out, &mut cursor, modality_tag);
    write_u64(out, &mut cursor, modality_id);
    write_u64(
        out,
        &mut cursor,
        record.physical.physical_quantity().value().unwrap_or(0),
    );
    write_u64(out, &mut cursor, record.physical.legacy_unit_hash());
    write_u64(
        out,
        &mut cursor,
        record.physical.typed_unit_id().value().unwrap_or(0),
    );
    write_payload_ref(out, &mut cursor, record.raw_payload);
    write_provenance(out, &mut cursor, record.provenance);
    write_calibration(out, &mut cursor, record.calibration);
    write_quality(out, &mut cursor, record.quality);
    write_optional_clock(out, &mut cursor, record.timestamp);
    write_projection(out, &mut cursor, record.projection);
    // Fixed-size record ends with zeroed reserved bytes for forward-compatible
    // readers. Cursor accounting is asserted in tests against the wire size.
    out[cursor..].fill(0);
}

fn modality_parts(modality: MeasurementModality) -> (u8, u64) {
    match modality {
        MeasurementModality::Unknown => (0, 0),
        MeasurementModality::Electromagnetic => (1, 0),
        MeasurementModality::Optical => (2, 0),
        MeasurementModality::Acoustic => (3, 0),
        MeasurementModality::Inertial => (4, 0),
        MeasurementModality::Thermal => (5, 0),
        MeasurementModality::Pressure => (6, 0),
        MeasurementModality::Registered(id) => (7, id.get()),
    }
}

fn write_payload_ref(out: &mut [u8], cursor: &mut usize, reference: PayloadReference) {
    write_bytes(out, cursor, &reference.container_id);
    write_u32(out, cursor, reference.section_id);
    write_u64(out, cursor, reference.offset);
    write_u64(out, cursor, reference.length);
}

fn write_provenance(out: &mut [u8], cursor: &mut usize, provenance: ProvenanceDeclaration) {
    match provenance {
        ProvenanceDeclaration::Unknown => {
            write_u8(out, cursor, 0);
            write_bytes(out, cursor, &[0; 36]);
        }
        ProvenanceDeclaration::NotAttached => {
            write_u8(out, cursor, 1);
            write_bytes(out, cursor, &[0; 36]);
        }
        ProvenanceDeclaration::Reference(reference) => {
            write_u8(out, cursor, 2);
            write_payload_ref(out, cursor, reference);
        }
    }
}

fn write_calibration(out: &mut [u8], cursor: &mut usize, state: CalibrationDeclaration) {
    let (tag, artifact) = match state {
        CalibrationDeclaration::Unknown => (0, [0; 32]),
        CalibrationDeclaration::Missing => (1, [0; 32]),
        CalibrationDeclaration::Expired => (2, [0; 32]),
        CalibrationDeclaration::OutOfRange => (3, [0; 32]),
        CalibrationDeclaration::Uncalibrated => (4, [0; 32]),
        CalibrationDeclaration::Calibrated(reference) => (5, reference.0),
    };
    write_u8(out, cursor, tag);
    write_bytes(out, cursor, &artifact);
}

fn write_quality(out: &mut [u8], cursor: &mut usize, quality: MeasurementQuality) {
    match quality {
        MeasurementQuality::Unknown => {
            write_u8(out, cursor, 0);
            write_u32(out, cursor, 0);
            write_u32(out, cursor, 0);
        }
        MeasurementQuality::Known(metrics) => {
            write_u8(out, cursor, 1);
            write_u32(out, cursor, metrics.confidence_millionths);
            write_u32(out, cursor, metrics.relative_uncertainty_millionths);
        }
    }
}

fn write_optional_clock(out: &mut [u8], cursor: &mut usize, stamp: Option<RecordClockStamp>) {
    match stamp {
        Some(stamp) => {
            write_u8(out, cursor, 1);
            write_u64(out, cursor, stamp.clock_id);
            write_u32(out, cursor, stamp.restart_epoch);
            write_u64(out, cursor, stamp.ticks_ns);
            write_u64(out, cursor, stamp.uncertainty_ns);
        }
        None => {
            write_u8(out, cursor, 0);
            write_bytes(out, cursor, &[0; 28]);
        }
    }
}

fn write_projection(out: &mut [u8], cursor: &mut usize, projection: ProjectionDeclaration) {
    match projection {
        ProjectionDeclaration::Unknown => {
            write_u8(out, cursor, 0);
            write_bytes(out, cursor, &[0; 32]);
        }
        ProjectionDeclaration::NotProjected => {
            write_u8(out, cursor, 1);
            write_bytes(out, cursor, &[0; 32]);
        }
        ProjectionDeclaration::Reference(projection) => {
            write_u8(out, cursor, 2);
            write_u64(out, cursor, projection.projection_record_id);
            write_u64(out, cursor, projection.profile_id.get());
            write_u64(out, cursor, projection.domain_id.get());
            write_u64(out, cursor, projection.coordinate_convention_id.get());
        }
    }
}

fn write_u8(out: &mut [u8], cursor: &mut usize, value: u8) {
    out[*cursor] = value;
    *cursor += 1;
}

fn write_u16(out: &mut [u8], cursor: &mut usize, value: u16) {
    write_bytes(out, cursor, &value.to_le_bytes());
}

fn write_u32(out: &mut [u8], cursor: &mut usize, value: u32) {
    write_bytes(out, cursor, &value.to_le_bytes());
}

fn write_u64(out: &mut [u8], cursor: &mut usize, value: u64) {
    write_bytes(out, cursor, &value.to_le_bytes());
}

fn write_bytes(out: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    let end = *cursor + bytes.len();
    out[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
}
