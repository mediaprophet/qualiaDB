use super::*;
use crate::dicom::{self, DicomMetadata};
use GeometryAttribute::*;
fn input() -> FrameGeometryInput<'static> {
    FrameGeometryInput {
        image_position_patient: Some([10.0, 20.0, 30.0]),
        image_orientation_patient: Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        pixel_spacing: Some([2.0, 3.0]),
        frame_of_reference_uid: Some("1.2.3"),
        ..Default::default()
    }
}
fn close(a: [f64; 3], b: [f64; 3]) {
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 1e-10, "axis {i}: {a:?} != {b:?}");
    }
}
fn element(group: u16, tag: u16, vr: &[u8; 2], text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if value.len() % 2 != 0 {
        value.push(if vr == b"UI" { 0 } else { b' ' });
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&group.to_le_bytes());
    bytes.extend_from_slice(&tag.to_le_bytes());
    bytes.extend_from_slice(vr);
    bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&value);
    bytes
}
fn fixture(elements: Vec<Vec<u8>>) -> Vec<u8> {
    let mut bytes = dicom::test_fixture_split_bytes();
    let at = dicom::split_dicom_payload(&bytes).unwrap().pixels.offset - 12;
    bytes.splice(at..at, elements.into_iter().flatten());
    bytes
}
fn geometry_elements() -> Vec<Vec<u8>> {
    vec![
        element(0x0020, 0x0032, b"DS", "10\\20\\30"),
        element(0x0020, 0x0037, b"DS", "1\\0\\0\\0\\1\\0"),
        element(0x0028, 0x0030, b"DS", "2\\3"),
        element(0x0020, 0x0052, b"UI", "1.2.3"),
        element(0x0010, 0x2210, b"CS", "BIPED"),
    ]
}

#[test]
fn axial_pixel_centers_use_column_first_vector_and_spacing_second_value() {
    let plane = PatientImagePlane::from_frame_geometry(input()).unwrap();
    assert_eq!(plane.origin_lps_mm(), [10.0, 20.0, 30.0]);
    assert_eq!(plane.pixel_spacing_mm(), [2.0, 3.0]);
    assert_eq!(plane.pixel_center_lps_mm(0, 0).unwrap(), [10.0, 20.0, 30.0]);
    assert_eq!(plane.pixel_center_lps_mm(4, 5).unwrap(), [22.0, 30.0, 30.0]);
    assert_eq!(plane.first_direction_lps(), [1.0, 0.0, 0.0]);
    assert_eq!(plane.second_direction_lps(), [0.0, 1.0, 0.0]);
    assert_eq!(plane.normal_lps(), [0.0, 0.0, 1.0]);
    assert_eq!(plane.frame_of_reference_uid(), Some("1.2.3"));
    let mut rotated = input();
    rotated.image_position_patient = Some([0.0; 3]);
    rotated.image_orientation_patient = Some([0.0, 1.0, 0.0, -1.0, 0.0, 0.0]);
    assert_eq!(
        PatientImagePlane::from_frame_geometry(rotated)
            .unwrap()
            .pixel_center_lps_mm(2, 4)
            .unwrap(),
        [-8.0, 6.0, 0.0]
    );
}

#[test]
fn oblique_plane_and_explicit_frame_inputs_preserve_distinct_origins() {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let mut oblique = input();
    oblique.image_orientation_patient = Some([s, s, 0.0, 0.0, 0.0, 1.0]);
    oblique.pixel_spacing = Some([2.0, 4.0]);
    let plane = PatientImagePlane::from_frame_geometry(oblique).unwrap();
    close(
        plane.pixel_center_lps_mm(2, 3).unwrap(),
        [10.0 + 8.0 * s, 20.0 + 8.0 * s, 36.0],
    );
    close(plane.normal_lps(), [s, -s, 0.0]);
    oblique.image_position_patient = Some([10.0, 20.0, 41.0]);
    let next = PatientImagePlane::from_frame_geometry(oblique).unwrap();
    assert_eq!(next.pixel_center_lps_mm(0, 0).unwrap(), [10.0, 20.0, 41.0]);
    assert_eq!(plane.pixel_center_lps_mm(0, 0).unwrap(), [10.0, 20.0, 30.0]);
}

#[test]
fn missing_values_remain_distinct_from_invalid_geometry() {
    let mut raw = FrameGeometryInput::default();
    assert!(matches!(
        PatientImagePlane::from_frame_geometry(raw),
        Err(GeometryError::Missing(ImagePositionPatient))
    ));
    raw.image_position_patient = Some([0.0; 3]);
    assert!(matches!(
        PatientImagePlane::from_frame_geometry(raw),
        Err(GeometryError::Missing(ImageOrientationPatient))
    ));
    raw.image_orientation_patient = input().image_orientation_patient;
    assert!(matches!(
        PatientImagePlane::from_frame_geometry(raw),
        Err(GeometryError::Missing(PixelSpacing))
    ));
    for position in [[f64::NAN, 0.0, 0.0], [0.0, f64::INFINITY, 0.0]] {
        let mut raw = input();
        raw.image_position_patient = Some(position);
        assert!(matches!(
            PatientImagePlane::from_frame_geometry(raw),
            Err(GeometryError::Invalid(ImagePositionPatient))
        ));
    }
    for spacing in [
        [0.0, 1.0],
        [-1.0, 1.0],
        [1.0, f64::NAN],
        [f64::INFINITY, 1.0],
    ] {
        let mut raw = input();
        raw.pixel_spacing = Some(spacing);
        assert!(matches!(
            PatientImagePlane::from_frame_geometry(raw),
            Err(GeometryError::Invalid(PixelSpacing))
        ));
    }
}

#[test]
fn nonunit_nonorthogonal_nonfinite_directions_and_mapping_overflow_fail() {
    for orientation in [
        [0.0; 6],
        [2.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [f64::NAN, 0.0, 0.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 0.0, f64::INFINITY, 0.0],
    ] {
        let mut raw = input();
        raw.image_orientation_patient = Some(orientation);
        assert!(matches!(
            PatientImagePlane::from_frame_geometry(raw),
            Err(GeometryError::Invalid(ImageOrientationPatient))
        ));
    }
    let mut raw = input();
    raw.pixel_spacing = Some([f64::MAX, 1.0]);
    let plane = PatientImagePlane::from_frame_geometry(raw).unwrap();
    assert!(matches!(
        plane.pixel_center_lps_mm(0, 2),
        Err(GeometryError::NonFiniteMappedPosition)
    ));
}

#[test]
fn biped_default_quadruped_rejection_and_uid_absence_are_explicit() {
    let mut raw = input();
    raw.frame_of_reference_uid = None;
    let plane = PatientImagePlane::from_frame_geometry(raw).unwrap();
    assert_eq!(plane.frame_of_reference_uid(), None);
    raw.anatomical_orientation_type = Some("BIPED");
    assert_eq!(
        PatientImagePlane::from_frame_geometry(raw)
            .unwrap()
            .pixel_center_lps_mm(1, 1)
            .unwrap(),
        plane.pixel_center_lps_mm(1, 1).unwrap()
    );
    raw.anatomical_orientation_type = Some("QUADRUPED");
    assert!(matches!(
        PatientImagePlane::from_frame_geometry(raw),
        Err(GeometryError::UnsupportedQuadruped)
    ));
    raw.anatomical_orientation_type = Some("OTHER");
    assert!(matches!(
        PatientImagePlane::from_frame_geometry(raw),
        Err(GeometryError::Invalid(AnatomicalOrientationType))
    ));
    raw.anatomical_orientation_type = None;
    for uid in ["", "1..2", "1.02.3", "patient"] {
        raw.frame_of_reference_uid = Some(uid);
        assert!(matches!(
            PatientImagePlane::from_frame_geometry(raw),
            Err(GeometryError::Invalid(FrameOfReferenceUid))
        ));
    }
}

#[test]
fn real_part10_parser_populates_geometry_without_changing_overlay_or_pixels() {
    let source = fixture(geometry_elements());
    let meta = dicom::parse_dicom_metadata_bytes(&source).unwrap();
    let split = dicom::split_dicom_payload(&source).unwrap();
    assert_eq!(meta.image_position_patient, Some([10.0, 20.0, 30.0]));
    assert_eq!(
        meta.image_orientation_patient,
        input().image_orientation_patient
    );
    assert_eq!(meta.pixel_spacing, Some([2.0, 3.0]));
    assert_eq!(meta.frame_of_reference_uid.as_deref(), Some("1.2.3"));
    assert_eq!(meta.anatomical_orientation_type.as_deref(), Some("BIPED"));
    assert!(meta.geometry_parse_errors.is_empty());
    assert_eq!(meta, split.meta);
    assert_eq!(
        PatientImagePlane::from_metadata(&meta)
            .unwrap()
            .pixel_center_lps_mm(4, 5)
            .unwrap(),
        [22.0, 30.0, 30.0]
    );
    assert_eq!(
        &source[split.pixels.offset..split.pixels.offset + split.pixels.length],
        &[10, 20, 30, 40]
    );
    let old = dicom::parse_dicom_metadata_bytes(&dicom::test_fixture_split_bytes()).unwrap();
    assert_eq!(
        dicom::build_overlay_spec(&meta, None, "synthetic"),
        dicom::build_overlay_spec(&old, None, "synthetic")
    );
}

#[test]
fn parser_rejects_bad_multiplicity_lexical_values_and_sticky_duplicate_repairs() {
    for bad in [
        "1\\2",
        "1\\2\\3\\4",
        "NaN\\0\\0",
        "1e999\\0\\0",
        "1\\x\\3",
        "1\0\\2\\3",
        "12345678901234567\\0\\0",
    ] {
        let bytes = fixture(vec![
            element(0x0020, 0x0032, b"DS", bad),
            element(0x0020, 0x0032, b"DS", "1\\2\\3"),
        ]);
        let meta = dicom::parse_dicom_metadata_bytes(&bytes).unwrap();
        assert_eq!(meta.image_position_patient, None, "{bad:?}");
        assert_eq!(meta.geometry_parse_errors, vec![ImagePositionPatient]);
        assert!(matches!(
            PatientImagePlane::from_metadata(&meta),
            Err(GeometryError::Invalid(ImagePositionPatient))
        ));
    }
    let mut tags = geometry_elements();
    tags.push(element(0x0028, 0x0030, b"DS", "2"));
    let meta = dicom::parse_dicom_metadata_bytes(&fixture(tags)).unwrap();
    assert!(matches!(
        PatientImagePlane::from_metadata(&meta),
        Err(GeometryError::Invalid(PixelSpacing))
    ));
}

#[test]
fn serde_defaults_keep_old_metadata_missing_and_preserve_invalid_diagnostics() {
    let old = DicomMetadata::default();
    let json = serde_json::to_string(&old).unwrap();
    assert!(!json.contains("image_position_patient"));
    let decoded: DicomMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.image_position_patient, None);
    assert_eq!(decoded.frame_of_reference_uid, None);
    assert!(decoded.geometry_parse_errors.is_empty());
    let bytes = fixture(vec![element(0x0020, 0x0032, b"DS", "bad")]);
    let meta = dicom::parse_dicom_metadata_bytes(&bytes).unwrap();
    let roundtrip: DicomMetadata =
        serde_json::from_str(&serde_json::to_string(&meta).unwrap()).unwrap();
    assert_eq!(roundtrip.geometry_parse_errors, vec![ImagePositionPatient]);
    assert!(matches!(
        PatientImagePlane::from_metadata(&roundtrip),
        Err(GeometryError::Invalid(ImagePositionPatient))
    ));
}

#[test]
fn slice_thickness_location_and_unparsed_frame_metadata_are_not_fallbacks() {
    let meta = dicom::parse_dicom_metadata_bytes(&fixture(vec![
        element(0x0018, 0x0050, b"DS", "8"),
        element(0x0020, 0x1041, b"DS", "22"),
    ]))
    .unwrap();
    assert!(matches!(
        PatientImagePlane::from_metadata(&meta),
        Err(GeometryError::Missing(ImagePositionPatient))
    ));
    assert_eq!(meta.pixel_spacing, None);
    let mut meta = DicomMetadata {
        image_position_patient: Some([0.0; 3]),
        image_orientation_patient: input().image_orientation_patient,
        ..Default::default()
    };
    assert!(matches!(
        PatientImagePlane::from_metadata(&meta),
        Err(GeometryError::Missing(PixelSpacing))
    ));
    meta.pixel_spacing = Some([2.0, 3.0]);
    assert_eq!(
        PatientImagePlane::from_metadata(&meta)
            .unwrap()
            .frame_of_reference_uid(),
        None
    );
}

#[test]
fn wrong_explicit_vr_and_non_utf8_geometry_cannot_validate() {
    for (group, tag, text, field) in [
        (0x0020, 0x0032, "1\\2\\3", ImagePositionPatient),
        (0x0020, 0x0052, "1.2.3", FrameOfReferenceUid),
        (0x0010, 0x2210, "BIPED", AnatomicalOrientationType),
    ] {
        let meta =
            dicom::parse_dicom_metadata_bytes(&fixture(vec![element(group, tag, b"LO", text)]))
                .unwrap();
        assert_eq!(meta.geometry_parse_errors, vec![field]);
        assert!(matches!(
            PatientImagePlane::from_metadata(&meta),
            Err(GeometryError::Invalid(_))
        ));
    }
    let mut bad = element(0x0020, 0x0032, b"DS", "1\\2\\3");
    bad[8] = 0xff;
    let meta = dicom::parse_dicom_metadata_bytes(&fixture(vec![bad])).unwrap();
    assert_eq!(meta.geometry_parse_errors, vec![ImagePositionPatient]);
    assert_eq!(meta.image_position_patient, None);
}

#[test]
fn long_vr_private_elements_preserve_geometry_and_pixel_alignment() {
    for (vr, value) in [
        (*b"OD", vec![0; 8]),
        (*b"OL", 42u32.to_le_bytes().to_vec()),
        (*b"OV", 42u64.to_le_bytes().to_vec()),
        (*b"UC", b"metadata".to_vec()),
        (*b"UR", b"urn:x:00".to_vec()),
    ] {
        let mut long = vec![0x11, 0, 0x10, 0x10];
        long.extend_from_slice(&vr);
        long.extend_from_slice(&[0, 0]);
        long.extend_from_slice(&(value.len() as u32).to_le_bytes());
        long.extend_from_slice(&value);
        let mut tags = vec![long];
        tags.extend(geometry_elements());
        let bytes = fixture(tags);
        let parsed = dicom::split_dicom_payload(&bytes).unwrap();
        assert_eq!(
            parsed.meta.image_position_patient,
            Some([10.0, 20.0, 30.0]),
            "VR {vr:?}"
        );
        assert_eq!(
            PatientImagePlane::from_metadata(&parsed.meta)
                .unwrap()
                .pixel_center_lps_mm(2, 1)
                .unwrap(),
            [16.0, 22.0, 30.0]
        );
        assert_eq!(
            &bytes[parsed.pixels.offset..parsed.pixels.offset + parsed.pixels.length],
            &[10, 20, 30, 40]
        );
    }
}

#[test]
fn equal_or_conflicting_duplicate_geometry_tags_are_ambiguous() {
    for (group, tag, vr, value, field) in [
        (0x0020, 0x0032, *b"DS", "10\\20\\30", ImagePositionPatient),
        (0x0020, 0x0032, *b"DS", "10\\20\\31", ImagePositionPatient),
        (
            0x0020,
            0x0037,
            *b"DS",
            "0\\1\\0\\1\\0\\0",
            ImageOrientationPatient,
        ),
        (0x0020, 0x0052, *b"UI", "1.2.4", FrameOfReferenceUid),
        (0x0028, 0x0030, *b"DS", "2\\3", PixelSpacing),
        (0x0010, 0x2210, *b"CS", "BIPED", AnatomicalOrientationType),
    ] {
        let mut tags = geometry_elements();
        tags.push(element(group, tag, &vr, value));
        let meta = dicom::parse_dicom_metadata_bytes(&fixture(tags)).unwrap();
        assert_eq!(meta.geometry_parse_errors, vec![field]);
        match field {
            ImagePositionPatient => assert!(meta.image_position_patient.is_none()),
            ImageOrientationPatient => assert!(meta.image_orientation_patient.is_none()),
            PixelSpacing => assert!(meta.pixel_spacing.is_none()),
            FrameOfReferenceUid => assert!(meta.frame_of_reference_uid.is_none()),
            AnatomicalOrientationType => assert!(meta.anatomical_orientation_type.is_none()),
        }
        assert!(
            matches!(PatientImagePlane::from_metadata(&meta),Err(GeometryError::Invalid(actual)) if actual==field)
        );
    }
}
