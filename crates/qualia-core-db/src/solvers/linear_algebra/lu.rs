//! Dynamic LU decomposition with partial pivoting (`P·A = L·U`) and determinant.
//!
//! The engine's canonical **dynamic** LU. The fixed-size [`super::StaticLuDecomposition`]
//! handles only 4×4; this is the general `n×n` routine that the specialized libraries
//! call (they keep only a thin error-mapping facade). Row-major, fails closed on a
//! shape mismatch; a zero pivot is recorded in [`Lu::singular`] (not an error) so the
//! determinant correctly comes out 0.
//!
//! Doolittle elimination with partial pivoting — `O(n³)`, numerically robust.

use crate::solvers::SolversError;

/// An in-place LU decomposition with partial pivoting (Doolittle), `P·A = L·U`.
#[derive(Debug, Clone)]
pub struct Lu {
    /// Combined factors, row-major `n×n`: `U` on/above the diagonal, the strictly-lower
    /// part of `L` below it (L's unit diagonal is implicit).
    pub lu: Vec<f64>,
    /// Row permutation: `pivots[i]` is the original row now in position `i`.
    pub pivots: Vec<usize>,
    /// Sign of the permutation (`+1`/`-1`), i.e. `det(P)`.
    pub sign: f64,
    /// `true` if a zero pivot was encountered (matrix is singular).
    pub singular: bool,
    pub n: usize,
}

impl Lu {
    /// `det(A) = sign · Π U[i][i]`.
    pub fn determinant(&self) -> f64 {
        if self.singular {
            return 0.0;
        }
        let mut det = self.sign;
        for i in 0..self.n {
            det *= self.lu[i * self.n + i];
        }
        det
    }
}

/// LU-decompose a row-major `n×n` matrix with partial pivoting. The reusable primitive
/// behind [`determinant`] (and a building block for solves / condition estimates). O(n³).
/// Returns [`SolversError::InvalidDimension`] for an empty or non-square input.
pub fn lu_decompose(n: usize, data: &[f64]) -> Result<Lu, SolversError> {
    if n == 0 || data.len() != n * n {
        return Err(SolversError::InvalidDimension);
    }
    let mut a = data.to_vec();
    let mut pivots: Vec<usize> = (0..n).collect();
    let mut sign = 1.0_f64;
    let mut singular = false;

    for col in 0..n {
        // Partial pivot: largest magnitude in this column at/below the diagonal.
        let mut pivot = col;
        let mut maxv = a[col * n + col].abs();
        for r in (col + 1)..n {
            let v = a[r * n + col].abs();
            if v > maxv {
                maxv = v;
                pivot = r;
            }
        }
        if maxv == 0.0 {
            singular = true;
            continue; // leave a zero on the diagonal; det → 0
        }
        if pivot != col {
            for k in 0..n {
                a.swap(col * n + k, pivot * n + k);
            }
            pivots.swap(col, pivot);
            sign = -sign;
        }
        let diag = a[col * n + col];
        for r in (col + 1)..n {
            let factor = a[r * n + col] / diag;
            a[r * n + col] = factor; // store L's multiplier in the lower triangle
            for k in (col + 1)..n {
                a[r * n + k] -= factor * a[col * n + k];
            }
        }
    }

    Ok(Lu { lu: a, pivots, sign, singular, n })
}

/// Determinant of a row-major `n×n` matrix via LU decomposition with partial pivoting.
/// O(n³), numerically robust; returns 0.0 for a singular matrix.
pub fn determinant(n: usize, data: &[f64]) -> Result<f64, SolversError> {
    Ok(lu_decompose(n, data)?.determinant())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinant_2x2_and_3x3() {
        // det[[1,2],[3,4]] = -2
        assert!((determinant(2, &[1.0, 2.0, 3.0, 4.0]).unwrap() + 2.0).abs() < 1e-12);
        // det[[6,1,1],[4,-2,5],[2,8,7]] = -306
        let d = determinant(3, &[6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0]).unwrap();
        assert!((d + 306.0).abs() < 1e-9, "det = {d}");
    }

    #[test]
    fn singular_has_zero_determinant() {
        let sing = lu_decompose(2, &[1.0, 2.0, 2.0, 4.0]).unwrap();
        assert!(sing.singular && sing.determinant() == 0.0);
    }

    #[test]
    fn reconstructs_permuted_a() {
        // Rebuild L and U from the factors and verify L·U == P·A.
        let n = 3;
        let a = [4.0, 3.0, 2.0, 2.0, 1.0, 3.0, 3.0, 2.0, 1.0];
        let f = lu_decompose(n, &a).unwrap();
        assert!(!f.singular);
        let mut l = vec![0.0; n * n];
        let mut u = vec![0.0; n * n];
        for i in 0..n {
            l[i * n + i] = 1.0;
            for j in 0..n {
                if j < i {
                    l[i * n + j] = f.lu[i * n + j];
                } else {
                    u[i * n + j] = f.lu[i * n + j];
                }
            }
        }
        // P·A
        let mut pa = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                pa[i * n + j] = a[f.pivots[i] * n + j];
            }
        }
        // L·U
        for i in 0..n {
            for j in 0..n {
                let mut s = 0.0;
                for k in 0..n {
                    s += l[i * n + k] * u[k * n + j];
                }
                assert!((s - pa[i * n + j]).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn rejects_bad_dims() {
        assert_eq!(determinant(2, &[1.0, 2.0, 3.0]), Err(SolversError::InvalidDimension));
    }
}
