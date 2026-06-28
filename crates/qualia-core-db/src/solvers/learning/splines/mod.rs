//! Regression splines & polynomial regression (ISL ch 7) — flexible non-linear
//! fits expressed as a linear model in a fixed basis, then solved by OLS
//! (`learning::regression::linear`, no new solver).
//!
//! A degree-`d` spline with interior knots `k₁…k_K` uses the **truncated power
//! basis** `[1, x, …, xᵈ, (x−k₁)ᵈ₊, …, (x−k_K)ᵈ₊]`; polynomial regression is the
//! special case with no knots. The basis columns form the design matrix; the fit is
//! ordinary least squares over them (kernel-class `DenseLinear`).

pub mod gam;
pub mod smoothing;
pub use gam::Gam;
pub use smoothing::SmoothingSpline;

use crate::solvers::learning::regression::linear;
use crate::solvers::learning::LearningError;

/// A fitted regression spline (or polynomial, when `knots` is empty).
#[derive(Debug, Clone)]
pub struct RegressionSpline {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub coefficients: Vec<f64>,
}

/// Number of basis columns for `degree` and `n_knots` interior knots.
pub(crate) fn basis_len(degree: usize, n_knots: usize) -> usize {
    (degree + 1) + n_knots
}

/// Evaluate the truncated-power basis row for a scalar `x` into `out`
/// (length `basis_len`).
pub(crate) fn basis_row(x: f64, degree: usize, knots: &[f64], out: &mut [f64]) {
    // Polynomial part 1, x, …, xᵈ.
    let mut pw = 1.0;
    for c in out.iter_mut().take(degree + 1) {
        *c = pw;
        pw *= x;
    }
    // Truncated power terms (x − kⱼ)ᵈ₊.
    for (j, &k) in knots.iter().enumerate() {
        let d = x - k;
        out[degree + 1 + j] = if d > 0.0 { d.powi(degree as i32) } else { 0.0 };
    }
}

impl RegressionSpline {
    /// Fit a degree-`degree` regression spline of `y` on scalar `x` (length `n`)
    /// with the given interior `knots`. `degree = 3` is the usual cubic spline;
    /// `knots = []` gives polynomial regression. Fails closed via the OLS solver.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        degree: usize,
        knots: &[f64],
    ) -> Result<Self, LearningError> {
        if n == 0 || x.len() != n || y.len() != n || degree == 0 {
            return Err(LearningError::InvalidDimension);
        }
        let m = basis_len(degree, knots.len());
        // Build the n × m design matrix (basis already contains the constant column,
        // so OLS is fit WITHOUT an extra intercept).
        let mut design = vec![0.0; n * m];
        for i in 0..n {
            basis_row(x[i], degree, knots, &mut design[i * m..(i + 1) * m]);
        }
        let model = linear::fit(&design, y, n, m, false)?;
        Ok(Self {
            degree,
            knots: knots.to_vec(),
            coefficients: model.coefficients,
        })
    }

    /// Predict at a scalar `x`.
    pub fn predict_one(&self, x: f64) -> f64 {
        let m = basis_len(self.degree, self.knots.len());
        let mut row = vec![0.0; m];
        basis_row(x, self.degree, &self.knots, &mut row);
        row.iter().zip(&self.coefficients).map(|(b, c)| b * c).sum()
    }

    /// Predict over a slice of scalar inputs.
    pub fn predict(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| self.predict_one(xi)).collect()
    }
}

/// Convenience: degree-`degree` polynomial regression (a spline with no knots).
pub fn polynomial_regression(
    x: &[f64],
    y: &[f64],
    n: usize,
    degree: usize,
) -> Result<RegressionSpline, LearningError> {
    RegressionSpline::fit(x, y, n, degree, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::r2_score;

    #[test]
    fn polynomial_recovers_a_quadratic_exactly() {
        // y = 2 − 3x + x².
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 2.0 - 3.0 * xi + xi * xi).collect();
        let m = polynomial_regression(&x, &y, 8, 2).unwrap();
        assert!((m.coefficients[0] - 2.0).abs() < 1e-6);
        assert!((m.coefficients[1] + 3.0).abs() < 1e-6);
        assert!((m.coefficients[2] - 1.0).abs() < 1e-6);
        assert!((m.predict_one(10.0) - (2.0 - 30.0 + 100.0)).abs() < 1e-5);
    }

    #[test]
    fn cubic_spline_fits_a_kinked_curve() {
        // A curve with a change of behaviour at x≈5 — a cubic spline with a knot
        // there fits it far better than a single global cubic could.
        let n = 30;
        let x: Vec<f64> = (0..n).map(|i| i as f64 / 3.0).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| {
                if xi < 5.0 {
                    (xi).sin()
                } else {
                    0.3 * (xi - 5.0) + (5.0f64).sin()
                }
            })
            .collect();
        let m = RegressionSpline::fit(&x, &y, n, 3, &[3.0, 5.0, 7.0]).unwrap();
        let preds = m.predict(&x);
        assert!(
            r2_score(&y, &preds).unwrap() > 0.97,
            "spline should fit well"
        );
    }

    #[test]
    fn guards() {
        assert_eq!(
            RegressionSpline::fit(&[1.0, 2.0], &[1.0], 2, 3, &[]).unwrap_err(),
            LearningError::InvalidDimension
        );
        assert_eq!(
            RegressionSpline::fit(&[1.0], &[1.0], 1, 0, &[]).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
