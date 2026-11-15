//! Validated single-frame BIPED patient-space geometry in LPS millimeters.
//!
//! DICOM PS3.3 2026c C.7.6.2: https://dicom.nema.org/medical/dicom/2026c/output/chtml/part03/sect_C.7.6.2.html
//! These planes do not establish registration, slice spacing, segmentation or
//! enhanced/multi-frame functional-group parsing. Existing heuristic overlay
//! placements remain a separate API and are not patient-space registrations.
mod metadata;
mod plane;
pub(super) use metadata::apply_geometry_value;
pub use metadata::GeometryAttribute;
pub use plane::{FrameGeometryInput, GeometryError, PatientImagePlane, ORTHONORMAL_TOLERANCE};
#[cfg(test)]
mod tests;
