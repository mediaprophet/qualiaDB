use crate::dicom::DicomMetadata;
use serde::{Deserialize, Serialize};

/// Identifies missing or invalid source geometry without replacing it with zeros.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryAttribute {
    ImagePositionPatient,
    ImageOrientationPatient,
    PixelSpacing,
    FrameOfReferenceUid,
    AnatomicalOrientationType,
}

/// Strict DS multiplicity/lexical/finite parsing. Numeric DS values may be padded
/// with spaces, but not NULs, lossy UTF-8, tabs, NaN or infinities.
fn decimal_array<const N: usize>(value: &[u8]) -> Option<[f64; N]> {
    let text = std::str::from_utf8(value).ok()?;
    let mut parts = text.split('\\');
    let mut values = [0.0; N];
    for slot in &mut values {
        let raw = parts.next()?;
        if raw.len() > 16 {
            return None;
        }
        let part = raw.trim_matches(' ');
        if part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
        {
            return None;
        }
        let value = part.parse::<f64>().ok()?;
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(values)
}

pub(super) fn valid_uid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && value
            .split('.')
            .all(|part| !part.is_empty() && (part.len() == 1 || !part.starts_with('0')))
}
fn uid(value: &[u8]) -> Option<String> {
    let value = value.strip_suffix(&[0]).unwrap_or(value);
    let text = std::str::from_utf8(value).ok()?;
    valid_uid(text).then(|| text.to_owned())
}
fn orientation_type(value: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(value).ok()?.trim_matches(' ');
    matches!(text, "BIPED" | "QUADRUPED").then(|| text.to_owned())
}

/// Returns true for handled tags. Every duplicate is ambiguous and invalid,
/// including equal values; invalid diagnostics are sticky. Only five unique
/// diagnostic entries can arise from this parser.
pub(crate) fn apply_geometry_value(
    meta: &mut DicomMetadata,
    tag: u32,
    value: &[u8],
    vr: Option<[u8; 2]>,
) -> bool {
    use GeometryAttribute::*;
    let field = match tag {
        0x0020_0032 => ImagePositionPatient,
        0x0020_0037 => ImageOrientationPatient,
        0x0028_0030 => PixelSpacing,
        0x0020_0052 => FrameOfReferenceUid,
        0x0010_2210 => AnatomicalOrientationType,
        _ => return false,
    };
    if meta.geometry_parse_errors.contains(&field) {
        return true;
    }
    let seen = match field {
        ImagePositionPatient => meta.image_position_patient.is_some(),
        ImageOrientationPatient => meta.image_orientation_patient.is_some(),
        PixelSpacing => meta.pixel_spacing.is_some(),
        FrameOfReferenceUid => meta.frame_of_reference_uid.is_some(),
        AnatomicalOrientationType => meta.anatomical_orientation_type.is_some(),
    };
    if seen {
        clear_value(meta, field);
        meta.geometry_parse_errors.push(field);
        return true;
    }
    let expected_vr = match field {
        ImagePositionPatient | ImageOrientationPatient | PixelSpacing => *b"DS",
        FrameOfReferenceUid => *b"UI",
        AnatomicalOrientationType => *b"CS",
    };
    // None is the implicit-VR dataset path; explicit wrong VR is malformed.
    if vr.is_some_and(|actual| actual != expected_vr) {
        meta.geometry_parse_errors.push(field);
        clear_value(meta, field);
        return true;
    }
    let valid = match field {
        ImagePositionPatient => {
            meta.image_position_patient = decimal_array(value);
            meta.image_position_patient.is_some()
        }
        ImageOrientationPatient => {
            meta.image_orientation_patient = decimal_array(value);
            meta.image_orientation_patient.is_some()
        }
        PixelSpacing => {
            meta.pixel_spacing = decimal_array(value);
            meta.pixel_spacing.is_some()
        }
        FrameOfReferenceUid => {
            meta.frame_of_reference_uid = uid(value);
            meta.frame_of_reference_uid.is_some()
        }
        AnatomicalOrientationType => {
            meta.anatomical_orientation_type = orientation_type(value);
            meta.anatomical_orientation_type.is_some()
        }
    };
    if !valid {
        meta.geometry_parse_errors.push(field);
    }
    true
}

fn clear_value(meta: &mut DicomMetadata, field: GeometryAttribute) {
    match field {
        GeometryAttribute::ImagePositionPatient => meta.image_position_patient = None,
        GeometryAttribute::ImageOrientationPatient => meta.image_orientation_patient = None,
        GeometryAttribute::PixelSpacing => meta.pixel_spacing = None,
        GeometryAttribute::FrameOfReferenceUid => meta.frame_of_reference_uid = None,
        GeometryAttribute::AnatomicalOrientationType => meta.anatomical_orientation_type = None,
    }
}
