use super::{
    encode_measurement_index_records, CalibrationArtifactReference, CalibrationDeclaration,
    MeasurementQuality, MeasurementRecord, PayloadReference, ProjectionDeclaration,
    ProjectionReference, ProvenanceDeclaration, QualityMetrics, RecordClockStamp, RecordError,
    ResolvedFieldPayloadBinding, MAX_MEASUREMENT_INDEX_RECORDS, MEASUREMENT_INDEX_RECORD_SIZE,
    MEASUREMENT_INDEX_VERSION,
};
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
    typed_payloads::TypedPayload,
};
use crate::sensing::{MeasurementModality, PhysicalFieldSemantics, SemanticId};

fn typed() -> TypedFieldRecord {
    TypedFieldRecord {
        field_id: 9,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 11,
        payload_section_id: 2,
        payload_offset: 0,
        payload_length: 16,
    }
}

fn physical() -> PhysicalFieldSemantics {
    let field = FieldDescriptor {
        name_hash: 0x1122,
        unit_hash: 0x3344,
        type_kind: FieldTypeKind::Spectral as u8,
        support: FieldSupportKind::Stream as u8,
        representation: FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let typed = typed();
    PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(13).unwrap(),
    )
    .unwrap()
}

fn field_payload_binding() -> ResolvedFieldPayloadBinding {
    let bytes = [0u8; 16];
    ResolvedFieldPayloadBinding::bind_payload(
        physical(),
        TypedPayload {
            record: typed(),
            bytes: &bytes,
        },
        [1; 16],
    )
    .unwrap()
}

fn provenance_ref() -> PayloadReference {
    PayloadReference::new([2; 16], 4, 16, 80).unwrap()
}

fn record(record_id: u64) -> MeasurementRecord {
    MeasurementRecord::new(
        record_id,
        SemanticId::new(17).unwrap(),
        field_payload_binding(),
        ProvenanceDeclaration::Reference(provenance_ref()),
        CalibrationDeclaration::Calibrated(CalibrationArtifactReference::new([3; 32]).unwrap()),
        MeasurementQuality::Known(QualityMetrics::new(900_000, 25_000).unwrap()),
        Some(RecordClockStamp::new(5, 2, 1000, 4).unwrap()),
        ProjectionDeclaration::Reference(ProjectionReference::new(6, 7, 8, 9).unwrap()),
    )
    .unwrap()
}

#[test]
fn fixed_little_endian_record_preserves_semantics_and_references() {
    let records = [record(21)];
    let mut output = [0xAA; MEASUREMENT_INDEX_RECORD_SIZE + 3];
    let written = encode_measurement_index_records(&records, &mut output).unwrap();
    assert_eq!(written, MEASUREMENT_INDEX_RECORD_SIZE);
    assert_eq!(&output[0..4], b"SMR1");
    assert_eq!(
        u16::from_le_bytes([output[4], output[5]]),
        MEASUREMENT_INDEX_VERSION
    );
    assert_eq!(
        u16::from_le_bytes([output[6], output[7]]) as usize,
        MEASUREMENT_INDEX_RECORD_SIZE
    );
    assert_eq!(u64::from_le_bytes(output[8..16].try_into().unwrap()), 21);
    assert_eq!(u64::from_le_bytes(output[24..32].try_into().unwrap()), 9); // field_id
    assert_eq!(
        u64::from_le_bytes(output[32..40].try_into().unwrap()),
        0x1122
    );
    assert_eq!(
        u16::from_le_bytes([output[40], output[41]]),
        FIELD_KIND_ELECTROMAGNETIC
    );
    assert_eq!(output[45], 1); // Electromagnetic modality tag
    assert_eq!(u64::from_le_bytes(output[54..62].try_into().unwrap()), 13); // physical quantity
    assert_eq!(
        u64::from_le_bytes(output[62..70].try_into().unwrap()),
        0x3344
    ); // legacy unit hash
    assert_eq!(u64::from_le_bytes(output[70..78].try_into().unwrap()), 11); // MID2 unit id
    assert_eq!(&output[78..94], &[1; 16]); // resolver-bound container reference
    assert_eq!(u32::from_le_bytes(output[94..98].try_into().unwrap()), 2);
    assert_eq!(u64::from_le_bytes(output[98..106].try_into().unwrap()), 0);
    assert_eq!(u64::from_le_bytes(output[106..114].try_into().unwrap()), 16);
    assert_eq!(output[114], 2); // provenance reference present
    assert_eq!(&output[115..131], &[2; 16]);
    assert_eq!(output[151], 5); // calibrated declaration, not proof
    assert_eq!(&output[152..184], &[3; 32]);
    assert_eq!(output[184], 1); // known quality
    assert_eq!(output[193], 1); // timestamp present
    assert_eq!(output[222], 2); // projection is a separate record reference
    assert_eq!(u64::from_le_bytes(output[223..231].try_into().unwrap()), 6);
    assert!(output[255..MEASUREMENT_INDEX_RECORD_SIZE]
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(&output[MEASUREMENT_INDEX_RECORD_SIZE..], &[0xAA; 3]);
}

#[test]
fn unknown_optional_references_have_explicit_wire_tags() {
    let unknown = MeasurementRecord::new(
        1,
        SemanticId::Unknown,
        field_payload_binding(),
        ProvenanceDeclaration::Unknown,
        CalibrationDeclaration::Unknown,
        MeasurementQuality::Unknown,
        None,
        ProjectionDeclaration::Unknown,
    )
    .unwrap();
    let mut output = [0xAA; MEASUREMENT_INDEX_RECORD_SIZE];
    encode_measurement_index_records(&[unknown], &mut output).unwrap();
    assert_eq!(u64::from_le_bytes(output[16..24].try_into().unwrap()), 0); // unknown source
    assert_eq!(output[114], 0); // no provenance link
    assert_eq!(&output[115..151], &[0; 36]);
    assert_eq!(output[151], 0); // unknown calibration
    assert_eq!(&output[152..184], &[0; 32]);
    assert_eq!(output[184], 0); // unknown quality, not nominal quality
    assert_eq!(&output[185..193], &[0; 8]);
    assert_eq!(output[193], 0); // no clock stamp
    assert_eq!(&output[194..222], &[0; 28]);
    assert_eq!(output[222], 0); // no projection
    assert_eq!(&output[223..255], &[0; 32]);
}

#[test]
fn confirmed_absence_and_unknown_optional_references_are_distinct() {
    let not_attached = MeasurementRecord::new(
        2,
        SemanticId::Unknown,
        field_payload_binding(),
        ProvenanceDeclaration::NotAttached,
        CalibrationDeclaration::Missing,
        MeasurementQuality::Unknown,
        None,
        ProjectionDeclaration::NotProjected,
    )
    .unwrap();
    let mut output = [0; MEASUREMENT_INDEX_RECORD_SIZE];
    encode_measurement_index_records(&[not_attached], &mut output).unwrap();
    assert_eq!(output[114], 1); // provenance known not attached
    assert_eq!(output[151], 1); // calibration known missing
    assert_eq!(output[222], 1); // projection known not performed
}

#[test]
fn validation_rejects_invalid_refs_calibration_time_and_quality() {
    assert_eq!(
        PayloadReference::new([0; 16], 1, 0, 1),
        Err(RecordError::InvalidPayloadReference)
    );
    assert_eq!(
        PayloadReference::new([1; 16], 1, u64::MAX, 1),
        Err(RecordError::RangeOverflow)
    );
    assert_eq!(
        CalibrationArtifactReference::new([0; 32]),
        Err(RecordError::InvalidCalibration)
    );
    assert_eq!(
        RecordClockStamp::new(0, 0, 0, 0),
        Err(RecordError::InvalidIdentifier)
    );
    assert_eq!(
        RecordClockStamp::new(1, 0, u64::MAX, 1),
        Err(RecordError::InvalidTime)
    );
    assert_eq!(
        QualityMetrics::new(1_000_001, 0),
        Err(RecordError::InvalidQuality)
    );
    assert_eq!(
        ProjectionReference::new(1, 2, 0, 4),
        Err(RecordError::InvalidIdentifier)
    );
}

#[test]
fn caller_buffer_failures_and_duplicate_ids_are_transactional() {
    let records = [record(1), record(1)];
    let mut duplicate_out = [0x5A; MEASUREMENT_INDEX_RECORD_SIZE * 2];
    let before = duplicate_out;
    assert_eq!(
        encode_measurement_index_records(&records, &mut duplicate_out),
        Err(RecordError::DuplicateRecordId)
    );
    assert_eq!(duplicate_out, before);

    let mut short = [0xA5; MEASUREMENT_INDEX_RECORD_SIZE - 1];
    let before = short;
    assert_eq!(
        encode_measurement_index_records(&[record(2)], &mut short),
        Err(RecordError::OutputTooSmall)
    );
    assert_eq!(short, before);
}

#[test]
fn record_batch_limit_is_explicit() {
    assert_eq!(MAX_MEASUREMENT_INDEX_RECORDS, 256);
}

#[test]
fn payload_binding_rejects_unrelated_field_or_range() {
    let bytes = [0u8; 16];
    let mut unrelated_field = typed();
    unrelated_field.field_id = 10;
    assert_eq!(
        ResolvedFieldPayloadBinding::bind_payload(
            physical(),
            TypedPayload {
                record: unrelated_field,
                bytes: &bytes
            },
            [1; 16],
        ),
        Err(RecordError::FieldPayloadMismatch)
    );

    let mut unrelated_kind = typed();
    unrelated_kind.kind = 1;
    assert_eq!(
        ResolvedFieldPayloadBinding::bind_payload(
            physical(),
            TypedPayload {
                record: unrelated_kind,
                bytes: &bytes
            },
            [1; 16],
        ),
        Err(RecordError::FieldPayloadMismatch)
    );

    let mut wrong_length = typed();
    wrong_length.payload_length = 8;
    assert_eq!(
        ResolvedFieldPayloadBinding::bind_payload(
            physical(),
            TypedPayload {
                record: wrong_length,
                bytes: &bytes
            },
            [1; 16],
        ),
        Err(RecordError::FieldPayloadMismatch)
    );
}
