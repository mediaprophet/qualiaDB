//! DICOM Part-10 metadata parsing, stream traversal, discovery, and overlay specs.
//!
//! File ingest path — heap allocation is permitted here (not a hot-path evaluator).

mod discovery;
mod overlay;
mod parser;
mod stream;
mod types;

#[cfg(test)]
pub(crate) mod tests;

pub use discovery::{
    collect_dicom_image_paths_under, collect_dicom_paths_under, dicom_has_pixel_data,
    is_dicom_part10, resolve_local_dicom_dir, DEFAULT_LOCAL_DICOM_DIR,
};
pub use overlay::{
    build_overlay_spec, chat_mentions_imaging, default_organ_matchers,
    default_placement_for_organ, infer_organ_from_haystack, infer_organ_from_metadata,
    infer_overlay_spec_from_text, metadata_json_from_file, normalize_dicom_token,
    overlay_spec_from_file, overlay_spec_json_from_file,
};
pub use parser::{parse_dicom_file, parse_dicom_metadata_bytes, split_dicom_payload};
pub use types::{
    decode_blob_pointer, encode_blob_pointer, pack_volume_metadata, unpack_volume_metadata,
    DicomError, DicomMetadata, DicomOrganMapFile, DicomOverlaySpec, DicomPixelSlice,
    DicomPlacement, DicomSplitPayload, DicomTagMatcher, INLINE_TAG_BLOB_POINTER,
    TS_EXPLICIT_VR_LITTLE_ENDIAN, TS_IMPLICIT_VR_LITTLE_ENDIAN, TS_JPEG2000,
    TS_JPEG2000_LOSSLESS, TS_JPEG_BASELINE, TS_RLE_LOSSLESS,
};

#[cfg(test)]
pub(crate) use tests::test_fixtures;
#[cfg(test)]
pub use tests::test_fixture_split_bytes;
