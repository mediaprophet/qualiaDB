use super::*;
fn event_bytes(tick: i64, channel: u32, count: u64) -> [u8; 20] {
    let mut b = [0; 20];
    b[..8].copy_from_slice(&tick.to_le_bytes());
    b[8..12].copy_from_slice(&channel.to_le_bytes());
    b[12..].copy_from_slice(&count.to_le_bytes());
    b
}
#[test]
fn event_counts_are_not_images_or_float_amplitudes() {
    let b = [event_bytes(-1, 1, u64::MAX), event_bytes(3, 0, 0)].concat();
    let o = Original::admit(&b, 40).unwrap();
    let v = EventView::admit(
        o.region(0, 40).unwrap(),
        2,
        ByteOrder::Little,
        EventOrdering::NondecreasingTick,
        clock(),
        context(),
        2,
    )
    .unwrap();
    assert_eq!(v.event(0).unwrap().count, u64::MAX);
    assert_eq!(v.event(1).unwrap().count, 0);
    assert_eq!(
        v.time_at(1).unwrap(),
        ExactTime {
            numerator: 1,
            denominator: 32000
        }
    );
    assert_eq!(v.region().original().digest(), o.digest());
    assert!(matches!(
        EventView::admit(
            o.region(0, 40).unwrap(),
            2,
            ByteOrder::Little,
            EventOrdering::NondecreasingTick,
            clock(),
            context(),
            1
        ),
        Err(Error::Budget)
    ));
}
#[test]
fn event_ordering_and_truncation_are_checked() {
    let b = [event_bytes(3, 0, 1), event_bytes(-1, 0, 1)].concat();
    let o = Original::admit(&b, 40).unwrap();
    assert!(matches!(
        EventView::admit(
            o.region(0, 40).unwrap(),
            1,
            ByteOrder::Little,
            EventOrdering::NondecreasingTick,
            clock(),
            context(),
            2
        ),
        Err(Error::Ordering)
    ));
    assert!(EventView::admit(
        o.region(0, 40).unwrap(),
        1,
        ByteOrder::Little,
        EventOrdering::AcquisitionOrder,
        clock(),
        context(),
        2
    )
    .is_ok());
    assert!(matches!(
        EventView::admit(
            o.region(0, 39).unwrap(),
            1,
            ByteOrder::Little,
            EventOrdering::AcquisitionOrder,
            clock(),
            context(),
            2
        ),
        Err(Error::Shape)
    ));
    let b = event_bytes(0, 1, 1);
    let o = Original::admit(&b, 20).unwrap();
    assert!(matches!(
        EventView::admit(
            o.region(0, 20).unwrap(),
            1,
            ByteOrder::Little,
            EventOrdering::AcquisitionOrder,
            clock(),
            context(),
            1
        ),
        Err(Error::Bounds)
    ));
}
#[test]
fn correlations_retain_lag_pair_order_and_normalization() {
    let b = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>();
    let o = Original::admit(&b, 32).unwrap();
    let l = Layout {
        rank: 3,
        extents: [2, 1, 2, 0],
        strides: [16, 16, 8, 0],
        axes: [
            Axis::Lag,
            Axis::LeftChannel,
            Axis::RightChannel,
            Axis::Sample,
        ],
        encoding: Encoding::Shared(Scalar::ComplexF32),
        byte_order: ByteOrder::Little,
        components: ComponentOrder::RealImag,
    };
    let v = TensorView::admit(o.region(0, 32).unwrap(), l, context(), None, axes()).unwrap();
    let c = CorrelationView::admit(
        v,
        clock(),
        -1,
        2,
        ProductConvention::LeftTimesConjugateRight,
        Normalization::Mean {
            contributing_pairs: 4,
        },
    )
    .unwrap();
    assert_eq!(
        c.lag_seconds(0).unwrap(),
        ExactTime {
            numerator: -1,
            denominator: 96000
        }
    );
    assert_eq!(
        c.lag_seconds(1).unwrap(),
        ExactTime {
            numerator: 1,
            denominator: 96000
        }
    );
    assert_eq!(c.tensor().numeric([1, 0, 1, 0]).unwrap(), [7.0, 8.0]);
    assert!(matches!(
        CorrelationView::admit(
            v,
            clock(),
            0,
            1,
            ProductConvention::LeftTimesRight,
            Normalization::Mean {
                contributing_pairs: 0
            }
        ),
        Err(Error::Semantics)
    ));
    assert!(matches!(
        CorrelationView::admit(
            v,
            clock(),
            i64::MAX,
            1,
            ProductConvention::LeftTimesRight,
            Normalization::Sum
        ),
        Err(Error::Overflow)
    ));
}
#[test]
fn unknown_metadata_is_not_zero_or_calibration() {
    let b = [0u8; 2];
    let o = Original::admit(&b, 2).unwrap();
    let mut ctx = context();
    ctx.quantity.unit = SemanticId::Unknown;
    let v = TensorView::admit(
        o.region(0, 2).unwrap(),
        layout(Encoding::U16, 1),
        ctx,
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.element([0; 4]).unwrap(), b);
    assert_eq!(v.numeric([0; 4]), Err(Error::Semantics));
    let mut a = axes();
    a.carrier = Carrier::Hertz(0.0);
    assert_eq!(a.validate(), Err(Error::Semantics));
    a = axes();
    a.baseband_hz = Some([f64::NEG_INFINITY, 1.0]);
    assert_eq!(a.validate(), Err(Error::Semantics));
    ctx = context();
    ctx.calibration = Some(ArtifactLink {
        identifier: SemanticId::Unknown,
        ..link()
    });
    assert_eq!(ctx.validate(), Err(Error::Identity));
}
#[test]
fn derivation_requires_declared_losses_and_keeps_all_originals() {
    let raw = Original::admit(b"raw phase and metadata", 32).unwrap();
    let out = Original::admit(b"magnitude", 32).unwrap();
    let input = [raw];
    let loss = [Loss::PhaseDiscarded, Loss::Quantization];
    let d = Derivation::admit(out, &input, link(), link(), &loss, 3).unwrap();
    assert_eq!(d.inputs()[0].bytes(), b"raw phase and metadata");
    assert_ne!(d.inputs()[0].digest(), d.output().digest());
    assert_eq!(d.losses(), loss);
    assert!(matches!(
        Derivation::admit(out, &input, link(), link(), &[], 3),
        Err(Error::LossDeclaration)
    ));
    assert!(matches!(
        Derivation::admit(
            out,
            &input,
            link(),
            link(),
            &[Loss::NoneDeclared, Loss::PhaseDiscarded],
            3
        ),
        Err(Error::LossDeclaration)
    ));
    assert!(matches!(
        Derivation::admit(out, &input, link(), link(), &loss, 2),
        Err(Error::Budget)
    ));
}

#[test]
fn opaque_encoding_and_targeted_uncertainty_fail_closed() {
    let bytes = [0x42, 0xff, 0x00, 0x80, 0x01];
    let o = Original::admit(&bytes, 5).unwrap();
    let enc = Encoding::Opaque {
        profile: id(99),
        element_bytes: 5,
    };
    let v = TensorView::admit(
        o.region(0, 5).unwrap(),
        layout(enc, 1),
        context(),
        None,
        axes(),
    )
    .unwrap();
    assert_eq!(v.element([0; 4]).unwrap(), bytes);
    assert_eq!(v.numeric([0; 4]), Err(Error::Unsupported));
    let mut ctx = context();
    ctx.uncertainty = Some(UncertaintyLink {
        artifact: link(),
        target: Quantity {
            kind: id(1),
            unit: id(3),
        },
    });
    assert_eq!(ctx.validate(), Err(Error::Semantics));
    ctx.uncertainty = Some(UncertaintyLink {
        artifact: link(),
        target: ctx.quantity,
    });
    assert!(ctx.validate().is_ok());
}
