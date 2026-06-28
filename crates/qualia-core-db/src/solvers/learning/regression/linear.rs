//! Multiple linear regression (ISL ch 3) — ordinary least squares with full
//! inference, solved through the engine's linear-algebra library (no re-implemented
//! solver) and the statistics distributions (real p-values).
//!
//! Fit `y = β₀ + β₁x₁ + … + β_p x_p` by the normal equations `(XᵀX)β = Xᵀy`, formed
//! with `linear_algebra::gemm`/`matvec` and solved (and inverted, for the coefficient
//! standard errors) with `linear_algebra::cholesky`. Inference — t-tests on each
//! coefficient and the overall F-test — uses `statistics::distributions`.
//!
//! Kernel-class: `DenseLinear` (the GEMM/solve), so it is dispatch-ready against
//! `ComputePolicy`; for the small p×p normal-equations solve the CPU path is the
//! right one — the GPU win is in forming `XᵀX` for large n, wired with the bridge.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};
use crate::solvers::statistics::descriptive::mean;
use crate::solvers::statistics::distributions::{fisher_f, students_t};

/// A fitted OLS model with inferential output. When `fit_intercept` is true,
/// `coefficients[0]` is the intercept and `coefficients[1..]` align with the
/// predictor columns; the `*_per_coef` vectors are aligned the same way.
#[derive(Debug, Clone)]
pub struct LinearModel {
    pub coefficients: Vec<f64>,
    pub fit_intercept: bool,
    pub std_errors: Vec<f64>,
    pub t_values: Vec<f64>,
    pub p_values: Vec<f64>,
    pub r_squared: f64,
    pub adj_r_squared: f64,
    /// Overall F-statistic (all slopes = 0) and its p-value. `None` without an intercept.
    pub f_statistic: Option<f64>,
    pub f_p_value: Option<f64>,
    pub residual_std_error: f64,
    pub df_residual: usize,
    pub n: usize,
}

impl LinearModel {
    /// Predict for one feature row (length `p`, predictors only — the intercept is
    /// applied internally).
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        let (b0, betas) = if self.fit_intercept {
            (self.coefficients[0], &self.coefficients[1..])
        } else {
            (0.0, &self.coefficients[..])
        };
        b0 + betas.iter().zip(x_row).map(|(b, x)| b * x).sum::<f64>()
    }

    /// Predict for a row-major `n × p` feature matrix.
    pub fn predict(&self, x: &[f64], n: usize, p: usize) -> Vec<f64> {
        (0..n)
            .map(|i| self.predict_row(&x[i * p..(i + 1) * p]))
            .collect()
    }
}

/// Fit OLS of `y` (length `n`) on a row-major `n × p` predictor matrix `x`.
/// `fit_intercept` prepends a constant column. Fails closed:
/// `InvalidDimension` on a shape mismatch, `InsufficientData` if `n ≤ params`,
/// `Singular` on collinear predictors.
pub fn fit(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    fit_intercept: bool,
) -> Result<LinearModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    let k = p + usize::from(fit_intercept); // total parameters
    if n <= k {
        return Err(LearningError::InsufficientData);
    }

    // Build the design matrix D (n × k), row-major, with a leading 1s column if
    // an intercept is fit.
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

    // Normal equations: A = DᵀD (k×k), b = Dᵀy (k).
    let mut a = vec![0.0; k * k];
    gemm(
        Transpose::Yes,
        Transpose::No,
        k,
        k,
        n,
        1.0,
        &d,
        &d,
        0.0,
        &mut a,
    )?;
    let mut b = vec![0.0; k];
    matvec(Transpose::Yes, k, n, &d, y, &mut b)?;

    // Cholesky factor of the SPD Gram matrix; fail closed if not positive-definite
    // (collinear predictors).
    let mut l = vec![0.0; k * k];
    cholesky_factor(k, &a, &mut l).map_err(|_| LearningError::Singular)?;

    // Coefficients: solve A·β = b.
    let mut coefficients = vec![0.0; k];
    cholesky_solve(k, &l, &b, &mut coefficients)?;

    // Residuals and sums of squares.
    let mut yhat = vec![0.0; n];
    matvec(Transpose::No, n, k, &d, &coefficients, &mut yhat)?;
    let ybar = mean(y).ok_or(LearningError::InsufficientData)?;
    let mut sse = 0.0;
    let mut sst = 0.0;
    for i in 0..n {
        sse += (y[i] - yhat[i]).powi(2);
        sst += (y[i] - ybar).powi(2);
    }
    let df_residual = n - k;
    let sigma2 = sse / df_residual as f64;
    let residual_std_error = sigma2.sqrt();

    // (XᵀX)⁻¹ diagonal for coefficient standard errors: solve A·cⱼ = eⱼ via the
    // existing Cholesky factor and read cⱼ[j].
    let mut std_errors = vec![0.0; k];
    let mut t_values = vec![0.0; k];
    let mut p_values = vec![0.0; k];
    let df = df_residual as f64;
    let mut ej = vec![0.0; k];
    let mut cj = vec![0.0; k];
    for j in 0..k {
        ej.iter_mut().for_each(|v| *v = 0.0);
        ej[j] = 1.0;
        cholesky_solve(k, &l, &ej, &mut cj)?;
        let var = sigma2 * cj[j];
        let se = if var > 0.0 { var.sqrt() } else { 0.0 };
        std_errors[j] = se;
        if se > 0.0 {
            let t = coefficients[j] / se;
            t_values[j] = t;
            p_values[j] = students_t::two_sided_p(t, df);
        } else {
            t_values[j] = if coefficients[j] == 0.0 {
                0.0
            } else {
                f64::INFINITY
            };
            p_values[j] = if coefficients[j] == 0.0 { 1.0 } else { 0.0 };
        }
    }

    let r_squared = if sst > 0.0 { 1.0 - sse / sst } else { 1.0 };
    let adj_r_squared = if df_residual > 0 && sst > 0.0 {
        1.0 - (1.0 - r_squared) * (n as f64 - 1.0) / df as f64
    } else {
        r_squared
    };

    // Overall F-test (only meaningful with an intercept): F = MSR/MSE.
    let (f_statistic, f_p_value) = if fit_intercept && p >= 1 && sst > 0.0 {
        let df_model = p as f64;
        let f = ((sst - sse) / df_model) / sigma2;
        (Some(f), Some(fisher_f::upper_p(f, df_model, df)))
    } else {
        (None, None)
    };

    Ok(LinearModel {
        coefficients,
        fit_intercept,
        std_errors,
        t_values,
        p_values,
        r_squared,
        adj_r_squared,
        f_statistic,
        f_p_value,
        residual_std_error,
        df_residual,
        n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_exact_plane() {
        // y = 1 + 2·x1 + 3·x2 exactly, 5 points.
        let x = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0];
        let y = [1.0, 3.0, 4.0, 6.0, 8.0]; // 1+2x1+3x2
        let m = fit(&x, &y, 5, 2, true).unwrap();
        assert!(
            (m.coefficients[0] - 1.0).abs() < 1e-9,
            "intercept {}",
            m.coefficients[0]
        );
        assert!(
            (m.coefficients[1] - 2.0).abs() < 1e-9,
            "b1 {}",
            m.coefficients[1]
        );
        assert!(
            (m.coefficients[2] - 3.0).abs() < 1e-9,
            "b2 {}",
            m.coefficients[2]
        );
        assert!((m.r_squared - 1.0).abs() < 1e-12);
        // Prediction matches.
        assert!((m.predict_row(&[3.0, 2.0]) - (1.0 + 6.0 + 6.0)).abs() < 1e-9);
    }

    #[test]
    fn matches_simple_regression_for_one_predictor() {
        // Compare against the closed-form simple OLS already in statistics.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.1, 3.9, 6.1, 7.9, 10.2];
        let m = fit(&x, &y, 5, 1, true).unwrap();
        let simple =
            crate::solvers::statistics::regression::simple_linear_regression(&x, &y).unwrap();
        assert!((m.coefficients[0] - simple.intercept).abs() < 1e-9);
        assert!((m.coefficients[1] - simple.slope).abs() < 1e-9);
        // Slope p-value agrees with the simple-regression module.
        assert!((m.p_values[1] - simple.slope_p_value).abs() < 1e-9);
    }

    #[test]
    fn detects_collinear_predictors() {
        // x2 = 2·x1 → singular normal equations → fail closed (no bogus fit).
        let x = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0, 4.0, 8.0, 5.0, 10.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(
            fit(&x, &y, 5, 2, true).unwrap_err(),
            LearningError::Singular
        );
    }

    #[test]
    fn guards_insufficient_data() {
        // 2 samples, 2 predictors + intercept = 3 params → n ≤ k.
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [1.0, 2.0];
        assert_eq!(
            fit(&x, &y, 2, 2, true).unwrap_err(),
            LearningError::InsufficientData
        );
    }

    #[test]
    fn significant_predictor_has_small_p() {
        // Strong linear signal in x1, noise-free → tiny p-value for its slope.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [2.0, 4.1, 5.9, 8.0, 10.1, 12.0];
        let m = fit(&x, &y, 6, 1, true).unwrap();
        assert!(m.p_values[1] < 1e-4);
        assert!(m.f_p_value.unwrap() < 1e-4);
    }
}
