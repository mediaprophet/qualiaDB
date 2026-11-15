use super::*;
use crate::medical::{
    dicom_dataset::{ParseLimits, Part10Dataset},
    dicom_geometry::{GeometryAttribute, GeometryError},
};
fn parse(bytes: &[u8]) -> Part10Dataset<'_> {
    Part10Dataset::parse(bytes, ParseLimits::default()).unwrap()
}
#[test]
fn enhanced_frames_have_distinct_geometry_identities_and_scaling() {
    for bytes in [
        include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm").as_slice(),
        include_bytes!("../dicom_dataset/fixtures/enhanced-undefined.dcm").as_slice(),
    ] {
        let ds = parse(bytes);
        let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
        assert!(frames.enhanced);
        assert_eq!(frames.frames.len(), 2);
        assert_eq!(frames.frames[0].frame_number(), 1);
        assert_eq!(frames.frames[1].frame_number(), 2);
        assert_eq!(
            frames.frames[0]
                .plane()
                .unwrap()
                .pixel_center_lps_mm(1, 1)
                .unwrap(),
            [0.75, 0.5, 0.0]
        );
        assert_eq!(
            frames.frames[1].plane().unwrap().origin_lps_mm(),
            [0.0, 0.0, 5.0]
        );
        assert_eq!(
            frames.identity.sop_instance_uid,
            Some("1.2.826.0.1.3680043.10.100.1")
        );
        assert!(frames.identity.study_instance_uid.is_some());
        assert!(frames.identity.series_instance_uid.is_some());
        let a = frames.frames[0].quantitative().modality.unwrap();
        let b = frames.frames[1].quantitative().modality.unwrap();
        assert_eq!(a.apply(-2048.0).unwrap(), -5096.0);
        assert_eq!(b.apply(2047.0).unwrap(), 6151.0);
        assert_eq!(a.source_blake3(), ds.source_blake3());
        assert_eq!(b.frame_number(), 2);
        assert_eq!(a.units(), ValueUnits::RescaleType("HU"));
    }
}
#[test]
fn shared_per_frame_top_level_and_item_count_ambiguity_fail_closed() {
    for (bytes, tag) in [
        (
            include_bytes!("../dicom_dataset/fixtures/enhanced-conflict.dcm").as_slice(),
            0x00209113,
        ),
        (
            include_bytes!("../dicom_dataset/fixtures/enhanced-top-conflict.dcm").as_slice(),
            0x00200032,
        ),
    ] {
        let ds = parse(bytes);
        assert!(
            matches!(FrameSet::extract(&ds,FrameLimits::default()),Err(FrameError::SharedPerFrameConflict(t)|FrameError::TopLevelConflict(t)) if t==tag)
        );
    }
    let ds = parse(include_bytes!(
        "../dicom_dataset/fixtures/enhanced-count-mismatch.dcm"
    ));
    assert!(matches!(
        FrameSet::extract(&ds, FrameLimits::default()),
        Err(FrameError::MissingFrames)
    ));
}
#[test]
fn missing_invalid_and_frame_budget_are_distinct() {
    let ds = parse(include_bytes!(
        "../dicom_dataset/fixtures/enhanced-missing.dcm"
    ));
    let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    assert!(matches!(
        frames.frames[1].plane(),
        Err(GeometryError::Missing(
            GeometryAttribute::ImagePositionPatient
        ))
    ));
    assert!(frames.frames[0].plane().is_ok());
    assert!(matches!(
        FrameSet::extract(
            &ds,
            FrameLimits {
                max_frames: 1,
                ..Default::default()
            }
        ),
        Err(FrameError::FrameLimit)
    ));
    let ds = parse(include_bytes!(
        "../dicom_dataset/fixtures/enhanced-invalid.dcm"
    ));
    let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    assert!(matches!(
        frames.frames[1].plane(),
        Err(GeometryError::Invalid(
            GeometryAttribute::ImagePositionPatient
        ))
    ));
}
#[test]
fn real_world_units_are_frame_source_bound_and_not_composed_with_rescale() {
    let ds = parse(include_bytes!(
        "../dicom_dataset/fixtures/native-u8-rwvm.dcm"
    ));
    let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    let q = frames.frames[0].quantitative();
    assert_eq!(q.modality.unwrap().apply(128.0).unwrap(), 261.0);
    assert_eq!(q.modality.unwrap().units(), ValueUnits::Unspecified);
    let rw = q.real_world.unwrap();
    assert_eq!(rw.apply(128.0).unwrap(), 33.5);
    assert_eq!(
        rw.units(),
        ValueUnits::Code {
            scheme: "UCUM",
            value: "mm",
            meaning: Some("millimeter")
        }
    );
    assert_eq!(rw.frame_number(), 1);
    assert_eq!(rw.source_blake3(), ds.source_blake3());
    assert!(rw.apply(256.0).is_err());
}
#[test]
fn non_image_objects_have_identity_without_invented_frames_or_pixels() {
    let ds = parse(include_bytes!("../dicom_dataset/fixtures/non-image.dcm"));
    let f = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    assert!(f.frames.is_empty());
    assert_eq!(f.identity.source_blake3, ds.source_blake3());
}

#[test]
fn quantitative_sample_requires_matching_source_and_frame() {
    use crate::medical::{dicom_codecs::NativePixelView, dicom_dataset::WorkBudget};
    let ds = parse(include_bytes!(
        "../dicom_dataset/fixtures/enhanced-defined.dcm"
    ));
    let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    let pixels = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(
        frames.frames[0]
            .quantitative_sample(&pixels, 0, 0, false)
            .unwrap(),
        Some(-5096.0)
    );
    assert_eq!(
        frames.frames[1]
            .quantitative_sample(&pixels, 0, 1, false)
            .unwrap(),
        Some(6151.0)
    );
    assert_eq!(
        frames.frames[0]
            .quantitative_sample(&pixels, 0, 0, true)
            .unwrap(),
        None
    );
    let other = parse(include_bytes!(
        "../dicom_dataset/fixtures/enhanced-undefined.dcm"
    ));
    let other_pixels =
        NativePixelView::from_dataset(&other, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert!(matches!(
        frames.frames[0].quantitative_sample(&other_pixels, 0, 0, false),
        Err(FrameError::IdentityMismatch)
    ));
    assert!(matches!(
        FrameSet::extract(
            &ds,
            FrameLimits {
                max_frame_bytes: 0,
                ..Default::default()
            }
        ),
        Err(FrameError::FrameLimit)
    ));
}
#[test]
fn nonfinite_real_world_mapping_is_invalid_not_missing_or_identity() {
    let mut bytes = include_bytes!("../dicom_dataset/fixtures/native-u8-rwvm.dcm").to_vec();
    let pattern = [0x40, 0, 0x25, 0x92, b'F', b'D', 8, 0];
    let at = bytes.windows(8).position(|b| b == pattern).unwrap() + 8;
    bytes[at..at + 8].copy_from_slice(&f64::NAN.to_le_bytes());
    let ds = parse(&bytes);
    assert!(matches!(
        FrameSet::extract(&ds, FrameLimits::default()),
        Err(FrameError::Dataset(
            crate::medical::dicom_dataset::DatasetError::Invalid(0x00409225)
        ))
    ));
}
