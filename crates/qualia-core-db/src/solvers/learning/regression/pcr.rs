//! Principal Components Regression (ISL ch 6.3.1) — regress the response on the
//! first `m` principal components of the predictors. Reuses
//! `dimensionality::pca` for the projection and `regression::linear` for the OLS
//! (no duplicated math): it is literally PCA followed by least squares on the
//! component scores, which tames collinearity by discarding low-variance directions.

use crate::solvers::learning::dimensionality::pca::{self, Pca};
use crate::solvers::learning::regression::linear::{self, LinearModel};
use crate::solvers::learning::LearningError;

/// A fitted PCR model: the PCA projection plus an OLS fit on the component scores.
#[derive(Debug, Clone)]
pub struct PcrModel {
    pca: Pca,
    ols: LinearModel,
    n_components: usize,
    p: usize,
}

impl PcrModel {
    /// Fit PCR with `n_components` principal components (clamped to `p`). Fails
    /// closed via the PCA / OLS solvers.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        n_components: usize,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        let m = n_components.clamp(1, p);
        let pca = pca::fit(x, n, p)?;
        let scores = pca
            .transform(x, n, m)
            .ok_or(LearningError::InvalidDimension)?;
        let ols = linear::fit(&scores, y, n, m, true)?;
        Ok(Self {
            pca,
            ols,
            n_components: m,
            p,
        })
    }

    /// Predict for one predictor row (length `p`).
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        // Project the (single) row onto the components, then apply the OLS fit.
        let scores = self
            .pca
            .transform(x_row, 1, self.n_components)
            .unwrap_or_default();
        self.ols.predict_row(&scores)
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<f64> {
        (0..m)
            .map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p]))
            .collect()
    }

    pub fn n_components(&self) -> usize {
        self.n_components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::r2_score;

    #[test]
    fn full_components_matches_ols_fit_quality() {
        // With all components retained, PCR explains the data as well as OLS.
        let x = [1.0, 2.0, 2.0, 1.0, 3.0, 0.0, 4.0, 5.0, 5.0, 4.0, 6.0, 1.0];
        let y = [3.0, 5.0, 4.0, 9.0, 13.0, 8.0];
        let pcr = PcrModel::fit(&x, &y, 6, 2, 2).unwrap();
        let preds = pcr.predict(&x, 6);
        assert!(r2_score(&y, &preds).unwrap() > 0.5);
    }

    #[test]
    fn one_component_captures_dominant_direction() {
        // y depends mostly on the high-variance direction; 1 component suffices.
        let n = 20;
        let mut x = vec![0.0; n * 2];
        let mut y = vec![0.0; n];
        for i in 0..n {
            let t = i as f64;
            x[i * 2] = t; // high variance
            x[i * 2 + 1] = 0.01 * ((i % 3) as f64); // tiny variance
            y[i] = 2.0 * t + 1.0;
        }
        let pcr = PcrModel::fit(&x, &y, n, 2, 1).unwrap();
        assert_eq!(pcr.n_components(), 1);
        let preds = pcr.predict(&x, n);
        assert!(r2_score(&y, &preds).unwrap() > 0.99);
    }

    #[test]
    fn guards() {
        assert_eq!(
            PcrModel::fit(&[1.0, 2.0, 3.0], &[1.0, 2.0], 2, 2, 1).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
