//! Regression — ordinary least squares. Simple (one-predictor) linear regression
//! with the full inferential output: coefficients, R², residual standard error, and
//! real t-based standard errors / p-values from the [`distributions`](super::distributions)
//! library (no placeholder significance).
//!
//! Multiple linear regression (the normal-equations / QR solve over
//! `solvers::linear_algebra`) is the natural next module here; simple OLS is a
//! complete capability on its own and is what the domain libs need first.

use super::descriptive::{covariance, mean, variance};
use super::distributions::students_t;

/// Ordinary-least-squares fit of `y = intercept + slope·x`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRegression {
    pub slope: f64,
    pub intercept: f64,
    /// Coefficient of determination R² (fraction of variance explained).
    pub r_squared: f64,
    /// Residual standard error `s = √(SSE/(n−2))`.
    pub residual_std_error: f64,
    pub slope_std_error: f64,
    pub slope_t: f64,
    /// Two-sided p-value for `slope = 0` (df = n−2).
    pub slope_p_value: f64,
    pub intercept_std_error: f64,
    pub intercept_p_value: f64,
    pub n: usize,
}

/// Simple linear regression of `y` on `x`. `None` if the lengths differ, `n < 3`
/// (need `n−2 ≥ 1` residual degrees of freedom for inference), or `x` has zero
/// variance (slope undefined).
pub fn simple_linear_regression(x: &[f64], y: &[f64]) -> Option<LinearRegression> {
    let n = x.len();
    if n != y.len() || n < 3 {
        return None;
    }
    let mx = mean(x)?;
    let my = mean(y)?;
    let var_x = variance(x, false)?; // population moment is fine; ratios cancel n
    if var_x <= 0.0 {
        return None;
    }
    let cov_xy = covariance(x, y, false)?;
    let slope = cov_xy / var_x;
    let intercept = my - slope * mx;

    // Sums of squares.
    let mut ss_tot = 0.0; // Σ(y-ȳ)²
    let mut ss_res = 0.0; // Σ(y-ŷ)²
    let mut sxx = 0.0; // Σ(x-x̄)²
    for i in 0..n {
        let yhat = intercept + slope * x[i];
        ss_tot += (y[i] - my).powi(2);
        ss_res += (y[i] - yhat).powi(2);
        sxx += (x[i] - mx).powi(2);
    }

    let df = (n - 2) as f64;
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    let s2 = ss_res / df; // residual variance
    let residual_std_error = s2.sqrt();

    let slope_std_error = (s2 / sxx).sqrt();
    let (slope_t, slope_p_value) = if slope_std_error > 0.0 {
        let t = slope / slope_std_error;
        (t, students_t::two_sided_p(t, df))
    } else {
        // Perfect fit (zero residuals): slope is exact → infinitely significant.
        (f64::INFINITY.copysign(slope), 0.0)
    };

    let intercept_std_error = (s2 * (1.0 / n as f64 + mx * mx / sxx)).sqrt();
    let intercept_p_value = if intercept_std_error > 0.0 {
        students_t::two_sided_p(intercept / intercept_std_error, df)
    } else {
        0.0
    };

    Some(LinearRegression {
        slope,
        intercept,
        r_squared,
        residual_std_error,
        slope_std_error,
        slope_t,
        slope_p_value,
        intercept_std_error,
        intercept_p_value,
        n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_line_is_recovered() {
        // y = 3 + 2x exactly.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [3.0, 5.0, 7.0, 9.0, 11.0];
        let r = simple_linear_regression(&x, &y).unwrap();
        assert!((r.slope - 2.0).abs() < 1e-9);
        assert!((r.intercept - 3.0).abs() < 1e-9);
        assert!((r.r_squared - 1.0).abs() < 1e-12);
        assert!(r.residual_std_error < 1e-9);
        assert_eq!(r.slope_p_value, 0.0); // perfect fit → exact
    }

    #[test]
    fn noisy_trend_matches_known_fit() {
        // Classic small dataset; OLS slope/intercept are standard reference values.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.1, 3.9, 6.1, 7.9, 10.2];
        let r = simple_linear_regression(&x, &y).unwrap();
        // Hand/np.polyfit: slope ≈ 2.0, intercept ≈ 0.02.
        assert!((r.slope - 2.0).abs() < 0.05, "slope={}", r.slope);
        assert!(r.intercept.abs() < 0.2, "intercept={}", r.intercept);
        assert!(r.r_squared > 0.99, "r2={}", r.r_squared);
        assert!(r.slope_p_value < 1e-4, "strong trend must be significant");
    }

    #[test]
    fn no_relationship_is_not_significant() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [5.0, 4.0, 6.0, 5.0, 6.0, 4.0]; // flat-ish noise
        let r = simple_linear_regression(&x, &y).unwrap();
        assert!(
            r.slope_p_value > 0.2,
            "no real trend: p={}",
            r.slope_p_value
        );
        assert!(r.r_squared < 0.3);
    }

    #[test]
    fn guards_degenerate_input() {
        assert!(simple_linear_regression(&[1.0, 2.0], &[1.0, 2.0]).is_none()); // n<3
        assert!(simple_linear_regression(&[2.0, 2.0, 2.0], &[1.0, 2.0, 3.0]).is_none()); // zero var x
        assert!(simple_linear_regression(&[1.0, 2.0, 3.0], &[1.0, 2.0]).is_none());
        // mismatch
    }
}
