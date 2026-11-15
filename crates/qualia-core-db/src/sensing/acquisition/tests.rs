use super::*;
use crate::sensing::SemanticId;
fn id(x: u64) -> SemanticId {
    SemanticId::new(x).unwrap()
}
fn context() -> Context {
    Context {
        quantity: Quantity {
            kind: id(1),
            unit: id(2),
        },
        stage: SampleStage::RawCodes,
        sensor: SemanticId::Unknown,
        geometry: None,
        calibration: None,
        uncertainty: None,
    }
}
fn axes() -> SignalAxes {
    SignalAxes {
        carrier: Carrier::Unknown,
        baseband_hz: None,
        pulse_rate: None,
        complex_convention: None,
        phase_origin: None,
    }
}
fn clock() -> Clock {
    Clock::new(
        id(7),
        3,
        Timing {
            scale: 0,
            epoch_seconds: 0,
            epoch_nanoseconds: 0,
            ticks: Rate::new(96000, 1).unwrap(),
            samples: Rate::new(48000, 1).unwrap(),
            start_tick: -1,
        },
    )
    .unwrap()
}
fn layout(encoding: Encoding, count: u32) -> Layout {
    Layout {
        rank: 1,
        extents: [count, 0, 0, 0],
        strides: [encoding.bytes(), 0, 0, 0],
        axes: [Axis::Sample; 4],
        encoding,
        byte_order: ByteOrder::Little,
        components: if encoding.complex() {
            ComponentOrder::RealImag
        } else {
            ComponentOrder::Scalar
        },
    }
}
fn link() -> ArtifactLink {
    ArtifactLink {
        digest: [1; 32],
        identifier: id(22),
    }
}
#[test]
fn original_metadata_and_samples_are_exactly_retained() {
    let bytes = include_bytes!("fixtures/iq-with-metadata.le");
    let original = Original::admit(bytes, bytes.len()).unwrap();
    assert_eq!(original.digest(), *blake3::hash(bytes).as_bytes());
    let mut a = axes();
    a.carrier = Carrier::Hertz(2_400_000_000.0);
    a.baseband_hz = Some([-2000.0, 2000.0]);
    a.complex_convention = Some(ComplexConvention::RealPlusIImagPositiveExponent);
    a.phase_origin = Some(link());
    let view = TensorView::admit(
        original.region(3, 32).unwrap(),
        layout(Encoding::Shared(Scalar::ComplexF32), 4),
        context(),
        Some(clock()),
        a,
    )
    .unwrap();
    assert_eq!(view.numeric([0, 0, 0, 0]).unwrap(), [1.0, 0.0]);
    assert_eq!(view.numeric([1, 0, 0, 0]).unwrap(), [0.0, 1.0]);
    assert_eq!(view.numeric([2, 0, 0, 0]).unwrap(), [-1.0, 0.0]);
    assert_eq!(view.numeric([3, 0, 0, 0]).unwrap(), [0.0, -1.0]);
    assert_eq!(view.region().original().bytes(), bytes);
    assert_eq!(view.region().offset(), 3);
    assert_eq!(view.signal_axes().carrier, Carrier::Hertz(2_400_000_000.0));
}
#[test]
fn ieee_raw_bits_survive_while_scientific_access_fails_closed() {
    let bytes = [0x00, 0x00, 0x00, 0x80, 0x45, 0x23, 0xc1, 0x7f];
    let original = Original::admit(&bytes, 8).unwrap();
    let view = TensorView::admit(
        original.region(0, 8).unwrap(),
        layout(Encoding::Shared(Scalar::F32), 2),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(view.element([0, 0, 0, 0]).unwrap(), &bytes[..4]);
    assert!(view.numeric([0, 0, 0, 0]).unwrap()[0].is_sign_negative());
    assert_eq!(view.element([1, 0, 0, 0]).unwrap(), &bytes[4..]);
    assert_eq!(view.numeric([1, 0, 0, 0]), Err(Error::Nonfinite));
}
#[test]
fn unsigned_adc_and_complex_f64_have_no_silent_precision_loss() {
    let bytes = u64::MAX.to_le_bytes();
    let o = Original::admit(&bytes, 8).unwrap();
    let v = TensorView::admit(
        o.region(0, 8).unwrap(),
        layout(Encoding::U64, 1),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.element([0; 4]).unwrap(), &bytes);
    assert_eq!(v.numeric([0; 4]), Err(Error::Unsupported));
    let bytes = [u32::MAX.to_le_bytes(), 0u32.to_le_bytes()].concat();
    let o = Original::admit(&bytes, 8).unwrap();
    let v = TensorView::admit(
        o.region(0, 8).unwrap(),
        layout(Encoding::U32, 2),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.numeric([0; 4]).unwrap()[0], 4294967295.0);
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&(-2.5f64).to_be_bytes());
    bytes[8..].copy_from_slice(&1.25f64.to_be_bytes());
    let o = Original::admit(&bytes, 16).unwrap();
    let mut l = layout(Encoding::ComplexF64, 1);
    l.components = ComponentOrder::ImagReal;
    l.byte_order = ByteOrder::Big;
    let v = TensorView::admit(o.region(0, 16).unwrap(), l, context(), None, axes()).unwrap();
    assert_eq!(v.numeric([0; 4]).unwrap(), [1.25, -2.5]);
}
#[test]
fn provider_complex_i16_and_unsigned_u16_are_explicit() {
    let bytes = [0xff, 0x7f, 0x00, 0x80];
    let o = Original::admit(&bytes, 4).unwrap();
    let v = TensorView::admit(
        o.region(0, 4).unwrap(),
        layout(Encoding::ComplexI16, 1),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.numeric([0; 4]).unwrap(), [32767.0, -32768.0]);
    let v = TensorView::admit(
        o.region(0, 4).unwrap(),
        layout(Encoding::U16, 2),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.numeric([1, 0, 0, 0]).unwrap(), [32768.0, 0.0]);
}
#[test]
fn source_and_shape_admission_reject_before_unbounded_work() {
    assert!(matches!(Original::admit(&[1, 2], 1), Err(Error::Budget)));
    let o = Original::admit(&[1, 2], 2).unwrap();
    assert!(matches!(o.region(usize::MAX, 2), Err(Error::Overflow)));
    assert!(matches!(o.region(1, 2), Err(Error::Bounds)));
    assert!(matches!(
        TensorView::admit(
            o.region(0, 1).unwrap(),
            layout(Encoding::U16, 1),
            context(),
            None,
            axes()
        ),
        Err(Error::Bounds)
    ));
    let mut l = layout(Encoding::Shared(Scalar::F64), u32::MAX);
    l.strides[0] = usize::MAX;
    assert_eq!(l.span(), Err(Error::Overflow));
    l = layout(Encoding::U16, 2);
    l.strides[0] = 1;
    assert_eq!(l.span(), Err(Error::Shape));
    l = layout(Encoding::ComplexI16, 1);
    l.components = ComponentOrder::Scalar;
    assert_eq!(l.span(), Err(Error::Shape));
    l = layout(Encoding::U16, 1);
    l.rank = 5;
    assert_eq!(l.span(), Err(Error::Shape));
}
#[test]
fn strided_channel_order_is_preserved_with_unaligned_source() {
    let bytes = [9, 1, 0, 3, 0, 2, 0, 4, 0, 8];
    let o = Original::admit(&bytes, 10).unwrap();
    let l = Layout {
        rank: 2,
        extents: [2, 2, 0, 0],
        strides: [2, 4, 0, 0],
        axes: [Axis::Sample, Axis::Channel, Axis::Sample, Axis::Sample],
        encoding: Encoding::U16,
        byte_order: ByteOrder::Little,
        components: ComponentOrder::Scalar,
    };
    let v =
        TensorView::admit(o.region(1, 8).unwrap(), l, context(), Some(clock()), axes()).unwrap();
    assert_eq!(v.numeric([1, 0, 0, 0]).unwrap()[0], 3.0);
    assert_eq!(v.numeric([0, 1, 0, 0]).unwrap()[0], 2.0);
    assert_eq!(v.element([0, 0, 1, 0]), Err(Error::Bounds));
}
#[test]
fn exact_time_and_clock_identity_do_not_infer_synchronization() {
    assert_eq!(
        clock().time_at(0).unwrap(),
        ExactTime {
            numerator: -1,
            denominator: 96000
        }
    );
    assert_eq!(
        clock().time_at(1).unwrap(),
        ExactTime {
            numerator: 1,
            denominator: 96000
        }
    );
    let c = clock();
    let other = Clock::new(id(8), c.restart(), c.timing()).unwrap();
    assert_eq!(c.require_same_domain(other), Err(Error::Clock));
    assert_eq!(
        c.require_same_domain(Clock::new(id(7), 4, c.timing()).unwrap()),
        Err(Error::Clock)
    );
    assert!(Clock::new(SemanticId::Unknown, 0, c.timing()).is_err());
    let mut t = c.timing();
    t.samples = Rate {
        numerator: 0,
        denominator: 1,
    };
    assert!(Clock::new(id(7), 0, t).is_err());
}
#[path = "tests/records.rs"]
mod records;
