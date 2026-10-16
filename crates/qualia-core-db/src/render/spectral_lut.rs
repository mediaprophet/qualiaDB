//! Deterministic Zero-Heap Spectral Inversion LUT & Levenberg-Marquardt Solver in Pure Rust.
//!
//! Provides the mathematical solver and runtime lookup engine to map linear sRGB triplets
//! $(R, G, B)$ to the Jakob & Hanika (2019) bounded spectral coefficients $(c_0, c_1, c_2)$.
//!
//! # 1. Offline Rust Levenberg-Marquardt Optimizer
//! Solves the non-linear least squares problem:
//! $$\min_{\mathbf{c}} \frac{1}{2} \|\mathbf{C}(\mathbf{c}) - \mathbf{C}_{\text{target}}\|^2 + \gamma \cdot 4 c_0^2 + \epsilon \|\mathbf{c}\|^2$$
//! using an analytical $3 \times 3$ Jacobian and exact $3 \times 3$ linear solver in zero heap.
//!
//! # 2. Runtime $O(1)$ Zero-Heap LUT Sampling
//! Takes a 192 KiB ($32 \times 32 \times 32 \times 3 \times 2\text{ bytes}$) static byte slice
//! of IEEE 754 `f16` values and performs trilinear or tetrahedral interpolation.

use crate::modalities::evidential_etau::{f16_to_f32, f32_to_f16};
use crate::render::spectral_basis::JakobHanikaCoeffs;
use crate::render::spectral_kernel::{
    CIE_1931_CMF_X, CIE_1931_CMF_Y, CIE_1931_CMF_Z, LAMBDA_MIN, LAMBDA_STEP, SPD_SAMPLES,
};

pub const LUT_RES: usize = 32;
pub const LUT_TOTAL_VOXELS: usize = LUT_RES * LUT_RES * LUT_RES;
pub const LUT_BYTES_PER_VOXEL: usize = 6; // 3 x f16 (2 bytes each)
pub const LUT_TOTAL_BYTE_SIZE: usize = LUT_TOTAL_VOXELS * LUT_BYTES_PER_VOXEL; // 196,608 bytes (192 KiB)

/// Standard sRGB to XYZ (D65) transformation matrix.
pub const M_XYZ_TO_SRGB: [[f32; 3]; 3] = [
    [3.240_454_2, -1.537_138_5, -0.498_531_4],
    [-0.969_266_0, 1.876_010_8, 0.041_556_0],
    [0.055_643_4, -0.204_025_9, 1.057_225_2],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralLutError {
    BufferTooSmall { need: usize, have: usize },
    SolverDivergence,
}

impl core::fmt::Display for SpectralLutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall { need, have } => {
                write!(f, "spectral LUT buffer too small: need {need}, have {have}")
            }
            Self::SolverDivergence => write!(f, "spectral Levenberg-Marquardt solver diverged"),
        }
    }
}

impl std::error::Error for SpectralLutError {}

// ───────────────────────────────────────────────────────────────────────────
//  Forward Spectral Integration & Analytical Jacobian
// ───────────────────────────────────────────────────────────────────────────

/// Forward projection: evaluates sRGB from polynomial coefficients $(c_0, c_1, c_2)$.
#[inline]
pub fn project_coeffs_to_srgb(c: [f32; 3]) -> ([f32; 3], [[f32; 3]; 3]) {
    let mut xyz = [0.0f32; 3];
    let mut j_xyz = [[0.0f32; 3]; 3]; // d(XYZ)/d(c_j)

    let mut k_norm = 0.0f32;
    for i in 0..SPD_SAMPLES {
        k_norm += CIE_1931_CMF_Y[i];
    }
    let inv_k = 1.0 / k_norm.max(1e-6);

    for i in 0..SPD_SAMPLES {
        let lambda = LAMBDA_MIN + (i as f32) * LAMBDA_STEP;
        let u = ((lambda - 380.0) / 400.0).clamp(0.0, 1.0);

        let f = c[0] * u * u + c[1] * u + c[2];
        let denom_sq = 1.0 + f * f;
        let denom = denom_sq.sqrt();
        let r = 0.5 + 0.5 * (f / denom);

        // Derivative dR/dc = 0.5 / (1 + f^2)^(3/2) * [u^2, u, 1]
        let dr_df = 0.5 / (denom_sq * denom);
        let df_dc = [u * u, u, 1.0];
        let dr_dc = [dr_df * df_dc[0], dr_df * df_dc[1], dr_df * df_dc[2]];

        let cmf_x = CIE_1931_CMF_X[i];
        let cmf_y = CIE_1931_CMF_Y[i];
        let cmf_z = CIE_1931_CMF_Z[i];

        xyz[0] += r * cmf_x;
        xyz[1] += r * cmf_y;
        xyz[2] += r * cmf_z;

        for j in 0..3 {
            j_xyz[0][j] += dr_dc[j] * cmf_x;
            j_xyz[1][j] += dr_dc[j] * cmf_y;
            j_xyz[2][j] += dr_dc[j] * cmf_z;
        }
    }

    xyz[0] *= inv_k;
    xyz[1] *= inv_k;
    xyz[2] *= inv_k;

    for row in 0..3 {
        for col in 0..3 {
            j_xyz[row][col] *= inv_k;
        }
    }

    // Multiply by M_XYZ_TO_SRGB to get sRGB and Jacobian J_sRGB
    let srgb = [
        M_XYZ_TO_SRGB[0][0] * xyz[0] + M_XYZ_TO_SRGB[0][1] * xyz[1] + M_XYZ_TO_SRGB[0][2] * xyz[2],
        M_XYZ_TO_SRGB[1][0] * xyz[0] + M_XYZ_TO_SRGB[1][1] * xyz[1] + M_XYZ_TO_SRGB[1][2] * xyz[2],
        M_XYZ_TO_SRGB[2][0] * xyz[0] + M_XYZ_TO_SRGB[2][1] * xyz[1] + M_XYZ_TO_SRGB[2][2] * xyz[2],
    ];

    let mut j_srgb = [[0.0f32; 3]; 3];
    for r in 0..3 {
        for c_idx in 0..3 {
            j_srgb[r][c_idx] = M_XYZ_TO_SRGB[r][0] * j_xyz[0][c_idx]
                + M_XYZ_TO_SRGB[r][1] * j_xyz[1][c_idx]
                + M_XYZ_TO_SRGB[r][2] * j_xyz[2][c_idx];
        }
    }

    (srgb, j_srgb)
}

// ───────────────────────────────────────────────────────────────────────────
//  Pure Rust 3x3 Linear System Solver
// ───────────────────────────────────────────────────────────────────────────

/// Analytical 3x3 linear system solver via Cramer's rule in zero heap.
#[inline]
fn solve_3x3(a: [[f32; 3]; 3], b: [f32; 3]) -> Option<[f32; 3]> {
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);

    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;

    let d0 = b[0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (b[1] * a[2][2] - a[1][2] * b[2])
        + a[0][2] * (b[1] * a[2][1] - a[1][1] * b[2]);

    let d1 = a[0][0] * (b[1] * a[2][2] - a[1][2] * b[2])
        - b[0] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * b[2] - b[1] * a[2][0]);

    let d2 = a[0][0] * (a[1][1] * b[2] - b[1] * a[2][1])
        - a[0][1] * (a[1][0] * b[2] - b[1] * a[2][0])
        + b[0] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);

    Some([d0 * inv_det, d1 * inv_det, d2 * inv_det])
}

// ───────────────────────────────────────────────────────────────────────────
//  Levenberg-Marquardt Inversion Solver (Pure Rust, Zero Heap)
// ───────────────────────────────────────────────────────────────────────────

/// Solve for the Jakob-Hanika coefficients matching a target linear sRGB color.
pub fn invert_srgb_to_coeffs(target_srgb: [f32; 3], initial_guess: [f32; 3]) -> [f32; 3] {
    let mut c = initial_guess;
    let mut lambda_lm = 1e-3f32;
    let gamma = 1e-6f32; // Curvature penalty weight; retain neutral-color accuracy
    let epsilon = 1e-6f32; // Magnitude regularizer

    for _ in 0..15 {
        let (current_srgb, j) = project_coeffs_to_srgb(c);
        let err = [
            target_srgb[0] - current_srgb[0],
            target_srgb[1] - current_srgb[1],
            target_srgb[2] - current_srgb[2],
        ];

        let err_norm_sq = err[0] * err[0] + err[1] * err[1] + err[2] * err[2];
        if err_norm_sq < 1e-7 {
            break;
        }

        // Form J^T J (3x3)
        let mut jt_j = [[0.0f32; 3]; 3];
        for row in 0..3 {
            for col in 0..3 {
                jt_j[row][col] =
                    j[0][row] * j[0][col] + j[1][row] * j[1][col] + j[2][row] * j[2][col];
            }
        }

        // Form J^T e (gradient)
        let g = [
            j[0][0] * err[0] + j[1][0] * err[1] + j[2][0] * err[2]
                - (8.0 * gamma * c[0])
                - 2.0 * epsilon * c[0],
            j[0][1] * err[0] + j[1][1] * err[1] + j[2][1] * err[2] - 2.0 * epsilon * c[1],
            j[0][2] * err[0] + j[1][2] * err[1] + j[2][2] * err[2] - 2.0 * epsilon * c[2],
        ];

        // Add LM damping and regularization to diagonal: A = J^T J + (lambda + ...) I
        jt_j[0][0] += lambda_lm + 8.0 * gamma + 2.0 * epsilon;
        jt_j[1][1] += lambda_lm + 2.0 * epsilon;
        jt_j[2][2] += lambda_lm + 2.0 * epsilon;

        if let Some(delta) = solve_3x3(jt_j, g) {
            let candidate_c = [
                (c[0] + delta[0]).clamp(-128.0, 128.0),
                (c[1] + delta[1]).clamp(-128.0, 128.0),
                (c[2] + delta[2]).clamp(-32.0, 32.0),
            ];
            let (cand_srgb, _) = project_coeffs_to_srgb(candidate_c);
            let cand_err = [
                target_srgb[0] - cand_srgb[0],
                target_srgb[1] - cand_srgb[1],
                target_srgb[2] - cand_srgb[2],
            ];
            let cand_norm_sq =
                cand_err[0] * cand_err[0] + cand_err[1] * cand_err[1] + cand_err[2] * cand_err[2];

            if cand_norm_sq < err_norm_sq {
                c = candidate_c;
                lambda_lm = (lambda_lm * 0.5).max(1e-7);
            } else {
                lambda_lm = (lambda_lm * 2.0).min(1e3);
            }
        } else {
            lambda_lm *= 4.0;
        }
    }

    c
}

// ───────────────────────────────────────────────────────────────────────────
//  Rust Offline Baker (Cold Construction)
// ───────────────────────────────────────────────────────────────────────────

/// Bake a full 32x32x32 spectral inversion LUT into a caller-supplied buffer in IEEE 754 f16 format.
///
/// Runs deterministically in pure Rust with zero heap allocations.
pub fn bake_spectral_inversion_lut(out_bytes: &mut [u8]) -> Result<usize, SpectralLutError> {
    if out_bytes.len() < LUT_TOTAL_BYTE_SIZE {
        return Err(SpectralLutError::BufferTooSmall {
            need: LUT_TOTAL_BYTE_SIZE,
            have: out_bytes.len(),
        });
    }

    let mut guess = [0.0f32; 3];
    let n = (LUT_RES - 1) as f32;

    for r in 0..LUT_RES {
        for g in 0..LUT_RES {
            for b in 0..LUT_RES {
                let target = [(r as f32) / n, (g as f32) / n, (b as f32) / n];
                let coeffs = invert_srgb_to_coeffs(target, guess);
                guess = coeffs;

                let offset = (r * LUT_RES * LUT_RES + g * LUT_RES + b) * LUT_BYTES_PER_VOXEL;
                let c0_f16 = f32_to_f16(coeffs[0]);
                let c1_f16 = f32_to_f16(coeffs[1]);
                let c2_f16 = f32_to_f16(coeffs[2]);

                out_bytes[offset..offset + 2].copy_from_slice(&c0_f16.to_le_bytes());
                out_bytes[offset + 2..offset + 4].copy_from_slice(&c1_f16.to_le_bytes());
                out_bytes[offset + 4..offset + 6].copy_from_slice(&c2_f16.to_le_bytes());
            }
        }
    }

    Ok(LUT_TOTAL_BYTE_SIZE)
}

// ───────────────────────────────────────────────────────────────────────────
//  Runtime Zero-Heap Inversion Lookup
// ───────────────────────────────────────────────────────────────────────────

/// Sample coefficients from pre-baked LUT slice using trilinear interpolation. Zero heap.
pub fn sample_spectral_lut_trilinear(lut_bytes: &[u8], rgb: [f32; 3]) -> JakobHanikaCoeffs {
    if lut_bytes.len() < LUT_TOTAL_BYTE_SIZE {
        return JakobHanikaCoeffs::FLAT_MID;
    }

    let n = (LUT_RES - 1) as f32;
    let r = (rgb[0].clamp(0.0, 1.0) * n).min(n);
    let g = (rgb[1].clamp(0.0, 1.0) * n).min(n);
    let b = (rgb[2].clamp(0.0, 1.0) * n).min(n);

    let r0 = r.floor() as usize;
    let g0 = g.floor() as usize;
    let b0 = b.floor() as usize;
    let r1 = (r0 + 1).min(LUT_RES - 1);
    let g1 = (g0 + 1).min(LUT_RES - 1);
    let b1 = (b0 + 1).min(LUT_RES - 1);

    let fr = r - r0 as f32;
    let fg = g - g0 as f32;
    let fb = b - b0 as f32;

    let read_voxel = |rx: usize, gx: usize, bx: usize| -> [f32; 3] {
        let offset = (rx * LUT_RES * LUT_RES + gx * LUT_RES + bx) * LUT_BYTES_PER_VOXEL;
        let c0_u16 = u16::from_le_bytes([lut_bytes[offset], lut_bytes[offset + 1]]);
        let c1_u16 = u16::from_le_bytes([lut_bytes[offset + 2], lut_bytes[offset + 3]]);
        let c2_u16 = u16::from_le_bytes([lut_bytes[offset + 4], lut_bytes[offset + 5]]);
        [f16_to_f32(c0_u16), f16_to_f32(c1_u16), f16_to_f32(c2_u16)]
    };

    let v000 = read_voxel(r0, g0, b0);
    let v100 = read_voxel(r1, g0, b0);
    let v010 = read_voxel(r0, g1, b0);
    let v110 = read_voxel(r1, g1, b0);
    let v001 = read_voxel(r0, g0, b1);
    let v101 = read_voxel(r1, g0, b1);
    let v011 = read_voxel(r0, g1, b1);
    let v111 = read_voxel(r1, g1, b1);

    let mut out = [0.0f32; 3];
    for ch in 0..3 {
        let c00 = v000[ch] * (1.0 - fr) + v100[ch] * fr;
        let c10 = v010[ch] * (1.0 - fr) + v110[ch] * fr;
        let c01 = v001[ch] * (1.0 - fr) + v101[ch] * fr;
        let c11 = v011[ch] * (1.0 - fr) + v111[ch] * fr;

        let c0 = c00 * (1.0 - fg) + c10 * fg;
        let c1 = c01 * (1.0 - fg) + c11 * fg;

        out[ch] = c0 * (1.0 - fb) + c1 * fb;
    }

    JakobHanikaCoeffs::new(out[0], out[1], out[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mid_gray_inversion_roundtrip() {
        let target = [0.5, 0.5, 0.5];
        let coeffs = invert_srgb_to_coeffs(target, [0.0, 0.0, 0.0]);
        let (recovered, _) = project_coeffs_to_srgb(coeffs);

        assert!((recovered[0] - 0.5).abs() < 0.02);
        assert!(
            (recovered[1] - 0.5).abs() < 0.02,
            "recovered={recovered:?}, coeffs={coeffs:?}"
        );
        assert!((recovered[2] - 0.5).abs() < 0.02);
    }

    #[test]
    fn test_solve_3x3_exact() {
        let a = [[2.0, 1.0, -1.0], [-3.0, -1.0, 2.0], [-2.0, 1.0, 2.0]];
        let b = [8.0, -11.0, -3.0];
        let x = solve_3x3(a, b).unwrap();

        assert!((x[0] - 2.0).abs() < 1e-5);
        assert!((x[1] - 3.0).abs() < 1e-5);
        assert!((x[2] - (-1.0)).abs() < 1e-5);
    }
}
