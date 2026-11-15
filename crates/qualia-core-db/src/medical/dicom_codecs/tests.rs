use super::*;
use crate::medical::dicom_dataset::{ParseLimits, Part10Dataset, WorkBudget};
#[test]
fn signed_12_bit_samples_ignore_unused_bits_without_changing_originals() {
    let bytes = include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm");
    let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
    let p = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.sample(0, 0, 0).unwrap(), StoredSample::Signed(-2048));
    assert_eq!(p.sample(0, 0, 1).unwrap(), StoredSample::Signed(-1));
    assert_eq!(p.sample(1, 0, 0).unwrap(), StoredSample::Signed(0));
    assert_eq!(p.sample(1, 0, 1).unwrap(), StoredSample::Signed(2047));
    assert_eq!(
        p.stored_sample_bytes(),
        [0, 0xf8, 0xff, 0xaf, 0, 0x10, 0xff, 0x77]
    );
    assert_eq!(ds.original_bytes(), bytes);
    assert!(p.sample(2, 0, 0).is_err());
}
#[test]
fn unsigned_8_16_and_signed_32_native_values_are_exact_without_display_inversion() {
    for bytes in [
        include_bytes!("../dicom_dataset/fixtures/native-u16.dcm").as_slice(),
        include_bytes!("../dicom_dataset/fixtures/native-implicit-u16.dcm").as_slice(),
    ] {
        let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
        let p = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
        assert_eq!(p.sample(0, 0, 0).unwrap(), StoredSample::Unsigned(0));
        assert_eq!(p.sample(0, 0, 1).unwrap(), StoredSample::Unsigned(32768));
        assert_eq!(p.sample(0, 0, 2).unwrap(), StoredSample::Unsigned(65535));
    }
    let ds = Part10Dataset::parse(
        include_bytes!("../dicom_dataset/fixtures/native-u8-rwvm.dcm"),
        ParseLimits::default(),
    )
    .unwrap();
    let p = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.photometric(), "MONOCHROME1");
    assert_eq!(p.sample(0, 0, 1).unwrap(), StoredSample::Unsigned(128));
    assert_eq!(p.stored_sample_bytes(), [0, 128, 255]);
    let ds = Part10Dataset::parse(
        include_bytes!("../dicom_dataset/fixtures/native-i32.dcm"),
        ParseLimits::default(),
    )
    .unwrap();
    let p = NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.sample(0, 0, 0).unwrap(), StoredSample::Signed(i32::MIN));
    assert_eq!(p.sample(0, 0, 2).unwrap(), StoredSample::Signed(i32::MAX));
}
#[test]
fn pixel_budget_truncation_and_unsupported_encapsulation_do_not_reinterpret_bytes() {
    let bytes = include_bytes!("../dicom_dataset/fixtures/native-u16.dcm");
    let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
    assert!(matches!(
        NativePixelView::from_dataset(&ds, 1, &mut WorkBudget::new(10000)),
        Err(PixelError::ByteLimit)
    ));
    let bytes = include_bytes!("../dicom_dataset/fixtures/encapsulated-opaque.dcm");
    let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
    assert!(matches!(
        NativePixelView::from_dataset(&ds, 1024, &mut WorkBudget::new(10000)),
        Err(PixelError::Unsupported(_))
    ));
    let profile = transfer_syntax_profile(ds.transfer_syntax_uid());
    assert!(profile.preserve_original);
    assert!(profile.nested_dataset);
    assert!(profile.encapsulated_decode);
    assert!(!profile.native_integer_samples);

    // Opaque non-JPEG fragment payload must still fail closed cleanly leaving sentinel intact.
    let mut sentinel = [0xA5u8; 64];
    assert!(decode_jpeg_into(&ds, &mut sentinel, JpegLimits::default(), &mut WorkBudget::new(10000)).is_err());
    assert!(sentinel.iter().all(|&b| b == 0xA5));

    // Verify honest HTJ2K fail-closed default vs implemented JPEG-LS / J2K / JXL / JPEG profiles.
    for uid in [
        "1.2.840.10008.1.2.4.50",
        "1.2.840.10008.1.2.4.51",
        "1.2.840.10008.1.2.4.80",
        "1.2.840.10008.1.2.4.81",
        "1.2.840.10008.1.2.4.90",
        "1.2.840.10008.1.2.4.91",
        "1.2.840.10008.1.2.4.110",
        "1.2.840.10008.1.2.4.111",
        "1.2.840.10008.1.2.4.112",
        "1.2.840.10008.1.2.5",
    ] {
        let p = transfer_syntax_profile(uid);
        assert!(p.preserve_original && p.nested_dataset && p.encapsulated_decode, "uid {uid}");
    }
    let htj2k = transfer_syntax_profile("1.2.840.10008.1.2.4.201");
    assert!(htj2k.preserve_original && htj2k.nested_dataset && !htj2k.encapsulated_decode);

    let unknown = transfer_syntax_profile("1.2.3.999");
    assert!(unknown.preserve_original);
    assert!(!unknown.nested_dataset);
    assert!(!unknown.encapsulated_decode);
}
