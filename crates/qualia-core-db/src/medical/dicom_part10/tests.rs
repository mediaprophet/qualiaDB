//! Test fixtures and unit tests for DICOM Part-10 metadata parsing and overlay inference.

use super::*;
use super::types::{
    TAG_BODY_PART_EXAMINED, TAG_COLUMNS, TAG_MODALITY, TAG_ROWS, TAG_SERIES_DESCRIPTION,
    TAG_SERIES_INSTANCE_UID, TAG_TRANSFER_SYNTAX_UID,
};

pub(crate) mod test_fixtures {
    use super::*;

    /// Minimal Part-10 bytes for unit tests (explicit VR, single 2x2 pixel frame).
    pub fn test_fixture_split_bytes() -> Vec<u8> {
        let mut bytes = build_explicit_meta_file(TS_EXPLICIT_VR_LITTLE_ENDIAN);
        push_explicit_lo(TAG_MODALITY, "CT", &mut bytes);
        push_explicit_lo(TAG_BODY_PART_EXAMINED, "CHEST", &mut bytes);
        push_explicit_lo(TAG_SERIES_DESCRIPTION, "CORONARY CTA", &mut bytes);
        push_explicit_us(TAG_ROWS, 2, &mut bytes);
        push_explicit_us(TAG_COLUMNS, 2, &mut bytes);
        push_explicit_string(TAG_SERIES_INSTANCE_UID, "1.2.3", &mut bytes);
        let pixels = [10u8, 20, 30, 40];
        bytes.extend_from_slice(&[0xE0, 0x7F, 0x10, 0x00, b'O', b'B', 0x00, 0x00]);
        bytes.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&pixels);
        bytes
    }

    fn build_explicit_meta_file(transfer_syntax: &str) -> Vec<u8> {
        let mut out = vec![0u8; 128];
        out.extend_from_slice(b"DICM");
        push_explicit_string(TAG_TRANSFER_SYNTAX_UID, transfer_syntax, &mut out);
        out
    }

    fn push_explicit_string(tag: u32, value: &str, out: &mut Vec<u8>) {
        let group = (tag >> 16) as u16;
        let element = tag as u16;
        let bytes = value.as_bytes();
        out.extend_from_slice(&group.to_le_bytes());
        out.extend_from_slice(&element.to_le_bytes());
        out.extend_from_slice(b"UI");
        out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(bytes);
    }

    fn push_explicit_lo(tag: u32, value: &str, out: &mut Vec<u8>) {
        let group = (tag >> 16) as u16;
        let element = tag as u16;
        let bytes = value.as_bytes();
        out.extend_from_slice(&group.to_le_bytes());
        out.extend_from_slice(&element.to_le_bytes());
        out.extend_from_slice(b"LO");
        out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(bytes);
    }

    fn push_explicit_us(tag: u32, value: u16, out: &mut Vec<u8>) {
        let group = (tag >> 16) as u16;
        let element = tag as u16;
        out.extend_from_slice(&group.to_le_bytes());
        out.extend_from_slice(&element.to_le_bytes());
        out.extend_from_slice(b"US");
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
}

pub use test_fixtures::test_fixture_split_bytes;

#[test]
fn infer_organ_from_cardiac_ct_description() {
    let meta = DicomMetadata {
        modality: "CT".into(),
        body_part_examined: "CHEST".into(),
        series_description: "Coronary CTA".into(),
        ..Default::default()
    };
    let organ = infer_organ_from_metadata(&meta, &default_organ_matchers());
    assert_eq!(organ.as_deref(), Some("Heart"));
}

#[test]
fn chat_inference_requires_imaging_marker() {
    let spec = infer_overlay_spec_from_text("patient has diabetes", &default_organ_matchers());
    assert!(spec.is_none());
}

#[test]
fn chat_inference_maps_mri_brain() {
    let text = "Review the brain MRI slices for demyelination.";
    let spec = infer_overlay_spec_from_text(text, &default_organ_matchers()).unwrap();
    assert_eq!(spec.organ.as_deref(), Some("Brain (Allen)"));
    assert_eq!(spec.source.as_deref(), Some("chat-inferred"));
}

#[test]
fn parse_explicit_vr_dataset_tags() {
    let meta = parse_dicom_metadata_bytes(&test_fixture_split_bytes()).expect("parse");
    assert_eq!(meta.modality, "CT");
    assert_eq!(meta.body_part_examined, "CHEST");
    assert_eq!(meta.rows, 2);
    assert_eq!(meta.columns, 2);
    assert_eq!(
        infer_organ_from_metadata(&meta, &default_organ_matchers()).as_deref(),
        Some("Heart")
    );
}

#[test]
fn placement_defaults_exist_for_common_organs() {
    let heart = default_placement_for_organ("Heart");
    assert!(heart.scale > 0.0);
    let unknown = default_placement_for_organ("Unknown Organ");
    assert!(unknown.scale > 0.0);
}

/// Auto-skips when private fixtures are absent (CI); runs when `app-development/DICOM-*` exists.
#[test]
fn local_private_dicom_metadata_and_split() {
    let Some(root) = resolve_local_dicom_dir() else {
        eprintln!("skip: set QUALIA_DICOM_FIXTURE_DIR or add {DEFAULT_LOCAL_DICOM_DIR}");
        return;
    };
    let all = collect_dicom_paths_under(&root, 8);
    assert!(
        !all.is_empty(),
        "no Part-10 DICOM files under {}",
        root.display()
    );
    for path in &all {
        let meta = parse_dicom_file(path).expect("metadata parse");
        assert!(
            !meta.modality.is_empty(),
            "modality missing in {}",
            path.display()
        );
    }

    let images = collect_dicom_image_paths_under(&root, 4);
    assert!(
        !images.is_empty(),
        "no image slices with Pixel Data under {}",
        root.display()
    );
    for path in &images {
        let bytes = std::fs::read(path).unwrap();
        let split = split_dicom_payload(&bytes).expect("split ingest");
        assert!(
            split.pixels.length > 0,
            "pixel payload empty in {}",
            path.display()
        );
        assert!(split.meta.rows > 0 && split.meta.columns > 0);
    }
}
