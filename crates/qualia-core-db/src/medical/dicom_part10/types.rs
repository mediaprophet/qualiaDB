//! Shared DICOM Part-10 transfer-syntax UIDs, metadata structs, errors, and inline blob pointers.

use serde::{Deserialize, Serialize};

pub const TS_IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";
pub const TS_EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";
pub const TS_JPEG2000_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
pub const TS_JPEG2000: &str = "1.2.840.10008.1.2.4.91";
pub const TS_JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
pub const TS_RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

/// Pack a (group, element) pair into the u32 DICOM tag key.
pub(super) const fn tag(group: u16, element: u16) -> u32 {
    ((group as u32) << 16) | element as u32
}

pub(super) const TAG_MODALITY: u32 = tag(0x0008, 0x0060);
pub(super) const TAG_STUDY_DESCRIPTION: u32 = tag(0x0008, 0x1030);
pub(super) const TAG_SERIES_DESCRIPTION: u32 = tag(0x0008, 0x103E);
pub(super) const TAG_BODY_PART_EXAMINED: u32 = tag(0x0018, 0x0015);
pub(super) const TAG_SERIES_INSTANCE_UID: u32 = tag(0x0020, 0x000E);
pub(super) const TAG_INSTANCE_NUMBER: u32 = tag(0x0020, 0x0013);
pub(super) const TAG_ROWS: u32 = tag(0x0028, 0x0010);
pub(super) const TAG_COLUMNS: u32 = tag(0x0028, 0x0011);
pub(super) const TAG_WINDOW_CENTER: u32 = tag(0x0028, 0x1050);
pub(super) const TAG_WINDOW_WIDTH: u32 = tag(0x0028, 0x1051);
pub(super) const TAG_PHOTOMETRIC_INTERPRETATION: u32 = tag(0x0028, 0x0004);
pub(super) const TAG_BITS_ALLOCATED: u32 = tag(0x0028, 0x0100);
pub(super) const TAG_PIXEL_REPRESENTATION: u32 = tag(0x0028, 0x0103);
pub(super) const TAG_PATIENT_ID: u32 = tag(0x0010, 0x0020);
pub(super) const TAG_STUDY_DATE: u32 = tag(0x0008, 0x0020);
pub(super) const TAG_PIXEL_DATA: u32 = tag(0x7FE0, 0x0010);
pub(super) const TAG_TRANSFER_SYNTAX_UID: u32 = tag(0x0002, 0x0010);

/// Inline datatype tag for mmap blob byte-offset pointers (resolver bits 60–62 = 0b100).
pub const INLINE_TAG_BLOB_POINTER: u64 = 0b100u64 << 60;
const INLINE_TAG_MASK: u64 = 0b111u64 << 60;
const INLINE_VALUE_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

pub(super) const ITEM_DELIMITER: u32 = tag(0xFFFE, 0xE000);
pub(super) const ITEM_END_DELIMITER: u32 = tag(0xFFFE, 0xE00D);
pub(super) const SEQ_DELIMITER: u32 = tag(0xFFFE, 0xE0DD);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DicomError {
    TooShort,
    MissingDicmMagic,
    UnexpectedEof,
    InvalidVr,
    UnsupportedTransferSyntax(String),
    Io(String),
}

impl std::fmt::Display for DicomError {
    /// Human-readable `DicomError` rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "DICOM file too short"),
            Self::MissingDicmMagic => write!(f, "DICOM magic (DICM) not found"),
            Self::UnexpectedEof => write!(f, "unexpected end of DICOM stream"),
            Self::InvalidVr => write!(f, "invalid DICOM VR"),
            Self::UnsupportedTransferSyntax(ts) => {
                write!(f, "unsupported DICOM transfer syntax: {ts}")
            }
            Self::Io(msg) => write!(f, "DICOM IO error: {msg}"),
        }
    }
}

impl std::error::Error for DicomError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DicomMetadata {
    pub modality: String,
    pub body_part_examined: String,
    pub study_description: String,
    pub series_description: String,
    pub protocol_name: String,
    pub series_instance_uid: String,
    pub instance_number: i32,
    pub rows: u16,
    pub columns: u16,
    pub window_center: Option<f64>,
    pub window_width: Option<f64>,
    pub photometric_interpretation: String,
    pub transfer_syntax_uid: String,
    pub patient_id: String,
    pub study_date: String,
    pub bits_allocated: u16,
    pub pixel_representation: u16,
    /// Top-level single-frame geometry only; no enhanced functional-group extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_position_patient: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_orientation_patient: Option<[f64; 6]>,
    /// [row spacing, column spacing], in millimeters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixel_spacing: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_of_reference_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anatomical_orientation_type: Option<String>,
    /// Malformed tag occurrences stay distinct from missing optional metadata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometry_parse_errors: Vec<crate::medical::dicom_geometry::GeometryAttribute>,
}

/// Pixel payload location inside a Part 10 file (not copied — Core 3 streams to blob store).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DicomPixelSlice {
    pub offset: usize,
    pub length: usize,
}

/// Result of split-ingestion parse: semantic metadata + raw pixel slice bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct DicomSplitPayload {
    pub meta: DicomMetadata,
    pub pixels: DicomPixelSlice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DicomPlacement {
    #[serde(rename = "offsetX")]
    pub offset_x: f32,
    #[serde(rename = "offsetY")]
    pub offset_y: f32,
    #[serde(rename = "offsetZ")]
    pub offset_z: f32,
    pub scale: f32,
    #[serde(rename = "rotationY")]
    pub rotation_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DicomOverlaySpec {
    pub version: String,
    pub organ: Option<String>,
    pub opacity: f32,
    pub visible: bool,
    pub placement: DicomPlacement,
    #[serde(rename = "seriesInstanceUID", skip_serializing_if = "Option::is_none")]
    pub series_instance_uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,
    #[serde(rename = "bodyPartExamined", skip_serializing_if = "Option::is_none")]
    pub body_part_examined: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DicomOrganMapFile {
    #[serde(default)]
    pub tag_matchers: Vec<DicomTagMatcher>,
}

#[derive(Debug, Deserialize)]
pub struct DicomTagMatcher {
    #[serde(default)]
    pub tokens: Vec<String>,
    #[serde(default)]
    pub organ: Option<String>,
}

/// Encode a Core-3 blob-store byte offset into an inline Object field pointer.
#[inline]
pub fn encode_blob_pointer(byte_offset: u64) -> u64 {
    (byte_offset & INLINE_VALUE_MASK) | INLINE_TAG_BLOB_POINTER
}

/// Decode a blob pointer from an Object field; returns `None` if tag mismatch.
#[inline]
pub fn decode_blob_pointer(field: u64) -> Option<u64> {
    if (field & INLINE_TAG_MASK) == INLINE_TAG_BLOB_POINTER {
        Some(field & INLINE_VALUE_MASK)
    } else {
        None
    }
}

/// Pack rows/cols/pixel-byte-length into the metadata lane for volume queries.
#[inline]
pub fn pack_volume_metadata(rows: u16, cols: u16, byte_length: u32) -> u64 {
    ((rows as u64) << 48) | ((cols as u64) << 32) | (byte_length as u64)
}

/// Inverse of `pack_volume_metadata` — (rows, cols, pixel byte length).
pub fn unpack_volume_metadata(metadata: u64) -> (u16, u16, u32) {
    let rows = ((metadata >> 48) & 0xFFFF) as u16;
    let cols = ((metadata >> 32) & 0xFFFF) as u16;
    let byte_length = (metadata & 0xFFFF_FFFF) as u32;
    (rows, cols, byte_length)
}
