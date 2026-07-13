//! Gaussian Process regression (PRML ch 6.4) — a nonparametric Bayesian regressor
//! that returns a full predictive distribution `(mean, variance)` at every input,
//! with the squared-exponential (RBF) kernel. The training solve reuses
//! `linear_algebra::cholesky` (no new solver). Kernel-class `DenseLinear` (the
//! `n×n` kernel solve) + `AllPairs` (the kernel evaluations).
//!
//! Given training `(X, y)` and noise variance `σ²ₙ`:
//! `mean(x*) = k*ᵀ (K + σ²ₙI)⁻¹ y`,
//! `var(x*)  = k(x*,x*) − k*ᵀ (K + σ²ₙI)⁻¹ k*`,
//! the calibrated uncertainty that collapses near training points and widens away
//! from them.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};

/// A fitted GP regressor (squared-exponential kernel).
#[derive(Debug, Clone)]
pub struct GpRegressor {
    x_train: Vec<f64>,
    alpha: Vec<f64>, // (K + σ²ₙI)⁻¹ y
    l: Vec<f64>,     // Cholesky factor of (K + σ²ₙI), n×n
    length_scale: f64,
    signal_var: f64,
    noise_var: f64,
    n: usize,
    p: usize,
}

/// Squared-exponential kernel `σ²_f · exp(−‖a−b‖² / (2ℓ²))`.
fn rbf(a: &[f64], b: &[f64], length_scale: f64, signal_var: f64) -> f64 {
    let d2: f64 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    signal_var * (-d2 / (2.0 * length_scale * length_scale)).exp()
}

impl GpRegressor {
    /// Fit a GP with squared-exponential kernel (`length_scale ℓ`, `signal_var σ²_f`)
    /// and Gaussian noise `noise_var σ²ₙ`. Fails closed on shape mismatch /
    /// non-positive hyper-parameters / a non-PD kernel matrix.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        length_scale: f64,
        signal_var: f64,
        noise_var: f64,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        if !(length_scale > 0.0) || !(signal_var > 0.0) || !(noise_var > 0.0) {
            return Err(LearningError::InsufficientData);
        }

        // K + σ²ₙ I.
        let mut k = vec![0.0; n * n];
        for i in 0..n {
            for j in i..n {
                let v = rbf(
                    &x[i * p..(i + 1) * p],
                    &x[j * p..(j + 1) * p],
                    length_scale,
                    signal_var,
                );
                k[i * n + j] = v;
                k[j * n + i] = v;
            }
            k[i * n + i] += noise_var;
        }

        let mut l = vec![0.0; n * n];
        cholesky_factor(n, &k, &mut l).map_err(|_| LearningError::Singular)?;
        let mut alpha = vec![0.0; n];
        cholesky_solve(n, &l, y, &mut alpha)?;

        Ok(Self {
            x_train: x.to_vec(),
            alpha,
            l,
            length_scale,
            signal_var,
            noise_var,
            n,
            p,
        })
    }

    /// Training noise variance σ²ₙ used when fitting the regressor.
    pub fn noise_variance(&self) -> f64 {
        self.noise_var
    }

    /// Predictive distribution at one input: `(mean, variance)`. The variance
    /// includes the model uncertainty (small near training data, large away from it).
    pub fn predict_row(&self, x_star: &[f64]) -> (f64, f64) {
        // k* = [k(x*, xᵢ)].
        let mut kstar = vec![0.0; self.n];
        for i in 0..self.n {
            kstar[i] = rbf(
                &self.x_train[i * self.p..(i + 1) * self.p],
                x_star,
                self.length_scale,
                self.signal_var,
            );
        }
        let mean: f64 = kstar.iter().zip(&self.alpha).map(|(k, a)| k * a).sum();
        // var = k(x*,x*) − k*ᵀ (K+σ²I)⁻¹ k*.
        let mut v = vec![0.0; self.n];
        let _ = cholesky_solve(self.n, &self.l, &kstar, &mut v);
        let reduction: f64 = kstar.iter().zip(&v).map(|(k, vi)| k * vi).sum();
        let kxx = self.signal_var; // k(x*,x*) for the SE kernel
        let floor = self.noise_var;
        (mean, (kxx - reduction).max(floor))
    }

    /// Predictive `(mean, variance)` for each row of a row-major `m × p` matrix.
    pub fn predict(&self, x: &[f64], m: usize) -> Vec<(f64, f64)> {
        (0..m)
            .map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_training_points_with_low_variance() {
        // Low noise → the GP nearly interpolates the training targets, with small
        // predictive variance there.
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| (xi * 0.7).sin()).collect();
        let gp = GpRegressor::fit(&x, &y, 8, 1, 1.0, 1.0, 1e-6).unwrap();
        for i in 0..8 {
            let (m, v) = gp.predict_row(&[x[i]]);
            assert!(
                (m - y[i]).abs() < 1e-2,
                "mean at train point {i}: {m} vs {}",
                y[i]
            );
            assert!(v < 1e-2, "variance at train point should be small: {v}");
        }
    }

    #[test]
    fn variance_grows_far_from_data() {
        let x: Vec<f64> = (0..6).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 0.5 * xi).collect();
        let gp = GpRegressor::fit(&x, &y, 6, 1, 1.0, 1.0, 1e-4).unwrap();
        let (_, v_near) = gp.predict_row(&[2.5]); // within the data
        let (_, v_far) = gp.predict_row(&[50.0]); // far away
        assert!(
            v_far > v_near,
            "var_far {v_far} should exceed var_near {v_near}"
        );
        // Far from all data the predictive variance tends to the prior signal var.
        assert!((v_far - 1.0).abs() < 1e-3);
    }

    #[test]
    fn predicts_a_smooth_function_between_points() {
        // A GP on y=sin(x) predicts a sensible value between training points.
        let x: Vec<f64> = (0..13).map(|i| i as f64 * 0.5).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();
        let gp = GpRegressor::fit(&x, &y, 13, 1, 1.0, 1.0, 1e-6).unwrap();
        let (m, _) = gp.predict_row(&[1.25]); // between 1.0 and 1.5
        assert!(
            (m - (1.25f64).sin()).abs() < 0.1,
            "interp {m} vs {}",
            (1.25f64).sin()
        );
    }

    #[test]
    fn guards() {
        assert_eq!(
            GpRegressor::fit(&[1.0, 2.0], &[1.0], 2, 1, 1.0, 1.0, 1e-3).unwrap_err(),
            LearningError::InvalidDimension
        );
        assert_eq!(
            GpRegressor::fit(&[1.0, 2.0], &[1.0, 2.0], 2, 1, 0.0, 1.0, 1e-3).unwrap_err(),
            LearningError::InsufficientData
        );
    }
}
