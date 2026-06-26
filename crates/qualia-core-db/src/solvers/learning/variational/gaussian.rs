//! Mean-field variational inference for a univariate Gaussian (PRML ch 10.1.3) —
//! the canonical CAVI example. Data `xₙ ~ N(μ, τ⁻¹)` with a Normal-Gamma prior; the
//! variational posterior is factorized `q(μ,τ) = q(μ)·q(τ)` (`q(μ)` Gaussian, `q(τ)`
//! Gamma), and the coordinate-ascent updates iterate to a fixed point.
//!
//! This is the worked instance of the general principle: approximate an intractable
//! posterior by the closest factorized distribution, maximizing the ELBO.

use crate::solvers::learning::LearningError;

/// The fitted factorized posterior `q(μ) = N(μ_n, λ_n⁻¹)`, `q(τ) = Gamma(a_n, b_n)`.
#[derive(Debug, Clone, Copy)]
pub struct VariationalGaussian {
    pub mu_n: f64,
    pub lambda_n: f64,
    pub a_n: f64,
    pub b_n: f64,
    pub n_iter: usize,
    pub converged: bool,
}

impl VariationalGaussian {
    /// Posterior mean of μ.
    pub fn mean(&self) -> f64 {
        self.mu_n
    }
    /// Posterior mean of the precision τ, `E[τ] = a_n/b_n`.
    pub fn precision_mean(&self) -> f64 {
        self.a_n / self.b_n
    }
    /// Implied posterior-mean variance `1/E[τ]`.
    pub fn variance_mean(&self) -> f64 {
        self.b_n / self.a_n
    }
}

/// Run CAVI for the univariate Gaussian. Priors: `μ ~ N(μ0, (λ0·τ)⁻¹)` (`mu0`,
/// `lambda0`), `τ ~ Gamma(a0, b0)`. Fails closed on too little data.
pub fn fit(
    data: &[f64],
    mu0: f64,
    lambda0: f64,
    a0: f64,
    b0: f64,
    max_iter: usize,
    tol: f64,
) -> Result<VariationalGaussian, LearningError> {
    let n = data.len();
    if n < 2 || lambda0 < 0.0 || !(a0 > 0.0) || !(b0 > 0.0) {
        return Err(LearningError::InsufficientData);
    }
    let nf = n as f64;
    let xbar = data.iter().sum::<f64>() / nf;

    // q(μ) mean is fixed by the data + prior; only its precision depends on E[τ].
    let mu_n = (lambda0 * mu0 + nf * xbar) / (lambda0 + nf);
    let a_n = a0 + (nf + 1.0) / 2.0;

    // Initialise E[τ] from the sample variance.
    let s2 = data.iter().map(|&x| (x - xbar).powi(2)).sum::<f64>() / nf;
    let mut e_tau = 1.0 / s2.max(1e-9);
    let mut lambda_n = (lambda0 + nf) * e_tau;
    let mut b_n = b0;
    let mut converged = false;
    let mut iters = 0;

    for it in 1..=max_iter.max(1) {
        iters = it;
        // q(μ): precision λ_n = (λ0 + N)·E[τ].
        lambda_n = (lambda0 + nf) * e_tau;
        let var_mu = 1.0 / lambda_n;
        // q(τ): b_n = b0 + ½·E_μ[ Σ(xₙ−μ)² + λ0(μ−μ0)² ].
        let sum_sq: f64 = data.iter().map(|&x| (x - mu_n).powi(2)).sum::<f64>() + nf * var_mu;
        let prior_term = lambda0 * ((mu_n - mu0).powi(2) + var_mu);
        let new_b = b0 + 0.5 * (sum_sq + prior_term);
        let new_e_tau = a_n / new_b;
        let delta = (new_e_tau - e_tau).abs();
        b_n = new_b;
        e_tau = new_e_tau;
        if delta < tol {
            converged = true;
            break;
        }
    }

    Ok(VariationalGaussian { mu_n, lambda_n, a_n, b_n, n_iter: iters, converged })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_mean_and_precision() {
        // Data spread around 5 with variance ~1 → E[μ]≈5, E[τ]≈1.
        let data: Vec<f64> = (0..200).map(|i| 5.0 + ((i * 37 % 100) as f64 - 50.0) / 29.0).collect();
        let q = fit(&data, 0.0, 1e-3, 1e-3, 1e-3, 100, 1e-10).unwrap();
        assert!(q.converged);
        let xbar = data.iter().sum::<f64>() / data.len() as f64;
        // mu_n is shrunk toward the prior mean (0) by lambda0 — very close to xbar
        // for a vague prior, but not exactly equal.
        assert!((q.mean() - xbar).abs() < 1e-3, "mean {} vs xbar {xbar}", q.mean());
        // E[τ] ≈ 1/sample_variance.
        let s2 = data.iter().map(|&x| (x - xbar).powi(2)).sum::<f64>() / data.len() as f64;
        assert!((q.precision_mean() - 1.0 / s2).abs() / (1.0 / s2) < 0.05, "prec {}", q.precision_mean());
    }

    #[test]
    fn tighter_data_gives_higher_precision() {
        let tight: Vec<f64> = (0..100).map(|i| 2.0 + ((i % 5) as f64 - 2.0) * 0.05).collect();
        let loose: Vec<f64> = (0..100).map(|i| 2.0 + ((i % 5) as f64 - 2.0) * 1.0).collect();
        let qt = fit(&tight, 0.0, 1e-3, 1e-3, 1e-3, 100, 1e-12).unwrap();
        let ql = fit(&loose, 0.0, 1e-3, 1e-3, 1e-3, 100, 1e-12).unwrap();
        assert!(qt.precision_mean() > ql.precision_mean());
    }

    #[test]
    fn guards() {
        assert_eq!(fit(&[1.0], 0.0, 1.0, 1.0, 1.0, 10, 1e-6).unwrap_err(), LearningError::InsufficientData);
    }
}
