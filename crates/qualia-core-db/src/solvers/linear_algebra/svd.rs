//! Thin singular value decomposition `A = U·Σ·Vᵀ` of a row-major `m×n` matrix.
//!
//! Computed from the symmetric eigendecomposition of `AᵀA` (right singular vectors +
//! squared singular values), then `U = A·V·Σ⁻¹`. Builds on the engine's GEMM-style
//! accumulation and [`super::eigen::symmetric_eigen`] — the engine's single eigen home,
//! so there is no second Jacobi here.
//!
//! Allocating (the outputs are inherently dynamic), but all scratch is local and the
//! algorithm reads caller-owned input. The specialized lib keeps a thin facade.

use crate::solvers::linear_algebra::eigen::symmetric_eigen;
use crate::solvers::SolversError;

/// Result of a (thin) SVD `A = U·Σ·Vᵀ`. `singular_values` (length `n`, descending) is
/// the diagonal of Σ; `u` is row-major `m×n` with left singular vectors as columns; `v`
/// is row-major `n×n` with right singular vectors as columns.
/// Reconstruction: `A[i][j] = Σ_k u[i][k]·σ_k·v[j][k]`.
#[derive(Debug, Clone)]
pub struct Svd {
    pub singular_values: Vec<f64>,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
}

/// Singular value decomposition of a row-major `m×n` matrix. Singular values are
/// returned in descending order. Returns [`SolversError::InvalidDimension`] for an
/// empty/mis-sized input (and propagates eigen failures).
pub fn svd(m: usize, n: usize, data: &[f64]) -> Result<Svd, SolversError> {
    if m == 0 || n == 0 || data.len() != m * n {
        return Err(SolversError::InvalidDimension);
    }

    // M = AᵀA  (n×n, symmetric positive semi-definite). Routed through the engine GEMM,
    // which offloads `Aᵀ·A` to the best path on this machine above `GEMM_GPU_THRESHOLD`
    // and runs the exact f64 CPU floor otherwise (byte-identical off-accelerator). For a
    // tall `A` (m ≫ n) this n²·m product is the SVD's heaviest step.
    let mut ata = vec![0.0_f64; n * n];
    super::gemm::gemm(
        super::gemm::Transpose::Yes,
        super::gemm::Transpose::No,
        n,
        n,
        m,
        1.0,
        data,
        data,
        0.0,
        &mut ata,
    )?;

    // Engine symmetric eigensolver: ata's diagonal ← eigenvalues, eigvecs ← eigenvectors.
    let mut eigvecs = vec![0.0_f64; n * n];
    symmetric_eigen(n, &mut ata, &mut eigvecs)?;
    let eigvals: Vec<f64> = (0..n).map(|i| ata[i * n + i]).collect();

    // Sort columns by descending eigenvalue (= descending σ²).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        eigvals[j]
            .partial_cmp(&eigvals[i])
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    let mut singular_values = vec![0.0_f64; n];
    let mut v = vec![0.0_f64; n * n];
    for (new_col, &old_col) in order.iter().enumerate() {
        singular_values[new_col] = eigvals[old_col].max(0.0).sqrt();
        for row in 0..n {
            v[row * n + new_col] = eigvecs[row * n + old_col];
        }
    }

    // U[:,k] = A·V[:,k] / σ_k  (zero column when σ_k ≈ 0). Compute the full `AV = A·V`
    // (m×n) once through the engine GEMM (best-path offload above threshold; exact CPU
    // floor otherwise), then column-scale by 1/σ_k.
    let mut av = vec![0.0_f64; m * n];
    super::gemm::matmul(m, n, n, data, &v, &mut av)?;
    let mut u = vec![0.0_f64; m * n];
    let smax = singular_values.first().copied().unwrap_or(0.0).max(1.0);
    for k in 0..n {
        let sigma = singular_values[k];
        if sigma <= 1e-12 * smax {
            continue;
        }
        for i in 0..m {
            u[i * n + k] = av[i * n + k] / sigma;
        }
    }

    Ok(Svd {
        singular_values,
        u,
        v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_square() {
        // A·V = U·Σ ⇒ A[i][j] = Σ_k u[i][k]·σ_k·v[j][k].
        let m = 3;
        let n = 3;
        let a = [4.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 5.0];
        let s = svd(m, n, &a).unwrap();
        // Singular values of a diagonal matrix are |diagonal|, descending: 5,4,3.
        assert!((s.singular_values[0] - 5.0).abs() < 1e-9);
        assert!((s.singular_values[1] - 4.0).abs() < 1e-9);
        assert!((s.singular_values[2] - 3.0).abs() < 1e-9);
        for i in 0..m {
            for j in 0..n {
                let mut acc = 0.0;
                for k in 0..n {
                    acc += s.u[i * n + k] * s.singular_values[k] * s.v[j * n + k];
                }
                assert!(
                    (acc - a[i * n + j]).abs() < 1e-6,
                    "({i},{j}) {acc} != {}",
                    a[i * n + j]
                );
            }
        }
    }

    #[test]
    fn reconstructs_tall_rectangular() {
        let m = 4;
        let n = 2;
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let s = svd(m, n, &a).unwrap();
        for i in 0..m {
            for j in 0..n {
                let mut acc = 0.0;
                for k in 0..n {
                    acc += s.u[i * n + k] * s.singular_values[k] * s.v[j * n + k];
                }
                assert!(
                    (acc - a[i * n + j]).abs() < 1e-6,
                    "({i},{j}) {acc} != {}",
                    a[i * n + j]
                );
            }
        }
        // Descending order.
        assert!(s.singular_values[0] >= s.singular_values[1]);
    }

    #[test]
    fn rejects_bad_dims() {
        assert!(matches!(
            svd(2, 2, &[1.0, 2.0, 3.0]),
            Err(SolversError::InvalidDimension)
        ));
    }
}
