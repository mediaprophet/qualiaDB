//! Honest partial DICOM helpers — **not** a full pydicom replacement.
//!
//! - Little-endian **explicit VR** tag scan for a small common set
//! - PHI tag redact on a simple attribute map
//! - SUV body-weight formula (pure arithmetic)
//!
//! Unsupported transfer syntaxes / implicit VR / compressed pixel data
//! fail closed.

pub mod anonymize_tag_map;
pub mod parse_dicom_tags_basic;
pub mod suv_from_activity;

pub use anonymize_tag_map::{anonymize_tag_map, AnonymizeReport, PHI_TAG_KEYS};
pub use parse_dicom_tags_basic::{
    parse_dicom_tags_basic, DicomLiteError, DicomTagMap, ParsedDicomTags,
};
pub use suv_from_activity::{suv_bw, suv_from_activity};
