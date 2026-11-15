use super::*;
use crate::medical::{
    dicom_codecs::{
        decode_rle_lossless_into,
        float_pixels::{FloatBits, FloatPixelView},
        NativePixelView, RleLimits,
    },
    dicom_dataset::{ParseLimits, Part10Dataset, WorkBudget},
    dicom_frames::FrameSet,
};
fn parse(b: &[u8]) -> Part10Dataset<'_> {
    Part10Dataset::parse(b, ParseLimits::default()).unwrap()
}
#[test]
fn signed_integer_padding_is_stored_domain_inclusive_and_precedes_scaling() {
    let ds = parse(include_bytes!("fixtures/integer-signed12-range.dcm"));
    let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    let p = IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)).unwrap();
    let frames = FrameSet::extract(&ds, Default::default()).unwrap();
    assert_eq!(p.padding().endpoints(), Some([-2048, -2040]));
    for c in [0, 1, 2] {
        assert_eq!(p.sample(0, 0, c).unwrap().validity, SampleValidity::Padding);
        assert_eq!(
            p.quantitative_sample(&frames.frames[0], 0, c, false)
                .unwrap(),
            QuantitativeSample::Padding
        );
    }
    assert!(matches!(
        p.quantitative_sample(&frames.frames[0], 0, 3, false)
            .unwrap(),
        QuantitativeSample::Value { value: -5078.0, .. }
    ));
    assert_eq!(p.sample(0, 0, 4).unwrap().validity, SampleValidity::Valid);
}
#[test]
fn monochrome1_reversed_endpoint_order_and_single_value_are_correct() {
    let ds = parse(include_bytes!("fixtures/integer-u16-mono1-range.dcm"));
    let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    let p = IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.padding().endpoints(), Some([65535, 65530]));
    for c in 0..3 {
        assert_eq!(p.sample(0, 0, c).unwrap().validity, SampleValidity::Padding);
    }
    assert_eq!(p.sample(0, 0, 3).unwrap().validity, SampleValidity::Valid);
    let ds = parse(include_bytes!("fixtures/integer-u8-single.dcm"));
    let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    let p = IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.padding().endpoints(), Some([0, 0]));
    assert_eq!(p.sample(0, 0, 0).unwrap().validity, SampleValidity::Padding);
    assert_eq!(p.sample(0, 0, 1).unwrap().validity, SampleValidity::Valid);
}
#[test]
fn lossless_rle_padding_uses_identical_numeric_semantics_and_source_binding() {
    let ds = parse(include_bytes!("fixtures/integer-signed12-rle-range.dcm"));
    let mut out = [0; 12];
    let mut scratch = [0; 6];
    let pixels = decode_rle_lossless_into(
        &ds,
        &mut out,
        &mut scratch,
        RleLimits::default(),
        &mut WorkBudget::new(10000),
    )
    .unwrap();
    let p = IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.sample(0, 0, 0).unwrap().validity, SampleValidity::Padding);
    assert_eq!(p.sample(0, 0, 3).unwrap().validity, SampleValidity::Valid);
    let frames = FrameSet::extract(&ds, Default::default()).unwrap();
    assert!(matches!(
        p.quantitative_sample(&frames.frames[0], 0, 3, false)
            .unwrap(),
        QuantitativeSample::Value { value: -5078.0, .. }
    ));
    let other = parse(include_bytes!("fixtures/integer-signed12-range.dcm"));
    assert!(matches!(
        IntegerPaddingView::bind(&other, &pixels, &mut WorkBudget::new(10000)),
        Err(PaddingError::SourceMismatch)
    ));
}
#[test]
fn malformed_integer_vr_domain_and_photometric_order_fail_closed() {
    for bytes in [
        include_bytes!("fixtures/integer-wrong-vr.dcm").as_slice(),
        include_bytes!("fixtures/integer-out-of-range.dcm").as_slice(),
        include_bytes!("fixtures/integer-reversed-range.dcm").as_slice(),
    ] {
        let ds = parse(bytes);
        let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
        assert!(IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)).is_err());
    }
    let mut bytes = include_bytes!("fixtures/integer-signed12-range.dcm").to_vec();
    let at = bytes
        .windows(6)
        .position(|b| b == [0x28, 0, 0x20, 1, b'S', b'S'])
        .unwrap();
    bytes[at + 2] = 0x1f;
    let ds = parse(&bytes);
    let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert!(matches!(
        IntegerPaddingView::bind(&ds, &pixels, &mut WorkBudget::new(10000)),
        Err(PaddingError::Missing(0x00280120))
    ));
}
#[test]
fn float_pair_requirements_and_nan_declarations_preserve_uncertainty_and_bits() {
    let ds = parse(include_bytes!(
        "../float_pixels/fixtures/float32-nan-padding.dcm"
    ));
    let p = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(
        p.padding().endpoints(),
        Some([FloatBits::F32(0x7fc00001), FloatBits::F32(0x7fc00002)])
    );
    assert_eq!(
        p.sample(0, 0, 5).unwrap().validity,
        SampleValidity::PaddingIndeterminate
    );
    assert_eq!(p.sample_bits(1, 0, 2).unwrap(), FloatBits::F32(0x7fc12345));
    let frames = FrameSet::extract(&ds, Default::default()).unwrap();
    assert_eq!(
        p.quantitative_sample(&frames.frames[0], 0, 5, true)
            .unwrap(),
        QuantitativeSample::PaddingIndeterminate
    );
    let mut bytes = include_bytes!("../float_pixels/fixtures/float32-enhanced.dcm").to_vec();
    let at = bytes
        .windows(6)
        .position(|b| b == [0x28, 0, 0x24, 1, b'F', b'L'])
        .unwrap();
    bytes[at + 2] = 0x26;
    let ds = parse(&bytes);
    assert!(matches!(
        FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)),
        Err(PaddingError::Missing(0x00280124))
    ));
}

#[test]
fn infinity_padding_is_ordered_and_float_single_values_still_require_both_attributes() {
    let original = include_bytes!("../float_pixels/fixtures/float32-enhanced.dcm");
    let mut bytes = original.to_vec();
    let value = bytes
        .windows(6)
        .position(|b| b == [0x28, 0, 0x22, 1, b'F', b'L'])
        .unwrap()
        + 8;
    bytes[value..value + 4].copy_from_slice(&0xff800000u32.to_le_bytes());
    let ds = parse(&bytes);
    let view = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(
        view.sample_bits(1, 0, 1).unwrap(),
        FloatBits::F32(0xff800000)
    );
    assert_eq!(
        view.sample(1, 0, 1).unwrap().validity,
        SampleValidity::Padding
    );
    assert_eq!(
        view.sample(1, 0, 0).unwrap().validity,
        SampleValidity::NonFinite
    );
    let mut bytes = original.to_vec();
    let at = bytes
        .windows(6)
        .position(|b| b == [0x28, 0, 0x22, 1, b'F', b'L'])
        .unwrap();
    bytes[at + 2] = 0x19;
    let ds = parse(&bytes);
    assert!(matches!(
        FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)),
        Err(PaddingError::Missing(0x00280122))
    ));
    let mut bytes = original.to_vec();
    let at = bytes
        .windows(6)
        .position(|b| b == [0x28, 0, 0x22, 1, b'F', b'L'])
        .unwrap();
    bytes[at + 4..at + 6].copy_from_slice(b"US");
    let ds = parse(&bytes);
    assert!(FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).is_err());
}
