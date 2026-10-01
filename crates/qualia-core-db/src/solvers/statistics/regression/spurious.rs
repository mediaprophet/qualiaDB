//! Spurious-regression guards: trend / ADF-proxy warnings for time-ordered series.

use serde::Serialize;

use crate::solvers::statistics::{adf_proxy, descriptive::mean};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpuriousGuard {
    /// ADF-like coefficient on lagged level (more negative ⇒ more stationary).
    pub y_adf_proxy: f64,
    pub x_adf_proxy: Option<f64>,
    pub y_looks_trended: bool,
    pub x_looks_trended: bool,
    /// True when both series look non-stationary — do not trust classical t/F.
    pub warn_spurious: bool,
    pub detail: String,
}

fn looks_trended(series: &[f64]) -> bool {
    let n = series.len();
    if n < 8 {
        return false;
    }
    // Linear time trend R² via simple correlation with t.
    let t: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let mt = mean(&t).unwrap_or(0.0);
    let my = mean(series).unwrap_or(0.0);
    let mut num = 0.0;
    let mut dt = 0.0;
    let mut dy = 0.0;
    for i in 0..n {
        let a = t[i] - mt;
        let b = series[i] - my;
        num += a * b;
        dt += a * a;
        dy += b * b;
    }
    if dt <= 0.0 || dy <= 0.0 {
        return false;
    }
    let r2 = (num * num) / (dt * dy);
    r2 > 0.5
}

/// Guard for bivariate time-series OLS. Pass `x_series = None` for y-only check.
pub fn spurious_regression_guard(y: &[f64], x_series: Option<&[f64]>) -> Option<SpuriousGuard> {
    if y.len() < 8 {
        return None;
    }
    let y_adf = adf_proxy(y);
    if !y_adf.is_finite() {
        return None;
    }
    let y_trended = looks_trended(y) || y_adf > -0.1;
    let (x_adf, x_trended) = if let Some(x) = x_series {
        if x.len() != y.len() {
            return None;
        }
        let a = adf_proxy(x);
        (Some(a), looks_trended(x) || a > -0.1)
    } else {
        (None, false)
    };
    let warn = match x_adf {
        Some(_) => y_trended && x_trended,
        None => y_trended,
    };
    let detail = if warn {
        "both series look non-stationary / trended — classical significance may be spurious"
            .to_string()
    } else {
        "no strong spurious-regression warning".to_string()
    };
    Some(SpuriousGuard {
        y_adf_proxy: y_adf,
        x_adf_proxy: x_adf,
        y_looks_trended: y_trended,
        x_looks_trended: x_trended,
        warn_spurious: warn,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_common_trend() {
        let y: Vec<f64> = (0..30).map(|i| i as f64 + 0.1).collect();
        let x: Vec<f64> = (0..30).map(|i| 2.0 * i as f64).collect();
        let g = spurious_regression_guard(&y, Some(&x)).unwrap();
        assert!(g.warn_spurious, "{g:?}");
    }

    #[test]
    fn clean_noise_not_flagged() {
        let y = [
            0.1, -0.2, 0.15, -0.1, 0.05, -0.08, 0.12, -0.03, 0.07, -0.11, 0.02, -0.06, 0.09,
            -0.04, 0.01, -0.07, 0.11, -0.02, 0.08, -0.05,
        ];
        let g = spurious_regression_guard(&y, None).unwrap();
        assert!(!g.warn_spurious);
    }
}
