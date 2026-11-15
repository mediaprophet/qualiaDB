//! Leverage, Cook's distance, and studentized residuals (flag only — never drop).

use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, Transpose};
use crate::solvers::statistics::regression::MultipleOls;

use serde::Serialize;

/// Per-observation influence diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InfluenceRow {
    pub index: usize,
    pub leverage: f64,
    pub cook_d: f64,
    pub studentized: f64,
    /// True when leverage > 2k/n or Cook's D > 4/n or |studentized| > 3.
    pub flagged: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InfluenceMeasures {
    pub rows: Vec<InfluenceRow>,
    pub leverage_threshold: f64,
    pub cook_threshold: f64,
}

/// Compute influence from a fitted [`MultipleOls`] (uses its design + residuals).
pub fn influence_measures(model: &MultipleOls) -> Option<InfluenceMeasures> {
    let n = model.n;
    let k = model.n_params;
    if model.design.len() != n * k || model.residuals.len() != n {
        return None;
    }
    // H = D (DᵀD)⁻¹ Dᵀ → leverage h_ii via solving (DᵀD) w = d_i then h = d_i·w.
    let mut a = vec![0.0; k * k];
    gemm(
        Transpose::Yes,
        Transpose::No,
        k,
        k,
        n,
        1.0,
        &model.design,
        &model.design,
        0.0,
        &mut a,
    )
    .ok()?;
    let mut l = vec![0.0; k * k];
    cholesky_factor(k, &a, &mut l).ok()?;

    let mse = model.residual_std_error.powi(2);
    if mse <= 0.0 {
        return None;
    }
    let lev_thr = 2.0 * k as f64 / n as f64;
    let cook_thr = 4.0 / n as f64;
    let mut rows = Vec::with_capacity(n);
    let mut di = vec![0.0; k];
    let mut wi = vec![0.0; k];
    for i in 0..n {
        di.copy_from_slice(&model.design[i * k..(i + 1) * k]);
        cholesky_solve(k, &l, &di, &mut wi).ok()?;
        let h: f64 = di.iter().zip(wi.iter()).map(|(a, b)| a * b).sum();
        let h = h.clamp(0.0, 1.0 - 1e-12);
        let e = model.residuals[i];
        let studentized = e / (model.residual_std_error * (1.0 - h).sqrt());
        let cook_d = (e * e / (k as f64 * mse)) * (h / (1.0 - h).powi(2));
        let flagged = h > lev_thr || cook_d > cook_thr || studentized.abs() > 3.0;
        rows.push(InfluenceRow {
            index: i,
            leverage: h,
            cook_d,
            studentized,
            flagged,
        });
    }
    Some(InfluenceMeasures {
        rows,
        leverage_threshold: lev_thr,
        cook_threshold: cook_thr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::regression::multiple_ols;

    #[test]
    fn flags_high_leverage_point() {
        // Mostly x≈1..5; one far x=50 pulls leverage.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 50.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0, 20.0];
        let m = multiple_ols(&x, &y, 6, 1, true).unwrap();
        let inf = influence_measures(&m).unwrap();
        assert!(inf.rows[5].flagged, "far x should be flagged");
        assert!(inf.rows[5].leverage > inf.leverage_threshold);
    }
}
