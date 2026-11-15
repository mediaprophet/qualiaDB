use super::super::record::MeasurementRecord;
use crate::{
    frame_layout::{self, INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER, INLINE_VALUE_MASK},
    NQuin,
};

/// Proposed predicates; a host ontology must explicitly adopt their meaning.
pub mod proposed_predicates {
    pub const RECORD_ID: u64 = crate::q_hash("urn:qualia:proposed:sensing:measurementRecordId");
    pub const FIELD_ID: u64 = crate::q_hash("urn:qualia:proposed:sensing:measurementFieldId");
    pub const FIELD_KIND: u64 = crate::q_hash("urn:qualia:proposed:sensing:measurementFieldKind");
    pub const PAYLOAD_REFERENCE: u64 =
        crate::q_hash("urn:qualia:proposed:sensing:measurementPayloadReference");
    pub const SOURCE_KNOWN: u64 =
        crate::q_hash("urn:qualia:proposed:sensing:measurementSourceKnown");
    pub const SOURCE_ID: u64 = crate::q_hash("urn:qualia:proposed:sensing:measurementSourceId");
}

pub const MAX_MEASUREMENT_LINK_QUINS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementQuinError {
    InvalidMapping,
    IdentifierOutOfRange,
    RecordMismatch,
    OutputTooSmall,
    VocabularyCollision,
}

/// Explicit record-to-event mapping. The payload IRI is a host-registered
/// 60-bit token, not a pointer or a digest inferred from the container bytes.
/// The host must bind it to this record's exact payload reference and policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasurementQuinMapping {
    record_id: u64,
    subject: u64,
    context: u64,
    payload_iri_token: u64,
}

impl MeasurementQuinMapping {
    pub const fn new(
        record_id: u64,
        subject: u64,
        context: u64,
        payload_iri_token: u64,
    ) -> Result<Self, MeasurementQuinError> {
        if record_id == 0 || subject == 0 || context == 0 || payload_iri_token == 0 {
            return Err(MeasurementQuinError::InvalidMapping);
        }
        if record_id > INLINE_VALUE_MASK || payload_iri_token > INLINE_VALUE_MASK {
            return Err(MeasurementQuinError::IdentifierOutOfRange);
        }
        Ok(Self {
            record_id,
            subject,
            context,
            payload_iri_token,
        })
    }
}

/// Write links only, with zero metadata and parity valid under both current
/// conventions. Unknown source produces an explicit false flag and no source
/// ID assertion. Every validation precedes output changes; no ID is truncated.
pub fn measurement_record_to_quins(
    record: MeasurementRecord,
    mapping: MeasurementQuinMapping,
    output: &mut [NQuin],
) -> Result<usize, MeasurementQuinError> {
    use proposed_predicates as p;
    if record.record_id() != mapping.record_id {
        return Err(MeasurementQuinError::RecordMismatch);
    }
    let field_id = record.physical().field_id();
    let source_id = record.source_id().value();
    if field_id > INLINE_VALUE_MASK || source_id.is_some_and(|id| id > INLINE_VALUE_MASK) {
        return Err(MeasurementQuinError::IdentifierOutOfRange);
    }
    let predicates = [
        p::RECORD_ID,
        p::FIELD_ID,
        p::FIELD_KIND,
        p::PAYLOAD_REFERENCE,
        p::SOURCE_KNOWN,
        p::SOURCE_ID,
    ];
    for i in 0..predicates.len() {
        if predicates[..i].contains(&predicates[i]) {
            return Err(MeasurementQuinError::VocabularyCollision);
        }
    }
    let count = if source_id.is_some() { 6 } else { 5 };
    if output.len() < count {
        return Err(MeasurementQuinError::OutputTooSmall);
    }
    let objects = [
        INLINE_TAG_INTEGER | mapping.record_id,
        INLINE_TAG_INTEGER | field_id,
        INLINE_TAG_INTEGER | u64::from(record.physical().typed_field_kind()),
        mapping.payload_iri_token,
        INLINE_TAG_BOOLEAN | u64::from(source_id.is_some()),
        INLINE_TAG_INTEGER | source_id.unwrap_or(0),
    ];
    let mut staged = [NQuin::default(); MAX_MEASUREMENT_LINK_QUINS];
    for i in 0..count {
        staged[i] = frame_layout::sealed(NQuin {
            subject: mapping.subject,
            predicate: predicates[i],
            object: objects[i],
            context: mapping.context,
            metadata: 0,
            parity: 0,
        });
    }
    output[..count].copy_from_slice(&staged[..count]);
    Ok(count)
}
