//! DICOM Part 10 metadata parsing and anatomy overlay spec generation.
//!
//! File ingest path — heap allocation is permitted here (not a hot-path evaluator).

pub use super::dicom_part10::{
    build_overlay_spec, chat_mentions_imaging, collect_dicom_image_paths_under,
    collect_dicom_paths_under, decode_blob_pointer, default_organ_matchers,
    default_placement_for_organ, dicom_has_pixel_data, encode_blob_pointer,
    infer_organ_from_haystack, infer_organ_from_metadata, infer_overlay_spec_from_text,
    is_dicom_part10, metadata_json_from_file, normalize_dicom_token, overlay_spec_from_file,
    overlay_spec_json_from_file, pack_volume_metadata, parse_dicom_file,
    parse_dicom_metadata_bytes, resolve_local_dicom_dir, split_dicom_payload,
    unpack_volume_metadata, DicomError, DicomMetadata, DicomOrganMapFile, DicomOverlaySpec,
    DicomPixelSlice, DicomPlacement, DicomSplitPayload, DicomTagMatcher, DEFAULT_LOCAL_DICOM_DIR,
    INLINE_TAG_BLOB_POINTER, TS_EXPLICIT_VR_LITTLE_ENDIAN, TS_IMPLICIT_VR_LITTLE_ENDIAN,
    TS_JPEG2000, TS_JPEG2000_LOSSLESS, TS_JPEG_BASELINE, TS_RLE_LOSSLESS,
};

#[cfg(test)]
pub use super::dicom_part10::test_fixture_split_bytes;
