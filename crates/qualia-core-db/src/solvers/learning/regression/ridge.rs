//! Ridge regression (ISL ch 6.2.1, PRML ch 3) — L2-penalized least squares.
//!
//! Minimise `‖y − Xβ‖² + λ‖β‖²` (the intercept is **not** penalized). Centering y
//! and the predictors removes the intercept from the penalized solve, leaving
//! `(XcᵀXc + λI)β = Xcᵀyc`, solved with `linear_algebra::cholesky` (the penalty
//! makes the system positive-definite even for collinear predictors — ridge's whole
//! point). Kernel-class `DenseLinear`, dispatch-ready.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};
use crate::solvers::statistics::descriptive::mean;

/// A fitted ridge model: slope coefficients (predictor-aligned) plus an
/// un-penalized intercept.
#[derive(Debug, Clone)]
pub struct RidgeModel {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub lambda: f64,
}

impl RidgeModel {
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        self.intercept
            + self
                .coefficients
                .iter()
                .zip(x_row)
                .map(|(b, x)| b * x)
                .sum::<f64>()
    }
    pub fn predict(&self, x: &[f64], n: usize, p: usize) -> Vec<f64> {
        (0..n)
            .map(|i| self.predict_row(&x[i * p..(i + 1) * p]))
            .collect()
    }
}

/// Fit ridge regression with penalty `lambda ≥ 0`. `lambda = 0` reproduces OLS.
/// Fails closed on shape mismatch / `n < 2`.
pub fn fit(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    lambda: f64,
) -> Result<RidgeModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if n < 2 || lambda < 0.0 {
        return Err(LearningError::InsufficientData);
    }

    // Column means and centred predictors; centred response.
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
    let mut yc = vec![0.0; n];
    for i in 0..n {
        for j in 0..p {
            xc[i * p + j] = x[i * p + j] - xbar[j];
        }
        yc[i] = y[i] - ybar;
    }

    // A = XcᵀXc + λI, b = Xcᵀyc.
    let mut a = vec![0.0; p * p];
    gemm(
        Transpose::Yes,
        Transpose::No,
        p,
        p,
        n,
        1.0,
        &xc,
        &xc,
        0.0,
        &mut a,
    )?;
    for j in 0..p {
        a[j * p + j] += lambda;
    }
    let mut b = vec![0.0; p];
    matvec(Transpose::Yes, p, n, &xc, &yc, &mut b)?;

    let mut l = vec![0.0; p * p];
    cholesky_factor(p, &a, &mut l).map_err(|_| LearningError::Singular)?;
    let mut coefficients = vec![0.0; p];
    cholesky_solve(p, &l, &b, &mut coefficients)?;

    // Recover the un-penalized intercept.
    let intercept = ybar
        - coefficients
            .iter()
            .zip(xbar.iter())
            .map(|(b, m)| b * m)
            .sum::<f64>();

    Ok(RidgeModel {
        coefficients,
        intercept,
        lambda,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::regression::linear;

    #[test]
    fn lambda_zero_matches_ols() {
        let x = [1.0, 2.0, 2.0, 1.0, 3.0, 0.0, 4.0, 5.0, 5.0, 4.0];
        let y = [3.0, 5.0, 4.0, 9.0, 13.0];
        let ridge = fit(&x, &y, 5, 2, 0.0).unwrap();
        let ols = linear::fit(&x, &y, 5, 2, true).unwrap();
        assert!((ridge.intercept - ols.coefficients[0]).abs() < 1e-7);
        assert!((ridge.coefficients[0] - ols.coefficients[1]).abs() < 1e-7);
        assert!((ridge.coefficients[1] - ols.coefficients[2]).abs() < 1e-7);
    }

    #[test]
    fn penalty_shrinks_coefficients() {
        let x = [1.0, 2.0, 2.0, 1.0, 3.0, 0.0, 4.0, 5.0, 5.0, 4.0];
        let y = [3.0, 5.0, 4.0, 9.0, 13.0];
        let small = fit(&x, &y, 5, 2, 0.0).unwrap();
        let big = fit(&x, &y, 5, 2, 50.0).unwrap();
        let norm = |m: &RidgeModel| m.coefficients.iter().map(|c| c * c).sum::<f64>().sqrt();
        assert!(
            norm(&big) < norm(&small),
            "ridge must shrink: {} !< {}",
            norm(&big),
            norm(&small)
        );
    }

    #[test]
    fn stable_on_collinear_where_ols_fails() {
        // x2 = 2·x1 (collinear) — OLS is singular, ridge regularizes and solves.
        let x = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0, 4.0, 8.0, 5.0, 10.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(
            linear::fit(&x, &y, 5, 2, true).unwrap_err(),
            LearningError::Singular
        );
        let ridge = fit(&x, &y, 5, 2, 1.0).unwrap();
        assert!(ridge.coefficients.iter().all(|c| c.is_finite()));
    }
}
