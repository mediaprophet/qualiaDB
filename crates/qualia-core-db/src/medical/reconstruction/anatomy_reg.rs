//! Source-bound patient-space (DICOM LPS mm) to anatomical/target coordinate frame
//! registration with `FrameOfReferenceUID` verification, `SO(3)` / affine invertibility,
//! roundtrip consistency enforcement, and distinct BLAKE3 derivation stamps.
use crate::medical::dicom_geometry::{GeometryError, PatientImagePlane, ORTHONORMAL_TOLERANCE};
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};

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
    EmptyInput,
    LengthMismatch,
    InvalidWeight,
    NumericalSolverFailed,
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

/// Canonical cortical bone elastic modulus reference (GPa) at `rho_app = 1.85 g/cm^3`.
pub const REF_CORTICAL_MODULUS_GPA: f64 = 16.5;

/// Compute cortical/trabecular bone mineral stiffness `E(rho_app)` in GPa via
/// Carter-Hayes / Morgan-Keaveny power-law scaling for NRICP regularization weights.
pub fn cortical_bmd_stiffness_gpa(apparent_density_g_cm3: f64, mineralization_mu: f64) -> f64 {
    if !apparent_density_g_cm3.is_finite()
        || !mineralization_mu.is_finite()
        || apparent_density_g_cm3 <= 0.0
        || mineralization_mu <= 0.0
    {
        return 0.0;
    }
    let r = (apparent_density_g_cm3 / 1.85).clamp(0.10, 1.40);
    REF_CORTICAL_MODULUS_GPA * mineralization_mu.clamp(0.05, 1.5) * r.powf(2.2)
}

/// Configuration for stiffness-penalized locally affine Non-Rigid ICP (Amberg et al. 2007)
/// weighted by per-vertex cortical BMD stiffness `E(rho_app)` and observation confidence `w_k = c_obs(t)`.
#[derive(Clone, Debug, PartialEq)]
pub struct StiffnessPenalizedNricpConfig {
    /// Decreasing stiffness schedule across outer continuation stages (`alpha > 0`).
    pub stiffness_schedule: Vec<f64>,
    /// Weight on explicit landmark correspondences (`beta >= 0`).
    pub landmark_weight: f64,
    /// Relative stiffness weight on the translational column vs linear 3x3 part (`gamma > 0`).
    pub translation_gamma: f64,
    /// Maximum ICP correspondence-update iterations per stiffness stage.
    pub max_inner_iterations: usize,
    /// Frobenius step convergence threshold.
    pub convergence_tol: f64,
    /// Optional correspondence distance cutoff in mm (outliers beyond this receive weight 0).
    pub max_correspondence_dist_mm: Option<f64>,
    /// Minimum allowed local Jacobian determinant `det(A_i) > 0` preventing surface foldover.
    pub min_jacobian_det: f64,
    /// Weak prior anchor keeping unobserved disconnected vertices well-posed (`>= 0`).
    pub prior_anchor_weight: f64,
}

impl Default for StiffnessPenalizedNricpConfig {
    fn default() -> Self {
        Self {
            stiffness_schedule: vec![40.0, 15.0, 5.0, 1.5],
            landmark_weight: 10.0,
            translation_gamma: 1.0,
            max_inner_iterations: 12,
            convergence_tol: 1e-4,
            max_correspondence_dist_mm: None,
            min_jacobian_det: 1e-3,
            prior_anchor_weight: 1e-6,
        }
    }
}

/// Output of [`register_stiffness_penalized_surface`] with per-vertex affine fields and residuals.
#[derive(Clone, Debug)]
pub struct NricpRegistrationResult {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub deformed_vertices: Vec<[f64; 3]>,
    /// Per-vertex locally affine `3x4` matrix `[A_i | t_i]`.
    pub per_vertex_affine_3x4: Vec<[[f64; 4]; 3]>,
    /// Per-vertex Jacobian determinant `det(A_i) > 0`.
    pub per_vertex_jacobian_det: Vec<f64>,
    pub mean_surface_residual_mm: f64,
    pub max_surface_residual_mm: f64,
    pub iterations_run: usize,
    pub converged: bool,
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn apply_3x4(x: &[[f64; 4]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        x[0][0] * v[0] + x[0][1] * v[1] + x[0][2] * v[2] + x[0][3],
        x[1][0] * v[0] + x[1][1] * v[1] + x[1][2] * v[2] + x[1][3],
        x[2][0] * v[0] + x[2][1] * v[1] + x[2][2] * v[2] + x[2][3],
    ]
}

fn build_fallback_knn_edges(vertices: &[[f64; 3]], k: usize) -> Vec<(usize, usize)> {
    let n = vertices.len();
    if n <= 1 || k == 0 {
        return Vec::new();
    }
    let mut edges = Vec::new();
    for i in 0..n {
        let mut nbrs: Vec<(f64, usize)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (dist2(vertices[i], vertices[j]), j))
            .collect();
        nbrs.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for &(_, j) in nbrs.iter().take(k) {
            let pair = if i < j { (i, j) } else { (j, i) };
            edges.push(pair);
        }
    }
    edges.sort_unstable();
    edges.dedup();
    edges
}

/// Enforce `det(A) >= min_det > 0` by blending toward `I_3` if a step folds a local patch.
fn enforce_positive_det_3x4(x: &mut [[f64; 4]; 3], min_det: f64) -> f64 {
    let a = [
        [x[0][0], x[0][1], x[0][2]],
        [x[1][0], x[1][1], x[1][2]],
        [x[2][0], x[2][1], x[2][2]],
    ];
    let det = mat3_det(&a);
    if det.is_finite() && det >= min_det {
        return det;
    }
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        let blended = [
            [
                (1.0 - mid) + mid * a[0][0],
                mid * a[0][1],
                mid * a[0][2],
            ],
            [
                mid * a[1][0],
                (1.0 - mid) + mid * a[1][1],
                mid * a[1][2],
            ],
            [
                mid * a[2][0],
                mid * a[2][1],
                (1.0 - mid) + mid * a[2][2],
            ],
        ];
        if mat3_det(&blended) >= min_det {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    for r in 0..3 {
        for c in 0..3 {
            let id = if r == c { 1.0 } else { 0.0 };
            x[r][c] = (1.0 - lo) * id + lo * a[r][c];
        }
    }
    let safe_a = [
        [x[0][0], x[0][1], x[0][2]],
        [x[1][0], x[1][1], x[1][2]],
        [x[2][0], x[2][1], x[2][2]],
    ];
    mat3_det(&safe_a).max(min_det)
}

/// Solve the `4n x 4n` SPD system `H x = b` via 4x4 block-Jacobi Preconditioned Conjugate Gradient.
fn solve_nricp_pcg(
    n: usize,
    diag_blocks: &[[f64; 16]],
    adj: &[Vec<(usize, f64)>],
    gamma2: f64,
    rhs: &[f64],
    x: &mut [f64],
) -> Result<(), AnatomyRegError> {
    let dim = 4 * n;
    let mut diag_chol = vec![[0.0_f64; 16]; n];
    for i in 0..n {
        if cholesky_factor(4, &diag_blocks[i], &mut diag_chol[i]).is_err() {
            return Err(AnatomyRegError::NumericalSolverFailed);
        }
    }

    let matvec = |v: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let b = &diag_blocks[i];
            let vi = &v[4 * i..4 * i + 4];
            let oi = &mut out[4 * i..4 * i + 4];
            for r in 0..4 {
                oi[r] = b[r * 4] * vi[0]
                    + b[r * 4 + 1] * vi[1]
                    + b[r * 4 + 2] * vi[2]
                    + b[r * 4 + 3] * vi[3];
            }
            for &(j, k_ij) in &adj[i] {
                let vj = &v[4 * j..4 * j + 4];
                oi[0] -= k_ij * vj[0];
                oi[1] -= k_ij * vj[1];
                oi[2] -= k_ij * vj[2];
                oi[3] -= k_ij * gamma2 * vj[3];
            }
        }
    };

    let apply_precond = |r: &[f64], z: &mut [f64]| -> Result<(), AnatomyRegError> {
        for i in 0..n {
            cholesky_solve(4, &diag_chol[i], &r[4 * i..4 * i + 4], &mut z[4 * i..4 * i + 4])
                .map_err(|_| AnatomyRegError::NumericalSolverFailed)?;
        }
        Ok(())
    };

    let mut ax = vec![0.0_f64; dim];
    matvec(x, &mut ax);
    let mut r: Vec<f64> = rhs.iter().zip(ax.iter()).map(|(b, a)| b - a).collect();
    let mut z = vec![0.0_f64; dim];
    apply_precond(&r, &mut z)?;
    let mut p = z.clone();
    let mut rz_old: f64 = r.iter().zip(z.iter()).map(|(ri, zi)| ri * zi).sum();
    let mut ap = vec![0.0_f64; dim];

    let max_cg = (dim * 2).max(64);
    for _ in 0..max_cg {
        if rz_old.abs() < 1e-22 {
            break;
        }
        matvec(&p, &mut ap);
        let p_ap: f64 = p.iter().zip(ap.iter()).map(|(pi, api)| pi * api).sum();
        if !p_ap.is_finite() || p_ap <= 1e-24 {
            break;
        }
        let alpha = rz_old / p_ap;
        let mut r_norm2 = 0.0_f64;
        for idx in 0..dim {
            x[idx] += alpha * p[idx];
            r[idx] -= alpha * ap[idx];
            r_norm2 += r[idx] * r[idx];
        }
        if r_norm2.sqrt() < 1e-10 {
            break;
        }
        apply_precond(&r, &mut z)?;
        let rz_new: f64 = r.iter().zip(z.iter()).map(|(ri, zi)| ri * zi).sum();
        if !rz_new.is_finite() || rz_old.abs() < 1e-30 {
            break;
        }
        let beta = rz_new / rz_old;
        for idx in 0..dim {
            p[idx] = z[idx] + beta * p[idx];
        }
        rz_old = rz_new;
    }
    Ok(())
}

/// Stiffness-penalized locally affine Non-Rigid ICP surface registration incorporating
/// cortical BMD stiffness weights `E(rho_app)` and temporal observation confidence weights `c_obs(t)`.
pub fn register_stiffness_penalized_surface(
    source_vertices: &[[f64; 3]],
    source_edges: &[(usize, usize)],
    target_vertices: &[[f64; 3]],
    cortical_bmd_stiffness_gpa: &[f64],
    observation_confidence_weights: &[f64],
    landmarks: &[(usize, [f64; 3], f64)],
    source_blake3: [u8; 32],
    config: &StiffnessPenalizedNricpConfig,
) -> Result<NricpRegistrationResult, AnatomyRegError> {
    let n = source_vertices.len();
    if n == 0 || target_vertices.is_empty() {
        return Err(AnatomyRegError::EmptyInput);
    }
    if cortical_bmd_stiffness_gpa.len() != n || observation_confidence_weights.len() != n {
        return Err(AnatomyRegError::LengthMismatch);
    }
    if source_vertices
        .iter()
        .chain(target_vertices.iter())
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err(AnatomyRegError::NonFiniteTransform);
    }
    if cortical_bmd_stiffness_gpa
        .iter()
        .any(|&s| !s.is_finite() || s <= 0.0)
        || observation_confidence_weights
            .iter()
            .any(|&w| !w.is_finite() || w < 0.0 || w > 1.0)
    {
        return Err(AnatomyRegError::InvalidWeight);
    }
    if config.stiffness_schedule.is_empty()
        || config.stiffness_schedule.iter().any(|&a| !a.is_finite() || a <= 0.0)
        || !config.landmark_weight.is_finite()
        || config.landmark_weight < 0.0
        || !config.translation_gamma.is_finite()
        || config.translation_gamma <= 0.0
        || !config.min_jacobian_det.is_finite()
        || config.min_jacobian_det <= 0.0
        || !config.prior_anchor_weight.is_finite()
        || config.prior_anchor_weight < 0.0
    {
        return Err(AnatomyRegError::InvalidWeight);
    }
    for &(i, j) in source_edges {
        if i >= n || j >= n {
            return Err(AnatomyRegError::LengthMismatch);
        }
    }
    for &(idx, pt, w) in landmarks {
        if idx >= n {
            return Err(AnatomyRegError::LengthMismatch);
        }
        if pt.iter().any(|v| !v.is_finite()) {
            return Err(AnatomyRegError::NonFiniteTransform);
        }
        if !w.is_finite() || w < 0.0 {
            return Err(AnatomyRegError::InvalidWeight);
        }
    }

    let owned_edges;
    let edges: &[(usize, usize)] = if source_edges.is_empty() && n > 1 {
        owned_edges = build_fallback_knn_edges(source_vertices, 4.min(n - 1));
        &owned_edges
    } else {
        source_edges
    };

    // Normalize edge stiffness relative to canonical cortical reference (16.5 GPa)
    let mut base_adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for &(i, j) in edges {
        if i == j {
            continue;
        }
        let s_ij = 0.5 * (cortical_bmd_stiffness_gpa[i] + cortical_bmd_stiffness_gpa[j])
            / REF_CORTICAL_MODULUS_GPA;
        base_adj[i].push((j, s_ij));
        base_adj[j].push((i, s_ij));
    }

    let gamma2 = config.translation_gamma * config.translation_gamma;
    let prior_eps = config.prior_anchor_weight.max(1e-8);
    let max_dist2 = config.max_correspondence_dist_mm.map(|d| d * d);

    let mut x_per_vert: Vec<[[f64; 4]; 3]> = vec![
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        n
    ];
    let mut deformed: Vec<[f64; 3]> = source_vertices.to_vec();
    let mut total_iters = 0usize;
    let mut last_stage_converged = false;

    for &alpha in &config.stiffness_schedule {
        let mut adj_alpha: Vec<Vec<(usize, f64)>> = Vec::with_capacity(n);
        for nbrs in &base_adj {
            adj_alpha.push(nbrs.iter().map(|&(j, s)| (j, alpha * s)).collect());
        }

        last_stage_converged = false;
        for _ in 0..config.max_inner_iterations.max(1) {
            total_iters += 1;
            let mut diag_blocks = vec![[0.0_f64; 16]; n];
            let mut rhs_axes = [vec![0.0_f64; 4 * n], vec![0.0_f64; 4 * n], vec![0.0_f64; 4 * n]];

            // 1. Stiffness Laplacian + prior anchor
            for i in 0..n {
                let mut deg = 0.0_f64;
                for &(_, k_ij) in &adj_alpha[i] {
                    deg += k_ij;
                }
                let blk = &mut diag_blocks[i];
                let stiff_diag = deg + prior_eps;
                blk[0] += stiff_diag;
                blk[5] += stiff_diag;
                blk[10] += stiff_diag;
                blk[15] += deg * gamma2 + prior_eps;

                // Prior anchor pulls lightly toward identity [I | 0]
                rhs_axes[0][4 * i] += prior_eps;
                rhs_axes[1][4 * i + 1] += prior_eps;
                rhs_axes[2][4 * i + 2] += prior_eps;

                // 2. Nearest-neighbor data term weighted by observation confidence w_k = c_obs(t)
                let w_conf = observation_confidence_weights[i];
                if w_conf > 0.0 {
                    let cur = deformed[i];
                    let mut best_d2 = f64::INFINITY;
                    let mut best_tgt = target_vertices[0];
                    for &t in target_vertices {
                        let d2 = dist2(cur, t);
                        if d2 < best_d2 {
                            best_d2 = d2;
                            best_tgt = t;
                        }
                    }
                    let accept = max_dist2.is_none_or(|lim| best_d2 <= lim);
                    if accept {
                        let v = source_vertices[i];
                        let vh = [v[0], v[1], v[2], 1.0];
                        for r in 0..4 {
                            for c in 0..4 {
                                blk[r * 4 + c] += w_conf * vh[r] * vh[c];
                            }
                        }
                        for axis in 0..3 {
                            for r in 0..4 {
                                rhs_axes[axis][4 * i + r] += w_conf * best_tgt[axis] * vh[r];
                            }
                        }
                    }
                }
            }

            // 3. Landmark term
            if config.landmark_weight > 0.0 {
                for &(idx, tgt, l_conf) in landmarks {
                    let w = config.landmark_weight * l_conf;
                    if w <= 0.0 {
                        continue;
                    }
                    let v = source_vertices[idx];
                    let vh = [v[0], v[1], v[2], 1.0];
                    let blk = &mut diag_blocks[idx];
                    for r in 0..4 {
                        for c in 0..4 {
                            blk[r * 4 + c] += w * vh[r] * vh[c];
                        }
                    }
                    for axis in 0..3 {
                        for r in 0..4 {
                            rhs_axes[axis][4 * idx + r] += w * tgt[axis] * vh[r];
                        }
                    }
                }
            }

            // 4. Solve decoupled 4n x 4n SPD system for x, y, z rows
            let mut max_step = 0.0_f64;
            let mut next_x = x_per_vert.clone();
            for axis in 0..3 {
                let mut x_vec = vec![0.0_f64; 4 * n];
                for i in 0..n {
                    x_vec[4 * i..4 * i + 4].copy_from_slice(&x_per_vert[i][axis]);
                }
                solve_nricp_pcg(
                    n,
                    &diag_blocks,
                    &adj_alpha,
                    gamma2,
                    &rhs_axes[axis],
                    &mut x_vec,
                )?;
                for i in 0..n {
                    next_x[i][axis].copy_from_slice(&x_vec[4 * i..4 * i + 4]);
                }
            }

            for i in 0..n {
                enforce_positive_det_3x4(&mut next_x[i], config.min_jacobian_det);
                let mut diff2 = 0.0_f64;
                for r in 0..3 {
                    for c in 0..4 {
                        let d = next_x[i][r][c] - x_per_vert[i][r][c];
                        diff2 += d * d;
                    }
                }
                max_step = max_step.max(diff2.sqrt());
                deformed[i] = apply_3x4(&next_x[i], source_vertices[i]);
            }
            x_per_vert = next_x;

            if max_step < config.convergence_tol {
                last_stage_converged = true;
                break;
            }
        }
    }

    let mut per_vertex_jacobian_det = Vec::with_capacity(n);
    let mut sum_res = 0.0_f64;
    let mut max_res = 0.0_f64;
    for i in 0..n {
        let x = &x_per_vert[i];
        let a = [
            [x[0][0], x[0][1], x[0][2]],
            [x[1][0], x[1][1], x[1][2]],
            [x[2][0], x[2][1], x[2][2]],
        ];
        per_vertex_jacobian_det.push(mat3_det(&a));

        let cur = deformed[i];
        let mut best_d2 = f64::INFINITY;
        for &t in target_vertices {
            let d2 = dist2(cur, t);
            if d2 < best_d2 {
                best_d2 = d2;
            }
        }
        let d = best_d2.sqrt();
        sum_res += d;
        max_res = max_res.max(d);
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&source_blake3);
    for p in deformed.iter().flatten() {
        hasher.update(&p.to_le_bytes());
    }

    Ok(NricpRegistrationResult {
        source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        deformed_vertices: deformed,
        per_vertex_affine_3x4: x_per_vert,
        per_vertex_jacobian_det,
        mean_surface_residual_mm: sum_res / n as f64,
        max_surface_residual_mm: max_res,
        iterations_run: total_iters,
        converged: last_stage_converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nricp_recovers_non_uniform_surface_warp_with_positive_jacobian() {
        let src = vec![
            [-10.0, -10.0, -10.0],
            [10.0, -10.0, -10.0],
            [10.0, 10.0, -10.0],
            [-10.0, 10.0, -10.0],
            [-10.0, -10.0, 10.0],
            [10.0, -10.0, 10.0],
            [10.0, 10.0, 10.0],
            [-10.0, 10.0, 10.0],
        ];
        let edges = vec![
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];
        let tgt: Vec<[f64; 3]> = src
            .iter()
            .map(|p| [p[0] * 1.15 + 2.0, p[1] * 0.92 - 1.0, p[2] * 1.08 + 0.5])
            .collect();
        let bmd = vec![16.5; src.len()];
        let conf = vec![1.0; src.len()];
        let landmarks = vec![(0, tgt[0], 1.0), (6, tgt[6], 1.0)];

        let res = register_stiffness_penalized_surface(
            &src,
            &edges,
            &tgt,
            &bmd,
            &conf,
            &landmarks,
            [0xAB; 32],
            &StiffnessPenalizedNricpConfig::default(),
        )
        .unwrap();

        assert!(res.mean_surface_residual_mm < 0.25, "mean residual {}", res.mean_surface_residual_mm);
        assert!(res.per_vertex_jacobian_det.iter().all(|&d| d > 0.0));
        assert_ne!(res.derivation_blake3, [0xAB; 32]);
    }

    #[test]
    fn nricp_high_cortical_bmd_stiffness_resists_local_perturbation_more_than_low_bmd() {
        let src = vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [20.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [10.0, 10.0, 0.0],
            [20.0, 10.0, 0.0],
        ];
        let edges = vec![(0, 1), (1, 2), (3, 4), (4, 5), (0, 3), (1, 4), (2, 5)];
        // Pull vertex 2 & 5 outward while vertex 0 & 3 are anchored
        let mut tgt = src.clone();
        tgt[2][0] += 6.0;
        tgt[5][0] += 6.0;
        let conf = vec![0.35; src.len()];
        let landmarks = vec![(0, src[0], 1.0), (3, src[3], 1.0)];
        let cfg = StiffnessPenalizedNricpConfig {
            stiffness_schedule: vec![12.0],
            max_inner_iterations: 6,
            ..Default::default()
        };

        let stiff_bmd = vec![cortical_bmd_stiffness_gpa(1.90, 1.0); src.len()];
        let osteo_bmd = vec![cortical_bmd_stiffness_gpa(0.75, 1.0); src.len()];

        let res_stiff = register_stiffness_penalized_surface(
            &src, &edges, &tgt, &stiff_bmd, &conf, &landmarks, [0x11; 32], &cfg,
        ).unwrap();
        let res_osteo = register_stiffness_penalized_surface(
            &src, &edges, &tgt, &osteo_bmd, &conf, &landmarks, [0x11; 32], &cfg,
        ).unwrap();

        let disp_stiff = res_stiff.deformed_vertices[2][0] - src[2][0];
        let disp_osteo = res_osteo.deformed_vertices[2][0] - src[2][0];
        assert!(
            disp_stiff < disp_osteo,
            "stiffer cortical BMD ({disp_stiff}) should resist distortion more than osteopenic ({disp_osteo})"
        );
        assert!(res_stiff.per_vertex_jacobian_det.iter().all(|&d| d > 0.0));
        assert!(res_osteo.per_vertex_jacobian_det.iter().all(|&d| d > 0.0));
    }

    #[test]
    fn nricp_rejects_invalid_inputs_and_preserves_positive_det_on_extreme_fold() {
        let src = vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 5.0]];
        let tgt_flip = vec![[0.0, 0.0, 0.0], [-5.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 5.0]];
        let bmd = vec![5.0; 4];
        let conf = vec![1.0; 4];

        // Fail-closed validation checks
        assert_eq!(
            register_stiffness_penalized_surface(
                &[], &[], &tgt_flip, &[], &[], &[], [0; 32], &StiffnessPenalizedNricpConfig::default()
            ).unwrap_err(),
            AnatomyRegError::EmptyInput
        );
        assert_eq!(
            register_stiffness_penalized_surface(
                &src, &[], &tgt_flip, &[1.0], &conf, &[], [0; 32], &StiffnessPenalizedNricpConfig::default()
            ).unwrap_err(),
            AnatomyRegError::LengthMismatch
        );
        assert_eq!(
            register_stiffness_penalized_surface(
                &src, &[], &tgt_flip, &[-1.0, 1.0, 1.0, 1.0], &conf, &[], [0; 32], &StiffnessPenalizedNricpConfig::default()
            ).unwrap_err(),
            AnatomyRegError::InvalidWeight
        );

        // Extreme inverted target still yields det(A_i) >= min_jacobian_det > 0
        let res = register_stiffness_penalized_surface(
            &src, &[], &tgt_flip, &bmd, &conf, &[], [0x22; 32], &StiffnessPenalizedNricpConfig::default()
        ).unwrap();
        assert!(res.per_vertex_jacobian_det.iter().all(|&d| d >= 1e-3 - 1e-12));
    }
}
