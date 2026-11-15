//! Low-level DICOM Part-10 binary stream primitives (headers, delimiters, scalar decoders).

use super::types::{DicomError, ITEM_DELIMITER, ITEM_END_DELIMITER, SEQ_DELIMITER, tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferSyntax {
    ImplicitVrLittleEndian,
    ExplicitVrLittleEndian,
    /// Compressed pixel data (JPEG / JPEG 2000 / RLE); dataset tags remain explicit VR.
    EncapsulatedExplicitVr,
}

/// Locate the dataset start: standard 128-byte preamble + `DICM`
/// magic, with a scan fallback when the preamble is absent.
pub(super) fn find_dataset_offset(data: &[u8]) -> Result<usize, DicomError> {
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

/// Bounds-checked little-endian u16 read at `offset`.
pub(super) fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, DicomError> {
    let end = offset + 2;
    if end > data.len() {
        return Err(DicomError::UnexpectedEof);
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Bounds-checked little-endian u32 read at `offset`.
pub(super) fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, DicomError> {
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

/// `true` for VRs whose explicit header carries a 32-bit length at
/// offset +8 (OB/OD/OF/OL/OV/OW/SQ/SV/UC/UN/UR/UT/UV), per PS3.5 7.1.2.
fn vr_is_long(vr: &[u8; 2]) -> bool {
    matches!(
        vr,
        b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"SQ"
            | b"SV"
            | b"UC"
            | b"UN"
            | b"UR"
            | b"UT"
            | b"UV"
    )
}

/// Lossy-UTF8 decode of a NUL-terminated, whitespace-trimmed DICOM text value.
pub(super) fn decode_dicom_string(bytes: &[u8]) -> String {
    let trimmed = bytes
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&trimmed).trim().to_string()
}

/// Parse the first `\`-separated component of a DS/IS value as f64.
pub(super) fn decode_numeric_string(bytes: &[u8]) -> Option<f64> {
    let text = decode_dicom_string(bytes);
    text.split('\\')
        .next()
        .and_then(|part| part.trim().parse::<f64>().ok())
}

/// Numeric-string decode to i32 (0 on failure).
pub(super) fn decode_i32(bytes: &[u8]) -> i32 {
    decode_numeric_string(bytes).map(|v| v as i32).unwrap_or(0)
}

/// Numeric-string decode to u16, falling back to a raw LE u16 read for binary-valued elements.
pub(super) fn decode_u16(bytes: &[u8]) -> u16 {
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

pub(super) struct ElementHeader {
    pub(super) tag: u32,
    pub(super) vr: Option<[u8; 2]>,
    pub(super) length: usize,
    pub(super) header_size: usize,
}

/// Parse one element header: delimiter tags are fixed 8-byte; explicit-VR headers add
/// the VR code (long VRs extend to 12-byte); implicit-VR is 8-byte with a 32-bit length.
/// `meta_group` forces explicit VR per Part 10.
pub(super) fn read_element_header(
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

/// Advance the cursor past one element's value — handles undefined-length items/sequences.
pub(super) fn skip_element(
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

/// Advance past a sequence to its sequence-delimitation item.
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

/// End offset of encapsulated Pixel Data fragment sequence (after sequence delimiter).
pub(super) fn encapsulated_pixel_end(data: &[u8], start: usize) -> Result<usize, DicomError> {
    let mut cursor = start;
    loop {
        if cursor + 8 > data.len() {
            return Err(DicomError::UnexpectedEof);
        }
        let group = read_u16_le(data, cursor)?;
        let element = read_u16_le(data, cursor + 2)?;
        let item_tag = tag(group, element);
        let length = read_u32_le(data, cursor + 4)? as usize;
        if item_tag == SEQ_DELIMITER {
            return Ok(cursor + 8);
        }
        if item_tag != ITEM_DELIMITER {
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
