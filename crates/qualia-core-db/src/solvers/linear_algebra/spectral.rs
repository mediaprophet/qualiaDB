//! Matrix-spectral routines that bridge linear algebra and polynomial algebra:
//! the characteristic polynomial and general (non-symmetric) eigenvalues.
//!
//! `characteristic_polynomial` uses Faddeev–LeVerrier; `eigenvalues_general` factors it
//! with the engine's [`crate::solvers::polynomial::polynomial_roots`]. For symmetric
//! matrices prefer [`super::eigen::symmetric_eigen`] (eigenvectors + better conditioning).

use crate::solvers::polynomial::{polynomial_roots, Complex};
use crate::solvers::SolversError;

/// Characteristic polynomial of a row-major `n×n` matrix via the Faddeev–LeVerrier
/// algorithm. Returns DESCENDING coefficients `[1, c₁, …, cₙ]` of
/// `p(λ) = λⁿ + c₁λⁿ⁻¹ + … + cₙ` (so `det(A) = (-1)ⁿ·cₙ`). Exact for integer matrices;
/// for large/ill-conditioned matrices prefer an iterative eigensolver.
pub fn characteristic_polynomial(n: usize, data: &[f64]) -> Result<Vec<f64>, SolversError> {
    if n == 0 || data.len() != n * n {
        return Err(SolversError::InvalidDimension);
    }
    // Each Faddeev–LeVerrier step needs the dense product `A·M` (`n×n · n×n`). Route it
    // through the engine's one GEMM (`super::gemm::matmul`), which itself picks the
    // best path on this machine — offloading to the forge dispatcher above
    // `GEMM_GPU_THRESHOLD` on an accelerator, and otherwise running its exact f64 CPU
    // floor (same increasing-`k` accumulation, so byte-identical off-accelerator). The
    // whole algorithm is `n` such products (O(n⁴)), so a large characteristic-polynomial
    // is exactly the case the accelerator earns its keep.
    let mul = |x: &[f64], y: &[f64]| -> Result<Vec<f64>, SolversError> {
        let mut out = vec![0.0_f64; n * n];
        super::gemm::matmul(n, n, n, x, y, &mut out)?;
        Ok(out)
    };
    let trace = |x: &[f64]| -> f64 { (0..n).map(|i| x[i * n + i]).sum() };

    let mut coeffs = vec![0.0_f64; n + 1];
    coeffs[0] = 1.0;
    // M starts as the identity.
    let mut m = vec![0.0_f64; n * n];
    for i in 0..n {
        m[i * n + i] = 1.0;
    }
    for k in 1..=n {
        let am = mul(data, &m)?;
        let ck = -trace(&am) / (k as f64);
        coeffs[k] = ck;
        // M ← A·M + ck·I  (not needed after the final iteration).
        m = am;
        for i in 0..n {
            m[i * n + i] += ck;
        }
    }
    Ok(coeffs)
}

/// Eigenvalues of a GENERAL (not necessarily symmetric) row-major `n×n` matrix, as
/// complex numbers. Computes the characteristic polynomial (Faddeev–LeVerrier) and finds
/// its roots. Returns all `n` eigenvalues; real ones have `im ≈ 0`.
pub fn eigenvalues_general(n: usize, data: &[f64]) -> Result<Vec<Complex>, SolversError> {
    let charpoly = characteristic_polynomial(n, data)?;
    polynomial_roots(&charpoly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charpoly_of_2x2() {
        // [[2,0],[0,3]] → (λ−2)(λ−3) = λ² − 5λ + 6 → [1, −5, 6]
        let c = characteristic_polynomial(2, &[2.0, 0.0, 0.0, 3.0]).unwrap();
        assert!((c[0] - 1.0).abs() < 1e-12);
        assert!((c[1] + 5.0).abs() < 1e-9);
        assert!((c[2] - 6.0).abs() < 1e-9);
    }

    #[test]
    fn general_eigenvalues_real() {
        // [[2,0],[0,3]] → eigenvalues {2,3}.
        let ev = eigenvalues_general(2, &[2.0, 0.0, 0.0, 3.0]).unwrap();
        assert_eq!(ev.len(), 2);
        let mut reals: Vec<f64> = ev.iter().map(|z| z.re).collect();
        reals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((reals[0] - 2.0).abs() < 1e-6 && (reals[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn general_eigenvalues_complex_rotation() {
        // 90° rotation [[0,-1],[1,0]] → eigenvalues ±i.
        let ev = eigenvalues_general(2, &[0.0, -1.0, 1.0, 0.0]).unwrap();
        assert_eq!(ev.len(), 2);
        assert!(ev.iter().any(|z| (z.im.abs() - 1.0).abs() < 1e-6));
    }

    #[test]
    fn rejects_bad_dims() {
        assert!(matches!(
            characteristic_polynomial(2, &[1.0, 2.0, 3.0]),
            Err(SolversError::InvalidDimension)
        ));
    }
}
