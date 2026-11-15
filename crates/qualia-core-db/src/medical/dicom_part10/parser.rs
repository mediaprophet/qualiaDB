//! DICOM Part-10 metadata parsing and zero-copy Pixel Data split entry points.

use std::fs;
use std::path::Path;

use super::stream::{
    decode_dicom_string, decode_i32, decode_numeric_string, decode_u16, encapsulated_pixel_end,
    find_dataset_offset, read_element_header, skip_element, TransferSyntax,
};
use super::types::{
    DicomError, DicomMetadata, DicomPixelSlice, DicomSplitPayload, ITEM_DELIMITER,
    ITEM_END_DELIMITER, SEQ_DELIMITER, TAG_BITS_ALLOCATED, TAG_BODY_PART_EXAMINED, TAG_COLUMNS,
    TAG_INSTANCE_NUMBER, TAG_MODALITY, TAG_PATIENT_ID, TAG_PHOTOMETRIC_INTERPRETATION,
    TAG_PIXEL_DATA, TAG_PIXEL_REPRESENTATION, TAG_ROWS, TAG_SERIES_DESCRIPTION,
    TAG_SERIES_INSTANCE_UID, TAG_STUDY_DATE, TAG_STUDY_DESCRIPTION, TAG_TRANSFER_SYNTAX_UID,
    TAG_WINDOW_CENTER, TAG_WINDOW_WIDTH, TS_EXPLICIT_VR_LITTLE_ENDIAN,
    TS_IMPLICIT_VR_LITTLE_ENDIAN, TS_JPEG2000, TS_JPEG2000_LOSSLESS, TS_JPEG_BASELINE,
    TS_RLE_LOSSLESS,
};

/// Walk the group-0x0002 file-meta elements (always explicit-VR LE)
/// to extract the transfer-syntax UID; returns it plus the dataset cursor.
fn parse_meta_information(data: &[u8], offset: usize) -> Result<(String, usize), DicomError> {
    let mut transfer_syntax = TS_EXPLICIT_VR_LITTLE_ENDIAN.to_string();
    let mut cursor = offset;

    loop {
        let Some(header) =
            read_element_header(data, cursor, TransferSyntax::ExplicitVrLittleEndian, true)?
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
        cursor = skip_element(
            data,
            cursor,
            &header,
            TransferSyntax::ExplicitVrLittleEndian,
            true,
        )?;
    }

    Ok((transfer_syntax, cursor))
}

/// Map a transfer-syntax UID to the `TransferSyntax` variant;
/// encapsulated syntaxes (JPEG/JPEG2000/RLE) fold into
/// `EncapsulatedExplicitVr`. Unknown UIDs fail closed.
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

/// Decode `value` into the matching `DicomMetadata` field for each
/// recognized tag; unrecognized tags are skipped.
fn apply_value(meta: &mut DicomMetadata, tag: u32, value: &[u8], vr: Option<[u8; 2]>) {
    if crate::medical::dicom_geometry::apply_geometry_value(meta, tag, value, vr) {
        return;
    }
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

/// Sequential element walk: applies recognized tags into `meta` and,
/// when `capture_pixels`, returns the Pixel Data element's byte
/// slice (offset+length — no copy, zero-copy ingest seam).
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
                header.vr,
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
