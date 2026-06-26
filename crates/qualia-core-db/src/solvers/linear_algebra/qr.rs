//! Householder QR decomposition `A = Q·R` and least-squares solve.
//!
//! Functionality parity with nalgebra's `linalg::qr`, in the qualia idiom:
//! **zero allocation**, operating on caller-owned **row-major** slices, fail-closed.
//! No `DMatrix`, no heap, no dependency.
//!
//! QR is the stable workhorse the engine was missing entirely (the prior LA had
//! only fixed-size `Matrix4x4` LU and the silo's heap routines). It gives:
//! - orthogonal factorisation `A(m×n) = Q(m×m)·R(m×n)`, `m ≥ n`;
//! - least-squares `min‖A·x − b‖` for overdetermined full-rank systems
//!   (the normal-equations route proved in [`super::gemm`], but numerically
//!   stable — no `AᵀA` conditioning blow-up);
//! - a square linear solve as the `m == n` case (alternative to LU).
//!
//! Storage convention (LAPACK `geqrf` style): [`qr_factor`] overwrites `a` in
//! place — the upper triangle (incl. diagonal) becomes `R`; below the diagonal
//! holds the essential Householder vectors `v` (with implicit `v[j] = 1`); the
//! per-column scalings go in `tau`.

use crate::solvers::SolversError;

/// Compute the Householder QR factorisation of the `m×n` row-major matrix `a`
/// (`m ≥ n`) **in place**. On return:
/// - the upper triangle of `a` (including the diagonal) holds `R`;
/// - below the diagonal holds the essential Householder reflector vectors;
/// - `tau[j]` holds the reflector scaling for column `j`.
///
/// `a` must be length `m*n`; `tau` must be length `n`. Returns
/// [`SolversError::InvalidDimension`] on a length/shape mismatch (including `m < n`).
/// A zero sub-column yields `tau[j] = 0` (identity reflector) — rank deficiency is
/// not an error here; it surfaces at solve time as a zero `R` pivot.
pub fn qr_factor(m: usize, n: usize, a: &mut [f64], tau: &mut [f64]) -> Result<(), SolversError> {
    if m < n || a.len() != m * n || tau.len() != n {
        return Err(SolversError::InvalidDimension);
    }
    for j in 0..n {
        // Norm of the sub-column a[j..m, j].
        let mut norm_sq = 0.0;
        for i in j..m {
            let v = a[i * n + j];
            norm_sq += v * v;
        }
        let s = norm_sq.sqrt();
        if s == 0.0 {
            tau[j] = 0.0;
            continue;
        }
        let a_jj = a[j * n + j];
        // beta = new R[j][j]; choose sign to avoid cancellation.
        let beta = if a_jj >= 0.0 { -s } else { s };
        tau[j] = (beta - a_jj) / beta;
        let denom = a_jj - beta;
        // v[j] = 1 (implicit); v[i>j] = a[i][j]/denom, stored below the diagonal.
        for i in (j + 1)..m {
            a[i * n + j] /= denom;
        }
        a[j * n + j] = beta;
        // Apply (I − tau·v·vᵀ) to the trailing columns j+1..n.
        for col in (j + 1)..n {
            let mut w = a[j * n + col]; // v[j]·A[j][col], v[j]=1
            for i in (j + 1)..m {
                w += a[i * n + j] * a[i * n + col];
            }
            w *= tau[j];
            a[j * n + col] -= w; // v[j]=1
            for i in (j + 1)..m {
                a[i * n + col] -= w * a[i * n + j];
            }
        }
    }
    Ok(())
}

/// Materialise the **thin** orthogonal factor `Q` (`m×n`, row-major) from a
/// factored `a`/`tau` (from [`qr_factor`]). `q` must be length `m*n`. The thin
/// `Q` satisfies `Q·R_n = A` (with `R_n` the `n×n` upper triangle of `a`) and
/// has orthonormal columns (`Qᵀ·Q = I_n`).
pub fn qr_form_q(m: usize, n: usize, a: &[f64], tau: &[f64], q: &mut [f64]) -> Result<(), SolversError> {
    if m < n || a.len() != m * n || tau.len() != n || q.len() != m * n {
        return Err(SolversError::InvalidDimension);
    }
    // Start Q = first n columns of I_m.
    for x in q.iter_mut() {
        *x = 0.0;
    }
    for j in 0..n {
        q[j * n + j] = 1.0;
    }
    // Q = H_0 · H_1 · … · H_{n-1}; apply in reverse so the product accumulates.
    for j in (0..n).rev() {
        if tau[j] == 0.0 {
            continue;
        }
        for col in 0..n {
            let mut w = q[j * n + col]; // v[j]=1
            for i in (j + 1)..m {
                w += a[i * n + j] * q[i * n + col];
            }
            w *= tau[j];
            q[j * n + col] -= w;
            for i in (j + 1)..m {
                q[i * n + col] -= w * a[i * n + j];
            }
        }
    }
    Ok(())
}

/// Least-squares solve `min‖A·x − b‖` for an `m×n` (`m ≥ n`) full-rank system,
/// given the factored `a`/`tau` (from [`qr_factor`]).
///
/// `b` (length `m`) is overwritten with `Qᵀ·b`; the first `n` entries are then
/// back-substituted through `R` into `x` (length `n`). For `m == n` this is an
/// exact square solve. Returns [`SolversError::SingularMatrix`] if an `R` pivot
/// is ~0 (rank deficient) — fail closed, never a bogus solution.
pub fn qr_solve_least_squares(
    m: usize,
    n: usize,
    a: &[f64],
    tau: &[f64],
    b: &mut [f64],
    x: &mut [f64],
) -> Result<(), SolversError> {
    if m < n || a.len() != m * n || tau.len() != n || b.len() != m || x.len() != n {
        return Err(SolversError::InvalidDimension);
    }
    // Apply Qᵀ to b: b ← H_{n-1} … H_0 · b.
    for j in 0..n {
        if tau[j] == 0.0 {
            continue;
        }
        let mut w = b[j]; // v[j]=1
        for i in (j + 1)..m {
            w += a[i * n + j] * b[i];
        }
        w *= tau[j];
        b[j] -= w;
        for i in (j + 1)..m {
            b[i] -= w * a[i * n + j];
        }
    }
    // Rank check: a pivot small relative to the largest R diagonal means the
    // columns are (near-)dependent — fail closed rather than divide by ~0.
    let mut scale = 0.0_f64;
    for i in 0..n {
        let d = a[i * n + i].abs();
        if d > scale {
            scale = d;
        }
    }
    let tol = 1e-12 * scale * (n as f64);
    // Back-substitute R·x = (Qᵀb)[0..n].
    for i in (0..n).rev() {
        let pivot = a[i * n + i];
        if !pivot.is_finite() || pivot.abs() <= tol {
            return Err(SolversError::SingularMatrix);
        }
        let mut s = b[i];
        for k in (i + 1)..n {
            s -= a[i * n + k] * x[k];
        }
        x[i] = s / pivot;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::linear_algebra::gemm::{gemm, matmul, Transpose};

    fn approx(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len());
        for i in 0..a.len() {
            assert!((a[i] - b[i]).abs() < tol, "idx {i}: {} != {} (tol {tol})", a[i], b[i]);
        }
    }

    // Extract the n×n upper-triangular R from a factored buffer.
    fn extract_r(m: usize, n: usize, a: &[f64]) -> Vec<f64> {
        let mut r = vec![0.0; n * n];
        for i in 0..n {
            for j in i..n {
                r[i * n + j] = a[i * n + j];
            }
        }
        let _ = m;
        r
    }

    #[test]
    fn factor_reconstructs_a_square() {
        // A = [[12,-51,4],[6,167,-68],[-4,24,-41]] — the classic Householder example.
        let a0 = [12.0, -51.0, 4.0, 6.0, 167.0, -68.0, -4.0, 24.0, -41.0];
        let (m, n) = (3, 3);
        let mut a = a0;
        let mut tau = [0.0; 3];
        qr_factor(m, n, &mut a, &mut tau).unwrap();

        let mut q = [0.0; 9];
        qr_form_q(m, n, &a, &tau, &mut q).unwrap();
        let r = extract_r(m, n, &a);

        // Q·R == A
        let mut recon = [0.0; 9];
        matmul(m, n, n, &q, &r, &mut recon).unwrap();
        approx(&recon, &a0, 1e-9);

        // R[2][2] of this matrix is known to be 35 (up to sign).
        assert!((a[2 * 3 + 2].abs() - 35.0).abs() < 1e-9);
    }

    #[test]
    fn q_has_orthonormal_columns() {
        let a0 = [12.0, -51.0, 4.0, 6.0, 167.0, -68.0, -4.0, 24.0, -41.0];
        let mut a = a0;
        let mut tau = [0.0; 3];
        qr_factor(3, 3, &mut a, &mut tau).unwrap();
        let mut q = [0.0; 9];
        qr_form_q(3, 3, &a, &tau, &mut q).unwrap();
        // QᵀQ == I
        let mut qtq = [0.0; 9];
        gemm(Transpose::Yes, Transpose::No, 3, 3, 3, 1.0, &q, &q, 0.0, &mut qtq).unwrap();
        approx(&qtq, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1e-9);
    }

    #[test]
    fn square_solve_matches_known() {
        // [[2,1],[1,3]] x = [3,5] → x = [0.8, 1.4]
        let a0 = [2.0, 1.0, 1.0, 3.0];
        let mut a = a0;
        let mut tau = [0.0; 2];
        qr_factor(2, 2, &mut a, &mut tau).unwrap();
        let mut b = [3.0, 5.0];
        let mut x = [0.0; 2];
        qr_solve_least_squares(2, 2, &a, &tau, &mut b, &mut x).unwrap();
        approx(&x, &[0.8, 1.4], 1e-9);
    }

    #[test]
    fn least_squares_overdetermined_line_fit() {
        // Fit y = c0 + c1·t to (0,1),(1,2),(2,3),(3,4): exact line y=1+t → c=[1,1].
        // Design matrix A (4×2): columns [1, t].
        let a0 = [1.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let mut a = a0;
        let mut tau = [0.0; 2];
        qr_factor(4, 2, &mut a, &mut tau).unwrap();
        let mut b = [1.0, 2.0, 3.0, 4.0];
        let mut x = [0.0; 2];
        qr_solve_least_squares(4, 2, &a, &tau, &mut b, &mut x).unwrap();
        approx(&x, &[1.0, 1.0], 1e-9);
    }

    #[test]
    fn least_squares_overdetermined_noisy() {
        // Slightly perturbed: residual minimised, slope/intercept near the trend.
        // (0,1),(1,3),(2,4),(3,6). Normal equations: Σt=6, Σt²=14, Σy=14, Σty=29
        // → 4c0+6c1=14, 6c0+14c1=29 → c1=1.6, c0=3.5−1.5·1.6=1.1.
        let a0 = [1.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let mut a = a0;
        let mut tau = [0.0; 2];
        qr_factor(4, 2, &mut a, &mut tau).unwrap();
        let mut b = [1.0, 3.0, 4.0, 6.0];
        let mut x = [0.0; 2];
        qr_solve_least_squares(4, 2, &a, &tau, &mut b, &mut x).unwrap();
        // Closed-form normal-equations solution: intercept 1.1, slope 1.6.
        approx(&x, &[1.1, 1.6], 1e-9);
    }

    #[test]
    fn reconstructs_tall_matrix() {
        // Tall A (4×2): Q_thin(4×2)·R(2×2) == A.
        let a0 = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (m, n) = (4, 2);
        let mut a = a0;
        let mut tau = [0.0; 2];
        qr_factor(m, n, &mut a, &mut tau).unwrap();
        let mut q = [0.0; 8];
        qr_form_q(m, n, &a, &tau, &mut q).unwrap();
        let r = extract_r(m, n, &a);
        let mut recon = [0.0; 8];
        matmul(m, n, n, &q, &r, &mut recon).unwrap();
        approx(&recon, &a0, 1e-9);
    }

    #[test]
    fn rank_deficient_fails_closed() {
        // Columns identical → rank 1 → zero R pivot → SingularMatrix on solve.
        let a0 = [1.0, 1.0, 2.0, 2.0, 3.0, 3.0]; // 3×2, col1==col2
        let mut a = a0;
        let mut tau = [0.0; 2];
        qr_factor(3, 2, &mut a, &mut tau).unwrap();
        let mut b = [1.0, 2.0, 3.0];
        let mut x = [0.0; 2];
        assert_eq!(
            qr_solve_least_squares(3, 2, &a, &tau, &mut b, &mut x),
            Err(SolversError::SingularMatrix)
        );
    }

    #[test]
    fn rejects_bad_dims() {
        let mut a = [1.0, 2.0, 3.0, 4.0];
        let mut tau = [0.0; 3]; // wrong length
        assert_eq!(qr_factor(2, 2, &mut a, &mut tau), Err(SolversError::InvalidDimension));
        // m < n rejected
        let mut a2 = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut tau2 = [0.0; 3];
        assert_eq!(qr_factor(2, 3, &mut a2, &mut tau2), Err(SolversError::InvalidDimension));
    }
}
