//! Multiple ordinary least squares with residuals, fitted values, and overall F.

use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};
use crate::solvers::statistics::descriptive::mean;
use crate::solvers::statistics::distributions::{fisher_f, students_t};

/// Fitted multiple OLS (`y = Xβ`) with inferential output and residual vector.
#[derive(Debug, Clone)]
pub struct MultipleOls {
    /// Coefficients; `[0]` is intercept when `fit_intercept`.
    pub coefficients: Vec<f64>,
    pub fit_intercept: bool,
    pub std_errors: Vec<f64>,
    pub t_values: Vec<f64>,
    pub p_values: Vec<f64>,
    pub r_squared: f64,
    pub adj_r_squared: f64,
    pub f_statistic: Option<f64>,
    pub f_p_value: Option<f64>,
    pub residual_std_error: f64,
    pub df_model: usize,
    pub df_residual: usize,
    pub n: usize,
    /// Number of predictors (excludes intercept).
    pub k_predictors: usize,
    pub fitted: Vec<f64>,
    pub residuals: Vec<f64>,
    /// Design matrix used (row-major `n × params`), retained for diagnostics.
    pub design: Vec<f64>,
    pub n_params: usize,
}

/// Fit OLS of `y` (length `n`) on row-major `n × p` predictors `x`.
/// Returns `None` on shape mismatch, `n ≤ params`, or singular Gram matrix.
pub fn multiple_ols(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    fit_intercept: bool,
) -> Option<MultipleOls> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return None;
    }
    let k = p + usize::from(fit_intercept);
    if n <= k {
        return None;
    }

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
    )
    .ok()?;
    let mut b = vec![0.0; k];
    matvec(Transpose::Yes, k, n, &d, y, &mut b).ok()?;

    let mut l = vec![0.0; k * k];
    cholesky_factor(k, &a, &mut l).ok()?;

    let mut coefficients = vec![0.0; k];
    cholesky_solve(k, &l, &b, &mut coefficients).ok()?;

    let mut fitted = vec![0.0; n];
    matvec(Transpose::No, n, k, &d, &coefficients, &mut fitted).ok()?;
    let ybar = mean(y)?;
    let mut sse = 0.0;
    let mut sst = 0.0;
    let mut residuals = vec![0.0; n];
    for i in 0..n {
        residuals[i] = y[i] - fitted[i];
        sse += residuals[i].powi(2);
        sst += (y[i] - ybar).powi(2);
    }
    let df_residual = n - k;
    let sigma2 = sse / df_residual as f64;
    let residual_std_error = sigma2.sqrt();

    let mut std_errors = vec![0.0; k];
    let mut t_values = vec![0.0; k];
    let mut p_values = vec![0.0; k];
    let df = df_residual as f64;
    let mut ej = vec![0.0; k];
    let mut cj = vec![0.0; k];
    for j in 0..k {
        ej.iter_mut().for_each(|v| *v = 0.0);
        ej[j] = 1.0;
        cholesky_solve(k, &l, &ej, &mut cj).ok()?;
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
        1.0 - (1.0 - r_squared) * (n as f64 - 1.0) / df
    } else {
        r_squared
    };

    let (f_statistic, f_p_value) = if fit_intercept && p >= 1 && sst > 0.0 {
        let df_model = p as f64;
        let f = ((sst - sse) / df_model) / sigma2;
        (Some(f), Some(fisher_f::upper_p(f, df_model, df)))
    } else {
        (None, None)
    };

    Some(MultipleOls {
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
        df_model: p,
        df_residual,
        n,
        k_predictors: p,
        fitted,
        residuals,
        design: d,
        n_params: k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::regression::simple_linear_regression;

    #[test]
    fn recovers_exact_plane() {
        let x = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0];
        let y = [1.0, 3.0, 4.0, 6.0, 8.0];
        let m = multiple_ols(&x, &y, 5, 2, true).unwrap();
        assert!((m.coefficients[0] - 1.0).abs() < 1e-9);
        assert!((m.coefficients[1] - 2.0).abs() < 1e-9);
        assert!((m.coefficients[2] - 3.0).abs() < 1e-9);
        assert!(m.residuals.iter().all(|r| r.abs() < 1e-9));
        assert!((m.r_squared - 1.0).abs() < 1e-12);
        assert!(m.f_statistic.is_some());
    }

    #[test]
    fn matches_simple_ols() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.1, 3.9, 6.1, 7.9, 10.2];
        let m = multiple_ols(&x, &y, 5, 1, true).unwrap();
        let s = simple_linear_regression(&x, &y).unwrap();
        assert!((m.coefficients[0] - s.intercept).abs() < 1e-9);
        assert!((m.coefficients[1] - s.slope).abs() < 1e-9);
    }

    #[test]
    fn singular_returns_none() {
        let x = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0, 4.0, 8.0, 5.0, 10.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(multiple_ols(&x, &y, 5, 2, true).is_none());
    }
}
