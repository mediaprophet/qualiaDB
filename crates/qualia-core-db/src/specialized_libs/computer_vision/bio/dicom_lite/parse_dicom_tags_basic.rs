//! Minimal little-endian explicit-VR DICOM tag scanner.
//!
//! Scans for a small set of common tags (PatientID, Rows, Columns, …).
//! Fails closed on missing preamble/prefix, implicit VR, or truncated data.
//!
//! **Not a full DICOM parser.** Pixel data, sequences, and private tags are
//! not decoded beyond length skip.

use std::collections::HashMap;

/// DICOM-lite errors (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DicomLiteError {
    TooShort,
    MissingPrefix,
    UnsupportedTransferSyntax,
    TruncatedElement,
    InvalidParameter,
}

impl core::fmt::Display for DicomLiteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort => write!(f, "buffer too short for DICOM"),
            Self::MissingPrefix => write!(f, "missing DICM prefix"),
            Self::UnsupportedTransferSyntax => {
                write!(f, "unsupported transfer syntax (need LE explicit VR)")
            }
            Self::TruncatedElement => write!(f, "truncated data element"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
        }
    }
}

/// Tag key as `(group, element)` — DICOM standard numbering.
pub type TagKey = (u16, u16);

/// Simple string-valued attribute map after parse/anonymize.
pub type DicomTagMap = HashMap<TagKey, String>;

/// Common tags we attempt to extract.
pub const TAG_PATIENT_ID: TagKey = (0x0010, 0x0020);
pub const TAG_PATIENT_NAME: TagKey = (0x0010, 0x0010);
pub const TAG_PATIENT_BIRTH: TagKey = (0x0010, 0x0030);
pub const TAG_STUDY_DATE: TagKey = (0x0008, 0x0020);
pub const TAG_MODALITY: TagKey = (0x0008, 0x0060);
pub const TAG_ROWS: TagKey = (0x0028, 0x0010);
pub const TAG_COLUMNS: TagKey = (0x0028, 0x0011);
pub const TAG_BITS_ALLOCATED: TagKey = (0x0028, 0x0100);
pub const TAG_TRANSFER_SYNTAX: TagKey = (0x0002, 0x0010);
pub const TAG_SOP_CLASS: TagKey = (0x0008, 0x0016);

/// Subset returned as typed convenience fields + full map of found strings.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDicomTags {
    pub patient_id: Option<String>,
    pub rows: Option<u16>,
    pub columns: Option<u16>,
    pub modality: Option<String>,
    pub transfer_syntax_uid: Option<String>,
    /// All string-decoded tags we recognised (including PHI — scrub before export).
    pub tags: DicomTagMap,
}

/// Explicit VR little-endian UIDs we accept.
const TS_LE_EXPLICIT: &str = "1.2.840.10008.1.2.1";
const TS_LE_EXPLICIT_DEFLATE: &str = "1.2.840.10008.1.2.1.99";
/// Implicit VR little endian — **rejected** (fail closed).
const TS_LE_IMPLICIT: &str = "1.2.840.10008.1.2";

/// Scan `bytes` for common DICOM tags (little-endian explicit VR).
///
/// Expects standard 128-byte preamble + `"DICM"`. Fails if transfer syntax
/// is missing or not LE explicit (or if implicit is detected).
pub fn parse_dicom_tags_basic(bytes: &[u8]) -> Result<ParsedDicomTags, DicomLiteError> {
    if bytes.len() < 132 {
        return Err(DicomLiteError::TooShort);
    }
    if &bytes[128..132] != b"DICM" {
        return Err(DicomLiteError::MissingPrefix);
    }

    let mut tags: DicomTagMap = HashMap::new();
    let mut pos = 132usize;
    let mut saw_transfer_syntax = false;
    let mut ts_uid = String::new();
    let mut in_meta = true;

    while pos + 8 <= bytes.len() {
        let group = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        let element = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        let key = (group, element);

        // File meta group is always explicit VR LE.
        // After meta, we require LE explicit (check transfer syntax once).
        if in_meta && group != 0x0002 {
            in_meta = false;
            if !saw_transfer_syntax {
                // No transfer syntax → cannot safely assume VR layout.
                return Err(DicomLiteError::UnsupportedTransferSyntax);
            }
            if ts_uid.trim_end_matches('\0').trim() == TS_LE_IMPLICIT {
                return Err(DicomLiteError::UnsupportedTransferSyntax);
            }
            let ts = ts_uid.trim_end_matches('\0').trim();
            if ts != TS_LE_EXPLICIT && ts != TS_LE_EXPLICIT_DEFLATE && !ts.is_empty() {
                // Other explicit (BE, JPEG, etc.) — fail closed for this lite path.
                // Allow empty-only already handled; if unknown non-empty non-LE-explicit → reject.
                if !ts.starts_with("1.2.840.10008.1.2.1") {
                    return Err(DicomLiteError::UnsupportedTransferSyntax);
                }
            }
        }

        // Explicit VR: bytes 4-5 = VR
        if pos + 6 > bytes.len() {
            break;
        }
        let vr = [bytes[pos + 4], bytes[pos + 5]];
        let (value_start, value_len) = parse_explicit_vr_length(bytes, pos, vr)?;
        if value_start.saturating_add(value_len) > bytes.len() {
            return Err(DicomLiteError::TruncatedElement);
        }
        let value = &bytes[value_start..value_start + value_len];

        if key == TAG_TRANSFER_SYNTAX {
            saw_transfer_syntax = true;
            ts_uid = decode_string(value);
            tags.insert(key, ts_uid.clone());
        } else if is_string_vr(vr) {
            let s = decode_string(value);
            if !s.is_empty() {
                tags.insert(key, s);
            }
        } else if is_u16_vr(vr) && value_len >= 2 {
            let v = u16::from_le_bytes([value[0], value[1]]);
            tags.insert(key, v.to_string());
        }

        // Advance; stop if we hit Pixel Data (7FE0,0010) — no need to scan pixels.
        pos = value_start + value_len;
        if key == (0x7FE0, 0x0010) {
            break;
        }
        // Safety: if length was undefined (0xFFFF_FFFF) we can't skip — fail closed.
        if value_len == 0xFFFF_FFFF {
            return Err(DicomLiteError::UnsupportedTransferSyntax);
        }
    }

    if !saw_transfer_syntax {
        return Err(DicomLiteError::UnsupportedTransferSyntax);
    }

    let patient_id = tags.get(&TAG_PATIENT_ID).cloned();
    let modality = tags.get(&TAG_MODALITY).cloned();
    let rows = tags.get(&TAG_ROWS).and_then(|s| s.parse::<u16>().ok());
    let columns = tags.get(&TAG_COLUMNS).and_then(|s| s.parse::<u16>().ok());
    let transfer_syntax_uid = Some(ts_uid.trim_end_matches('\0').trim().to_string());

    Ok(ParsedDicomTags {
        patient_id,
        rows,
        columns,
        modality,
        transfer_syntax_uid,
        tags,
    })
}

fn parse_explicit_vr_length(
    bytes: &[u8],
    pos: usize,
    vr: [u8; 2],
) -> Result<(usize, usize), DicomLiteError> {
    // OB/OW/OF/SQ/UT/UN: 2-byte VR + 2 reserved + 4-byte length
    // others: 2-byte VR + 2-byte length
    let long_vr = matches!(
        &vr,
        b"OB" | b"OW" | b"OF" | b"SQ" | b"UT" | b"UN" | b"OD" | b"OL" | b"UC" | b"UR"
    );
    if long_vr {
        if pos + 12 > bytes.len() {
            return Err(DicomLiteError::TruncatedElement);
        }
        let len = u32::from_le_bytes([
            bytes[pos + 8],
            bytes[pos + 9],
            bytes[pos + 10],
            bytes[pos + 11],
        ]);
        Ok((pos + 12, len as usize))
    } else {
        if pos + 8 > bytes.len() {
            return Err(DicomLiteError::TruncatedElement);
        }
        let len = u16::from_le_bytes([bytes[pos + 6], bytes[pos + 7]]) as usize;
        Ok((pos + 8, len))
    }
}

fn is_string_vr(vr: [u8; 2]) -> bool {
    matches!(
        &vr,
        b"AE"
            | b"AS"
            | b"CS"
            | b"DA"
            | b"DS"
            | b"DT"
            | b"IS"
            | b"LO"
            | b"LT"
            | b"PN"
            | b"SH"
            | b"ST"
            | b"TM"
            | b"UC"
            | b"UI"
            | b"UR"
            | b"UT"
    )
}

fn is_u16_vr(vr: [u8; 2]) -> bool {
    matches!(&vr, b"US" | b"SS")
}

fn decode_string(value: &[u8]) -> String {
    let s = String::from_utf8_lossy(value);
    s.trim_end_matches('\0').trim().to_string()
}

/// Build a minimal synthetic LE-explicit DICOM buffer for tests (not a full IOD).
#[cfg(test)]
pub fn synth_le_explicit_dicom(patient_id: &str, rows: u16, cols: u16, modality: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 128];
    buf.extend_from_slice(b"DICM");

    // (0002,0010) UI TransferSyntaxUID = LE explicit
    push_ui(&mut buf, 0x0002, 0x0010, TS_LE_EXPLICIT);
    // (0002,0000) UL FileMetaInformationGroupLength — skip optional

    // (0010,0020) LO PatientID
    push_lo(&mut buf, 0x0010, 0x0020, patient_id);
    // (0008,0060) CS Modality
    push_cs(&mut buf, 0x0008, 0x0060, modality);
    // (0028,0010) US Rows
    push_us(&mut buf, 0x0028, 0x0010, rows);
    // (0028,0011) US Columns
    push_us(&mut buf, 0x0028, 0x0011, cols);

    buf
}

#[cfg(test)]
fn push_ui(buf: &mut Vec<u8>, g: u16, e: u16, uid: &str) {
    // UI uses short length form (2-byte)
    let mut v = uid.as_bytes().to_vec();
    if v.len() % 2 == 1 {
        v.push(0); // pad
    }
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(b"UI");
    buf.extend_from_slice(&(v.len() as u16).to_le_bytes());
    buf.extend_from_slice(&v);
}

#[cfg(test)]
fn push_lo(buf: &mut Vec<u8>, g: u16, e: u16, s: &str) {
    let mut v = s.as_bytes().to_vec();
    if v.len() % 2 == 1 {
        v.push(b' ');
    }
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(b"LO");
    buf.extend_from_slice(&(v.len() as u16).to_le_bytes());
    buf.extend_from_slice(&v);
}

#[cfg(test)]
fn push_cs(buf: &mut Vec<u8>, g: u16, e: u16, s: &str) {
    let mut v = s.as_bytes().to_vec();
    if v.len() % 2 == 1 {
        v.push(b' ');
    }
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(b"CS");
    buf.extend_from_slice(&(v.len() as u16).to_le_bytes());
    buf.extend_from_slice(&v);
}

#[cfg(test)]
fn push_us(buf: &mut Vec<u8>, g: u16, e: u16, v: u16) {
    buf.extend_from_slice(&g.to_le_bytes());
    buf.extend_from_slice(&e.to_le_bytes());
    buf.extend_from_slice(b"US");
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synthetic_le_explicit() {
        let buf = synth_le_explicit_dicom("P123", 512, 512, "CT");
        let p = parse_dicom_tags_basic(&buf).unwrap();
        assert_eq!(p.patient_id.as_deref(), Some("P123"));
        assert_eq!(p.rows, Some(512));
        assert_eq!(p.columns, Some(512));
        assert_eq!(p.modality.as_deref(), Some("CT"));
        assert!(p
            .transfer_syntax_uid
            .as_deref()
            .unwrap()
            .starts_with("1.2.840.10008.1.2.1"));
    }

    #[test]
    fn missing_dicm_fails() {
        let mut buf = vec![0u8; 132];
        buf[128..132].copy_from_slice(b"XXXX");
        assert_eq!(
            parse_dicom_tags_basic(&buf).unwrap_err(),
            DicomLiteError::MissingPrefix
        );
    }

    #[test]
    fn too_short_fails() {
        assert_eq!(
            parse_dicom_tags_basic(&[0u8; 10]).unwrap_err(),
            DicomLiteError::TooShort
        );
    }

    #[test]
    fn implicit_vr_fails_closed() {
        let mut buf = vec![0u8; 128];
        buf.extend_from_slice(b"DICM");
        push_ui(&mut buf, 0x0002, 0x0010, TS_LE_IMPLICIT);
        push_lo(&mut buf, 0x0010, 0x0020, "X");
        assert_eq!(
            parse_dicom_tags_basic(&buf).unwrap_err(),
            DicomLiteError::UnsupportedTransferSyntax
        );
    }
}
