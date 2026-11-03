use super::*;
use crate::{
    container_10d::{
        field_section::{
            FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind,
        },
        manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
        typed_payloads::TypedPayload,
    },
    frame_layout::{INLINE_TAG_BOOLEAN, INLINE_VALUE_MASK},
    sensing::{
        quin_index::{
            measurement_record_to_quins, proposed_predicates, validate_measurement_quin_parity,
            MeasurementQuinError, MeasurementQuinMapping,
        },
        MeasurementModality, PhysicalFieldSemantics, SemanticId,
    },
    NQuin,
};

fn record(id: u64, source: SemanticId, field_id: u64) -> MeasurementRecord {
    let descriptor = FieldDescriptor {
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
        field_id,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 11,
        payload_section_id: 2,
        payload_offset: 0,
        payload_length: 16,
    };
    let physical = PhysicalFieldSemantics::from_existing(
        &descriptor,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::Unknown,
    )
    .unwrap();
    let binding = ResolvedFieldPayloadBinding::bind_payload(
        physical,
        TypedPayload {
            record: typed,
            bytes: &[0; 16],
        },
        [1; 16],
    )
    .unwrap();
    MeasurementRecord::new(
        id,
        source,
        binding,
        ProvenanceDeclaration::Unknown,
        CalibrationDeclaration::Unknown,
        MeasurementQuality::Unknown,
        None,
        ProjectionDeclaration::Unknown,
    )
    .unwrap()
}

#[test]
fn links_preserve_known_ids_payload_token_and_both_parities() {
    let mapping = MeasurementQuinMapping::new(7, 100, 200, 300).unwrap();
    let mut out = [NQuin::default(); 7];
    let count = measurement_record_to_quins(
        record(7, SemanticId::new(17).unwrap(), 9),
        mapping,
        &mut out,
    )
    .unwrap();
    assert_eq!(count, 6);
    assert!(out[..count]
        .iter()
        .all(|q| q.metadata == 0 && validate_measurement_quin_parity(q).conventions_agree_valid()));
    assert_eq!(out[3].predicate, proposed_predicates::PAYLOAD_REFERENCE);
    assert_eq!(out[3].object, 300);
    assert_eq!(out[4].object, INLINE_TAG_BOOLEAN | 1);
    assert_eq!(out[6], NQuin::default());
}

#[test]
fn unknown_source_is_explicit_and_not_zero_source_assertion() {
    let mut out = [NQuin::default(); 6];
    let n = measurement_record_to_quins(
        record(7, SemanticId::Unknown, 9),
        MeasurementQuinMapping::new(7, 100, 200, 300).unwrap(),
        &mut out,
    )
    .unwrap();
    assert_eq!(n, 5);
    assert_eq!(out[4].object, INLINE_TAG_BOOLEAN);
    assert!(out[..n]
        .iter()
        .all(|q| q.predicate != proposed_predicates::SOURCE_ID));
}

#[test]
fn overflow_mismatch_and_capacity_do_not_change_output() {
    assert_eq!(
        MeasurementQuinMapping::new(INLINE_VALUE_MASK + 1, 1, 2, 3),
        Err(MeasurementQuinError::IdentifierOutOfRange)
    );
    assert_eq!(
        MeasurementQuinMapping::new(7, 1, 2, INLINE_VALUE_MASK + 1),
        Err(MeasurementQuinError::IdentifierOutOfRange)
    );
    let mapping = MeasurementQuinMapping::new(7, 1, 2, 3).unwrap();
    let mut out = [NQuin::default(); 6];
    for (rec, expected) in [
        (
            record(8, SemanticId::Unknown, 9),
            MeasurementQuinError::RecordMismatch,
        ),
        (
            record(7, SemanticId::Unknown, INLINE_VALUE_MASK + 1),
            MeasurementQuinError::IdentifierOutOfRange,
        ),
        (
            record(7, SemanticId::new(INLINE_VALUE_MASK + 1).unwrap(), 9),
            MeasurementQuinError::IdentifierOutOfRange,
        ),
    ] {
        assert_eq!(
            measurement_record_to_quins(rec, mapping, &mut out),
            Err(expected)
        );
        assert_eq!(out, [NQuin::default(); 6]);
    }
    assert_eq!(
        measurement_record_to_quins(record(7, SemanticId::Unknown, 9), mapping, &mut out[..4]),
        Err(MeasurementQuinError::OutputTooSmall)
    );
    assert_eq!(out, [NQuin::default(); 6]);
}
