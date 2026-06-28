//! Partial Least Squares regression (ISL ch 6.3.2) — PLS1 (univariate response) by
//! the NIPALS algorithm. Unlike PCR (which picks directions of high predictor
//! variance), PLS picks directions of high *covariance with the response*. The small
//! component-space solve reuses `linear_algebra::qr` (no new solver). Kernel-class
//! `DenseLinear`.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::qr::{qr_factor, qr_solve_least_squares};
use crate::solvers::statistics::descriptive::mean;

/// A fitted PLS model collapsed to predictor-space coefficients + intercept.
#[derive(Debug, Clone)]
pub struct PlsModel {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub n_components: usize,
}

impl PlsModel {
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

/// Fit PLS1 with `n_components` (clamped to `p`). Fails closed on shape mismatch /
/// `n < 2` / a degenerate (zero-covariance) extraction.
pub fn fit(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    n_components: usize,
) -> Result<PlsModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if n < 2 {
        return Err(LearningError::InsufficientData);
    }
    let a = n_components.clamp(1, p);

    // Centre X and y.
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

    // NIPALS: collect weight (w), loading (p_load) columns and scalar q per component.
    let mut w_mat = vec![0.0; p * a]; // p × a, column per component
    let mut p_mat = vec![0.0; p * a];
    let mut q_vec = vec![0.0; a];

    for comp in 0..a {
        // w = Xcᵀ yc, normalized.
        let mut w = vec![0.0; p];
        for j in 0..p {
            let mut s = 0.0;
            for i in 0..n {
                s += xc[i * p + j] * yc[i];
            }
            w[j] = s;
        }
        let wn = (w.iter().map(|v| v * v).sum::<f64>()).sqrt();
        if wn < 1e-12 {
            // No remaining covariance — stop early (use the components found so far).
            if comp == 0 {
                return Err(LearningError::InsufficientData);
            }
            return finalize(&w_mat, &p_mat, &q_vec, comp, p, &xbar, ybar);
        }
        for v in w.iter_mut() {
            *v /= wn;
        }
        // Scores t = Xc w.
        let mut t = vec![0.0; n];
        for i in 0..n {
            t[i] = (0..p).map(|j| xc[i * p + j] * w[j]).sum();
        }
        let tt: f64 = t.iter().map(|v| v * v).sum();
        if tt < 1e-12 {
            return finalize(&w_mat, &p_mat, &q_vec, comp, p, &xbar, ybar);
        }
        // Loadings p_load = Xcᵀ t / tt, q = ycᵀ t / tt.
        let mut p_load = vec![0.0; p];
        for j in 0..p {
            let mut s = 0.0;
            for i in 0..n {
                s += xc[i * p + j] * t[i];
            }
            p_load[j] = s / tt;
        }
        let q = yc.iter().zip(&t).map(|(yi, ti)| yi * ti).sum::<f64>() / tt;
        // Deflate.
        for i in 0..n {
            for j in 0..p {
                xc[i * p + j] -= t[i] * p_load[j];
            }
            yc[i] -= t[i] * q;
        }
        for j in 0..p {
            w_mat[j * a + comp] = w[j];
            p_mat[j * a + comp] = p_load[j];
        }
        q_vec[comp] = q;
    }

    finalize(&w_mat, &p_mat, &q_vec, a, p, &xbar, ybar)
}

/// Collapse `a` components into predictor-space coefficients
/// `β = W (PᵀW)⁻¹ q`, intercept `ȳ − x̄ᵀβ`.
fn finalize(
    w_mat: &[f64],
    p_mat: &[f64],
    q_vec: &[f64],
    a: usize,
    p: usize,
    xbar: &[f64],
    ybar: f64,
) -> Result<PlsModel, LearningError> {
    // M = PᵀW (a×a): M[r,c] = Σ_j p_mat[j,r]·w_mat[j,c].
    let stride = w_mat.len() / p; // = number of allocated components (>= a)
    let mut m = vec![0.0; a * a];
    for r in 0..a {
        for c in 0..a {
            let mut s = 0.0;
            for j in 0..p {
                s += p_mat[j * stride + r] * w_mat[j * stride + c];
            }
            m[r * a + c] = s;
        }
    }
    // Solve M α = q (square) via QR.
    let mut tau = vec![0.0; a];
    qr_factor(a, a, &mut m, &mut tau)?;
    let mut b = q_vec[..a].to_vec();
    let mut alpha = vec![0.0; a];
    qr_solve_least_squares(a, a, &m, &tau, &mut b, &mut alpha)?;
    // β = W α.
    let mut coefficients = vec![0.0; p];
    for j in 0..p {
        let mut s = 0.0;
        for c in 0..a {
            s += w_mat[j * stride + c] * alpha[c];
        }
        coefficients[j] = s;
    }
    let intercept = ybar
        - coefficients
            .iter()
            .zip(xbar)
            .map(|(b, m)| b * m)
            .sum::<f64>();
    Ok(PlsModel {
        coefficients,
        intercept,
        n_components: a,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::regression::linear;

    #[test]
    fn full_components_matches_ols() {
        // With min(n-1, p) components PLS reproduces the OLS fit.
        let x = [1.0, 2.0, 2.0, 1.0, 3.0, 0.0, 4.0, 5.0, 5.0, 4.0, 6.0, 1.0];
        let y = [3.0, 5.0, 4.0, 9.0, 13.0, 8.0];
        let pls = fit(&x, &y, 6, 2, 2).unwrap();
        let ols = linear::fit(&x, &y, 6, 2, true).unwrap();
        let p_pred = pls.predict(&x, 6, 2);
        let o_pred = ols.predict(&x, 6, 2);
        for i in 0..6 {
            assert!(
                (p_pred[i] - o_pred[i]).abs() < 1e-6,
                "{} vs {}",
                p_pred[i],
                o_pred[i]
            );
        }
    }

    #[test]
    fn one_component_tracks_covariance_direction() {
        // y driven by x0; PLS with 1 component fits well (it targets covariance).
        let n = 20;
        let mut x = vec![0.0; n * 2];
        let mut y = vec![0.0; n];
        for i in 0..n {
            let t = i as f64;
            x[i * 2] = t;
            x[i * 2 + 1] = ((i % 5) as f64) - 2.0; // unrelated to y
            y[i] = 3.0 * t + 2.0;
        }
        let pls = fit(&x, &y, n, 2, 1).unwrap();
        let preds = pls.predict(&x, n, 2);
        let r2 = crate::solvers::learning::metrics::regression::r2_score(&y, &preds).unwrap();
        assert!(r2 > 0.99, "r2 {r2}");
    }

    #[test]
    fn guards() {
        assert_eq!(
            fit(&[1.0, 2.0, 3.0], &[1.0, 2.0], 2, 2, 1).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
