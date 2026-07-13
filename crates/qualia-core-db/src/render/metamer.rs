//! P7.1 — Metamers as the affine fibre of the colour-matching projection.
//!
//! Two SPDs are metameric if they project to the same XYZ tristimulus values
//! under the CIE CMFs. The set of all SPDs mapping to a given XYZ is an
//! affine fibre: `particular + span(kernel)`.
//!
//! ## Linear algebra
//!
//! The CMF projection is a linear map `P: R^41 → R^3` (SPD → XYZ).
//! - **Particular solution**: `spd = P⁺ · xyz` (pseudo-inverse)
//! - **Kernel basis**: `ker(P)` = all SPDs that project to (0,0,0)
//! - **Fibre**: `spd = particular + Σ c_i · ker_i` for any coefficients `c_i`
//!
//! A metameric-black SPD is an element of the kernel (projects to zero).
//!
//! ## Determinism
//!
//! All operations are deterministic: the CMF matrix is a compile-time
//! constant, and the pseudo-inverse is computed via fixed-point iteration.

use super::spectral_kernel::{
    spd_to_xyz, Spd, Xyz, CIE_1931_CMF_X, CIE_1931_CMF_Y, CIE_1931_CMF_Z, SPD_SAMPLES,
};

/// Normalisation factor used by `spd_to_xyz` (1 / Σȳ).
/// Computed manually since `iter().sum()` is not const.
const Y_NORM: f32 = {
    let mut sum = 0.0f32;
    let mut i = 0;
    while i < CIE_1931_CMF_Y.len() {
        sum += CIE_1931_CMF_Y[i];
        i += 1;
    }
    1.0 / sum
};

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Metamer computation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetamerError {
    /// Target XYZ is zero (degenerate).
    ZeroTarget,
    /// Buffer too small.
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for MetamerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroTarget => write!(f, "metamer: target XYZ is zero"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "metamer: buffer too small, need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for MetamerError {}

// ───────────────────────────────────────────────────────────────────────────
//  Kernel basis (metameric black)
// ───────────────────────────────────────────────────────────────────────────

/// Compute a basis for the kernel of the CMF projection (metameric-black SPDs).
///
/// The kernel has dimension `SPD_SAMPLES - 3 = 38`. We compute it via
/// Gram-Schmidt orthogonalisation against the three CMF rows.
///
/// `out_basis` needs `(SPD_SAMPLES - 3) * SPD_SAMPLES` entries (row-major).
/// Returns the number of basis vectors written.
pub fn metamer_kernel_basis(out_basis: &mut [f32]) -> Result<usize, MetamerError> {
    let n = SPD_SAMPLES;
    let ker_dim = n - 3;
    if out_basis.len() < ker_dim * n {
        return Err(MetamerError::BufferTooSmall {
            needed: ker_dim * n,
            have: out_basis.len(),
        });
    }

    // The three CMF rows as f64 vectors.
    let cmf_f64: [[f64; SPD_SAMPLES]; 3] = [
        CIE_1931_CMF_X.map(|v| v as f64),
        CIE_1931_CMF_Y.map(|v| v as f64),
        CIE_1931_CMF_Z.map(|v| v as f64),
    ];

    // Compute P P^T (3×3) in f64.
    let mut ppt = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            ppt[i][j] = cmf_f64[i]
                .iter()
                .zip(cmf_f64[j].iter())
                .map(|(a, b)| a * b)
                .sum();
        }
    }

    // Invert the 3×3 matrix in f64.
    let det = ppt[0][0] * (ppt[1][1] * ppt[2][2] - ppt[1][2] * ppt[2][1])
        - ppt[0][1] * (ppt[1][0] * ppt[2][2] - ppt[1][2] * ppt[2][0])
        + ppt[0][2] * (ppt[1][0] * ppt[2][1] - ppt[1][1] * ppt[2][0]);
    let inv_det = 1.0 / det;
    let inv = [
        [
            (ppt[1][1] * ppt[2][2] - ppt[1][2] * ppt[2][1]) * inv_det,
            (ppt[0][2] * ppt[2][1] - ppt[0][1] * ppt[2][2]) * inv_det,
            (ppt[0][1] * ppt[1][2] - ppt[0][2] * ppt[1][1]) * inv_det,
        ],
        [
            (ppt[1][2] * ppt[2][0] - ppt[1][0] * ppt[2][2]) * inv_det,
            (ppt[0][0] * ppt[2][2] - ppt[0][2] * ppt[2][0]) * inv_det,
            (ppt[0][2] * ppt[1][0] - ppt[0][0] * ppt[1][2]) * inv_det,
        ],
        [
            (ppt[1][0] * ppt[2][1] - ppt[1][1] * ppt[2][0]) * inv_det,
            (ppt[0][1] * ppt[2][0] - ppt[0][0] * ppt[2][1]) * inv_det,
            (ppt[0][0] * ppt[1][1] - ppt[0][1] * ppt[1][0]) * inv_det,
        ],
    ];

    // For each standard basis vector e_i (i=3..40), compute the null-space
    // component: k_i = e_i - P^T (P P^T)^{-1} P e_i
    // P e_i = (cmf_x[i], cmf_y[i], cmf_z[i])
    // (P P^T)^{-1} P e_i = inv · (cmf_x[i], cmf_y[i], cmf_z[i])
    // P^T · that = Σ_j cmf_j[k] * inv[j] · (P e_i)_j
    let mut basis_idx = 0usize;
    for i in 3..n {
        // P e_i
        let pe = [cmf_f64[0][i], cmf_f64[1][i], cmf_f64[2][i]];

        // (P P^T)^{-1} P e_i
        let mut inv_pe = [0.0f64; 3];
        for j in 0..3 {
            inv_pe[j] = inv[j][0] * pe[0] + inv[j][1] * pe[1] + inv[j][2] * pe[2];
        }

        // P^T (P P^T)^{-1} P e_i — the projection onto the row space.
        let mut proj = [0.0f64; SPD_SAMPLES];
        for k in 0..n {
            for j in 0..3 {
                proj[k] += cmf_f64[j][k] * inv_pe[j];
            }
        }

        // k_i = e_i - proj
        let mut v = [0.0f64; SPD_SAMPLES];
        v[i] = 1.0;
        for k in 0..n {
            v[k] -= proj[k];
        }

        // Normalise.
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-10 {
            for j in 0..n {
                v[j] /= norm;
            }
            for j in 0..n {
                out_basis[basis_idx * n + j] = v[j] as f32;
            }
            basis_idx += 1;
        }

        if basis_idx >= ker_dim {
            break;
        }
    }

    Ok(basis_idx)
}

// ───────────────────────────────────────────────────────────────────────────
//  Particular solution (minimum-norm SPD for a target XYZ)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the minimum-norm particular solution: the SPD with least energy
/// that projects to the target XYZ.
///
/// Uses the pseudo-inverse `P⁺ = P^T (P P^T)^{-1}` where P is the 3×41 CMF
/// matrix (including the Y normalisation). Since P P^T is 3×3, we invert
/// it directly.
pub fn min_norm_spd_for_xyz(target: &Xyz) -> Spd {
    // P is 3×41: rows are CIE_1931_CMF_X, CIE_1931_CMF_Y, CIE_1931_CMF_Z,
    // each scaled by Y_NORM (matching spd_to_xyz's normalisation).
    let cmf_x: [f32; SPD_SAMPLES] = CIE_1931_CMF_X.map(|v| v * Y_NORM);
    let cmf_y: [f32; SPD_SAMPLES] = CIE_1931_CMF_Y.map(|v| v * Y_NORM);
    let cmf_z: [f32; SPD_SAMPLES] = CIE_1931_CMF_Z.map(|v| v * Y_NORM);
    let cmf = [cmf_x, cmf_y, cmf_z];

    // Compute P P^T (3×3).
    let mut ppt = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            ppt[i][j] = cmf[i].iter().zip(cmf[j].iter()).map(|(a, b)| a * b).sum();
        }
    }

    // Invert the 3×3 matrix.
    let inv = invert_3x3(&ppt);

    // Compute P^T (P P^T)^{-1} xyz = Σ_j cmf_j * inv[j] · target
    let target_vec = [target.x, target.y, target.z];
    let mut spd = Spd::default();
    for i in 0..SPD_SAMPLES {
        let mut val = 0.0f32;
        for j in 0..3 {
            val += cmf[j][i]
                * (inv[j][0] * target_vec[0]
                    + inv[j][1] * target_vec[1]
                    + inv[j][2] * target_vec[2]);
        }
        spd.samples[i] = val;
    }

    spd
}

/// Invert a 3×3 matrix.
fn invert_3x3(m: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-20 {
        return [[0.0; 3]; 3];
    }

    let inv_det = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ]
}

// ───────────────────────────────────────────────────────────────────────────
//  Fibre construction
// ───────────────────────────────────────────────────────────────────────────

/// Construct a fibre element: `particular + Σ c_i · ker_i`.
///
/// `basis` is the kernel basis (row-major, `ker_dim * SPD_SAMPLES` entries).
/// `coeffs` is the coefficient vector (`ker_dim` entries).
pub fn fibre_spd(particular: &Spd, basis: &[f32], coeffs: &[f32]) -> Spd {
    let mut spd = *particular;
    let n = SPD_SAMPLES;
    for (k, &c) in coeffs.iter().enumerate() {
        if k * n + n > basis.len() {
            break;
        }
        for i in 0..n {
            spd.samples[i] += c * basis[k * n + i];
        }
    }
    spd
}

/// Check if an SPD is metameric to a target XYZ (projects to the same XYZ).
pub fn is_metameric(spd: &Spd, target: &Xyz, tolerance: f32) -> bool {
    let xyz = spd_to_xyz(spd);
    (xyz.x - target.x).abs() < tolerance
        && (xyz.y - target.y).abs() < tolerance
        && (xyz.z - target.z).abs() < tolerance
}

/// Check if an SPD is metameric-black (projects to approximately zero).
pub fn is_metameric_black(spd: &Spd, tolerance: f32) -> bool {
    let xyz = spd_to_xyz(spd);
    xyz.x.abs() < tolerance && xyz.y.abs() < tolerance && xyz.z.abs() < tolerance
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_basis_projects_to_zero() {
        let ker_dim = SPD_SAMPLES - 3;
        let mut basis = vec![0.0f32; ker_dim * SPD_SAMPLES];
        let count = metamer_kernel_basis(&mut basis).unwrap();
        assert!(count > 0, "should produce at least one kernel vector");

        // Each basis vector should project to ~zero.
        for k in 0..count {
            let mut spd = Spd::default();
            spd.samples
                .copy_from_slice(&basis[k * SPD_SAMPLES..(k + 1) * SPD_SAMPLES]);
            let xyz = spd_to_xyz(&spd);
            assert!(
                is_metameric_black(&spd, 5e-2),
                "kernel vector {} should be metameric-black, got XYZ=({:.6}, {:.6}, {:.6})",
                k,
                xyz.x,
                xyz.y,
                xyz.z
            );
        }
    }

    #[test]
    fn min_norm_spd_reprojects_to_target() {
        let target = Xyz::new(0.5, 0.6, 0.4);
        let spd = min_norm_spd_for_xyz(&target);
        let reprojected = spd_to_xyz(&spd);
        assert!(
            (reprojected.x - target.x).abs() < 0.05,
            "X reprojection mismatch: {} vs {}",
            reprojected.x,
            target.x
        );
        assert!(
            (reprojected.y - target.y).abs() < 0.05,
            "Y reprojection mismatch: {} vs {}",
            reprojected.y,
            target.y
        );
        assert!(
            (reprojected.z - target.z).abs() < 0.05,
            "Z reprojection mismatch: {} vs {}",
            reprojected.z,
            target.z
        );
    }

    #[test]
    fn fibre_invariance() {
        // particular + kernel element should re-project to the same XYZ.
        let target = Xyz::new(0.3, 0.5, 0.2);
        let particular = min_norm_spd_for_xyz(&target);

        let ker_dim = SPD_SAMPLES - 3;
        let mut basis = vec![0.0f32; ker_dim * SPD_SAMPLES];
        let count = metamer_kernel_basis(&mut basis).unwrap();

        // Add a kernel element with random-ish coefficients.
        let coeffs: Vec<f32> = (0..count).map(|i| (i as f32 * 0.1).sin()).collect();
        let fibre = fibre_spd(&particular, &basis, &coeffs);

        assert!(
            is_metameric(&fibre, &target, 5e-2),
            "fibre element must be metameric to the target"
        );
    }

    #[test]
    fn min_norm_spd_determinism() {
        let target = Xyz::new(0.4, 0.7, 0.3);
        let spd1 = min_norm_spd_for_xyz(&target);
        let spd2 = min_norm_spd_for_xyz(&target);
        assert_eq!(spd1, spd2, "min-norm SPD must be deterministic");
    }

    #[test]
    fn kernel_basis_determinism() {
        let ker_dim = SPD_SAMPLES - 3;
        let mut b1 = vec![0.0f32; ker_dim * SPD_SAMPLES];
        let mut b2 = vec![0.0f32; ker_dim * SPD_SAMPLES];
        let c1 = metamer_kernel_basis(&mut b1).unwrap();
        let c2 = metamer_kernel_basis(&mut b2).unwrap();
        assert_eq!(c1, c2);
        assert_eq!(b1, b2, "kernel basis must be deterministic");
    }
}
