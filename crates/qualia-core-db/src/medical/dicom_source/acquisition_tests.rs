use super::*;
use crate::medical::{
    dicom_codecs::NativePixelView,
    dicom_dataset::{DatasetError, ParseLimits, WorkBudget},
    dicom_frames::{FrameError, FrameLimits},
};
fn retain(bytes: &[u8], root: &std::path::Path) -> RetainedDicom {
    let path = root.join("input.dcm");
    std::fs::write(&path, bytes).unwrap();
    RetainedDicom::retain(
        root,
        &path,
        DicomSourceBudget {
            max_source_bytes: bytes.len(),
        },
    )
    .unwrap()
}
#[test]
fn retained_enhanced_acquisition_binds_samples_frames_and_original() {
    let original = include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm");
    let root = tempfile::TempDir::new().unwrap();
    let retained = retain(original, root.path());
    // Replacing the input path cannot alter the retained snapshot/identity.
    std::fs::write(root.path().join("input.dcm"), b"replaced").unwrap();
    assert_eq!(retained.original_bytes(), original);
    assert_eq!(std::fs::read(&retained.source().path).unwrap(), original);
    let view = retained
        .inspect(ParseLimits::default(), FrameLimits::default())
        .unwrap();
    assert_eq!(view.dataset().source_blake3(), retained.source().blake3);
    let frames = view.frames().unwrap();
    assert_eq!(frames.frames.len(), 2);
    assert_eq!(frames.identity.source_blake3, retained.source().blake3);
    let pixels =
        NativePixelView::from_dataset(view.dataset(), 1024, &mut WorkBudget::new(10000)).unwrap();
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
        frames.frames[1].plane().unwrap().origin_lps_mm(),
        [0.0, 0.0, 5.0]
    );
}
#[test]
fn acquisition_retains_nonimage_and_uninterpretable_sources() {
    let root = tempfile::TempDir::new().unwrap();
    let original = include_bytes!("../dicom_dataset/fixtures/non-image.dcm");
    let retained = retain(original, root.path());
    let view = retained
        .inspect(ParseLimits::default(), FrameLimits::default())
        .unwrap();
    assert!(view.frames().unwrap().frames.is_empty());
    let mut unsupported = original.to_vec();
    let uid = b"1.2.840.10008.1.2.1";
    let at = unsupported
        .windows(uid.len())
        .position(|x| x == uid)
        .unwrap();
    unsupported[at + uid.len() - 1] = b'9';
    let retained = retain(&unsupported, root.path());
    assert!(matches!(
        retained.inspect(ParseLimits::default(), FrameLimits::default()),
        Err(DatasetError::UnsupportedSyntax)
    ));
    assert_eq!(std::fs::read(&retained.source().path).unwrap(), unsupported);
    assert_eq!(retained.original_bytes(), unsupported);
}
#[test]
fn interpretation_limits_and_frame_errors_preserve_custody() {
    let root = tempfile::TempDir::new().unwrap();
    let original = include_bytes!("../dicom_dataset/fixtures/enhanced-count-mismatch.dcm");
    let retained = retain(original, root.path());
    let view = retained
        .inspect(ParseLimits::default(), FrameLimits::default())
        .unwrap();
    assert!(matches!(view.frames(), Err(FrameError::MissingFrames)));
    assert!(matches!(
        retained.inspect(
            ParseLimits {
                max_bytes: original.len() - 1,
                ..Default::default()
            },
            FrameLimits::default()
        ),
        Err(DatasetError::ByteLimit)
    ));
    assert_eq!(std::fs::read(&retained.source().path).unwrap(), original);
    let absent = root.path().join("not-published");
    let path = root.path().join("input.dcm");
    assert!(matches!(
        RetainedDicom::retain(
            &absent,
            &path,
            DicomSourceBudget {
                max_source_bytes: original.len() - 1
            }
        ),
        Err(DicomSourceError::BudgetExceeded)
    ));
    assert!(!absent.exists());
}
