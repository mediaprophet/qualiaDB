//! Multiple linear regression (ISL ch 3) — ordinary least squares with full
//! inference. Canonical implementation lives in
//! [`crate::solvers::statistics::regression::multiple`]; this module is a thin
//! learning-crate facade preserving `LinearModel` / `fit` names.

use crate::solvers::learning::LearningError;
use crate::solvers::statistics::regression::{multiple_ols, MultipleOls};

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
    fn from_multiple(m: MultipleOls) -> Self {
        Self {
            coefficients: m.coefficients,
            fit_intercept: m.fit_intercept,
            std_errors: m.std_errors,
            t_values: m.t_values,
            p_values: m.p_values,
            r_squared: m.r_squared,
            adj_r_squared: m.adj_r_squared,
            f_statistic: m.f_statistic,
            f_p_value: m.f_p_value,
            residual_std_error: m.residual_std_error,
            df_residual: m.df_residual,
            n: m.n,
        }
    }

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
    let k = p + usize::from(fit_intercept);
    if n <= k {
        return Err(LearningError::InsufficientData);
    }
    match multiple_ols(x, y, n, p, fit_intercept) {
        Some(m) => Ok(LinearModel::from_multiple(m)),
        None => Err(LearningError::Singular),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_exact_plane() {
        let x = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0];
        let y = [1.0, 3.0, 4.0, 6.0, 8.0];
        let m = fit(&x, &y, 5, 2, true).unwrap();
        assert!((m.coefficients[0] - 1.0).abs() < 1e-9);
        assert!((m.coefficients[1] - 2.0).abs() < 1e-9);
        assert!((m.coefficients[2] - 3.0).abs() < 1e-9);
        assert!((m.r_squared - 1.0).abs() < 1e-12);
        assert!((m.predict_row(&[3.0, 2.0]) - 13.0).abs() < 1e-9);
    }

    #[test]
    fn matches_simple_regression_for_one_predictor() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.1, 3.9, 6.1, 7.9, 10.2];
        let m = fit(&x, &y, 5, 1, true).unwrap();
        let simple =
            crate::solvers::statistics::regression::simple_linear_regression(&x, &y).unwrap();
        assert!((m.coefficients[0] - simple.intercept).abs() < 1e-9);
        assert!((m.coefficients[1] - simple.slope).abs() < 1e-9);
        assert!((m.p_values[1] - simple.slope_p_value).abs() < 1e-9);
    }

    #[test]
    fn detects_collinear_predictors() {
        let x = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0, 4.0, 8.0, 5.0, 10.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(
            fit(&x, &y, 5, 2, true).unwrap_err(),
            LearningError::Singular
        );
    }

    #[test]
    fn guards_insufficient_data() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [1.0, 2.0];
        assert_eq!(
            fit(&x, &y, 2, 2, true).unwrap_err(),
            LearningError::InsufficientData
        );
    }

    #[test]
    fn significant_predictor_has_small_p() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [2.0, 4.1, 5.9, 8.0, 10.1, 12.0];
        let m = fit(&x, &y, 6, 1, true).unwrap();
        assert!(m.p_values[1] < 1e-4);
        assert!(m.f_p_value.unwrap() < 1e-4);
    }
}
