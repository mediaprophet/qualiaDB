//! Bayesian linear regression (PRML ch 3.3) — a conjugate Gaussian model that
//! returns a **predictive distribution** (mean + variance), not just a point
//! estimate. This is the mission-aligned payoff: a model that can say "ŷ, and here
//! is how sure" — calibrated uncertainty, with the predictive variance widening
//! away from the data.
//!
//! Prior `w ~ N(0, α⁻¹I)`, noise precision `β = 1/σ²`. Posterior (PRML 3.53–3.54):
//! `S_N⁻¹ = αI + β ΦᵀΦ`, `m_N = β S_N Φᵀy`. Predictive (3.58–3.59):
//! `mean = m_Nᵀφ`, `var = 1/β + φᵀ S_N φ`. The `k×k` solves reuse
//! `linear_algebra::cholesky` (no new solver). Kernel-class `DenseLinear`.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};

/// A fitted Bayesian linear model: the posterior over weights (`mean` = `m_N`,
/// `cov` = `S_N`) and the noise precision `beta`.
#[derive(Debug, Clone)]
pub struct BayesianLinear {
    pub mean: Vec<f64>, // posterior mean m_N (k)
    pub cov: Vec<f64>,  // posterior covariance S_N (k×k)
    pub beta: f64,
    pub fit_intercept: bool,
    pub p: usize,
}

fn design_row(x_row: &[f64], fit_intercept: bool, out: &mut [f64]) {
    if fit_intercept {
        out[0] = 1.0;
        out[1..].copy_from_slice(x_row);
    } else {
        out.copy_from_slice(x_row);
    }
}

impl BayesianLinear {
    /// Fit with prior precision `alpha` (> 0) and noise precision `beta` (> 0).
    /// Fails closed on shape mismatch / non-positive hyper-parameters.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        alpha: f64,
        beta: f64,
        fit_intercept: bool,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        if !(alpha > 0.0) || !(beta > 0.0) {
            return Err(LearningError::InsufficientData);
        }
        let k = p + usize::from(fit_intercept);

        // Design matrix Φ (n × k).
        let mut phi = vec![0.0; n * k];
        for i in 0..n {
            design_row(&x[i * p..(i + 1) * p], fit_intercept, &mut phi[i * k..(i + 1) * k]);
        }

        // A = αI + β ΦᵀΦ.
        let mut a = vec![0.0; k * k];
        gemm(Transpose::Yes, Transpose::No, k, k, n, beta, &phi, &phi, 0.0, &mut a)?;
        for j in 0..k {
            a[j * k + j] += alpha;
        }
        // b = β Φᵀy.
        let mut b = vec![0.0; k];
        matvec(Transpose::Yes, k, n, &phi, y, &mut b)?;
        for v in b.iter_mut() {
            *v *= beta;
        }

        // Posterior mean: solve A m_N = b.  Posterior covariance S_N = A⁻¹.
        let mut l = vec![0.0; k * k];
        cholesky_factor(k, &a, &mut l).map_err(|_| LearningError::Singular)?;
        let mut mean = vec![0.0; k];
        cholesky_solve(k, &l, &b, &mut mean)?;
        // S_N columns via A⁻¹ = solving A·s_j = e_j.
        let mut cov = vec![0.0; k * k];
        let mut ej = vec![0.0; k];
        let mut sj = vec![0.0; k];
        for j in 0..k {
            ej.iter_mut().for_each(|v| *v = 0.0);
            ej[j] = 1.0;
            cholesky_solve(k, &l, &ej, &mut sj)?;
            for i in 0..k {
                cov[i * k + j] = sj[i];
            }
        }

        Ok(Self { mean, cov, beta, fit_intercept, p })
    }

    /// Predictive distribution at one row: `(mean, variance)`.
    /// `variance = 1/β + φᵀ S_N φ` — the noise floor plus the model uncertainty,
    /// which grows away from the training data.
    pub fn predict_row(&self, x_row: &[f64]) -> (f64, f64) {
        let k = self.mean.len();
        let mut phi = vec![0.0; k];
        design_row(x_row, self.fit_intercept, &mut phi);
        let mean: f64 = phi.iter().zip(&self.mean).map(|(p, m)| p * m).sum();
        // φᵀ S_N φ.
        let mut sphi = vec![0.0; k];
        for i in 0..k {
            sphi[i] = (0..k).map(|j| self.cov[i * k + j] * phi[j]).sum();
        }
        let model_var: f64 = phi.iter().zip(&sphi).map(|(p, s)| p * s).sum();
        (mean, 1.0 / self.beta + model_var)
    }

    /// Predictive `(mean, variance)` for each row of a row-major `m × p` matrix.
    pub fn predict(&self, x: &[f64], m: usize) -> Vec<(f64, f64)> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::regression::linear;

    #[test]
    fn posterior_mean_approaches_ols_with_weak_prior() {
        // y = 1 + 2x; a weak prior + precise likelihood → posterior mean ≈ OLS.
        let x: Vec<f64> = (0..20).map(|i| i as f64 / 2.0).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi).collect();
        let bl = BayesianLinear::fit(&x, &y, 20, 1, 1e-6, 1e6, true).unwrap();
        let ols = linear::fit(&x, &y, 20, 1, true).unwrap();
        assert!((bl.mean[0] - ols.coefficients[0]).abs() < 1e-2, "intercept {}", bl.mean[0]);
        assert!((bl.mean[1] - ols.coefficients[1]).abs() < 1e-2, "slope {}", bl.mean[1]);
    }

    #[test]
    fn predictive_variance_grows_away_from_data() {
        // Train on x∈[0,10]; predictive std should be larger when extrapolating.
        let x: Vec<f64> = (0..11).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 0.5 * xi).collect();
        let bl = BayesianLinear::fit(&x, &y, 11, 1, 1.0, 25.0, true).unwrap();
        let (_, var_center) = bl.predict_row(&[5.0]); // middle of the data
        let (_, var_far) = bl.predict_row(&[100.0]); // far extrapolation
        assert!(var_far > var_center, "var_far {var_far} should exceed var_center {var_center}");
        // Variance never drops below the noise floor 1/β.
        assert!(var_center >= 1.0 / 25.0 - 1e-12);
    }

    #[test]
    fn stronger_prior_shrinks_weights() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 3.0 * xi).collect();
        let weak = BayesianLinear::fit(&x, &y, 10, 1, 0.01, 10.0, false).unwrap();
        let strong = BayesianLinear::fit(&x, &y, 10, 1, 100.0, 10.0, false).unwrap();
        // A stronger zero-mean prior pulls the slope toward 0.
        assert!(strong.mean[0].abs() < weak.mean[0].abs());
    }

    #[test]
    fn guards() {
        assert_eq!(BayesianLinear::fit(&[1.0, 2.0], &[1.0], 2, 1, 1.0, 1.0, true).unwrap_err(), LearningError::InvalidDimension);
        assert_eq!(BayesianLinear::fit(&[1.0, 2.0], &[1.0, 2.0], 2, 1, 0.0, 1.0, true).unwrap_err(), LearningError::InsufficientData);
    }
}
