//! Symmetric eigendecomposition — the engine's single home for eigenvalues of a
//! symmetric matrix.
//!
//! Before this, the same math lived in two silos: `specialized_libs/linear_algebra`
//! had a cyclic-Jacobi `eigen_symmetric`, and `specialized_libs/engineering_analysis`
//! had a closed-form 3×3 principal-stress solver — two implementations of one
//! operation. Both now route here.
//!
//! Two entry points, same modality:
//! - [`symmetric_eigen_3x3`] — closed-form (Smith's algorithm) for the symmetric
//!   3×3 case; zero-heap, no iteration, eigenvalues sorted descending.
//! - [`symmetric_eigen`] — cyclic-Jacobi for general `n×n`; in-place on a
//!   caller-owned buffer, also yields eigenvectors. Zero-heap.

use crate::solvers::SolversError;

/// Eigenvalues of a **symmetric 3×3** matrix `a` (row-major, length 9) by Smith's
/// closed-form algorithm — no iteration, no allocation. Returns the three
/// eigenvalues **sorted descending** (`e[0] ≥ e[1] ≥ e[2]`), the convention used
/// for principal stresses/strains.
///
/// Only the symmetric part is used (off-diagonals read from the upper triangle:
/// `(0,1)`, `(0,2)`, `(1,2)`), so a numerically-symmetric `a` need not be exact in
/// its lower half.
pub fn symmetric_eigen_3x3(a: &[f64; 9]) -> [f64; 3] {
    let (sxx, syy, szz) = (a[0], a[4], a[8]);
    // Upper-triangle off-diagonals: sxy=(0,1), szx=(0,2), syz=(1,2).
    let (sxy, szx, syz) = (a[1], a[2], a[5]);
    let p1 = sxy * sxy + syz * syz + szx * szx;
    let q = (sxx + syy + szz) / 3.0;
    if p1 <= 1e-18 {
        // Already diagonal — eigenvalues are the diagonal entries.
        let mut e = [sxx, syy, szz];
        e.sort_by(|a, b| b.partial_cmp(a).unwrap_or(core::cmp::Ordering::Equal));
        return e;
    }
    let p2 = (sxx - q).powi(2) + (syy - q).powi(2) + (szz - q).powi(2) + 2.0 * p1;
    let p = (p2 / 6.0).sqrt();
    // B = (1/p)·(A − qI); r = det(B)/2 ∈ [−1, 1].
    let (b00, b11, b22) = ((sxx - q) / p, (syy - q) / p, (szz - q) / p);
    let (b01, b12, b02) = (sxy / p, syz / p, szx / p);
    let det_b = b00 * (b11 * b22 - b12 * b12) - b01 * (b01 * b22 - b12 * b02)
        + b02 * (b01 * b12 - b11 * b02);
    let r = (det_b / 2.0).clamp(-1.0, 1.0);
    let phi = r.acos() / 3.0;
    let e1 = q + 2.0 * p * phi.cos();
    let e3 = q + 2.0 * p * (phi + 2.0 * core::f64::consts::PI / 3.0).cos();
    let e2 = 3.0 * q - e1 - e3; // trace is invariant: e1+e2+e3 = 3q
    [e1, e2, e3]
}

/// Eigendecomposition of a **symmetric `n×n`** matrix by cyclic Jacobi rotations.
///
/// On entry `a` (row-major, length `n*n`) holds the symmetric matrix; it is
/// **overwritten** — on return its diagonal `a[i*n+i]` holds the eigenvalues (in
/// Jacobi's natural order, not sorted). `eigvecs` (length `n*n`) receives the
/// orthonormal eigenvectors as **columns**: column `j` is the unit eigenvector for
/// the eigenvalue at `a[j*n+j]`. Zero allocation — both buffers are caller-owned.
///
/// Returns [`SolversError::InvalidDimension`] on a shape mismatch, or
/// [`SolversError::InvalidParameters`] if `a` is not (within tolerance) symmetric.
pub fn symmetric_eigen(n: usize, a: &mut [f64], eigvecs: &mut [f64]) -> Result<(), SolversError> {
    if n == 0 || a.len() != n * n || eigvecs.len() != n * n {
        return Err(SolversError::InvalidDimension);
    }
    let scale = a.iter().fold(0.0_f64, |m, &v| m.max(v.abs())).max(1.0);
    // Symmetry check.
    for i in 0..n {
        for j in (i + 1)..n {
            if (a[i * n + j] - a[j * n + i]).abs() > 1e-9 * scale {
                return Err(SolversError::InvalidParameters);
            }
        }
    }
    // eigvecs starts as the identity.
    for x in eigvecs.iter_mut() {
        *x = 0.0;
    }
    for i in 0..n {
        eigvecs[i * n + i] = 1.0;
    }

    const MAX_SWEEPS: usize = 100;
    for _ in 0..MAX_SWEEPS {
        // Off-diagonal Frobenius norm; stop when negligible.
        let mut off = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[p * n + q] * a[p * n + q];
            }
        }
        if off.sqrt() <= 1e-15 * scale {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p * n + q];
                if apq == 0.0 {
                    continue;
                }
                let app = a[p * n + p];
                let aqq = a[q * n + q];
                let theta = (aqq - app) / (2.0 * apq);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                // Rotate columns p,q of A.
                for k in 0..n {
                    let akp = a[k * n + p];
                    let akq = a[k * n + q];
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                // Rotate rows p,q of A.
                for k in 0..n {
                    let apk = a[p * n + k];
                    let aqk = a[q * n + k];
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                // Accumulate the rotation into the eigenvector matrix.
                for k in 0..n {
                    let vkp = eigvecs[k * n + p];
                    let vkq = eigvecs[k * n + q];
                    eigvecs[k * n + p] = c * vkp - s * vkq;
                    eigvecs[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} != {b} (tol {tol})");
    }

    #[test]
    fn closed_form_diagonal() {
        // Diagonal matrix → eigenvalues are the diagonal, sorted descending.
        let a = [3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0];
        let e = symmetric_eigen_3x3(&a);
        approx(e[0], 3.0, 1e-12);
        approx(e[1], 2.0, 1e-12);
        approx(e[2], 1.0, 1e-12);
    }

    #[test]
    fn closed_form_known_symmetric() {
        // [[2,0,0],[0,3,4],[0,4,9]] — block eigenvalues of [[3,4],[4,9]] are 1 and 11,
        // plus the isolated 2 → {11, 2, 1} descending.
        let a = [2.0, 0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 4.0, 9.0];
        let e = symmetric_eigen_3x3(&a);
        approx(e[0], 11.0, 1e-9);
        approx(e[1], 2.0, 1e-9);
        approx(e[2], 1.0, 1e-9);
        // Trace invariant.
        approx(e[0] + e[1] + e[2], 14.0, 1e-9);
    }

    #[test]
    fn closed_form_matches_jacobi() {
        let a = [4.0, 1.0, 2.0, 1.0, 5.0, 3.0, 2.0, 3.0, 6.0];
        let closed = symmetric_eigen_3x3(&a);
        let mut work = a;
        let mut v = [0.0; 9];
        symmetric_eigen(3, &mut work, &mut v).unwrap();
        let mut jac = [work[0], work[4], work[8]];
        jac.sort_by(|x, y| y.partial_cmp(x).unwrap());
        for i in 0..3 {
            approx(closed[i], jac[i], 1e-7);
        }
    }

    #[test]
    fn jacobi_eigenvectors_reconstruct() {
        // A·v_j == λ_j·v_j for each eigenpair.
        let a0 = [4.0, 1.0, 2.0, 1.0, 5.0, 3.0, 2.0, 3.0, 6.0];
        let mut a = a0;
        let mut v = [0.0; 9];
        symmetric_eigen(3, &mut a, &mut v).unwrap();
        for j in 0..3 {
            let lambda = a[j * 3 + j];
            // column j of v
            let vj = [v[j], v[3 + j], v[6 + j]];
            // A·vj
            for i in 0..3 {
                let mut s = 0.0;
                for k in 0..3 {
                    s += a0[i * 3 + k] * vj[k];
                }
                approx(s, lambda * vj[i], 1e-7);
            }
        }
    }

    #[test]
    fn jacobi_rejects_asymmetric() {
        let mut a = [1.0, 2.0, 3.0, 4.0]; // not symmetric (2 != 3)
        let mut v = [0.0; 4];
        assert_eq!(
            symmetric_eigen(2, &mut a, &mut v),
            Err(SolversError::InvalidParameters)
        );
    }

    #[test]
    fn jacobi_rejects_bad_dims() {
        let mut a = [1.0, 2.0, 3.0];
        let mut v = [0.0; 4];
        assert_eq!(
            symmetric_eigen(2, &mut a, &mut v),
            Err(SolversError::InvalidDimension)
        );
    }
}
