use super::{metadata::valid_uid, GeometryAttribute};
use crate::dicom::DicomMetadata;

/// Absolute tolerance for squared unit norms and vector dot products. Rounded
/// DS direction cosines within this bound are retained, never silently repaired.
pub const ORTHONORMAL_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    Missing(GeometryAttribute),
    Invalid(GeometryAttribute),
    UnsupportedQuadruped,
    NonFiniteMappedPosition,
}
impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "missing DICOM geometry: {field:?}"),
            Self::Invalid(field) => write!(f, "invalid DICOM geometry: {field:?}"),
            Self::UnsupportedQuadruped => write!(
                f,
                "QUADRUPED geometry is unsupported by the BIPED LPS adapter"
            ),
            Self::NonFiniteMappedPosition => {
                write!(f, "pixel mapping exceeds finite patient-space coordinates")
            }
        }
    }
}
impl std::error::Error for GeometryError {}

/// Explicit metadata for exactly one frame. Enhanced-frame extraction can supply
/// this input, but this API does not parse Shared/Per-Frame Functional Groups or
/// copy one top-level plane across a multi-frame dataset.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameGeometryInput<'a> {
    pub image_position_patient: Option<[f64; 3]>,
    pub image_orientation_patient: Option<[f64; 6]>,
    /// DICOM order: [row spacing, column spacing], in millimeters.
    pub pixel_spacing: Option<[f64; 2]>,
    pub frame_of_reference_uid: Option<&'a str>,
    /// Missing means BIPED under DICOM C.7.6.2.1.1, not inferred anatomy.
    pub anatomical_orientation_type: Option<&'a str>,
    pub invalid_source_fields: &'a [GeometryAttribute],
}

/// Validated immutable image plane. +X is patient Left, +Y Posterior, +Z Superior;
/// all positions/distances are millimeters. An absent frame UID stays absent;
/// neither equal nor absent UIDs establish a registration transform.
#[derive(Clone, Copy, Debug)]
pub struct PatientImagePlane<'a> {
    origin_mm: [f64; 3],
    first_direction: [f64; 3],
    second_direction: [f64; 3],
    spacing_mm: [f64; 2],
    frame_uid: Option<&'a str>,
}
impl<'a> PatientImagePlane<'a> {
    /// Interpret only the metadata provided for a single frame; no enhanced-frame
    /// extraction or missing-position/orientation/spacing fallback is performed.
    pub fn from_metadata(meta: &'a DicomMetadata) -> Result<Self, GeometryError> {
        Self::from_frame_geometry(FrameGeometryInput {
            image_position_patient: meta.image_position_patient,
            image_orientation_patient: meta.image_orientation_patient,
            pixel_spacing: meta.pixel_spacing,
            frame_of_reference_uid: meta.frame_of_reference_uid.as_deref(),
            anatomical_orientation_type: meta.anatomical_orientation_type.as_deref(),
            invalid_source_fields: &meta.geometry_parse_errors,
        })
    }
    pub fn from_frame_geometry(input: FrameGeometryInput<'a>) -> Result<Self, GeometryError> {
        use GeometryAttribute::*;
        if let Some(field) = input.invalid_source_fields.first() {
            return Err(GeometryError::Invalid(*field));
        }
        match input.anatomical_orientation_type {
            None | Some("BIPED") => {}
            Some("QUADRUPED") => return Err(GeometryError::UnsupportedQuadruped),
            _ => return Err(GeometryError::Invalid(AnatomicalOrientationType)),
        }
        if input
            .frame_of_reference_uid
            .is_some_and(|uid| !valid_uid(uid))
        {
            return Err(GeometryError::Invalid(FrameOfReferenceUid));
        }
        let origin = input
            .image_position_patient
            .ok_or(GeometryError::Missing(ImagePositionPatient))?;
        if !origin.iter().all(|v| v.is_finite()) {
            return Err(GeometryError::Invalid(ImagePositionPatient));
        }
        let orientation = input
            .image_orientation_patient
            .ok_or(GeometryError::Missing(ImageOrientationPatient))?;
        let first = [orientation[0], orientation[1], orientation[2]];
        let second = [orientation[3], orientation[4], orientation[5]];
        if !orientation.iter().all(|v| v.is_finite())
            || (dot(first, first) - 1.0).abs() > ORTHONORMAL_TOLERANCE
            || (dot(second, second) - 1.0).abs() > ORTHONORMAL_TOLERANCE
            || dot(first, second).abs() > ORTHONORMAL_TOLERANCE
        {
            return Err(GeometryError::Invalid(ImageOrientationPatient));
        }
        let spacing = input
            .pixel_spacing
            .ok_or(GeometryError::Missing(PixelSpacing))?;
        if !spacing.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(GeometryError::Invalid(PixelSpacing));
        }
        Ok(Self {
            origin_mm: origin,
            first_direction: first,
            second_direction: second,
            spacing_mm: spacing,
            frame_uid: input.frame_of_reference_uid,
        })
    }
    pub fn origin_lps_mm(&self) -> [f64; 3] {
        self.origin_mm
    }
    /// First orientation vector (DICOM row direction), advanced by COLUMN index.
    pub fn first_direction_lps(&self) -> [f64; 3] {
        self.first_direction
    }
    /// Second orientation vector (DICOM column direction), advanced by ROW index.
    pub fn second_direction_lps(&self) -> [f64; 3] {
        self.second_direction
    }
    pub fn pixel_spacing_mm(&self) -> [f64; 2] {
        self.spacing_mm
    }
    pub fn frame_of_reference_uid(&self) -> Option<&'a str> {
        self.frame_uid
    }
    /// Cross product of the two supplied orientation vectors; no slice spacing is
    /// inferred from this normal, SliceThickness or SliceLocation.
    pub fn normal_lps(&self) -> [f64; 3] {
        let a = self.first_direction;
        let b = self.second_direction;
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    /// Zero-based integer pixel CENTER mapping from C.7.6.2.1-1:
    /// origin + column * first_direction * column_spacing
    ///        + row * second_direction * row_spacing.
    /// The plane has no raster extent: caller checks row/column bounds separately.
    /// This is not the edge-based sub-pixel convention or a registration API.
    pub fn pixel_center_lps_mm(
        &self,
        column_index: u32,
        row_index: u32,
    ) -> Result<[f64; 3], GeometryError> {
        let column = column_index as f64 * self.spacing_mm[1];
        let row = row_index as f64 * self.spacing_mm[0];
        let mut point = self.origin_mm;
        for (axis, value) in point.iter_mut().enumerate() {
            *value += column * self.first_direction[axis] + row * self.second_direction[axis];
        }
        if !point.iter().all(|v| v.is_finite()) {
            return Err(GeometryError::NonFiniteMappedPosition);
        }
        Ok(point)
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
