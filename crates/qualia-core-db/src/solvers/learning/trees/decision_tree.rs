//! CART decision trees (ISL ch 8.1) — recursive binary splitting for regression
//! (variance / MSE reduction) and classification (Gini impurity), over a row-major
//! feature matrix. The arena-based node store avoids deep `Box` recursion. The same
//! builder powers random forests and gradient boosting (with feature subsampling /
//! shallow depth). Scalar split search → CPU.

use crate::solvers::learning::LearningError;

/// Split criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criterion {
    /// Regression: minimise within-node sum of squared error.
    Mse,
    /// Classification: minimise Gini impurity (labels carried as integers-in-f64).
    Gini,
}

/// Tree hyper-parameters.
#[derive(Debug, Clone, Copy)]
pub struct TreeParams {
    pub max_depth: usize,
    pub min_samples_split: usize,
    pub min_samples_leaf: usize,
    /// Features considered per split (`None` = all). `Some(m)` random-samples `m`
    /// features — the mechanism random forests use.
    pub max_features: Option<usize>,
    pub seed: u64,
}

impl Default for TreeParams {
    fn default() -> Self {
        Self { max_depth: 8, min_samples_split: 2, min_samples_leaf: 1, max_features: None, seed: 0 }
    }
}

#[derive(Debug, Clone)]
struct Node {
    feature: i32, // -1 ⇒ leaf
    threshold: f64,
    left: usize,
    right: usize,
    value: f64, // leaf prediction (regression mean / majority class label)
}

/// A fitted decision tree.
#[derive(Debug, Clone)]
pub struct DecisionTree {
    nodes: Vec<Node>,
    criterion: Criterion,
    p: usize,
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

/// Impurity of the y-values at the indices, per criterion.
fn impurity(y: &[f64], idx: &[usize], criterion: Criterion) -> f64 {
    let n = idx.len();
    if n == 0 {
        return 0.0;
    }
    match criterion {
        Criterion::Mse => {
            let mean = idx.iter().map(|&i| y[i]).sum::<f64>() / n as f64;
            idx.iter().map(|&i| (y[i] - mean) * (y[i] - mean)).sum::<f64>() / n as f64
        }
        Criterion::Gini => {
            // Class labels are small non-negative integers stored as f64.
            let max_label = idx.iter().map(|&i| y[i] as usize).max().unwrap_or(0);
            let mut counts = vec![0usize; max_label + 1];
            for &i in idx {
                counts[y[i] as usize] += 1;
            }
            let nf = n as f64;
            1.0 - counts.iter().map(|&c| { let p = c as f64 / nf; p * p }).sum::<f64>()
        }
    }
}

/// Leaf prediction for the y-values at the indices.
fn leaf_value(y: &[f64], idx: &[usize], criterion: Criterion) -> f64 {
    match criterion {
        Criterion::Mse => idx.iter().map(|&i| y[i]).sum::<f64>() / idx.len() as f64,
        Criterion::Gini => {
            let max_label = idx.iter().map(|&i| y[i] as usize).max().unwrap_or(0);
            let mut counts = vec![0usize; max_label + 1];
            for &i in idx {
                counts[y[i] as usize] += 1;
            }
            counts.iter().enumerate().max_by_key(|(_, &c)| c).map(|(l, _)| l).unwrap_or(0) as f64
        }
    }
}

struct Builder<'a> {
    x: &'a [f64],
    y: &'a [f64],
    p: usize,
    params: TreeParams,
    criterion: Criterion,
    nodes: Vec<Node>,
    rng: Lcg,
}

impl<'a> Builder<'a> {
    fn build(&mut self, idx: &[usize], depth: usize) -> usize {
        let value = leaf_value(self.y, idx, self.criterion);
        let node_impurity = impurity(self.y, idx, self.criterion);
        // Stopping rules.
        if depth >= self.params.max_depth
            || idx.len() < self.params.min_samples_split
            || node_impurity <= 1e-12
        {
            return self.push_leaf(value);
        }

        // Candidate feature set (optionally subsampled — random forests).
        let feats = self.candidate_features();

        let mut best_gain = 0.0;
        let mut best_feat = usize::MAX;
        let mut best_thr = 0.0;
        let parent = node_impurity;
        let n = idx.len() as f64;

        for &f in &feats {
            // Sort the node's rows by this feature.
            let mut order: Vec<usize> = idx.to_vec();
            order.sort_by(|&a, &b| {
                self.x[a * self.p + f]
                    .partial_cmp(&self.x[b * self.p + f])
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
            // Try thresholds between distinct consecutive values.
            for s in 1..order.len() {
                let v0 = self.x[order[s - 1] * self.p + f];
                let v1 = self.x[order[s] * self.p + f];
                if v1 <= v0 {
                    continue; // identical values — no split here
                }
                let (left, right) = order.split_at(s);
                if left.len() < self.params.min_samples_leaf || right.len() < self.params.min_samples_leaf {
                    continue;
                }
                let il = impurity(self.y, left, self.criterion);
                let ir = impurity(self.y, right, self.criterion);
                let gain = parent - (left.len() as f64 / n * il + right.len() as f64 / n * ir);
                if gain > best_gain {
                    best_gain = gain;
                    best_feat = f;
                    best_thr = 0.5 * (v0 + v1);
                }
            }
        }

        if best_feat == usize::MAX || best_gain <= 1e-12 {
            return self.push_leaf(value);
        }

        // Partition and recurse.
        let mut left_idx = Vec::new();
        let mut right_idx = Vec::new();
        for &i in idx {
            if self.x[i * self.p + best_feat] <= best_thr {
                left_idx.push(i);
            } else {
                right_idx.push(i);
            }
        }
        // Reserve this node's slot before recursing (children get later indices).
        let me = self.nodes.len();
        self.nodes.push(Node { feature: best_feat as i32, threshold: best_thr, left: 0, right: 0, value });
        let l = self.build(&left_idx, depth + 1);
        let r = self.build(&right_idx, depth + 1);
        self.nodes[me].left = l;
        self.nodes[me].right = r;
        me
    }

    fn candidate_features(&mut self) -> Vec<usize> {
        match self.params.max_features {
            None => (0..self.p).collect(),
            Some(m) => {
                let m = m.clamp(1, self.p);
                let mut feats: Vec<usize> = (0..self.p).collect();
                // Partial Fisher–Yates to pick m features.
                for i in 0..m {
                    let j = i + (self.rng.next() as usize) % (self.p - i);
                    feats.swap(i, j);
                }
                feats.truncate(m);
                feats
            }
        }
    }

    fn push_leaf(&mut self, value: f64) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(Node { feature: -1, threshold: 0.0, left: 0, right: 0, value });
        idx
    }
}

impl DecisionTree {
    fn fit_inner(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        criterion: Criterion,
        params: TreeParams,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        let mut builder = Builder {
            x,
            y,
            p,
            params,
            criterion,
            nodes: Vec::new(),
            rng: Lcg(params.seed ^ 0x9E3779B97F4A7C15),
        };
        let idx: Vec<usize> = (0..n).collect();
        builder.build(&idx, 0);
        Ok(Self { nodes: builder.nodes, criterion, p })
    }

    /// Fit a regression tree (`Criterion::Mse`).
    pub fn fit_regressor(x: &[f64], y: &[f64], n: usize, p: usize, params: TreeParams) -> Result<Self, LearningError> {
        Self::fit_inner(x, y, n, p, Criterion::Mse, params)
    }

    /// Fit a classification tree (`Criterion::Gini`); labels are small integers.
    pub fn fit_classifier(x: &[f64], y: &[usize], n: usize, p: usize, params: TreeParams) -> Result<Self, LearningError> {
        let yf: Vec<f64> = y.iter().map(|&v| v as f64).collect();
        Self::fit_inner(x, &yf, n, p, Criterion::Gini, params)
    }

    /// Raw leaf prediction (regression value, or class label as f64) for one row.
    pub fn predict_row(&self, q: &[f64]) -> f64 {
        let mut node = 0;
        loop {
            let nd = &self.nodes[node];
            if nd.feature < 0 {
                return nd.value;
            }
            node = if q[nd.feature as usize] <= nd.threshold { nd.left } else { nd.right };
        }
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<f64> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }

    /// Class label prediction (rounds the leaf value) for a classifier tree.
    pub fn predict_class(&self, q: &[f64]) -> usize {
        self.predict_row(q).round() as usize
    }

    pub fn criterion(&self) -> Criterion {
        self.criterion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_tree_fits_a_step_function() {
        // y is a step: 0 for x<5, 10 for x>=5. A depth-1 tree captures it exactly.
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| if xi < 5.0 { 0.0 } else { 10.0 }).collect();
        let t = DecisionTree::fit_regressor(&x, &y, 10, 1, TreeParams::default()).unwrap();
        assert!((t.predict_row(&[2.0]) - 0.0).abs() < 1e-9);
        assert!((t.predict_row(&[8.0]) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn classification_tree_separates_xor_free_data() {
        // Two features; class determined by x0 threshold.
        let x = [0.0, 0.0, 1.0, 1.0, 8.0, 0.0, 9.0, 1.0];
        let y = [0usize, 0, 1, 1];
        let t = DecisionTree::fit_classifier(&x, &y, 4, 2, TreeParams::default()).unwrap();
        assert_eq!(t.predict_class(&[0.5, 0.5]), 0);
        assert_eq!(t.predict_class(&[8.5, 0.5]), 1);
    }

    #[test]
    fn perfectly_fits_training_set_when_deep() {
        // Distinct x → a deep tree memorizes the targets (train error 0).
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [3.0, 1.0, 4.0, 1.0, 5.0];
        let t = DecisionTree::fit_regressor(&x, &y, 5, 1, TreeParams::default()).unwrap();
        for i in 0..5 {
            assert!((t.predict_row(&[x[i]]) - y[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn depth_limit_makes_a_single_leaf() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [1.0, 2.0, 3.0, 4.0];
        let params = TreeParams { max_depth: 0, ..TreeParams::default() };
        let t = DecisionTree::fit_regressor(&x, &y, 4, 1, params).unwrap();
        // A single leaf predicts the global mean (2.5) for everything.
        assert!((t.predict_row(&[1.0]) - 2.5).abs() < 1e-9);
        assert!((t.predict_row(&[4.0]) - 2.5).abs() < 1e-9);
    }
}
