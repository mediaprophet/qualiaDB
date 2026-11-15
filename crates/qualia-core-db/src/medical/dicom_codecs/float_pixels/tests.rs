use super::*;
use crate::medical::{
    dicom_codecs::padding::{PaddingError, QuantitativeSample, SampleValidity},
    dicom_dataset::{ParseLimits, Part10Dataset, WorkBudget},
    dicom_frames::{FrameSet, ValueUnits},
};
fn parse(b: &[u8]) -> Part10Dataset<'_> {
    Part10Dataset::parse(b, ParseLimits::default()).unwrap()
}
#[test]
fn exact_f32_bits_survive_nonfinite_signed_zero_and_subnormal_classification() {
    let bytes = include_bytes!("fixtures/float32-enhanced.dcm");
    let ds = parse(bytes);
    let p = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.width(), FloatWidth::F32);
    assert_eq!((p.rows(), p.columns(), p.frames()), (1, 6, 2));
    let expected = [
        0x80000000, 0, 0xc1200000, 0xc0e00000, 0xc0a00000, 0x3fc00000, 0x7f800000, 0xff800000,
        0x7fc12345, 0xff801234, 1, 0xc0200000,
    ];
    for (i, bits) in expected.into_iter().enumerate() {
        assert_eq!(
            p.sample_bits((i / 6) as u32, 0, (i % 6) as u16).unwrap(),
            FloatBits::F32(bits)
        );
    }
    assert_eq!(p.sample(0, 0, 0).unwrap().validity, SampleValidity::Valid);
    assert_eq!(
        p.sample_bits(0, 0, 0)
            .unwrap()
            .finite_f64()
            .unwrap()
            .to_bits(),
        (-0.0f64).to_bits()
    );
    for c in [0, 1, 2, 3] {
        assert_eq!(
            p.sample(1, 0, c).unwrap().validity,
            SampleValidity::NonFinite
        );
    }
    assert!(p.sample_bits(1, 0, 4).unwrap().finite_f64().unwrap() > 0.0);
    assert_eq!(ds.original_bytes(), bytes);
    assert_eq!(p.source_blake3(), *blake3::hash(bytes).as_bytes());
}
#[test]
fn inclusive_padding_precedes_mapping_and_per_frame_source_units_are_retained() {
    let ds = parse(include_bytes!("fixtures/float32-enhanced.dcm"));
    let p = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    let frames = FrameSet::extract(&ds, Default::default()).unwrap();
    for c in [2, 3, 4] {
        assert_eq!(p.sample(0, 0, c).unwrap().validity, SampleValidity::Padding);
        assert_eq!(
            p.quantitative_sample(&frames.frames[0], 0, c, true)
                .unwrap(),
            QuantitativeSample::Padding
        );
    }
    assert!(matches!(
        p.quantitative_sample(&frames.frames[0], 0, 5, true)
            .unwrap(),
        QuantitativeSample::Value {
            value: 4.0,
            units: ValueUnits::Code {
                scheme: "UCUM",
                value: "mm",
                ..
            },
            frame_number: 1,
            ..
        }
    ));
    assert!(matches!(
        p.quantitative_sample(&frames.frames[1], 0, 5, true)
            .unwrap(),
        QuantitativeSample::Value {
            value: 2.5,
            frame_number: 2,
            ..
        }
    ));
    assert_eq!(
        p.quantitative_sample(&frames.frames[1], 0, 2, true)
            .unwrap(),
        QuantitativeSample::NonFinite
    );
    let other = parse(include_bytes!("fixtures/float32-no-padding.dcm"));
    let other_frames = FrameSet::extract(&other, Default::default()).unwrap();
    assert!(matches!(
        p.quantitative_sample(&other_frames.frames[0], 0, 0, true),
        Err(PaddingError::SourceMismatch)
    ));
}
#[test]
fn f64_raw_lanes_and_zero_padding_do_not_canonicalize_bit_patterns() {
    let ds = parse(include_bytes!("fixtures/float64-zero-padding.dcm"));
    let p = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    let expected = [
        0x8000000000000000,
        0,
        1,
        0x400921fb54442d18,
        0x7ff8123456789abc,
        0xfff0000000001234,
        0x7ff0000000000000,
        0xfff0000000000000,
    ];
    for (i, b) in expected.into_iter().enumerate() {
        assert_eq!(
            p.sample_bits((i / 4) as u32, 0, (i % 4) as u16).unwrap(),
            FloatBits::F64(b)
        );
    }
    assert_eq!(p.sample(0, 0, 0).unwrap().validity, SampleValidity::Padding);
    assert_eq!(p.sample(0, 0, 1).unwrap().validity, SampleValidity::Padding);
    assert_eq!(p.sample(0, 0, 2).unwrap().validity, SampleValidity::Valid);
    assert_eq!(
        p.sample(0, 0, 3)
            .unwrap()
            .bits
            .finite_f64()
            .unwrap()
            .to_bits(),
        0x400921fb54442d18
    );
}
#[test]
fn unaligned_sources_and_explicit_byte_and_frame_bounds_are_checked() {
    let source = include_bytes!("fixtures/float32-no-padding.dcm");
    let mut shifted = vec![0x77];
    shifted.extend_from_slice(source);
    let ds = parse(&shifted[1..]);
    let p = FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.sample_bits(0, 0, 0).unwrap(), FloatBits::F32(0x80000000));
    assert!(p.sample_bits(2, 0, 0).is_err());
    assert!(p.sample_bits(0, 1, 0).is_err());
    assert!(p.sample_bits(0, 0, 6).is_err());
    assert!(FloatPixelView::from_dataset(&ds, 1, &mut WorkBudget::new(10000)).is_err());
    assert!(FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(0)).is_err());
}
#[test]
fn conflicting_integer_attributes_pixel_fields_and_wrong_float_vr_are_not_cast() {
    for bytes in [
        include_bytes!("fixtures/float32-forbidden-fields.dcm").as_slice(),
        include_bytes!("fixtures/float32-mixed-pixels.dcm").as_slice(),
        include_bytes!("fixtures/float64-wrong-vr.dcm").as_slice(),
    ] {
        let ds = parse(bytes);
        assert!(FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).is_err());
    }
    let ds = parse(include_bytes!("../../dicom_dataset/fixtures/non-image.dcm"));
    assert!(matches!(
        FloatPixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)),
        Err(PaddingError::Missing(0x7fe00008))
    ));
}
