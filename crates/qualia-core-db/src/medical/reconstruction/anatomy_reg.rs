//! Source-bound patient-space (DICOM LPS mm) to anatomical/target coordinate frame
//! registration with `FrameOfReferenceUID` verification, `SO(3)` / affine invertibility,
//! roundtrip consistency enforcement, and distinct BLAKE3 derivation stamps.
use crate::medical::dicom_geometry::{GeometryError, PatientImagePlane, ORTHONORMAL_TOLERANCE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnatomyRegError {
    MissingFrameOfReference,
    FrameOfReferenceMismatch,
    Geometry(GeometryError),
    NonFiniteTransform,
    NonOrthonormalRotation,
    ReflectionRejected,
    SingularTransform,
    InvalidAffineRow,
    RoundtripResidualExceeded,
}

impl From<GeometryError> for AnatomyRegError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}
impl std::fmt::Display for AnatomyRegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anatomy registration error: {self:?}")
    }
}
impl std::error::Error for AnatomyRegError {}

#[derive(Clone, Copy, Debug)]
pub enum AnatomyTransform {
    Rigid {
        rotation_3x3: [[f64; 3]; 3],
        translation_mm: [f64; 3],
    },
    Affine4x4 {
        matrix: [[f64; 4]; 4],
    },
}

#[derive(Clone, Copy, Debug)]
pub struct CrossFrameRegistrationCertificate<'a> {
    pub source_frame_of_reference_uid: &'a str,
    pub target_frame_of_reference_uid: &'a str,
    pub certificate_blake3: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct RegisteredAnatomyPoint<'a> {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub source_frame_of_reference_uid: &'a str,
    pub target_frame_of_reference_uid: &'a str,
    pub patient_lps_mm: [f64; 3],
    pub anatomy_coords_mm: [f64; 3],
    pub roundtrip_lps_mm: [f64; 3],
    pub roundtrip_residual_mm: f64,
    pub inverse_matrix_4x4: [[f64; 4]; 4],
}

fn mat3_det(r: &[[f64; 3]; 3]) -> f64 {
    r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
        - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
        + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
}

fn mat3_inv(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
    let c01 = -(m[1][0] * m[2][2] - m[1][2] * m[2][0]);
    let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
    let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
    if det.abs() < 1e-9 || !det.is_finite() {
        return None;
    }
    let inv = 1.0 / det;
    let c10 = -(m[0][1] * m[2][2] - m[0][2] * m[2][1]);
    let c11 = m[0][0] * m[2][2] - m[0][2] * m[2][0];
    let c12 = -(m[0][0] * m[2][1] - m[0][1] * m[2][0]);
    let c20 = m[0][1] * m[1][2] - m[0][2] * m[1][1];
    let c21 = -(m[0][0] * m[1][2] - m[0][2] * m[1][0]);
    let c22 = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    Some([
        [c00 * inv, c10 * inv, c20 * inv],
        [c01 * inv, c11 * inv, c21 * inv],
        [c02 * inv, c12 * inv, c22 * inv],
    ])
}

fn apply_4x4(m: &[[f64; 4]; 4], p: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    ]
}

fn validate_and_invert_transform(
    tf: &AnatomyTransform,
) -> Result<([[f64; 4]; 4], [[f64; 4]; 4]), AnatomyRegError> {
    match *tf {
        AnatomyTransform::Rigid { rotation_3x3: r, translation_mm: t } => {
            if r.iter().flatten().chain(t.iter()).any(|v| !v.is_finite()) {
                return Err(AnatomyRegError::NonFiniteTransform);
            }
            for i in 0..3 {
                for j in 0..3 {
                    let dot = r[0][i] * r[0][j] + r[1][i] * r[1][j] + r[2][i] * r[2][j];
                    let expect = if i == j { 1.0 } else { 0.0 };
                    if (dot - expect).abs() > ORTHONORMAL_TOLERANCE {
                        return Err(AnatomyRegError::NonOrthonormalRotation);
                    }
                }
            }
            let det = mat3_det(&r);
            if det < 0.0 {
                return Err(AnatomyRegError::ReflectionRejected);
            }
            if (det - 1.0).abs() > ORTHONORMAL_TOLERANCE {
                return Err(AnatomyRegError::NonOrthonormalRotation);
            }
            let fwd = [
                [r[0][0], r[0][1], r[0][2], t[0]],
                [r[1][0], r[1][1], r[1][2], t[1]],
                [r[2][0], r[2][1], r[2][2], t[2]],
                [0.0, 0.0, 0.0, 1.0],
            ];
            let rt = [
                [r[0][0], r[1][0], r[2][0]],
                [r[0][1], r[1][1], r[2][1]],
                [r[0][2], r[1][2], r[2][2]],
            ];
            let inv_t = [
                -(rt[0][0] * t[0] + rt[0][1] * t[1] + rt[0][2] * t[2]),
                -(rt[1][0] * t[0] + rt[1][1] * t[1] + rt[1][2] * t[2]),
                -(rt[2][0] * t[0] + rt[2][1] * t[1] + rt[2][2] * t[2]),
            ];
            let inv = [
                [rt[0][0], rt[0][1], rt[0][2], inv_t[0]],
                [rt[1][0], rt[1][1], rt[1][2], inv_t[1]],
                [rt[2][0], rt[2][1], rt[2][2], inv_t[2]],
                [0.0, 0.0, 0.0, 1.0],
            ];
            Ok((fwd, inv))
        }
        AnatomyTransform::Affine4x4 { matrix: m } => {
            if m.iter().flatten().any(|v| !v.is_finite()) {
                return Err(AnatomyRegError::NonFiniteTransform);
            }
            if (m[3][0]).abs() > 1e-9
                || (m[3][1]).abs() > 1e-9
                || (m[3][2]).abs() > 1e-9
                || (m[3][3] - 1.0).abs() > 1e-9
            {
                return Err(AnatomyRegError::InvalidAffineRow);
            }
            let lin = [
                [m[0][0], m[0][1], m[0][2]],
                [m[1][0], m[1][1], m[1][2]],
                [m[2][0], m[2][1], m[2][2]],
            ];
            let lin_inv = mat3_inv(&lin).ok_or(AnatomyRegError::SingularTransform)?;
            let inv_t = [
                -(lin_inv[0][0] * m[0][3] + lin_inv[0][1] * m[1][3] + lin_inv[0][2] * m[2][3]),
                -(lin_inv[1][0] * m[0][3] + lin_inv[1][1] * m[1][3] + lin_inv[1][2] * m[2][3]),
                -(lin_inv[2][0] * m[0][3] + lin_inv[2][1] * m[1][3] + lin_inv[2][2] * m[2][3]),
            ];
            let inv = [
                [lin_inv[0][0], lin_inv[0][1], lin_inv[0][2], inv_t[0]],
                [lin_inv[1][0], lin_inv[1][1], lin_inv[1][2], inv_t[1]],
                [lin_inv[2][0], lin_inv[2][1], lin_inv[2][2], inv_t[2]],
                [0.0, 0.0, 0.0, 1.0],
            ];
            Ok((m, inv))
        }
    }
}

/// Register image pixel `(col, row)` through `PatientImagePlane` and a verified `SO(3)` / affine
/// anatomical transform, refusing missing or mismatched `FrameOfReferenceUID` without a certificate.
pub fn register_patient_to_anatomy<'a>(
    plane: &PatientImagePlane<'a>,
    source_blake3: [u8; 32],
    col: u32,
    row: u32,
    target_frame_of_reference_uid: &'a str,
    transform: &AnatomyTransform,
    cross_frame_cert: Option<&CrossFrameRegistrationCertificate<'a>>,
    roundtrip_tolerance_mm: f64,
) -> Result<RegisteredAnatomyPoint<'a>, AnatomyRegError> {
    let src_for = plane
        .frame_of_reference_uid()
        .ok_or(AnatomyRegError::MissingFrameOfReference)?;
    if target_frame_of_reference_uid.trim().is_empty() {
        return Err(AnatomyRegError::MissingFrameOfReference);
    }
    if src_for != target_frame_of_reference_uid {
        let cert_ok = cross_frame_cert.is_some_and(|c| {
            c.source_frame_of_reference_uid == src_for
                && c.target_frame_of_reference_uid == target_frame_of_reference_uid
        });
        if !cert_ok {
            return Err(AnatomyRegError::FrameOfReferenceMismatch);
        }
    }

    let (fwd, inv) = validate_and_invert_transform(transform)?;
    let patient_lps_mm = plane.pixel_center_lps_mm(col, row)?;
    let anatomy_coords_mm = apply_4x4(&fwd, patient_lps_mm);
    let roundtrip_lps_mm = apply_4x4(&inv, anatomy_coords_mm);
    if anatomy_coords_mm.iter().chain(roundtrip_lps_mm.iter()).any(|v| !v.is_finite()) {
        return Err(AnatomyRegError::NonFiniteTransform);
    }
    let dx = roundtrip_lps_mm[0] - patient_lps_mm[0];
    let dy = roundtrip_lps_mm[1] - patient_lps_mm[1];
    let dz = roundtrip_lps_mm[2] - patient_lps_mm[2];
    let roundtrip_residual_mm = (dx * dx + dy * dy + dz * dz).sqrt();
    if roundtrip_residual_mm > roundtrip_tolerance_mm.max(1e-9) {
        return Err(AnatomyRegError::RoundtripResidualExceeded);
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&source_blake3);
    hasher.update(src_for.as_bytes());
    hasher.update(target_frame_of_reference_uid.as_bytes());
    if let Some(c) = cross_frame_cert {
        hasher.update(&c.certificate_blake3);
    }
    for v in fwd.iter().flatten().chain(anatomy_coords_mm.iter()) {
        hasher.update(&v.to_le_bytes());
    }

    Ok(RegisteredAnatomyPoint {
        source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        source_frame_of_reference_uid: src_for,
        target_frame_of_reference_uid,
        patient_lps_mm,
        anatomy_coords_mm,
        roundtrip_lps_mm,
        roundtrip_residual_mm,
        inverse_matrix_4x4: inv,
    })
}
