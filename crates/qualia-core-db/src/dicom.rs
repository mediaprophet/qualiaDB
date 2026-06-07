//! DICOM Part 10 metadata parsing and anatomy overlay spec generation.
//!
//! File ingest path — heap allocation is permitted here (not a hot-path evaluator).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const TS_IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";
pub const TS_EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";
pub const TS_JPEG2000_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
pub const TS_JPEG2000: &str = "1.2.840.10008.1.2.4.91";
pub const TS_JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
pub const TS_RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

const TAG_MODALITY: u32 = tag(0x0008, 0x0060);
const TAG_STUDY_DESCRIPTION: u32 = tag(0x0008, 0x1030);
const TAG_SERIES_DESCRIPTION: u32 = tag(0x0008, 0x103E);
const TAG_BODY_PART_EXAMINED: u32 = tag(0x0018, 0x0015);
const TAG_SERIES_INSTANCE_UID: u32 = tag(0x0020, 0x000E);
const TAG_INSTANCE_NUMBER: u32 = tag(0x0020, 0x0013);
const TAG_ROWS: u32 = tag(0x0028, 0x0010);
const TAG_COLUMNS: u32 = tag(0x0028, 0x0011);
const TAG_WINDOW_CENTER: u32 = tag(0x0028, 0x1050);
const TAG_WINDOW_WIDTH: u32 = tag(0x0028, 0x1051);
const TAG_PHOTOMETRIC_INTERPRETATION: u32 = tag(0x0028, 0x0004);
const TAG_BITS_ALLOCATED: u32 = tag(0x0028, 0x0100);
const TAG_PIXEL_REPRESENTATION: u32 = tag(0x0028, 0x0103);
const TAG_PATIENT_ID: u32 = tag(0x0010, 0x0020);
const TAG_STUDY_DATE: u32 = tag(0x0008, 0x0020);
const TAG_PIXEL_DATA: u32 = tag(0x7FE0, 0x0010);
const TAG_TRANSFER_SYNTAX_UID: u32 = tag(0x0002, 0x0010);

/// Inline datatype tag for mmap blob byte-offset pointers (resolver bits 60–62 = 0b100).
pub const INLINE_TAG_BLOB_POINTER: u64 = 0b100u64 << 60;
const INLINE_TAG_MASK: u64 = 0b111u64 << 60;
const INLINE_VALUE_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

const ITEM_DELIMITER: u32 = tag(0xFFFE, 0xE000);
const ITEM_END_DELIMITER: u32 = tag(0xFFFE, 0xE00D);
const SEQ_DELIMITER: u32 = tag(0xFFFE, 0xE0DD);

const fn tag(group: u16, element: u16) -> u32 {
    ((group as u32) << 16) | element as u32
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferSyntax {
    ImplicitVrLittleEndian,
    ExplicitVrLittleEndian,
    /// Compressed pixel data (JPEG / JPEG 2000 / RLE); dataset tags remain explicit VR.
    EncapsulatedExplicitVr,
}

fn find_dataset_offset(data: &[u8]) -> Result<usize, DicomError> {
    if data.len() < 132 {
        return Err(DicomError::TooShort);
    }
    if &data[128..132] == b"DICM" {
        return Ok(132);
    }
    for start in 0..data.len().saturating_sub(4) {
        if &data[start..start + 4] == b"DICM" {
            return Ok(start + 4);
        }
    }
    Err(DicomError::MissingDicmMagic)
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, DicomError> {
    let end = offset + 2;
    if end > data.len() {
        return Err(DicomError::UnexpectedEof);
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, DicomError> {
    let end = offset + 4;
    if end > data.len() {
        return Err(DicomError::UnexpectedEof);
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn vr_is_long(vr: &[u8; 2]) -> bool {
    matches!(
        vr,
        b"OB" | b"OW" | b"OF" | b"SQ" | b"UT" | b"UN" | b"SV" | b"UV"
    )
}

fn decode_dicom_string(bytes: &[u8]) -> String {
    let trimmed = bytes
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&trimmed).trim().to_string()
}

fn decode_numeric_string(bytes: &[u8]) -> Option<f64> {
    let text = decode_dicom_string(bytes);
    text.split('\\')
        .next()
        .and_then(|part| part.trim().parse::<f64>().ok())
}

fn decode_i32(bytes: &[u8]) -> i32 {
    decode_numeric_string(bytes).map(|v| v as i32).unwrap_or(0)
}

fn decode_u16(bytes: &[u8]) -> u16 {
    decode_numeric_string(bytes)
        .map(|v| v as u16)
        .unwrap_or_else(|| {
            if bytes.len() >= 2 {
                u16::from_le_bytes([bytes[0], bytes[1]])
            } else {
                0
            }
        })
}

struct ElementHeader {
    tag: u32,
    vr: Option<[u8; 2]>,
    length: usize,
    header_size: usize,
}

fn read_element_header(
    data: &[u8],
    offset: usize,
    syntax: TransferSyntax,
    meta_group: bool,
) -> Result<Option<ElementHeader>, DicomError> {
    if offset + 8 > data.len() {
        return Ok(None);
    }

    let group = read_u16_le(data, offset)?;
    let element = read_u16_le(data, offset + 2)?;
    let tag = tag(group, element);

    if tag == ITEM_DELIMITER || tag == ITEM_END_DELIMITER || tag == SEQ_DELIMITER {
        let length = read_u32_le(data, offset + 4)? as usize;
        return Ok(Some(ElementHeader {
            tag,
            vr: None,
            length,
            header_size: 8,
        }));
    }

    let use_explicit = meta_group
        || matches!(
            syntax,
            TransferSyntax::ExplicitVrLittleEndian | TransferSyntax::EncapsulatedExplicitVr
        );
    if use_explicit {
        let vr = [data[offset + 4], data[offset + 5]];
        if !vr[0].is_ascii_uppercase() || !vr[1].is_ascii_uppercase() {
            return Err(DicomError::InvalidVr);
        }
        let (length, header_size) = if vr_is_long(&vr) {
            let length = read_u32_le(data, offset + 8)? as usize;
            (length, 12)
        } else {
            let length = read_u16_le(data, offset + 6)? as usize;
            (length, 8)
        };
        Ok(Some(ElementHeader {
            tag,
            vr: Some(vr),
            length,
            header_size,
        }))
    } else {
        let length = read_u32_le(data, offset + 4)? as usize;
        Ok(Some(ElementHeader {
            tag,
            vr: None,
            length,
            header_size: 8,
        }))
    }
}

fn skip_element(
    data: &[u8],
    offset: usize,
    header: &ElementHeader,
    syntax: TransferSyntax,
    meta_group: bool,
) -> Result<usize, DicomError> {
    if header.tag == ITEM_DELIMITER {
        if header.length == 0xFFFF_FFFF {
            return skip_undefined_length_item(
                data,
                offset + header.header_size,
                syntax,
                meta_group,
            );
        }
        return Ok(offset + header.header_size + header.length);
    }
    if header.tag == SEQ_DELIMITER || header.tag == ITEM_END_DELIMITER {
        return Ok(offset + header.header_size);
    }

    if header.vr == Some(*b"SQ") || (header.vr.is_none() && header.length == 0xFFFF_FFFF) {
        return skip_sequence(data, offset + header.header_size, syntax, meta_group);
    }

    if header.length == 0xFFFF_FFFF {
        return Ok(data.len());
    }

    let end = offset
        .checked_add(header.header_size)
        .and_then(|v| v.checked_add(header.length))
        .ok_or(DicomError::UnexpectedEof)?;
    if end > data.len() {
        return Err(DicomError::UnexpectedEof);
    }
    Ok(end)
}

/// Skip nested elements inside an undefined-length sequence item until `(FFFE,E00D)`.
fn skip_undefined_length_item(
    data: &[u8],
    mut offset: usize,
    syntax: TransferSyntax,
    meta_group: bool,
) -> Result<usize, DicomError> {
    loop {
        if offset + 8 > data.len() {
            return Ok(offset);
        }
        let Some(header) = read_element_header(data, offset, syntax, meta_group)? else {
            return Ok(offset);
        };
        if header.tag == ITEM_END_DELIMITER {
            return Ok(offset + header.header_size);
        }
        offset = skip_element(data, offset, &header, syntax, meta_group)?;
    }
}

fn skip_sequence(
    data: &[u8],
    mut offset: usize,
    syntax: TransferSyntax,
    meta_group: bool,
) -> Result<usize, DicomError> {
    loop {
        if offset + 8 > data.len() {
            return Ok(offset);
        }
        let Some(header) = read_element_header(data, offset, syntax, meta_group)? else {
            return Ok(offset);
        };
        if header.tag == SEQ_DELIMITER {
            return Ok(offset + header.header_size);
        }
        offset = skip_element(data, offset, &header, syntax, meta_group)?;
    }
}

fn parse_meta_information(data: &[u8], offset: usize) -> Result<(String, usize), DicomError> {
    let mut transfer_syntax = TS_EXPLICIT_VR_LITTLE_ENDIAN.to_string();
    let mut cursor = offset;

    loop {
        let Some(header) = read_element_header(data, cursor, TransferSyntax::ExplicitVrLittleEndian, true)?
        else {
            break;
        };
        let value_offset = cursor + header.header_size;
        if header.tag == TAG_TRANSFER_SYNTAX_UID && header.length > 0 {
            let end = value_offset + header.length;
            if end > data.len() {
                return Err(DicomError::UnexpectedEof);
            }
            transfer_syntax = decode_dicom_string(&data[value_offset..end]);
        }
        if (header.tag >> 16) as u16 > 0x0002 {
            break;
        }
        cursor = skip_element(data, cursor, &header, TransferSyntax::ExplicitVrLittleEndian, true)?;
    }

    Ok((transfer_syntax, cursor))
}

fn transfer_syntax_from_uid(uid: &str) -> Result<TransferSyntax, DicomError> {
    match uid {
        TS_IMPLICIT_VR_LITTLE_ENDIAN => Ok(TransferSyntax::ImplicitVrLittleEndian),
        TS_EXPLICIT_VR_LITTLE_ENDIAN => Ok(TransferSyntax::ExplicitVrLittleEndian),
        TS_JPEG2000_LOSSLESS
        | TS_JPEG2000
        | TS_JPEG_BASELINE
        | "1.2.840.10008.1.2.4.51"
        | TS_RLE_LOSSLESS => Ok(TransferSyntax::EncapsulatedExplicitVr),
        other => Err(DicomError::UnsupportedTransferSyntax(other.to_string())),
    }
}

/// End offset of encapsulated Pixel Data fragment sequence (after item delimiter).
fn encapsulated_pixel_end(data: &[u8], start: usize) -> Result<usize, DicomError> {
    let mut cursor = start;
    loop {
        if cursor + 8 > data.len() {
            return Err(DicomError::UnexpectedEof);
        }
        let group = read_u16_le(data, cursor)?;
        let element = read_u16_le(data, cursor + 2)?;
        let tag = tag(group, element);
        let length = read_u32_le(data, cursor + 4)? as usize;
        if tag == SEQ_DELIMITER {
            return Ok(cursor + 8);
        }
        if tag != ITEM_DELIMITER {
            return Err(DicomError::InvalidVr);
        }
        cursor = cursor
            .checked_add(8)
            .and_then(|c| c.checked_add(length))
            .ok_or(DicomError::UnexpectedEof)?;
        if cursor > data.len() {
            return Err(DicomError::UnexpectedEof);
        }
    }
}

fn apply_value(meta: &mut DicomMetadata, tag: u32, value: &[u8]) {
    match tag {
        TAG_MODALITY => meta.modality = decode_dicom_string(value),
        TAG_STUDY_DESCRIPTION => meta.study_description = decode_dicom_string(value),
        TAG_SERIES_DESCRIPTION => meta.series_description = decode_dicom_string(value),
        TAG_BODY_PART_EXAMINED => meta.body_part_examined = decode_dicom_string(value),
        TAG_SERIES_INSTANCE_UID => meta.series_instance_uid = decode_dicom_string(value),
        TAG_INSTANCE_NUMBER => meta.instance_number = decode_i32(value),
        TAG_ROWS => meta.rows = decode_u16(value),
        TAG_COLUMNS => meta.columns = decode_u16(value),
        TAG_WINDOW_CENTER => meta.window_center = decode_numeric_string(value),
        TAG_WINDOW_WIDTH => meta.window_width = decode_numeric_string(value),
        TAG_PHOTOMETRIC_INTERPRETATION => {
            meta.photometric_interpretation = decode_dicom_string(value)
        }
        TAG_BITS_ALLOCATED => meta.bits_allocated = decode_u16(value),
        TAG_PIXEL_REPRESENTATION => meta.pixel_representation = decode_u16(value),
        TAG_PATIENT_ID => meta.patient_id = decode_dicom_string(value),
        TAG_STUDY_DATE => meta.study_date = decode_dicom_string(value),
        _ => {}
    }
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

#[inline]
pub fn unpack_volume_metadata(metadata: u64) -> (u16, u16, u32) {
    let rows = ((metadata >> 48) & 0xFFFF) as u16;
    let cols = ((metadata >> 32) & 0xFFFF) as u16;
    let byte_length = (metadata & 0xFFFF_FFFF) as u32;
    (rows, cols, byte_length)
}

fn walk_dataset(
    data: &[u8],
    mut cursor: usize,
    syntax: TransferSyntax,
    meta: &mut DicomMetadata,
    capture_pixels: bool,
) -> Result<Option<DicomPixelSlice>, DicomError> {
    let mut pixels = None;

    while cursor + 8 <= data.len() {
        let Some(header) = read_element_header(data, cursor, syntax, false)? else {
            break;
        };

        if header.tag == TAG_PIXEL_DATA {
            if capture_pixels {
                let value_offset = cursor + header.header_size;
                if header.length == 0xFFFF_FFFF {
                    let end = encapsulated_pixel_end(data, value_offset)?;
                    pixels = Some(DicomPixelSlice {
                        offset: value_offset,
                        length: end.saturating_sub(value_offset),
                    });
                } else {
                    let end = value_offset
                        .checked_add(header.length)
                        .ok_or(DicomError::UnexpectedEof)?;
                    if end <= data.len() {
                        pixels = Some(DicomPixelSlice {
                            offset: value_offset,
                            length: header.length,
                        });
                    }
                }
            }
            break;
        }

        if header.tag == ITEM_DELIMITER
            || header.tag == ITEM_END_DELIMITER
            || header.tag == SEQ_DELIMITER
        {
            cursor = skip_element(data, cursor, &header, syntax, false)?;
            continue;
        }

        let value_offset = cursor + header.header_size;
        if header.length != 0xFFFF_FFFF && value_offset + header.length <= data.len() {
            apply_value(
                meta,
                header.tag,
                &data[value_offset..value_offset + header.length],
            );
        }

        cursor = skip_element(data, cursor, &header, syntax, false)?;
    }

    Ok(pixels)
}

/// Parse DICOM Part 10 metadata from an on-disk byte buffer (pixel data optional).
pub fn parse_dicom_metadata_bytes(data: &[u8]) -> Result<DicomMetadata, DicomError> {
    let offset = find_dataset_offset(data)?;
    let (transfer_syntax_uid, cursor) = parse_meta_information(data, offset)?;
    let syntax = transfer_syntax_from_uid(&transfer_syntax_uid)?;
    let mut meta = DicomMetadata {
        transfer_syntax_uid,
        ..Default::default()
    };
    walk_dataset(data, cursor, syntax, &mut meta, false)?;
    Ok(meta)
}

/// Split-ingestion entry: metadata tags + pixel slice bounds (no heap copy of pixels).
pub fn split_dicom_payload(data: &[u8]) -> Result<DicomSplitPayload, DicomError> {
    let offset = find_dataset_offset(data)?;
    let (transfer_syntax_uid, cursor) = parse_meta_information(data, offset)?;
    let syntax = transfer_syntax_from_uid(&transfer_syntax_uid)?;

    let mut meta = DicomMetadata {
        transfer_syntax_uid,
        ..Default::default()
    };

    let pixels = walk_dataset(data, cursor, syntax, &mut meta, true)?
        .ok_or(DicomError::Io("DICOM has no Pixel Data (7FE0,0010)".into()))?;

    Ok(DicomSplitPayload { meta, pixels })
}

/// Parse DICOM metadata from a file path.
pub fn parse_dicom_file(path: &Path) -> Result<DicomMetadata, DicomError> {
    let bytes = fs::read(path).map_err(|e| DicomError::Io(e.to_string()))?;
    parse_dicom_metadata_bytes(&bytes)
}

pub fn normalize_dicom_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn default_organ_matchers() -> Vec<DicomTagMatcher> {
    vec![
        DicomTagMatcher {
            tokens: vec!["heart".into(), "cardiac".into(), "coronary".into(), "aorta".into()],
            organ: Some("Heart".into()),
        },
        DicomTagMatcher {
            tokens: vec![
                "lung".into(),
                "pulmonary".into(),
                "chest ct".into(),
                "thorax".into(),
            ],
            organ: Some("Lung".into()),
        },
        DicomTagMatcher {
            tokens: vec!["liver".into(), "hepatic".into()],
            organ: Some("Liver".into()),
        },
        DicomTagMatcher {
            tokens: vec!["brain".into(), "cerebral".into(), "cranial".into(), "head".into()],
            organ: Some("Brain (Allen)".into()),
        },
        DicomTagMatcher {
            tokens: vec!["kidney".into(), "renal".into()],
            organ: Some("Kidney (Left)".into()),
        },
        DicomTagMatcher {
            tokens: vec!["pancrea".into(), "pancreatic".into()],
            organ: Some("Pancreas".into()),
        },
        DicomTagMatcher {
            tokens: vec!["spleen".into(), "splenic".into()],
            organ: Some("Spleen".into()),
        },
        DicomTagMatcher {
            tokens: vec!["intestin".into(), "bowel".into(), "abdomen".into()],
            organ: Some("Small Intestine".into()),
        },
        DicomTagMatcher {
            tokens: vec!["prostate".into()],
            organ: Some("Prostate".into()),
        },
        DicomTagMatcher {
            tokens: vec!["uterus".into(), "uterine".into()],
            organ: Some("Uterus".into()),
        },
        DicomTagMatcher {
            tokens: vec!["ovary".into(), "ovarian".into()],
            organ: Some("Ovary (Left)".into()),
        },
    ]
}

pub fn infer_organ_from_metadata(
    meta: &DicomMetadata,
    matchers: &[DicomTagMatcher],
) -> Option<String> {
    let haystack = normalize_dicom_token(&[
        meta.body_part_examined.as_str(),
        meta.series_description.as_str(),
        meta.study_description.as_str(),
        meta.protocol_name.as_str(),
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .copied()
    .collect::<Vec<_>>()
    .join(" "));

    infer_organ_from_haystack(&haystack, matchers)
}

pub fn infer_organ_from_haystack(haystack: &str, matchers: &[DicomTagMatcher]) -> Option<String> {
    let normalized = normalize_dicom_token(haystack);
    for matcher in matchers {
        for token in &matcher.tokens {
            let token_norm = normalize_dicom_token(token);
            if !token_norm.is_empty() && normalized.contains(&token_norm) {
                return matcher.organ.clone();
            }
        }
    }
    None
}

pub fn default_placement_for_organ(organ: &str) -> DicomPlacement {
    let table: BTreeMap<&str, DicomPlacement> = BTreeMap::from([
        (
            "Heart",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.05,
                offset_z: 0.32,
                scale: 0.9,
                rotation_y: 0.0,
            },
        ),
        (
            "Lung",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.08,
                offset_z: 0.3,
                scale: 1.05,
                rotation_y: 0.0,
            },
        ),
        (
            "Liver",
            DicomPlacement {
                offset_x: 0.12,
                offset_y: -0.02,
                offset_z: 0.22,
                scale: 0.95,
                rotation_y: -0.35,
            },
        ),
        (
            "Brain (Allen)",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.18,
                offset_z: 0.12,
                scale: 1.0,
                rotation_y: 0.0,
            },
        ),
        (
            "Kidney (Left)",
            DicomPlacement {
                offset_x: -0.14,
                offset_y: -0.06,
                offset_z: 0.18,
                scale: 0.75,
                rotation_y: 0.2,
            },
        ),
    ]);

    table.get(organ).copied().unwrap_or(DicomPlacement {
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.28,
        scale: 0.85,
        rotation_y: 0.0,
    })
}

pub fn build_overlay_spec(
    meta: &DicomMetadata,
    organ: Option<String>,
    source: &str,
) -> DicomOverlaySpec {
    let organ_label = organ.or_else(|| infer_organ_from_metadata(meta, &default_organ_matchers()));
    let placement = organ_label
        .as_deref()
        .map(default_placement_for_organ)
        .unwrap_or_else(|| default_placement_for_organ("Heart"));

    DicomOverlaySpec {
        version: "1.0.0".to_string(),
        organ: organ_label,
        opacity: 0.72,
        visible: true,
        placement,
        series_instance_uid: if meta.series_instance_uid.is_empty() {
            None
        } else {
            Some(meta.series_instance_uid.clone())
        },
        modality: if meta.modality.is_empty() {
            None
        } else {
            Some(meta.modality.clone())
        },
        body_part_examined: if meta.body_part_examined.is_empty() {
            None
        } else {
            Some(meta.body_part_examined.clone())
        },
        source: Some(source.to_string()),
    }
}

pub fn overlay_spec_from_file(path: &Path) -> Result<DicomOverlaySpec, DicomError> {
    let meta = parse_dicom_file(path)?;
    Ok(build_overlay_spec(
        &meta,
        None,
        &format!("dicom-file:{}", path.display()),
    ))
}

pub fn overlay_spec_json_from_file(path: &Path) -> Result<String, String> {
    let spec = overlay_spec_from_file(path).map_err(|e| e.to_string())?;
    serde_json::to_string(&spec).map_err(|e| e.to_string())
}

pub fn metadata_json_from_file(path: &Path) -> Result<String, String> {
    let meta = parse_dicom_file(path).map_err(|e| e.to_string())?;
    serde_json::to_string(&meta).map_err(|e| e.to_string())
}

const IMAGING_CHAT_MARKERS: &[&str] = &[
    "dicom",
    "ct scan",
    " ct ",
    "mri",
    "x-ray",
    "xray",
    "radiograph",
    "ultrasound",
    "pet scan",
    "mammogram",
    "imaging study",
    "chest x",
    "slice thickness",
    "window level",
    "hounsfield",
];

pub fn chat_mentions_imaging(text: &str) -> bool {
    let haystack = format!(" {} ", text.to_lowercase());
    IMAGING_CHAT_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

pub fn infer_overlay_spec_from_text(
    text: &str,
    matchers: &[DicomTagMatcher],
) -> Option<DicomOverlaySpec> {
    if !chat_mentions_imaging(text) {
        return None;
    }

    let organ = infer_organ_from_haystack(text, matchers);
    let placement = organ
        .as_deref()
        .map(default_placement_for_organ)
        .unwrap_or_else(|| default_placement_for_organ("Heart"));

    Some(DicomOverlaySpec {
        version: "1.0.0".to_string(),
        organ,
        opacity: 0.72,
        visible: true,
        placement,
        series_instance_uid: None,
        modality: None,
        body_part_examined: None,
        source: Some("chat-inferred".to_string()),
    })
}

/// Default relative path for gitignored private DICOM slices (local dev only).
pub const DEFAULT_LOCAL_DICOM_DIR: &str = "app-development/DICOM-20250126T150544Z-001";

/// Resolve private local DICOM root: `QUALIA_DICOM_FIXTURE_DIR` env, then workspace-relative default.
pub fn resolve_local_dicom_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("QUALIA_DICOM_FIXTURE_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return path.canonicalize().ok().or(Some(path));
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(PathBuf::from(DEFAULT_LOCAL_DICOM_DIR));
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            PathBuf::from(manifest)
                .join("..")
                .join("..")
                .join(DEFAULT_LOCAL_DICOM_DIR),
        );
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(DEFAULT_LOCAL_DICOM_DIR));
        candidates.push(
            cwd.join("..")
                .join("..")
                .join(DEFAULT_LOCAL_DICOM_DIR),
        );
    }

    for path in candidates {
        if path.is_dir() {
            return path.canonicalize().ok().or(Some(path));
        }
    }
    None
}

/// Returns true when the byte slice looks like DICOM Part 10 (preamble + `DICM`).
pub fn is_dicom_part10(bytes: &[u8]) -> bool {
    bytes.len() >= 132 && &bytes[128..132] == b"DICM"
}

/// True when the buffer contains extractable Pixel Data for split-ingest.
pub fn dicom_has_pixel_data(data: &[u8]) -> bool {
    split_dicom_payload(data).is_ok()
}

/// Collect up to `max_files` DICOM Part-10 paths under `root` (cold-path helper; may allocate).
pub fn collect_dicom_paths_under(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_dicom_paths_recursive(root, max_files, &mut out);
    out
}

/// Image slices only (Pixel Data present), preferring `IM*` filenames typical of PACS export.
pub fn collect_dicom_image_paths_under(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut all = Vec::new();
    collect_dicom_paths_recursive(root, max_files.saturating_mul(8).max(32), &mut all);
    let mut images: Vec<PathBuf> = all
        .into_iter()
        .filter(|path| {
            fs::read(path)
                .ok()
                .map(|bytes| dicom_has_pixel_data(&bytes))
                .unwrap_or(false)
        })
        .collect();
    images.sort_by_key(|p| {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        (!(name.starts_with("IM") || name.starts_with("im")), name.to_string())
    });
    images.truncate(max_files);
    images
}

fn collect_dicom_paths_recursive(dir: &Path, max_files: usize, out: &mut Vec<PathBuf>) {
    if out.len() >= max_files {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= max_files {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_dicom_paths_recursive(&path, max_files, out);
        } else if path.is_file() {
            if let Ok(bytes) = fs::read(&path) {
                if is_dicom_part10(&bytes) {
                    out.push(path);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_fixtures {
    use super::*;

/// Minimal Part-10 bytes for unit tests (explicit VR, single 2×2 pixel frame).
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

#[cfg(test)]
pub use test_fixtures::test_fixture_split_bytes;

#[cfg(test)]
mod tests {
    use super::*;

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
        let meta = parse_dicom_metadata_bytes(&super::test_fixture_split_bytes()).expect("parse");
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
        let Some(root) = super::resolve_local_dicom_dir() else {
            eprintln!("skip: set QUALIA_DICOM_FIXTURE_DIR or add {DEFAULT_LOCAL_DICOM_DIR}");
            return;
        };
        let all = super::collect_dicom_paths_under(&root, 8);
        assert!(!all.is_empty(), "no Part-10 DICOM files under {}", root.display());
        for path in &all {
            let meta = super::parse_dicom_file(path).expect("metadata parse");
            assert!(!meta.modality.is_empty(), "modality missing in {}", path.display());
        }

        let images = super::collect_dicom_image_paths_under(&root, 4);
        assert!(
            !images.is_empty(),
            "no image slices with Pixel Data under {}",
            root.display()
        );
        for path in &images {
            let bytes = std::fs::read(path).unwrap();
            let split = super::split_dicom_payload(&bytes).expect("split ingest");
            assert!(split.pixels.length > 0, "pixel payload empty in {}", path.display());
            assert!(split.meta.rows > 0 && split.meta.columns > 0);
        }
    }
}
