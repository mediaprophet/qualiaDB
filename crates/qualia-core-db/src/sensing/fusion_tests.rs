use super::{
    plan_fusion, CoordinateMapping, FusionClockStamp, FusionError, FusionPolicy, FusionSample,
    FusionTarget, SampleQuality, TimeAlignment, MAX_FUSION_CANDIDATES,
};
use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::{TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC},
};
use crate::sensing::{AxisClaim, SemanticProjection};
use crate::sensing::{MeasurementModality, PhysicalFieldSemantics, SemanticId};

fn physical(quantity_id: u64) -> PhysicalFieldSemantics {
    let field = FieldDescriptor {
        name_hash: 3,
        unit_hash: 4,
        type_kind: FieldTypeKind::Spectral as u8,
        support: FieldSupportKind::Stream as u8,
        representation: FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let typed = TypedFieldRecord {
        field_id: 5,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 6,
        payload_section_id: 7,
        payload_offset: 8,
        payload_length: 9,
    };
    PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        MeasurementModality::Electromagnetic,
        SemanticId::new(quantity_id).unwrap(),
    )
    .unwrap()
}

fn sample(
    sample_id: u64,
    clock_id: u64,
    epoch: u32,
    ticks_ns: u64,
    frame_id: u64,
    priority: i32,
) -> FusionSample {
    FusionSample::new(
        sample_id,
        physical(10 + sample_id),
        Some(FusionClockStamp::new(clock_id, epoch, ticks_ns, 3).unwrap()),
        SemanticId::new(frame_id).unwrap(),
        None,
        SampleQuality::Unknown,
        priority,
    )
    .unwrap()
}

fn identity_mapping(frame_id: u64, mapping_id: u64) -> CoordinateMapping {
    CoordinateMapping::new(
        mapping_id,
        frame_id,
        100,
        0.0,
        SemanticId::new(200).unwrap(),
    )
    .unwrap()
}

fn target() -> FusionTarget {
    FusionTarget::new(1, 2, 100).unwrap()
}

fn policy(maximum_samples: usize) -> FusionPolicy {
    FusionPolicy::new(maximum_samples, u64::MAX).unwrap()
}

#[test]
fn same_clock_selection_is_bounded_and_deterministically_ordered() {
    let samples = [
        sample(30, 1, 2, 50, 9, 2),
        sample(20, 1, 2, 40, 9, 2),
        sample(10, 1, 2, 30, 9, 5),
    ];
    let mappings = [identity_mapping(9, 90)];
    let mut output = [None; 2];
    let count = plan_fusion(
        &samples,
        &[],
        &mappings,
        target(),
        FusionPolicy::new(2, 16).unwrap(),
        &mut output,
    )
    .unwrap();
    assert_eq!(count, 2);
    let first = output[0].unwrap();
    let second = output[1].unwrap();
    assert_eq!([first.sample_id(), second.sample_id()], [10, 20]);
    assert_eq!(first.alignment_id(), None);
    assert_eq!(first.aligned_window().lower_ns(), 27);
    assert_eq!(first.aligned_window().upper_ns(), 33);
    assert_eq!(
        first.physical().modality(),
        MeasurementModality::Electromagnetic
    );
    assert_eq!(first.quality(), SampleQuality::Unknown);
    assert_eq!(first.physical().physical_quantity().value(), Some(20));
}

#[test]
fn cross_clock_and_restart_require_explicit_uncertain_alignment() {
    let samples = [sample(1, 8, 4, 100, 9, 1)];
    let mappings = [identity_mapping(9, 90)];
    let mut output = [None; 1];
    assert_eq!(
        plan_fusion(&samples, &[], &mappings, target(), policy(1), &mut output,),
        Err(FusionError::MissingTimeAlignment)
    );
    let alignment = TimeAlignment::new(12, 8, 4, 1, 2, 97, 103, -10, 5).unwrap();
    plan_fusion(
        &samples,
        &[alignment],
        &mappings,
        target(),
        policy(1),
        &mut output,
    )
    .unwrap();
    let entry = output[0].unwrap();
    assert_eq!(entry.alignment_id(), Some(12));
    // Source ±3 ns plus alignment ±5 ns; target center is 90 ns.
    assert_eq!(entry.aligned_window().lower_ns(), 82);
    assert_eq!(entry.aligned_window().upper_ns(), 98);
}

#[test]
fn missing_or_ambiguous_coordinate_mapping_fails_without_output_mutation() {
    let samples = [sample(1, 1, 2, 100, 9, 0)];
    let mut output = [None; 1];
    let before = output;
    assert_eq!(
        plan_fusion(&samples, &[], &[], target(), policy(1), &mut output,),
        Err(FusionError::MissingCoordinateMapping)
    );
    assert_eq!(output, before);

    let mappings = [identity_mapping(9, 90), identity_mapping(9, 91)];
    assert_eq!(
        plan_fusion(&samples, &[], &mappings, target(), policy(1), &mut output,),
        Err(FusionError::AmbiguousCoordinateMapping)
    );
}

#[test]
fn unknown_time_and_mapping_uncertainty_fail_closed() {
    let unknown_time = FusionSample::new(
        1,
        physical(11),
        None,
        SemanticId::new(9).unwrap(),
        None,
        SampleQuality::Unknown,
        0,
    )
    .unwrap();
    let mut output = [None; 1];
    assert_eq!(
        plan_fusion(
            &[unknown_time],
            &[],
            &[identity_mapping(9, 90)],
            target(),
            policy(1),
            &mut output,
        ),
        Err(FusionError::UnknownTime)
    );
    assert_eq!(
        CoordinateMapping::new(1, 9, 100, 0.0, SemanticId::Unknown),
        Err(FusionError::UnknownMappingErrorUnit)
    );
}

#[test]
fn mapping_preserves_explicit_error_unit_and_quality_unknown() {
    let samples = [sample(1, 1, 2, 100, 9, 0)];
    let mappings = [CoordinateMapping::new(2, 9, 100, 0.25, SemanticId::new(3).unwrap()).unwrap()];
    let mut output = [None; 1];
    plan_fusion(&samples, &[], &mappings, target(), policy(1), &mut output).unwrap();
    let entry = output[0].unwrap();
    assert_eq!(entry.mapping_id(), 2);
    assert_eq!(entry.mapping_maximum_error(), 0.25);
    assert_eq!(entry.mapping_error_unit_id().value(), Some(3));
    assert_eq!(entry.quality(), SampleQuality::Unknown);
}

#[test]
fn aligned_interval_overflow_is_rejected() {
    let samples = [sample(1, 8, 4, u64::MAX - 3, 9, 0)];
    let alignment = TimeAlignment::new(11, 8, 4, 1, 2, 0, u64::MAX, 1, 0).unwrap();
    let mut output = [None; 1];
    assert_eq!(
        plan_fusion(
            &samples,
            &[alignment],
            &[identity_mapping(9, 90)],
            target(),
            policy(1),
            &mut output,
        ),
        Err(FusionError::InvalidTime)
    );
    // The standalone stamp also rejects an overflowing source uncertainty interval.
    assert_eq!(
        FusionClockStamp::new(1, 0, u64::MAX, 1),
        Err(FusionError::InvalidTime)
    );
}

#[test]
fn alignment_must_cover_the_full_source_uncertainty_interval() {
    let samples = [sample(1, 8, 4, 200, 9, 0)];
    let alignment = TimeAlignment::new(11, 8, 4, 1, 2, 0, 100, 0, 1).unwrap();
    let mut output = [None; 1];
    let before = output;
    assert_eq!(
        plan_fusion(
            &samples,
            &[alignment],
            &[identity_mapping(9, 90)],
            target(),
            policy(1),
            &mut output,
        ),
        Err(FusionError::TimeAlignmentOutsideValidity)
    );
    assert_eq!(output, before);

    assert_eq!(
        TimeAlignment::new(11, 8, 4, 1, 2, 101, 100, 0, 0),
        Err(FusionError::InvalidTimeAlignment)
    );
}

#[test]
fn temporal_span_failure_leaves_output_untouched() {
    let samples = [sample(1, 1, 2, 30, 9, 2), sample(2, 1, 2, 100, 9, 1)];
    let mut output = [None; 2];
    let before = output;
    assert_eq!(
        plan_fusion(
            &samples,
            &[],
            &[identity_mapping(9, 90)],
            target(),
            FusionPolicy::new(2, 20).unwrap(),
            &mut output,
        ),
        Err(FusionError::TemporalSpanExceeded)
    );
    assert_eq!(output, before);
}

#[test]
fn projection_semantics_must_match_sample_physical_semantics() {
    let projection = SemanticProjection::new(
        physical(99),
        SemanticId::new(1).unwrap(),
        SemanticId::new(2).unwrap(),
        SemanticId::new(3).unwrap(),
        [AxisClaim::Unknown; 10],
    )
    .unwrap();
    assert_eq!(
        FusionSample::new(
            1,
            physical(11),
            Some(FusionClockStamp::new(1, 2, 30, 3).unwrap()),
            SemanticId::new(9).unwrap(),
            Some(projection),
            SampleQuality::Unknown,
            0,
        ),
        Err(FusionError::ProjectionSemanticMismatch)
    );
}

#[test]
fn invalid_capacity_quality_and_duplicate_ids_are_rejected() {
    assert_eq!(
        FusionPolicy::new(0, u64::MAX),
        Err(FusionError::InvalidPolicy)
    );
    assert_eq!(
        FusionPolicy::new(MAX_FUSION_CANDIDATES + 1, u64::MAX),
        Err(FusionError::InvalidPolicy)
    );
    assert_eq!(
        SampleQuality::known(1_000_001, 0),
        Err(FusionError::InvalidQuality)
    );

    let samples = [sample(1, 1, 2, 10, 9, 0), sample(1, 1, 2, 11, 9, 0)];
    let mut output = [None; 2];
    assert_eq!(
        plan_fusion(
            &samples,
            &[],
            &[identity_mapping(9, 90)],
            target(),
            policy(2),
            &mut output,
        ),
        Err(FusionError::DuplicateSampleId)
    );
}

#[test]
fn undersized_output_is_transactional_and_ties_use_sample_id() {
    let samples = [sample(2, 1, 2, 10, 9, 1), sample(1, 1, 2, 10, 9, 1)];
    let mappings = [identity_mapping(9, 90)];
    let mut too_small = [None; 1];
    assert_eq!(
        plan_fusion(
            &samples,
            &[],
            &mappings,
            target(),
            policy(2),
            &mut too_small,
        ),
        Err(FusionError::OutputTooSmall)
    );

    let mut output = [None; 2];
    plan_fusion(&samples, &[], &mappings, target(), policy(2), &mut output).unwrap();
    assert_eq!(
        [
            output[0].unwrap().sample_id(),
            output[1].unwrap().sample_id()
        ],
        [1, 2]
    );
}
