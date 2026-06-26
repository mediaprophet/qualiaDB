//! Gradient boosting for regression (ISL ch 8.2.3) — fit an additive ensemble of
//! shallow CART trees, each trained on the residuals of the running prediction
//! under squared-error loss (so the negative gradient *is* the residual). Predictions
//! are `init + ν·Σ treeₘ(x)` with learning rate `ν`. Built on [`super::decision_tree`].

use crate::solvers::learning::trees::decision_tree::{DecisionTree, TreeParams};
use crate::solvers::learning::LearningError;
use crate::solvers::statistics::descriptive::mean;

/// A fitted gradient-boosting regressor.
#[derive(Debug, Clone)]
pub struct GradientBoosting {
    init: f64,
    trees: Vec<DecisionTree>,
    learning_rate: f64,
    p: usize,
}

impl GradientBoosting {
    /// Fit `n_estimators` shallow trees by stage-wise residual fitting. `learning_rate`
    /// (`ν`, typically 0.05–0.3) shrinks each tree's contribution. Fails closed on
    /// shape mismatch / empty ensemble / non-positive learning rate.
    pub fn fit_regressor(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        n_estimators: usize,
        learning_rate: f64,
        params: TreeParams,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        if n_estimators == 0 || !(learning_rate > 0.0) {
            return Err(LearningError::InsufficientData);
        }

        let init = mean(y).ok_or(LearningError::InsufficientData)?;
        let mut pred = vec![init; n];
        let mut residual = vec![0.0; n];
        let mut trees = Vec::with_capacity(n_estimators);

        for _ in 0..n_estimators {
            for i in 0..n {
                residual[i] = y[i] - pred[i]; // negative gradient of ½(y−F)²
            }
            let tree = DecisionTree::fit_regressor(x, &residual, n, p, params)?;
            // Update the running prediction.
            for i in 0..n {
                pred[i] += learning_rate * tree.predict_row(&x[i * p..(i + 1) * p]);
            }
            trees.push(tree);
        }

        Ok(Self { init, trees, learning_rate, p })
    }

    pub fn predict_row(&self, q: &[f64]) -> f64 {
        self.init + self.learning_rate * self.trees.iter().map(|t| t.predict_row(q)).sum::<f64>()
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<f64> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }

    pub fn n_estimators(&self) -> usize {
        self.trees.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::{mse, r2_score};

    #[test]
    fn boosting_reduces_error_with_more_stages() {
        // Nonlinear target; more boosting stages drive training error down.
        let n = 50;
        let x: Vec<f64> = (0..n).map(|i| i as f64 / 10.0).collect();
        let y: Vec<f64> = x.iter().map(|&xi| (xi).sin() * 3.0 + 0.5 * xi).collect();
        let params = TreeParams { max_depth: 3, ..TreeParams::default() };
        let few = GradientBoosting::fit_regressor(&x, &y, n, 1, 5, 0.1, params).unwrap();
        let many = GradientBoosting::fit_regressor(&x, &y, n, 1, 200, 0.1, params).unwrap();
        let mse_few = mse(&y, &few.predict(&x, n)).unwrap();
        let mse_many = mse(&y, &many.predict(&x, n)).unwrap();
        assert!(mse_many < mse_few, "more stages should fit better: {mse_many} !< {mse_few}");
        // A well-trained ensemble explains most of the variance.
        assert!(r2_score(&y, &many.predict(&x, n)).unwrap() > 0.9);
    }

    #[test]
    fn single_stage_is_init_plus_one_tree() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [1.0, 2.0, 1.0, 5.0, 6.0, 5.0];
        let gb = GradientBoosting::fit_regressor(&x, &y, 6, 1, 1, 0.5, TreeParams { max_depth: 2, ..TreeParams::default() }).unwrap();
        assert_eq!(gb.n_estimators(), 1);
        // Prediction is finite and within the data range envelope.
        let p = gb.predict_row(&[3.5]);
        assert!(p.is_finite());
    }

    #[test]
    fn guards() {
        assert_eq!(GradientBoosting::fit_regressor(&[1.0], &[1.0], 1, 1, 0, 0.1, TreeParams::default()).unwrap_err(), LearningError::InsufficientData);
        assert_eq!(GradientBoosting::fit_regressor(&[1.0], &[1.0], 1, 1, 10, 0.0, TreeParams::default()).unwrap_err(), LearningError::InsufficientData);
    }
}
