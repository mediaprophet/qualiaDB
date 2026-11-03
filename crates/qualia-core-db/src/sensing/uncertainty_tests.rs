use super::*;
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
};
use crate::sensing::projection_map::{
    AxisMapping, ObservationUncertainty, ObservationValue, PhysicalObservation, ProjectionMap,
};
use crate::sensing::MeasurementModality;

fn field() -> PhysicalFieldSemantics {
    let descriptor = FieldDescriptor {
        name_hash: 0x1234,
        unit_hash: 0x5678,
        type_kind: FieldTypeKind::Scalar as u8,
        support: FieldSupportKind::Point as u8,
        representation: FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let typed = TypedFieldRecord {
        field_id: 17,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 42,
        payload_section_id: 3,
        payload_offset: 64,
        payload_length: 8,
    };
    PhysicalFieldSemantics::from_existing(
        &descriptor,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(9001).unwrap(),
    )
    .unwrap()
}

fn id(value: u64) -> SemanticId {
    SemanticId::new(value).unwrap()
}

fn record(estimate: UncertaintyEstimate) -> UncertaintyRecord {
    UncertaintyRecord::new(700, field(), estimate, None, Some(701)).unwrap()
}

fn mapped() -> MappedProjection {
    let physical = field();
    let observation = PhysicalObservation::new(
        physical.physical_quantity(),
        physical.typed_unit_id(),
        ObservationValue::Known(0.1),
        ObservationUncertainty::Known(0.01),
    )
    .unwrap();
    let mut axes = [None; 10];
    axes[2] = Some(AxisMapping::new(2.0, 3.0).unwrap());
    ProjectionMap::new(
        801,
        physical,
        id(101),
        id(102),
        id(103),
        physical.physical_quantity(),
        physical.typed_unit_id(),
        axes,
    )
    .unwrap()
    .project(observation)
    .unwrap()
}

#[test]
fn unknown_is_explicit_and_export_uses_zero_metadata_five_field_parity() {
    let item = record(UncertaintyEstimate::Unknown);
    let mut out = [NQuin::default(); MAX_UNCERTAINTY_QUINS];
    let count = item.write_quins(&mut out).unwrap();
    assert!(count <= MAX_UNCERTAINTY_QUINS);
    assert!(out[..count].iter().all(|q| {
        q.metadata == 0
            && q.parity
                == NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata)
            && q.verify_ecc_parity()
    }));
    assert!(out[..count].iter().any(|q| {
        q.predicate == proposed_predicates::HAS_ESTIMATE_KIND
            && q.object == crate::frame_layout::INLINE_TAG_INTEGER
    }));
    assert_eq!(count, item.required_quins());
    assert_eq!(item.field().field_id(), 17);
    assert_eq!(item.field().typed_unit_id().value(), Some(42));
    assert_eq!(item.field().legacy_unit_hash(), 0x5678);
}

#[test]
fn absolute_and_interval_units_must_match_selected_field_units() {
    assert!(UncertaintyRecord::new(
        1,
        field(),
        UncertaintyEstimate::AbsoluteBound {
            value: 0.25,
            unit_id: id(42)
        },
        None,
        None,
    )
    .is_ok());
    assert_eq!(
        UncertaintyRecord::new(
            1,
            field(),
            UncertaintyEstimate::AbsoluteBound {
                value: 0.25,
                unit_id: id(43)
            },
            None,
            None,
        ),
        Err(UncertaintyError::UnitMismatch),
    );
    assert!(UncertaintyRecord::new(
        1,
        field(),
        UncertaintyEstimate::ConfidenceInterval {
            lower: -2.0,
            upper: -1.0,
            coverage_millionths: 950_000,
            unit_id: id(42),
        },
        None,
        None,
    )
    .is_ok());
    assert_eq!(
        UncertaintyRecord::new(
            1,
            field(),
            UncertaintyEstimate::ConfidenceInterval {
                lower: 2.0,
                upper: 1.0,
                coverage_millionths: 950_000,
                unit_id: id(42),
            },
            None,
            None,
        ),
        Err(UncertaintyError::InvalidInterval),
    );
}

#[test]
fn relative_bound_is_dimensionless_and_does_not_clamp_values_above_one() {
    let item = record(UncertaintyEstimate::RelativeBoundMillionths(1_500_000));
    assert_eq!(
        item.estimate(),
        UncertaintyEstimate::RelativeBoundMillionths(1_500_000)
    );
    let mut out = [NQuin::default(); MAX_UNCERTAINTY_QUINS];
    let count = item.write_quins(&mut out).unwrap();
    let relative = out[..count]
        .iter()
        .find(|q| q.predicate == proposed_predicates::HAS_RELATIVE_BOUND_MILLIONTHS)
        .unwrap();
    assert_eq!(
        relative.object,
        crate::frame_layout::INLINE_TAG_INTEGER | 1_500_000
    );
    assert_eq!(
        out[..count]
            .iter()
            .find(|q| q.predicate == proposed_predicates::HAS_FIELD_ID)
            .unwrap()
            .object,
        17,
        "field IDs remain identifier handles, not tagged numeric literals"
    );
}

#[test]
fn graph_numeric_literals_use_integer_and_float_tags_with_exact_capacity_counts() {
    let estimates = [
        UncertaintyEstimate::Unknown,
        UncertaintyEstimate::AbsoluteBound {
            value: 0.25,
            unit_id: id(42),
        },
        UncertaintyEstimate::RelativeBoundMillionths(1_250_000),
        UncertaintyEstimate::ConfidenceInterval {
            lower: -0.5,
            upper: 0.75,
            coverage_millionths: 950_000,
            unit_id: id(42),
        },
    ];
    let mut out = [NQuin::default(); MAX_UNCERTAINTY_QUINS];
    for estimate in estimates {
        let item = record(estimate);
        let count = item.write_quins(&mut out).unwrap();
        assert_eq!(count, item.required_quins());
        assert!(out[..count].iter().all(|q| q.metadata == 0));
        for q in &out[..count] {
            if matches!(
                q.predicate,
                p if p == proposed_predicates::HAS_FIELD_KIND
                    || p == proposed_predicates::HAS_LEGACY_UNIT_STATE
                    || p == proposed_predicates::HAS_QUANTITY_STATE
                    || p == proposed_predicates::HAS_TYPED_UNIT_STATE
                    || p == proposed_predicates::HAS_SCOPE_KIND
                    || p == proposed_predicates::HAS_METHOD_STATE
                    || p == proposed_predicates::HAS_METHOD_VERSION
                    || p == proposed_predicates::HAS_EVIDENCE_STATE
                    || p == proposed_predicates::HAS_ESTIMATE_KIND
                    || p == proposed_predicates::HAS_RELATIVE_BOUND_MILLIONTHS
                    || p == proposed_predicates::HAS_COVERAGE_MILLIONTHS
            ) {
                assert_eq!(
                    crate::frame_layout::object_tag(q.object),
                    crate::frame_layout::INLINE_TAG_INTEGER
                );
            }
            if q.predicate == proposed_predicates::HAS_ABSOLUTE_BOUND
                || q.predicate == proposed_predicates::HAS_INTERVAL_LOWER
                || q.predicate == proposed_predicates::HAS_INTERVAL_UPPER
            {
                assert_eq!(
                    crate::frame_layout::object_tag(q.object),
                    crate::frame_layout::INLINE_TAG_FLOAT
                );
            }
            assert_eq!(
                q.parity,
                NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata)
            );
        }
    }
}

#[test]
fn projection_conversion_requires_explicit_axis_unit_and_keeps_scope() {
    let projected = mapped();
    let converted = UncertaintyRecord::from_projection_axis(
        900,
        &projected,
        2,
        id(777),
        Some(MethodVersion::new(id(778), 2).unwrap()),
        Some(779),
    )
    .unwrap();
    assert!(matches!(
        converted.scope(),
        UncertaintyScope::ProjectionAxis {
            mapping_id: 801,
            axis_index: 2,
            axis_unit_id: SemanticId::Known(_),
            profile_id: SemanticId::Known(_),
            domain_id: SemanticId::Known(_),
            coordinate_convention_id: SemanticId::Known(_),
        }
    ));
    assert!(
        matches!(converted.estimate(), UncertaintyEstimate::AbsoluteBound { unit_id, .. } if unit_id.value() == Some(777))
    );
    assert_eq!(converted.field().typed_unit_id().value(), Some(42));
}

#[test]
fn graph_export_capacity_failure_does_not_modify_output() {
    let item = record(UncertaintyEstimate::AbsoluteBound {
        value: 0.1,
        unit_id: id(42),
    });
    let marker = NQuin {
        subject: 55,
        predicate: 66,
        object: 77,
        context: 88,
        metadata: 99,
        parity: 111,
    };
    let mut out = [marker; 2];
    let before = out;
    assert!(matches!(
        item.write_quins(&mut out),
        Err(UncertaintyError::Capacity { .. })
    ));
    assert_eq!(out, before);
}

#[test]
fn invalid_method_and_identifier_ranges_fail_closed() {
    assert_eq!(
        MethodVersion::new(SemanticId::Unknown, 1),
        Err(UncertaintyError::UnknownRequiredSemantics)
    );
    assert_eq!(
        MethodVersion::new(id(5), 0),
        Err(UncertaintyError::InvalidMethod)
    );
    assert_eq!(
        EvidenceReference::new(0),
        Err(UncertaintyError::InvalidIdentifier)
    );
    assert_eq!(
        UncertaintyRecord::new(
            ID_MASK + 1,
            field(),
            UncertaintyEstimate::Unknown,
            None,
            None,
        ),
        Err(UncertaintyError::InvalidIdentifier),
    );
}

#[test]
fn projection_unknown_axes_remain_explicitly_unknown() {
    let projected = mapped();
    let converted =
        UncertaintyRecord::from_projection_axis(901, &projected, 1, id(777), None, None).unwrap();
    assert_eq!(converted.estimate(), UncertaintyEstimate::Unknown);
}
