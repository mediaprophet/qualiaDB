//! Random forests (ISL ch 8.2.1–8.2.2) — bagging an ensemble of CART trees, each
//! grown on a bootstrap resample with a random feature subset per split
//! (decorrelating the trees). Regression averages the trees; classification takes
//! a majority vote. Built on [`super::decision_tree`] (no duplicated tree logic).

use crate::solvers::learning::trees::decision_tree::{Criterion, DecisionTree, TreeParams};
use crate::solvers::learning::LearningError;

/// A fitted random forest.
#[derive(Debug, Clone)]
pub struct RandomForest {
    trees: Vec<DecisionTree>,
    criterion: Criterion,
    p: usize,
}

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

/// `√p` (classification default) for the per-split feature subsample size.
fn default_features_classification(p: usize) -> usize {
    ((p as f64).sqrt().round() as usize).clamp(1, p)
}
/// `p/3` (regression default), at least 1.
fn default_features_regression(p: usize) -> usize {
    (p / 3).clamp(1, p)
}

fn fit_inner(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    criterion: Criterion,
    n_trees: usize,
    mut params: TreeParams,
    seed: u64,
    classification: bool,
) -> Result<RandomForest, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if n_trees == 0 {
        return Err(LearningError::InsufficientData);
    }
    if params.max_features.is_none() {
        params.max_features = Some(if classification {
            default_features_classification(p)
        } else {
            default_features_regression(p)
        });
    }

    let mut trees = Vec::with_capacity(n_trees);
    let mut bx = vec![0.0; n * p];
    let mut by = vec![0.0; n];
    for t in 0..n_trees {
        // Bootstrap resample (with replacement).
        let mut rng = Lcg(seed.wrapping_add(t as u64).wrapping_mul(0x9E3779B97F4A7C15) | 1);
        for i in 0..n {
            let s = rng.below(n);
            bx[i * p..(i + 1) * p].copy_from_slice(&x[s * p..(s + 1) * p]);
            by[i] = y[s];
        }
        // Each tree gets its own feature-subsample seed.
        let tree_params = TreeParams {
            seed: seed.wrapping_add(0xABCD ^ t as u64),
            ..params
        };
        let tree = match criterion {
            Criterion::Mse => DecisionTree::fit_regressor(&bx, &by, n, p, tree_params)?,
            Criterion::Gini => {
                let labels: Vec<usize> = by.iter().map(|&v| v as usize).collect();
                DecisionTree::fit_classifier(&bx, &labels, n, p, tree_params)?
            }
        };
        trees.push(tree);
    }
    Ok(RandomForest {
        trees,
        criterion,
        p,
    })
}

impl RandomForest {
    /// Fit a regression forest (averaged trees).
    pub fn fit_regressor(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        n_trees: usize,
        params: TreeParams,
        seed: u64,
    ) -> Result<Self, LearningError> {
        fit_inner(x, y, n, p, Criterion::Mse, n_trees, params, seed, false)
    }

    /// Fit a classification forest (majority vote); labels are small integers.
    pub fn fit_classifier(
        x: &[f64],
        y: &[usize],
        n: usize,
        p: usize,
        n_trees: usize,
        params: TreeParams,
        seed: u64,
    ) -> Result<Self, LearningError> {
        let yf: Vec<f64> = y.iter().map(|&v| v as f64).collect();
        fit_inner(x, &yf, n, p, Criterion::Gini, n_trees, params, seed, true)
    }

    /// Regression prediction: mean of the trees.
    pub fn predict_row(&self, q: &[f64]) -> f64 {
        match self.criterion {
            Criterion::Mse => {
                self.trees.iter().map(|t| t.predict_row(q)).sum::<f64>() / self.trees.len() as f64
            }
            Criterion::Gini => self.predict_class(q) as f64,
        }
    }

    /// Classification prediction: majority vote of the trees (lowest label on tie).
    pub fn predict_class(&self, q: &[f64]) -> usize {
        let labels: Vec<usize> = self.trees.iter().map(|t| t.predict_class(q)).collect();
        let max_label = labels.iter().copied().max().unwrap_or(0);
        let mut votes = vec![0usize; max_label + 1];
        for &l in &labels {
            votes[l] += 1;
        }
        votes
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .map(|(l, _)| l)
            .unwrap_or(0)
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<f64> {
        (0..m)
            .map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p]))
            .collect()
    }

    pub fn n_trees(&self) -> usize {
        self.trees.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::r2_score;

    #[test]
    fn regression_forest_fits_a_nonlinear_trend() {
        // y = x² — a single split can't capture it but an averaged forest tracks it.
        let n = 40;
        let x: Vec<f64> = (0..n).map(|i| i as f64 / 4.0).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi * xi).collect();
        let params = TreeParams {
            max_depth: 6,
            ..TreeParams::default()
        };
        let rf = RandomForest::fit_regressor(&x, &y, n, 1, 50, params, 7).unwrap();
        let preds = rf.predict(&x, n);
        // Forest explains most of the variance on the training data.
        assert!(r2_score(&y, &preds).unwrap() > 0.95);
        assert_eq!(rf.n_trees(), 50);
    }

    #[test]
    fn classification_forest_votes() {
        // class 0 around (0,0), class 1 around (10,10).
        let mut x = Vec::new();
        let mut y = Vec::new();
        for i in 0..10 {
            let t = i as f64 * 0.1;
            x.push(t);
            x.push(-t);
            y.push(0usize);
            x.push(10.0 + t);
            x.push(10.0 - t);
            y.push(1usize);
        }
        let n = 20;
        let rf = RandomForest::fit_classifier(&x, &y, n, 2, 30, TreeParams::default(), 3).unwrap();
        assert_eq!(rf.predict_class(&[0.2, 0.1]), 0);
        assert_eq!(rf.predict_class(&[10.1, 9.9]), 1);
    }

    #[test]
    fn guards() {
        assert_eq!(
            RandomForest::fit_regressor(&[1.0], &[1.0], 1, 1, 0, TreeParams::default(), 0)
                .unwrap_err(),
            LearningError::InsufficientData
        );
    }
}
