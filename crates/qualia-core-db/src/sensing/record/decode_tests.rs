use super::*;
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
    typed_payloads::TypedPayload,
};
use crate::sensing::{MeasurementModality, PhysicalFieldSemantics, SemanticId};

fn binding(container: [u8; 16]) -> ResolvedFieldPayloadBinding {
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
    let typed = TypedFieldRecord {
        field_id: 9,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 11,
        payload_section_id: 2,
        payload_offset: 0,
        payload_length: 16,
    };
    let physical = PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(13).unwrap(),
    )
    .unwrap();
    ResolvedFieldPayloadBinding::bind_payload(
        physical,
        TypedPayload {
            record: typed,
            bytes: &[0; 16],
        },
        container,
    )
    .unwrap()
}

fn encoded(full: bool) -> ([u8; MEASUREMENT_INDEX_RECORD_SIZE], MeasurementRecord) {
    let record = MeasurementRecord::new(
        21,
        SemanticId::new(17).unwrap(),
        binding([1; 16]),
        if full {
            ProvenanceDeclaration::Reference(PayloadReference::new([2; 16], 4, 16, 80).unwrap())
        } else {
            ProvenanceDeclaration::Unknown
        },
        if full {
            CalibrationDeclaration::Calibrated(CalibrationArtifactReference::new([3; 32]).unwrap())
        } else {
            CalibrationDeclaration::Unknown
        },
        if full {
            MeasurementQuality::Known(QualityMetrics::new(900_000, 25_000).unwrap())
        } else {
            MeasurementQuality::Unknown
        },
        if full {
            Some(RecordClockStamp::new(5, 2, 1000, 4).unwrap())
        } else {
            None
        },
        if full {
            ProjectionDeclaration::Reference(ProjectionReference::new(6, 7, 8, 9).unwrap())
        } else {
            ProjectionDeclaration::Unknown
        },
    )
    .unwrap();
    let mut out = [0u8; MEASUREMENT_INDEX_RECORD_SIZE];
    encode_measurement_index_records(&[record], &mut out).unwrap();
    (out, record)
}

#[test]
fn full_and_unknown_round_trip() {
    for full in [false, true] {
        let (bytes, expected) = encoded(full);
        assert_eq!(
            decode_measurement_index_record(&bytes, binding([1; 16])).unwrap(),
            expected
        );
    }
}

#[test]
fn rejects_mismatched_field_or_payload_binding_and_header() {
    let (bytes, _) = encoded(true);
    assert!(decode_measurement_index_record(&bytes, binding([9; 16])).is_err());
    for offset in (0..8).chain(24..114) {
        let mut altered = bytes;
        altered[offset] ^= 1;
        assert!(
            decode_measurement_index_record(&altered, binding([1; 16])).is_err(),
            "offset {offset}"
        );
    }
}

#[test]
fn rejects_nonzero_absent_padding_and_reserved_bytes() {
    let (bytes, _) = encoded(false);
    for offset in (115..151)
        .chain(152..184)
        .chain(185..193)
        .chain(194..222)
        .chain(223..MEASUREMENT_INDEX_RECORD_SIZE)
    {
        let mut altered = bytes;
        altered[offset] = 1;
        assert_eq!(
            decode_measurement_index_record(&altered, binding([1; 16])),
            Err(RecordError::InvalidEncoding),
            "offset {offset}"
        );
    }
}

#[test]
fn rejects_bad_tags_and_input_lengths() {
    let (bytes, _) = encoded(true);
    for (offset, tag) in [(114, 3), (151, 6), (184, 2), (193, 2), (222, 3)] {
        let mut altered = bytes;
        altered[offset] = tag;
        assert_eq!(
            decode_measurement_index_record(&altered, binding([1; 16])),
            Err(RecordError::InvalidEncoding)
        );
    }
    assert_eq!(
        decode_measurement_index_record(&bytes[..bytes.len() - 1], binding([1; 16])),
        Err(RecordError::InvalidEncoding)
    );
    let longer = [0; MEASUREMENT_INDEX_RECORD_SIZE + 1];
    assert_eq!(
        decode_measurement_index_record(&longer, binding([1; 16])),
        Err(RecordError::InvalidEncoding)
    );
}
