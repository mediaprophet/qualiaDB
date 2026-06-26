//! Generalized linear models (ISL ch 4) — logistic and Poisson regression by
//! iteratively reweighted least squares (IRLS).
//!
//! Each IRLS step is a weighted least-squares solve of `(DᵀWD)β = DᵀWz`, done with
//! the engine's `linear_algebra::cholesky` (no re-implemented solver); the Wald
//! standard errors come from `(DᵀWD)⁻¹` at convergence and the p-values from the
//! Normal CDF in `statistics::distributions`. Kernel-class: `DenseLinear` per step
//! (dispatch-ready); the IRLS loop itself is scalar CPU.

pub mod family;
pub mod multinomial;

pub use family::Family;
pub use multinomial::MultinomialLogistic;

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{matvec, Transpose};
use crate::solvers::statistics::distributions::normal;

/// A fitted GLM. `coefficients[0]` is the intercept when `fit_intercept`.
#[derive(Debug, Clone)]
pub struct GlmModel {
    pub family: Family,
    pub coefficients: Vec<f64>,
    pub fit_intercept: bool,
    pub std_errors: Vec<f64>,
    /// Wald z-statistics (coefficient / std error).
    pub z_values: Vec<f64>,
    /// Two-sided Wald p-values from the Normal CDF.
    pub p_values: Vec<f64>,
    pub n_iter: usize,
    pub converged: bool,
    /// Residual deviance `Σ unit_deviance(yᵢ, μ̂ᵢ)`.
    pub deviance: f64,
    pub n: usize,
}

impl GlmModel {
    /// Linear predictor `η` for one predictor row (length `p`).
    pub fn eta_row(&self, x_row: &[f64]) -> f64 {
        let (b0, betas) = if self.fit_intercept {
            (self.coefficients[0], &self.coefficients[1..])
        } else {
            (0.0, &self.coefficients[..])
        };
        b0 + betas.iter().zip(x_row).map(|(b, x)| b * x).sum::<f64>()
    }

    /// Predicted mean `μ = g⁻¹(η)` for one predictor row — a probability for
    /// logistic, an expected count for Poisson.
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        self.family.inv_link(self.eta_row(x_row))
    }

    /// Predicted means for a row-major `n × p` matrix.
    pub fn predict(&self, x: &[f64], n: usize, p: usize) -> Vec<f64> {
        (0..n).map(|i| self.predict_row(&x[i * p..(i + 1) * p])).collect()
    }
}

const MAX_ITER: usize = 100;
const TOL: f64 = 1e-10;

/// Fit a GLM of `y` (length `n`) on a row-major `n × p` predictor matrix by IRLS.
/// `fit_intercept` prepends a constant column. Fails closed: `InvalidDimension`,
/// `InsufficientData` (`n ≤ params`), `Singular` (collinear / perfectly separated),
/// `NotConverged`.
pub fn fit(
    family: Family,
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    fit_intercept: bool,
) -> Result<GlmModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    let k = p + usize::from(fit_intercept);
    if n <= k {
        return Err(LearningError::InsufficientData);
    }

    // Design matrix D (n × k).
    let mut d = vec![0.0; n * k];
    for i in 0..n {
        let base = i * k;
        if fit_intercept {
            d[base] = 1.0;
            d[base + 1..base + k].copy_from_slice(&x[i * p..(i + 1) * p]);
        } else {
            d[base..base + k].copy_from_slice(&x[i * p..(i + 1) * p]);
        }
    }

    let mut beta = vec![0.0; k];
    // Initialise η from a safe starting mean.
    let mut eta: Vec<f64> = y.iter().map(|&yi| {
        let mu = family.start_mu(yi);
        // η₀ = link(μ₀); use a tiny IRLS-friendly init via inverse of inv_link.
        match family {
            Family::Binomial => (mu / (1.0 - mu)).ln(),
            Family::Poisson => mu.ln(),
        }
    }).collect();

    let mut a = vec![0.0; k * k];
    let mut b = vec![0.0; k];
    let mut l = vec![0.0; k * k];
    let mut converged = false;
    let mut iters = 0;

    let mut w = vec![0.0; n];
    let mut z = vec![0.0; n];

    for it in 1..=MAX_ITER {
        iters = it;
        // Working weights and response.
        for i in 0..n {
            let mu = family.inv_link(eta[i]);
            let dmu = family.dmu_deta(mu).max(1e-12);
            let var = family.variance(mu).max(1e-12);
            w[i] = dmu * dmu / var;
            z[i] = eta[i] + (y[i] - mu) / dmu;
        }
        // A = DᵀWD, b = DᵀWz.
        for r in 0..k {
            for c in 0..k {
                let mut s = 0.0;
                for i in 0..n {
                    s += d[i * k + r] * w[i] * d[i * k + c];
                }
                a[r * k + c] = s;
            }
            let mut s = 0.0;
            for i in 0..n {
                s += d[i * k + r] * w[i] * z[i];
            }
            b[r] = s;
        }
        cholesky_factor(k, &a, &mut l).map_err(|_| LearningError::Singular)?;
        let mut beta_new = vec![0.0; k];
        cholesky_solve(k, &l, &b, &mut beta_new)?;

        // Update η and check convergence on the coefficient change.
        let mut delta = 0.0;
        for j in 0..k {
            delta += (beta_new[j] - beta[j]).powi(2);
        }
        beta = beta_new;
        matvec(Transpose::No, n, k, &d, &beta, &mut eta)?;
        if delta.sqrt() < TOL {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(LearningError::NotConverged);
    }

    // Wald standard errors from (DᵀWD)⁻¹ at the final weights (a, l already hold
    // the last factor). Deviance from the fitted means.
    let mut std_errors = vec![0.0; k];
    let mut z_values = vec![0.0; k];
    let mut p_values = vec![0.0; k];
    let mut ej = vec![0.0; k];
    let mut cj = vec![0.0; k];
    for j in 0..k {
        ej.iter_mut().for_each(|v| *v = 0.0);
        ej[j] = 1.0;
        cholesky_solve(k, &l, &ej, &mut cj)?;
        let se = if cj[j] > 0.0 { cj[j].sqrt() } else { 0.0 };
        std_errors[j] = se;
        if se > 0.0 {
            let zv = beta[j] / se;
            z_values[j] = zv;
            p_values[j] = normal::two_sided_p(zv);
        } else {
            z_values[j] = if beta[j] == 0.0 { 0.0 } else { f64::INFINITY };
            p_values[j] = if beta[j] == 0.0 { 1.0 } else { 0.0 };
        }
    }

    let mut deviance = 0.0;
    for i in 0..n {
        deviance += family.unit_deviance(y[i], family.inv_link(eta[i]));
    }

    Ok(GlmModel {
        family,
        coefficients: beta,
        fit_intercept,
        std_errors,
        z_values,
        p_values,
        n_iter: iters,
        converged,
        deviance,
        n,
    })
}

/// Convenience: logistic regression (Bernoulli `y ∈ {0,1}`).
pub fn fit_logistic(x: &[f64], y: &[f64], n: usize, p: usize, fit_intercept: bool) -> Result<GlmModel, LearningError> {
    fit(Family::Binomial, x, y, n, p, fit_intercept)
}

/// Convenience: Poisson regression (count `y ≥ 0`).
pub fn fit_poisson(x: &[f64], y: &[f64], n: usize, p: usize, fit_intercept: bool) -> Result<GlmModel, LearningError> {
    fit(Family::Poisson, x, y, n, p, fit_intercept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_recovers_positive_trend() {
        // Non-separable (so the MLE is finite) but with a clear upward trend:
        // mostly 0 at low x, mostly 1 at high x, with overlap in the middle.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let m = fit_logistic(&x, &y, 10, 1, true).unwrap();
        assert!(m.converged);
        assert!(m.coefficients[1] > 0.0, "slope should be positive: {}", m.coefficients[1]);
        // Predicted probability is higher for a large x than a small one.
        assert!(m.predict_row(&[10.0]) > m.predict_row(&[1.0]));
        assert!(m.predict_row(&[10.0]) > 0.5 && m.predict_row(&[1.0]) < 0.5);
        assert!(m.deviance.is_finite());
    }

    #[test]
    fn logistic_significance_and_inference() {
        // A stronger, larger non-separable signal → significant positive slope.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let y = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let m = fit_logistic(&x, &y, 12, 1, true).unwrap();
        assert!(m.coefficients[1] > 0.0);
        assert!(m.p_values[1] < 0.1, "trend should be significant: p={}", m.p_values[1]);
        assert!(m.std_errors[1] > 0.0 && m.std_errors[1].is_finite());
    }

    #[test]
    fn poisson_recovers_log_linear_rate() {
        // y ≈ exp(0.5 + 0.3 x); fit should recover a positive slope near 0.3.
        let x: [f64; 8] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let y: Vec<f64> = x.iter().map(|&xi| (0.5 + 0.3 * xi).exp().round()).collect();
        let m = fit_poisson(&x, &y, 8, 1, true).unwrap();
        assert!(m.converged);
        assert!((m.coefficients[1] - 0.3).abs() < 0.1, "slope {}", m.coefficients[1]);
        assert!((m.coefficients[0] - 0.5).abs() < 0.2, "intercept {}", m.coefficients[0]);
    }

    #[test]
    fn guards() {
        assert_eq!(fit_logistic(&[1.0, 2.0], &[1.0], 2, 1, true).unwrap_err(), LearningError::InvalidDimension);
        assert_eq!(fit_logistic(&[1.0, 2.0], &[1.0, 0.0], 2, 1, true).unwrap_err(), LearningError::InsufficientData);
    }
}
