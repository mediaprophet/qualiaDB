//! Lasso regression (ISL ch 6.2.2) — L1-penalized least squares by cyclic
//! coordinate descent with soft-thresholding.
//!
//! Minimise `½‖y − Xβ‖² + λ‖β‖₁` (intercept not penalized; handled by centering).
//! Unlike ridge, the L1 penalty drives some coefficients **exactly to zero**
//! (variable selection). Coordinate descent updates one `βⱼ` at a time via the
//! soft-threshold operator, keeping a running residual for O(np) per sweep.
//! Scalar fit-loop → CPU (the per-coordinate dot is `Reduction`-class).

use crate::solvers::learning::LearningError;
use crate::solvers::statistics::descriptive::mean;

/// A fitted lasso model.
#[derive(Debug, Clone)]
pub struct LassoModel {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub lambda: f64,
    pub n_iter: usize,
    pub converged: bool,
}

impl LassoModel {
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        self.intercept + self.coefficients.iter().zip(x_row).map(|(b, x)| b * x).sum::<f64>()
    }
    pub fn predict(&self, x: &[f64], n: usize, p: usize) -> Vec<f64> {
        (0..n).map(|i| self.predict_row(&x[i * p..(i + 1) * p])).collect()
    }
    /// Number of non-zero coefficients (the selected variables).
    pub fn n_selected(&self) -> usize {
        self.coefficients.iter().filter(|&&c| c != 0.0).count()
    }
}

#[inline]
fn soft_threshold(z: f64, gamma: f64) -> f64 {
    if z > gamma {
        z - gamma
    } else if z < -gamma {
        z + gamma
    } else {
        0.0
    }
}

/// Fit lasso with penalty `lambda ≥ 0` by coordinate descent. `lambda = 0` reduces
/// to OLS (up to the iteration tolerance). Fails closed on shape mismatch / `n < 2`.
pub fn fit(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    lambda: f64,
    max_iter: usize,
    tol: f64,
) -> Result<LassoModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if n < 2 || lambda < 0.0 {
        return Err(LearningError::InsufficientData);
    }

    // Centre predictors and response (so the intercept drops out of the penalty).
    let mut xbar = vec![0.0; p];
    let mut colbuf = vec![0.0; n];
    for j in 0..p {
        for i in 0..n {
            colbuf[i] = x[i * p + j];
        }
        xbar[j] = mean(&colbuf).ok_or(LearningError::InsufficientData)?;
    }
    let ybar = mean(y).ok_or(LearningError::InsufficientData)?;
    let mut xc = vec![0.0; n * p];
    for i in 0..n {
        for j in 0..p {
            xc[i * p + j] = x[i * p + j] - xbar[j];
        }
    }
    // Column squared norms zⱼ = Σ Xcᵢⱼ².
    let mut znorm = vec![0.0; p];
    for j in 0..p {
        let mut s = 0.0;
        for i in 0..n {
            s += xc[i * p + j] * xc[i * p + j];
        }
        znorm[j] = s;
    }

    let mut beta = vec![0.0; p];
    // Running residual r = yc − Xc·β  (β starts at 0 → r = yc).
    let mut r: Vec<f64> = y.iter().map(|&yi| yi - ybar).collect();

    let mut converged = false;
    let mut iters = 0;
    for it in 1..=max_iter.max(1) {
        iters = it;
        let mut max_delta = 0.0_f64;
        for j in 0..p {
            if znorm[j] == 0.0 {
                continue; // constant predictor contributes nothing
            }
            // ρⱼ = Xcⱼ·r + zⱼ·βⱼ  (add back coordinate j's own contribution).
            let mut rho = znorm[j] * beta[j];
            for i in 0..n {
                rho += xc[i * p + j] * r[i];
            }
            let new = soft_threshold(rho, lambda) / znorm[j];
            let delta = new - beta[j];
            if delta != 0.0 {
                // r ← r − Xcⱼ·Δβ.
                for i in 0..n {
                    r[i] -= xc[i * p + j] * delta;
                }
                beta[j] = new;
                max_delta = max_delta.max(delta.abs());
            }
        }
        if max_delta < tol {
            converged = true;
            break;
        }
    }

    let intercept = ybar - beta.iter().zip(xbar.iter()).map(|(b, m)| b * m).sum::<f64>();
    Ok(LassoModel { coefficients: beta, intercept, lambda, n_iter: iters, converged })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::regression::linear;

    #[test]
    fn lambda_zero_approaches_ols() {
        let x = [1.0, 5.0, 2.0, 4.0, 3.0, 3.0, 4.0, 2.0, 5.0, 1.0];
        let y = [3.0, 4.0, 6.0, 8.0, 11.0];
        let lasso = fit(&x, &y, 5, 2, 0.0, 5000, 1e-10).unwrap();
        let ols = linear::fit(&x, &y, 5, 2, true).unwrap();
        assert!((lasso.coefficients[0] - ols.coefficients[1]).abs() < 1e-4);
        assert!((lasso.coefficients[1] - ols.coefficients[2]).abs() < 1e-4);
        assert!((lasso.intercept - ols.coefficients[0]).abs() < 1e-4);
    }

    #[test]
    fn large_penalty_zeros_all_coefficients() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0];
        let m = fit(&x, &y, 5, 2, 1e6, 1000, 1e-9).unwrap();
        assert_eq!(m.n_selected(), 0);
        // Intercept falls back to the response mean.
        assert!((m.intercept - 6.0).abs() < 1e-9);
    }

    #[test]
    fn selects_the_relevant_predictor() {
        // x1 drives y; x2 is pure noise → lasso should keep x1, zero x2.
        let x = [
            1.0, 0.3, 2.0, -0.1, 3.0, 0.2, 4.0, -0.3, 5.0, 0.1, 6.0, 0.0,
        ];
        let y = [2.0, 4.1, 5.9, 8.0, 10.1, 12.0]; // ≈ 2·x1
        let m = fit(&x, &y, 6, 2, 1.0, 5000, 1e-10).unwrap();
        assert!(m.converged);
        assert!(m.coefficients[0].abs() > 0.5, "x1 should be selected: {}", m.coefficients[0]);
        assert_eq!(m.coefficients[1], 0.0, "x2 (noise) should be zeroed");
    }
}
