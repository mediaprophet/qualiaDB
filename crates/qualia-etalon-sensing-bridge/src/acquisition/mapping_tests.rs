use super::*;
use etalon_sense_core::{
    evidence::EvidenceOrigin,
    observation::{HostPolicyDecision, MeasurementUncertainty, ObservationKind, QuantityUnitIds},
    quality::{CalibrationRecord, CalibrationState, QualityRecord, SourceDescriptor, SourceKind},
    time::ClockStamp,
};
use qualia_core_db::sensing::SemanticId;
fn id(n: u64) -> SemanticId {
    SemanticId::new(n).unwrap()
}
fn mapping() -> AcquisitionMapping {
    AcquisitionMapping {
        axes: [
            a::Axis::Sample,
            a::Axis::Channel,
            a::Axis::Sensor,
            a::Axis::Lag,
        ],
        byte_order: a::ByteOrder::Big,
        components: a::ComponentOrder::Scalar,
        context: a::Context {
            quantity: a::Quantity {
                kind: SemanticId::Unknown,
                unit: SemanticId::Unknown,
            },
            stage: a::SampleStage::RawCodes,
            sensor: SemanticId::Unknown,
            geometry: None,
            calibration: None,
            uncertainty: None,
        },
        clock: None,
        signal_axes: a::SignalAxes {
            carrier: a::Carrier::Unknown,
            baseband_hz: None,
            pulse_rate: None,
            complex_convention: None,
            phase_origin: None,
        },
    }
}
fn observation(anchor: ObservationTimeAnchor) -> ObservationEnvelope {
    ObservationEnvelope::new(
        ObservationKind::RecordedFrame,
        SourceDescriptor::new(41, SourceKind::Recorded).unwrap(),
        None,
        None,
        None,
        Some(ClockStamp::new(2, 3, 100, 7).unwrap()),
        QuantityUnitIds::new(None, None).unwrap(),
        MeasurementUncertainty::Unknown,
        QualityRecord::Unknown,
        CalibrationRecord::new(CalibrationState::Missing).unwrap(),
        EvidenceOrigin::Unknown,
        None,
        HostPolicyDecision::NotEvaluated,
    )
    .unwrap()
    .with_time_anchor(anchor)
    .unwrap()
}
fn clock() -> a::Clock {
    a::Clock::new(
        id(2),
        3,
        a::Timing {
            scale: 0,
            epoch_seconds: 0,
            epoch_nanoseconds: 0,
            ticks: a::Rate::new(1_000_000_000, 1).unwrap(),
            samples: a::Rate::new(48_000, 1).unwrap(),
            start_tick: 100,
        },
    )
    .unwrap()
}
#[test]
fn maps_every_actual_etalon_encoding_without_fabricating_semantics() {
    for encoding in [
        SampleEncoding::U8,
        SampleEncoding::U16,
        SampleEncoding::I16,
        SampleEncoding::I32,
        SampleEncoding::F32,
        SampleEncoding::F64,
        SampleEncoding::ComplexI16,
        SampleEncoding::ComplexF32,
    ] {
        let l = SampleLayout::new(
            1,
            [1, 0, 0, 0],
            [u64::from(encoding.element_bytes()), 0, 0, 0],
            encoding,
        )
        .unwrap();
        let mut m = mapping();
        if encoding.is_complex() {
            m.components = a::ComponentOrder::ImagReal;
        }
        let result = map_layout(&l, m).unwrap();
        assert_eq!(result.span().unwrap(), encoding.element_bytes() as usize);
        assert_eq!(result.encoding.complex(), encoding.is_complex());
        assert_eq!(result.byte_order, a::ByteOrder::Big);
        assert_eq!(result.components, m.components);
        assert_eq!(m.context.quantity.unit, SemanticId::Unknown);
    }
}
#[test]
fn generic_provider_stride_and_axis_permutation_are_not_reshaped() {
    let l = SampleLayout::new(2, [2, 3, 0, 0], [2, 8, 0, 0], SampleEncoding::U16).unwrap();
    let mut m = mapping();
    m.axes[0] = a::Axis::Channel;
    m.axes[1] = a::Axis::Sample;
    let result = map_layout(&l, m).unwrap();
    assert_eq!(result.extents, [2, 3, 0, 0]);
    assert_eq!(result.strides, [2, 8, 0, 0]);
    assert_eq!(result.span().unwrap(), 20);
    assert_eq!(result.axes[0], a::Axis::Channel);
}
#[test]
fn complex_component_and_repeated_axis_ambiguity_reject() {
    let l = SampleLayout::new(1, [2, 0, 0, 0], [4, 0, 0, 0], SampleEncoding::ComplexI16).unwrap();
    assert!(matches!(map_layout(&l, mapping()), Err(a::Error::Shape)));
    let l = SampleLayout::new(2, [2, 2, 0, 0], [2, 4, 0, 0], SampleEncoding::U16).unwrap();
    let mut m = mapping();
    m.axes[1] = a::Axis::Sample;
    assert!(matches!(map_layout(&l, m), Err(a::Error::Shape)));
}
#[test]
fn first_sample_clock_is_exact_and_does_not_convert_native_completion_time() {
    let obs = observation(ObservationTimeAnchor::FirstSample);
    assert!(validate_clock(obs, Some(clock())).is_ok());
    assert!(matches!(
        validate_clock(
            observation(ObservationTimeAnchor::ReadCompletion),
            Some(clock())
        ),
        Err(a::Error::Clock)
    ));
    assert!(validate_clock(observation(ObservationTimeAnchor::ReadCompletion), None).is_ok());
    let mut t = clock().timing();
    t.start_tick = 101;
    assert!(validate_clock(obs, Some(a::Clock::new(id(2), 3, t).unwrap())).is_err());
    assert!(validate_clock(
        obs,
        Some(a::Clock::new(id(3), 3, clock().timing()).unwrap())
    )
    .is_err());
    assert!(validate_clock(
        obs,
        Some(a::Clock::new(id(2), 4, clock().timing()).unwrap())
    )
    .is_err());
    t = clock().timing();
    t.scale = 1;
    assert!(validate_clock(obs, Some(a::Clock::new(id(2), 3, t).unwrap())).is_err());
}
