use super::super::record::{
    encode_measurement_index_records, MeasurementRecord, MAX_MEASUREMENT_INDEX_RECORDS,
    MEASUREMENT_INDEX_RECORD_SIZE,
};
use super::{validate_container, validate_kind, SensingContainerError};
use crate::container_10d::{SectionType, TypedFieldRecord};

/// The declaration to add to MID2 and total FieldSidecar length. Pass the
/// resulting payload and extension kind to the existing extension-aware
/// container writer; retain the original raw-field declarations unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedSensingIndex {
    pub index_field: TypedFieldRecord,
    pub field_section_bytes: usize,
}

/// Append canonical index rows to an unchanged raw FieldSidecar prefix.
/// `output` is caller-owned. All refusals precede output modification. Raw
/// FieldSidecar references must remain entirely inside the prefix; references
/// to other sections require validation by the enclosing container writer.
/// This function does not assign or authenticate the container identifier.
pub fn pack_sensing_index_field(
    container_id: [u8; 16],
    index_field_id: u64,
    extension_kind: u16,
    raw_field_prefix: &[u8],
    records: &[MeasurementRecord],
    output: &mut [u8],
) -> Result<PackedSensingIndex, SensingContainerError> {
    validate_container(&container_id)?;
    validate_kind(extension_kind)?;
    if index_field_id == 0 {
        return Err(SensingContainerError::InvalidFieldId);
    }
    if records.is_empty() || records.len() > MAX_MEASUREMENT_INDEX_RECORDS {
        return Err(SensingContainerError::InvalidIndex);
    }
    let index_len = records
        .len()
        .checked_mul(MEASUREMENT_INDEX_RECORD_SIZE)
        .ok_or(SensingContainerError::Range)?;
    let total = raw_field_prefix
        .len()
        .checked_add(index_len)
        .ok_or(SensingContainerError::Range)?;
    if output.len() < total {
        return Err(SensingContainerError::Capacity);
    }
    for (i, record) in records.iter().enumerate() {
        if records[..i]
            .iter()
            .any(|other| other.record_id() == record.record_id())
        {
            return Err(SensingContainerError::DuplicateRecordId);
        }
        if record.physical().field_id() == index_field_id {
            return Err(SensingContainerError::IndexPayloadOverlap);
        }
        let raw = record.raw_payload();
        if raw.container_id() != container_id {
            return Err(SensingContainerError::ContainerMismatch);
        }
        if raw.section_id() == SectionType::FieldSidecar as u32
            && raw
                .offset()
                .checked_add(raw.length())
                .filter(|end| *end <= raw_field_prefix.len() as u64)
                .is_none()
        {
            return Err(SensingContainerError::IndexPayloadOverlap);
        }
    }
    let field = TypedFieldRecord {
        field_id: index_field_id,
        kind: extension_kind,
        flags: 0,
        unit_id: 0,
        payload_section_id: SectionType::FieldSidecar as u32,
        payload_offset: raw_field_prefix.len() as u64,
        payload_length: index_len as u64,
    };
    // Encoder preflight above exhausts its capacity/duplicate/count refusals.
    encode_measurement_index_records(records, &mut output[raw_field_prefix.len()..total])
        .map_err(SensingContainerError::Record)?;
    output[..raw_field_prefix.len()].copy_from_slice(raw_field_prefix);
    Ok(PackedSensingIndex {
        index_field: field,
        field_section_bytes: total,
    })
}
