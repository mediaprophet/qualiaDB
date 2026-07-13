//! Principal Component Analysis (ISL ch 12, PRML ch 12) — the eigendecomposition
//! of the feature covariance, reusing `linear_algebra::{gemm, eigen}` (no new
//! solver). Mission note: PCA is the principled way to choose the engine's 10D→5D
//! NQuin relevance projection.
//!
//! Centre the data, form the `p×p` covariance `C = Xcᵀ Xc /(n−1)` with `gemm`,
//! symmetric-eigendecompose it with `symmetric_eigen`, and sort the eigenpairs
//! descending. Eigenvalue `k` is the variance along principal component `k`.
//! Kernel-class `DenseLinear` (covariance GEMM), dispatch-ready.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::eigen::symmetric_eigen;
use crate::solvers::linear_algebra::gemm::{gemm, Transpose};
use crate::solvers::statistics::descriptive::mean;

/// A fitted PCA. `components` holds the principal axes as **rows** (`n_components ×
/// p`), ordered by descending explained variance; `explained_variance[k]` is the
/// variance (eigenvalue) along component `k`.
#[derive(Debug, Clone)]
pub struct Pca {
    pub mean: Vec<f64>,
    pub components: Vec<f64>, // n_components × p, row-major
    pub explained_variance: Vec<f64>,
    pub explained_variance_ratio: Vec<f64>,
    pub n_components: usize,
    pub p: usize,
}

impl Pca {
    /// Project a row-major `n × p` matrix onto the first `k` components, returning
    /// the `n × k` scores. `k` is clamped to `n_components`.
    pub fn transform(&self, x: &[f64], n: usize, k: usize) -> Option<Vec<f64>> {
        let p = self.p;
        if x.len() != n * p {
            return None;
        }
        let k = k.min(self.n_components);
        let mut out = vec![0.0; n * k];
        for i in 0..n {
            for c in 0..k {
                let comp = &self.components[c * p..(c + 1) * p];
                let mut s = 0.0;
                for j in 0..p {
                    s += (x[i * p + j] - self.mean[j]) * comp[j];
                }
                out[i * k + c] = s;
            }
        }
        Some(out)
    }
}

/// Fit PCA to a row-major `n × p` matrix. `None`-equivalent failures are returned
/// as `LearningError` (fail closed): `InvalidDimension`, `InsufficientData`
/// (`n < 2`), or `Singular` if the eigensolver cannot decompose the covariance.
pub fn fit(x: &[f64], n: usize, p: usize) -> Result<Pca, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p {
        return Err(LearningError::InvalidDimension);
    }
    if n < 2 {
        return Err(LearningError::InsufficientData);
    }

    // Column means and centred data.
    let mut col = vec![0.0; n];
    let mut means = vec![0.0; p];
    for j in 0..p {
        for i in 0..n {
            col[i] = x[i * p + j];
        }
        means[j] = mean(&col).ok_or(LearningError::InsufficientData)?;
    }
    let mut xc = vec![0.0; n * p];
    for i in 0..n {
        for j in 0..p {
            xc[i * p + j] = x[i * p + j] - means[j];
        }
    }

    // Covariance C = Xcᵀ Xc / (n-1)  (p×p, symmetric).
    let mut cov = vec![0.0; p * p];
    gemm(
        Transpose::Yes,
        Transpose::No,
        p,
        p,
        n,
        1.0 / (n as f64 - 1.0),
        &xc,
        &xc,
        0.0,
        &mut cov,
    )?;
    // Symmetrize against round-off so the eigensolver's symmetry check passes.
    for i in 0..p {
        for j in (i + 1)..p {
            let avg = 0.5 * (cov[i * p + j] + cov[j * p + i]);
            cov[i * p + j] = avg;
            cov[j * p + i] = avg;
        }
    }

    // Symmetric eigendecomposition: eigenvalues on the diagonal, eigenvectors as
    // columns of `vecs`.
    let mut vecs = vec![0.0; p * p];
    symmetric_eigen(p, &mut cov, &mut vecs).map_err(|_| LearningError::Singular)?;
    let eigvals: Vec<f64> = (0..p).map(|i| cov[i * p + i].max(0.0)).collect();

    // Order components by descending eigenvalue.
    let mut order: Vec<usize> = (0..p).collect();
    order.sort_by(|&a, &b| {
        eigvals[b]
            .partial_cmp(&eigvals[a])
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    let mut components = vec![0.0; p * p]; // p components (rows) × p
    let mut explained_variance = vec![0.0; p];
    for (rank, &e) in order.iter().enumerate() {
        explained_variance[rank] = eigvals[e];
        for j in 0..p {
            // eigenvector e is column `e` of `vecs`: vecs[j*p + e].
            components[rank * p + j] = vecs[j * p + e];
        }
    }
    let total: f64 = explained_variance.iter().sum();
    let explained_variance_ratio: Vec<f64> = explained_variance
        .iter()
        .map(|&v| if total > 0.0 { v / total } else { 0.0 })
        .collect();

    Ok(Pca {
        mean: means,
        components,
        explained_variance,
        explained_variance_ratio,
        n_components: p,
        p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_dominant_direction() {
        // Variance almost entirely along x; tiny along y → PC1 ~ x-axis, ratio ~1.
        let x = [-2.0, 0.01, -1.0, -0.01, 0.0, 0.0, 1.0, 0.01, 2.0, -0.01];
        let m = fit(&x, 5, 2).unwrap();
        assert!(
            m.explained_variance_ratio[0] > 0.99,
            "ratio {}",
            m.explained_variance_ratio[0]
        );
        // PC1 aligns with the x-axis (|component_x| ~ 1, |component_y| ~ 0).
        assert!(m.components[0].abs() > 0.99 && m.components[1].abs() < 0.05);
        // Ratios sum to 1.
        let s: f64 = m.explained_variance_ratio.iter().sum();
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn diagonal_correlation_axis() {
        // Perfectly correlated x=y → PC1 along the (1,1)/√2 diagonal, PC2 ~ 0 var.
        let x = [1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0];
        let m = fit(&x, 5, 2).unwrap();
        assert!(m.explained_variance_ratio[0] > 0.999);
        let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
        // PC1 components both ≈ ±1/√2.
        assert!((m.components[0].abs() - inv_sqrt2).abs() < 1e-6);
        assert!((m.components[1].abs() - inv_sqrt2).abs() < 1e-6);
        // Second component carries ~no variance.
        assert!(m.explained_variance[1] < 1e-9);
    }

    #[test]
    fn transform_projects_and_decorrelates() {
        let x = [1.0, 1.0, 2.0, 2.0, 3.0, 3.1, 4.0, 3.9, 5.0, 5.0];
        let m = fit(&x, 5, 2).unwrap();
        let scores = m.transform(&x, 5, 1).unwrap();
        assert_eq!(scores.len(), 5);
        // Scores are centered (mean ~0) because the data was centered.
        let mean_score: f64 = scores.iter().sum::<f64>() / 5.0;
        assert!(mean_score.abs() < 1e-9);
    }

    #[test]
    fn total_explained_variance_equals_total_variance() {
        let x = [2.0, 1.0, 4.0, 3.0, 6.0, 2.0, 8.0, 5.0, 10.0, 4.0];
        let m = fit(&x, 5, 2).unwrap();
        // Sum of eigenvalues == trace of covariance == sum of per-feature variances.
        use crate::solvers::statistics::descriptive::variance;
        let mut total_var = 0.0;
        for j in 0..2 {
            let col: Vec<f64> = (0..5).map(|i| x[i * 2 + j]).collect();
            total_var += variance(&col, true).unwrap();
        }
        let sum_eig: f64 = m.explained_variance.iter().sum();
        assert!(
            (sum_eig - total_var).abs() < 1e-9,
            "{sum_eig} vs {total_var}"
        );
    }

    #[test]
    fn guards() {
        assert_eq!(
            fit(&[1.0, 2.0], 1, 2).unwrap_err(),
            LearningError::InsufficientData
        );
        assert_eq!(
            fit(&[1.0, 2.0, 3.0], 2, 2).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
