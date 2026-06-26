//! Cholesky decomposition `A = L·Lᵀ` for symmetric positive-definite matrices.
//!
//! Functionality parity with nalgebra's `linalg::cholesky`, implemented in the
//! qualia idiom: **zero allocation**, operating on caller-owned row-major
//! slices with explicit dimension. No `DMatrix`, no heap, no dependency.
//!
//! Cholesky is the fast, numerically-stable path for SPD systems (covariance
//! solves, least-squares normal equations, Kalman updates, interior-point steps).

use crate::solvers::SolversError;

/// Compute the lower-triangular Cholesky factor `L` of the `n×n` symmetric
/// positive-definite matrix `a` (row-major), writing `L` row-major into `l`
/// (lower triangle filled, strictly-upper zeroed). `a` and `l` must each be
/// length `n*n`.
///
/// Returns [`SolversError::SingularMatrix`] if a non-positive pivot is reached
/// (i.e. `a` is not positive-definite) — fail closed, never a bogus factor.
/// Only the lower triangle of `a` is read, so a symmetric `a` need not be exact
/// in its upper half.
pub fn cholesky_factor(n: usize, a: &[f64], l: &mut [f64]) -> Result<(), SolversError> {
    if a.len() != n * n || l.len() != n * n {
        return Err(SolversError::InvalidDimension);
    }
    for x in l.iter_mut() {
        *x = 0.0;
    }
    for j in 0..n {
        // Diagonal: L[j][j] = sqrt(A[j][j] - Σ_{k<j} L[j][k]²)
        let mut diag = a[j * n + j];
        for k in 0..j {
            diag -= l[j * n + k] * l[j * n + k];
        }
        if !(diag > 0.0) {
            return Err(SolversError::SingularMatrix);
        }
        let ljj = diag.sqrt();
        l[j * n + j] = ljj;

        // Below the diagonal: L[i][j] = (A[i][j] - Σ_{k<j} L[i][k]·L[j][k]) / L[j][j]
        for i in (j + 1)..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            l[i * n + j] = s / ljj;
        }
    }
    Ok(())
}

/// Solve `A·x = b` for SPD `A`, given its Cholesky factor `l` (from
/// [`cholesky_factor`]): forward-substitute `L·y = b`, then back-substitute
/// `Lᵀ·x = y`. `l` is `n*n`; `b` and `x` are length `n`. The solution is written
/// into `x` (which is also used as scratch for `y`).
pub fn cholesky_solve(n: usize, l: &[f64], b: &[f64], x: &mut [f64]) -> Result<(), SolversError> {
    if l.len() != n * n || b.len() != n || x.len() != n {
        return Err(SolversError::InvalidDimension);
    }
    // Forward: L·y = b   (y accumulated in x)
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * n + k] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    // Backward: Lᵀ·x = y
    for i in (0..n).rev() {
        let mut s = x[i];
        for k in (i + 1)..n {
            s -= l[k * n + i] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    Ok(())
}

/// Determinant of an SPD matrix from its Cholesky factor: `det(A) = Π L[i][i]²`.
pub fn cholesky_determinant(n: usize, l: &[f64]) -> f64 {
    let mut prod = 1.0;
    for i in 0..n {
        let d = l[i * n + i];
        prod *= d * d;
    }
    prod
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    // Textbook SPD matrix with a known integer factor.
    // A = [[4,12,-16],[12,37,-43],[-16,-43,98]] = L·Lᵀ,
    // L = [[2,0,0],[6,1,0],[-8,5,3]].
    const A3: [f64; 9] = [4.0, 12.0, -16.0, 12.0, 37.0, -43.0, -16.0, -43.0, 98.0];

    #[test]
    fn factor_matches_known_lower_triangle() {
        let mut l = [0.0; 9];
        cholesky_factor(3, &A3, &mut l).unwrap();
        let expect = [2.0, 0.0, 0.0, 6.0, 1.0, 0.0, -8.0, 5.0, 3.0];
        for i in 0..9 {
            assert!((l[i] - expect[i]).abs() < EPS, "l[{i}]={} != {}", l[i], expect[i]);
        }
    }

    #[test]
    fn reconstructs_a() {
        let mut l = [0.0; 9];
        cholesky_factor(3, &A3, &mut l).unwrap();
        // L·Lᵀ == A
        for i in 0..3 {
            for j in 0..3 {
                let mut s = 0.0;
                for k in 0..3 {
                    s += l[i * 3 + k] * l[j * 3 + k];
                }
                assert!((s - A3[i * 3 + j]).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn solves_linear_system() {
        let mut l = [0.0; 9];
        cholesky_factor(3, &A3, &mut l).unwrap();
        let b = [1.0, 2.0, 3.0];
        let mut x = [0.0; 3];
        cholesky_solve(3, &l, &b, &mut x).unwrap();
        // Verify A·x ≈ b
        for i in 0..3 {
            let mut s = 0.0;
            for j in 0..3 {
                s += A3[i * 3 + j] * x[j];
            }
            assert!((s - b[i]).abs() < 1e-6, "row {i}: {} != {}", s, b[i]);
        }
    }

    #[test]
    fn determinant_via_factor() {
        let mut l = [0.0; 9];
        cholesky_factor(3, &A3, &mut l).unwrap();
        // det(A) = (2·1·3)² = 36
        assert!((cholesky_determinant(3, &l) - 36.0).abs() < 1e-6);
    }

    #[test]
    fn rejects_non_positive_definite() {
        // Symmetric but indefinite (negative eigenvalue): [[1,2],[2,1]].
        let a = [1.0, 2.0, 2.0, 1.0];
        let mut l = [0.0; 4];
        assert_eq!(cholesky_factor(2, &a, &mut l), Err(SolversError::SingularMatrix));
    }

    #[test]
    fn rejects_bad_dims() {
        let mut l = [0.0; 9];
        assert_eq!(cholesky_factor(2, &A3, &mut l), Err(SolversError::InvalidDimension));
    }
}
