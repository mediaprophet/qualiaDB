use super::{
    AxisClaim, MeasurementModality, PhysicalFieldSemantics, ProjectionError, SemanticId,
    SemanticProjection,
};
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
};

fn physical_semantics(unit_id: u64) -> PhysicalFieldSemantics {
    let field = FieldDescriptor {
        name_hash: 0x1234,
        unit_hash: 0x5678,
        type_kind: FieldTypeKind::Vector as u8,
        support: FieldSupportKind::Stream as u8,
        representation: FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let typed = TypedFieldRecord {
        field_id: 17,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id,
        payload_section_id: 3,
        payload_offset: 64,
        payload_length: 128,
    };
    PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(9001).unwrap(),
    )
    .unwrap()
}

fn explicit_axes() -> [AxisClaim; 10] {
    [
        AxisClaim::known(0.25).unwrap(),
        AxisClaim::known(1.0).unwrap(),
        AxisClaim::known(2.0).unwrap(),
        AxisClaim::known(3.0).unwrap(),
        AxisClaim::known(4.0).unwrap(),
        AxisClaim::known(5.0).unwrap(),
        AxisClaim::known(6.0).unwrap(),
        AxisClaim::known(7.0).unwrap(),
        AxisClaim::known(8.0).unwrap(),
        AxisClaim::known(9.0).unwrap(),
    ]
}

fn complete_projection() -> SemanticProjection {
    SemanticProjection::new(
        physical_semantics(42),
        SemanticId::new(1).unwrap(),
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        explicit_axes(),
    )
    .unwrap()
}

#[test]
fn electromagnetic_semantics_and_both_unit_registries_survive_projection() {
    let projected = complete_projection().materialize().unwrap();
    assert_eq!(projected.tensor().q, 0.25);
    assert_eq!(projected.tensor().sigma, 9.0);
    assert_eq!(
        projected.physical().modality(),
        MeasurementModality::Electromagnetic
    );
    assert_eq!(
        projected.physical().typed_field_kind(),
        FIELD_KIND_ELECTROMAGNETIC
    );
    assert_eq!(projected.physical().physical_quantity().value(), Some(9001));
    assert_eq!(projected.physical().legacy_unit_hash(), 0x5678);
    assert_eq!(projected.physical().typed_unit_id().value(), Some(42));
}

#[test]
fn unknown_axis_or_profile_cannot_become_tensor_defaults() {
    let mut axes = explicit_axes();
    axes[5] = AxisClaim::Unknown;
    let partial = SemanticProjection::new(
        physical_semantics(42),
        SemanticId::new(1).unwrap(),
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        axes,
    )
    .unwrap();
    assert_eq!(
        partial.materialize(),
        Err(ProjectionError::UnknownAxis { index: 5 })
    );

    let unknown_profile = SemanticProjection::new(
        physical_semantics(42),
        SemanticId::Unknown,
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        explicit_axes(),
    )
    .unwrap();
    assert_eq!(
        unknown_profile.materialize(),
        Err(ProjectionError::UnknownProfile)
    );
}

#[test]
fn unknown_unit_and_quantity_are_preserved_as_unknown() {
    let physical = physical_semantics(0);
    assert_eq!(physical.typed_unit_id(), SemanticId::Unknown);

    let field = FieldDescriptor {
        name_hash: 3,
        unit_hash: 0,
        ..FieldDescriptor::default()
    };
    let typed = TypedFieldRecord {
        field_id: 4,
        kind: 0xffff,
        flags: 0,
        unit_id: 0,
        payload_section_id: 1,
        payload_offset: 0,
        payload_length: 0,
    };
    let unknown = PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Unknown,
        SemanticId::Unknown,
    )
    .unwrap();
    assert_eq!(unknown.modality(), MeasurementModality::Unknown);
    assert_eq!(unknown.physical_quantity(), SemanticId::Unknown);
    assert_eq!(unknown.typed_field_kind(), 0xffff);
}

#[test]
fn nonfinite_coordinates_and_invalid_field_descriptors_are_rejected() {
    assert_eq!(
        AxisClaim::known(f32::NAN),
        Err(ProjectionError::InvalidCoordinate)
    );
    let field = FieldDescriptor {
        name_hash: 0,
        ..FieldDescriptor::default()
    };
    let typed = TypedFieldRecord {
        field_id: 1,
        kind: 1,
        flags: 0,
        unit_id: 0,
        payload_section_id: 1,
        payload_offset: 0,
        payload_length: 0,
    };
    assert_eq!(
        PhysicalFieldSemantics::from_existing(
            &field,
            &typed,
            MeasurementModality::Unknown,
            SemanticId::Unknown,
        ),
        Err(ProjectionError::InvalidFieldDescriptor)
    );
}
