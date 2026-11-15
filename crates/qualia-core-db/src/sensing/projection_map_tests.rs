use super::*;
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
};
use crate::sensing::MeasurementModality;

fn physical() -> PhysicalFieldSemantics {
    let field = FieldDescriptor {
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
        &field,
        &typed,
        MeasurementModality::Thermal,
        SemanticId::new(9001).unwrap(),
    )
    .unwrap()
}

fn map(axes: [Option<AxisMapping>; 10]) -> ProjectionMap {
    ProjectionMap::new(
        700,
        physical(),
        SemanticId::new(1).unwrap(),
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        SemanticId::new(9001).unwrap(),
        SemanticId::new(42).unwrap(),
        axes,
    )
    .unwrap()
}

fn observation(
    value: ObservationValue,
    uncertainty: ObservationUncertainty,
) -> PhysicalObservation {
    PhysicalObservation::new(
        SemanticId::new(9001).unwrap(),
        SemanticId::new(42).unwrap(),
        value,
        uncertainty,
    )
    .unwrap()
}

#[test]
fn explicit_affine_map_preserves_physical_value_quantity_unit_and_uncertainty() {
    let mut axes = [None; 10];
    axes[0] = Some(AxisMapping::new(2.0, 1.0).unwrap());
    axes[1] = Some(AxisMapping::new(0.0, 42.0).unwrap());
    let mapped = map(axes)
        .project(observation(
            ObservationValue::Known(2.5),
            ObservationUncertainty::Known(0.25),
        ))
        .unwrap();

    assert_eq!(mapped.mapping_id(), 700);
    assert_eq!(mapped.observation().value(), ObservationValue::Known(2.5));
    assert_eq!(mapped.observation().unit_id().value(), Some(42));
    assert_eq!(mapped.observation().quantity_id().value(), Some(9001));
    assert_eq!(mapped.projection().axes()[0], AxisClaim::Known(6.0));
    assert_eq!(mapped.projection().axes()[1], AxisClaim::Known(42.0));
    assert_eq!(mapped.projection().axes()[2], AxisClaim::Unknown);
    assert_eq!(mapped.axis_uncertainty()[0], AxisUncertainty::Known(0.5));
    assert_eq!(mapped.axis_uncertainty()[1], AxisUncertainty::Known(0.0));
    assert_eq!(mapped.axis_uncertainty()[2], AxisUncertainty::Unknown);
    assert_eq!(
        mapped.projection().materialize().unwrap_err(),
        ProjectionError::UnknownAxis { index: 2 }
    );
}

#[test]
fn unknown_measurement_remains_unknown_even_with_explicit_mapping() {
    let mut axes = [None; 10];
    axes[0] = Some(AxisMapping::new(10.0, 5.0).unwrap());
    let mapped = map(axes)
        .project(observation(
            ObservationValue::Unknown,
            ObservationUncertainty::Unknown,
        ))
        .unwrap();
    assert!(mapped
        .projection()
        .axes()
        .iter()
        .all(|axis| *axis == AxisClaim::Unknown));
    assert!(mapped
        .axis_uncertainty()
        .iter()
        .all(|uncertainty| *uncertainty == AxisUncertainty::Unknown));
}

#[test]
fn known_uncertainty_includes_f64_to_f32_coordinate_rounding() {
    let mut axes = [None; 10];
    axes[0] = Some(AxisMapping::new(1.0, 0.0).unwrap());
    let mapped = map(axes)
        .project(observation(
            ObservationValue::Known(0.1),
            ObservationUncertainty::Known(0.0),
        ))
        .unwrap();
    assert!(matches!(
        mapped.axis_uncertainty()[0],
        AxisUncertainty::Known(value) if value > 0.0
    ));
}

#[test]
fn uncertainty_f32_conversion_rounds_bound_outward() {
    let mut axes = [None; 10];
    axes[0] = Some(AxisMapping::new(1.0, 0.0).unwrap());
    let declared_bound = 1.0 + 2.0f64.powi(-25);
    let mapped = map(axes)
        .project(observation(
            ObservationValue::Known(0.0),
            ObservationUncertainty::Known(declared_bound),
        ))
        .unwrap();
    let AxisUncertainty::Known(represented_bound) = mapped.axis_uncertainty()[0] else {
        panic!("known input uncertainty should remain known");
    };
    assert!((represented_bound as f64) >= declared_bound);
    assert_eq!(represented_bound, f32::from_bits(1.0f32.to_bits() + 1));
}

#[test]
fn mismatched_quantity_or_unit_is_rejected_without_conversion_guess() {
    let wrong_quantity = PhysicalObservation::new(
        SemanticId::new(9002).unwrap(),
        SemanticId::new(42).unwrap(),
        ObservationValue::Known(1.0),
        ObservationUncertainty::Unknown,
    )
    .unwrap();
    assert_eq!(
        map([None; 10]).project(wrong_quantity),
        Err(ProjectionMapError::QuantityMismatch)
    );

    let wrong_unit = PhysicalObservation::new(
        SemanticId::new(9001).unwrap(),
        SemanticId::new(43).unwrap(),
        ObservationValue::Known(1.0),
        ObservationUncertainty::Unknown,
    )
    .unwrap();
    assert_eq!(
        map([None; 10]).project(wrong_unit),
        Err(ProjectionMapError::UnitMismatch)
    );
}

#[test]
fn unknown_mapping_ids_and_invalid_numbers_fail_closed() {
    let invalid = ProjectionMap::new(
        700,
        physical(),
        SemanticId::Unknown,
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        SemanticId::new(9001).unwrap(),
        SemanticId::new(42).unwrap(),
        [None; 10],
    );
    assert_eq!(invalid, Err(ProjectionMapError::UnknownInterpretation));
    assert_eq!(
        AxisMapping::new(f64::NAN, 0.0),
        Err(ProjectionMapError::InvalidMapping)
    );
    assert_eq!(
        PhysicalObservation::new(
            SemanticId::new(9001).unwrap(),
            SemanticId::new(42).unwrap(),
            ObservationValue::Known(f64::INFINITY),
            ObservationUncertainty::Unknown,
        ),
        Err(ProjectionMapError::InvalidObservation)
    );
    assert_eq!(
        PhysicalObservation::new(
            SemanticId::new(9001).unwrap(),
            SemanticId::new(42).unwrap(),
            ObservationValue::Known(1.0),
            ObservationUncertainty::Known(-0.1),
        ),
        Err(ProjectionMapError::InvalidObservation)
    );
}

#[test]
fn coordinate_and_uncertainty_overflow_reject_the_mapping() {
    let mut axes = [None; 10];
    axes[0] = Some(AxisMapping::new(f64::MAX, 0.0).unwrap());
    assert_eq!(
        map(axes).project(observation(
            ObservationValue::Known(2.0),
            ObservationUncertainty::Unknown,
        )),
        Err(ProjectionMapError::CoordinateOverflow { axis: 0 })
    );

    axes[0] = Some(AxisMapping::new(2.0, 0.0).unwrap());
    assert_eq!(
        map(axes).project(observation(
            ObservationValue::Known(1.0),
            ObservationUncertainty::Known(f64::MAX),
        )),
        Err(ProjectionMapError::UncertaintyOverflow { axis: 0 })
    );
}
