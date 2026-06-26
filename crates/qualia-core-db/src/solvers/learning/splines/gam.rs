//! Generalized Additive Model (ISL ch 7.7) — `y = β₀ + Σⱼ fⱼ(xⱼ)`, each `fⱼ` a
//! regression spline, fit by **backfitting**: cycle through the features, fitting
//! each smooth term to the partial residual of all the others. Reuses
//! [`super::RegressionSpline`] (no duplicated basis/OLS). Kernel-class `DenseLinear`.

use crate::solvers::learning::splines::RegressionSpline;
use crate::solvers::learning::LearningError;
use crate::solvers::statistics::descriptive::mean;

/// A fitted additive model: an intercept plus one centered smooth term per feature.
#[derive(Debug, Clone)]
pub struct Gam {
    pub intercept: f64,
    /// One `(spline, mean_offset)` per feature; the term value is
    /// `spline(x) − mean_offset` (centered for identifiability).
    terms: Vec<(RegressionSpline, f64)>,
    p: usize,
}

impl Gam {
    /// Fit by backfitting. `degree` and `knots_per_feature[j]` define each feature's
    /// spline; `max_iter` backfitting sweeps. Fails closed on shape mismatch.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        degree: usize,
        knots_per_feature: &[Vec<f64>],
        max_iter: usize,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n || knots_per_feature.len() != p {
            return Err(LearningError::InvalidDimension);
        }
        let intercept = mean(y).ok_or(LearningError::InsufficientData)?;

        // Per-feature columns (contiguous) for spline fitting.
        let cols: Vec<Vec<f64>> = (0..p)
            .map(|j| (0..n).map(|i| x[i * p + j]).collect::<Vec<f64>>())
            .collect();

        // Current fitted-term values at each training point, n×p (start at 0).
        let mut term_vals = vec![0.0; n * p];
        // Placeholder splines (degree-`degree`, replaced during the first sweep).
        let mut terms: Vec<(RegressionSpline, f64)> = Vec::with_capacity(p);
        for j in 0..p {
            let spline = RegressionSpline::fit(&cols[j], &vec![0.0; n], n, degree, &knots_per_feature[j])?;
            terms.push((spline, 0.0));
        }

        for _ in 0..max_iter.max(1) {
            for j in 0..p {
                // Partial residual: y − intercept − Σ_{k≠j} f_k.
                let mut resid = vec![0.0; n];
                for i in 0..n {
                    let mut s = y[i] - intercept;
                    for k in 0..p {
                        if k != j {
                            s -= term_vals[i * p + k];
                        }
                    }
                    resid[i] = s;
                }
                // Fit f_j to the residual, then center it (mean 0 over the data).
                let spline = RegressionSpline::fit(&cols[j], &resid, n, degree, &knots_per_feature[j])?;
                let raw: Vec<f64> = cols[j].iter().map(|&xi| spline.predict_one(xi)).collect();
                let offset = mean(&raw).unwrap_or(0.0);
                for i in 0..n {
                    term_vals[i * p + j] = raw[i] - offset;
                }
                terms[j] = (spline, offset);
            }
        }

        Ok(Self { intercept, terms, p })
    }

    /// Predict for one feature row.
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        let mut s = self.intercept;
        for (j, (spline, offset)) in self.terms.iter().enumerate() {
            s += spline.predict_one(x_row[j]) - offset;
        }
        s
    }

    pub fn predict(&self, x: &[f64], n: usize) -> Vec<f64> {
        (0..n).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::r2_score;

    #[test]
    fn fits_an_additive_nonlinear_surface() {
        // y = sin(x0) + 0.1·x1²  — additive but nonlinear in each feature.
        let n = 40;
        let mut x = vec![0.0; n * 2];
        let mut y = vec![0.0; n];
        for i in 0..n {
            let x0 = (i as f64) * 0.15;
            let x1 = ((i * 7) % 40) as f64 * 0.1;
            x[i * 2] = x0;
            x[i * 2 + 1] = x1;
            y[i] = x0.sin() + 0.1 * x1 * x1;
        }
        let knots = vec![vec![1.5, 3.0, 4.5], vec![1.0, 2.0, 3.0]];
        let gam = Gam::fit(&x, &y, n, 2, 3, &knots, 10).unwrap();
        let preds = gam.predict(&x, n);
        assert!(r2_score(&y, &preds).unwrap() > 0.95, "GAM should fit the additive surface");
    }

    #[test]
    fn recovers_a_linear_additive_model() {
        // y = 2·x0 − 3·x1 + 1: an additive (linear) model; GAM recovers it.
        let n = 25;
        let mut x = vec![0.0; n * 2];
        let mut y = vec![0.0; n];
        for i in 0..n {
            let x0 = i as f64;
            let x1 = (i % 5) as f64;
            x[i * 2] = x0;
            x[i * 2 + 1] = x1;
            y[i] = 2.0 * x0 - 3.0 * x1 + 1.0;
        }
        let knots = vec![vec![], vec![]]; // no knots → linear terms
        let gam = Gam::fit(&x, &y, n, 1, &knots, 20).unwrap();
        let preds = gam.predict(&x, n);
        assert!(r2_score(&y, &preds).unwrap() > 0.999);
    }

    #[test]
    fn guards() {
        assert_eq!(Gam::fit(&[1.0, 2.0], &[1.0], 2, 1, 3, &[vec![]], 5).unwrap_err(), LearningError::InvalidDimension);
    }
}
