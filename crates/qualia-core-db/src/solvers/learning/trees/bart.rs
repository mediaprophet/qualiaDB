//! Bayesian Additive Regression Trees (ISL ch 8.2.4, Chipman-George-McCulloch 2010)
//! — a sum-of-trees regression model `y = Σⱼ gⱼ(x) + ε` fit by **Bayesian
//! backfitting MCMC**: each tree is updated in turn against the partial residual via
//! a grow/prune Metropolis-Hastings step with conjugate-normal leaves, and the noise
//! variance is drawn from its inverse-gamma full conditional.
//!
//! The trees are kept deliberately small by the depth prior `p_split(d) =
//! α(1+d)^{−β}`, so the ensemble is a sum of weak learners (like boosting, but with
//! a full posterior). Prediction is the posterior-mean over retained MCMC draws,
//! giving a smooth fit with built-in uncertainty. Kernel-class `Divergent` (the MCMC).

use crate::solvers::learning::LearningError;

#[derive(Debug, Clone)]
struct Node {
    leaf: bool,
    feature: usize,
    threshold: f64,
    left: usize,
    right: usize,
    depth: usize,
    mu: f64,
}

#[derive(Debug, Clone)]
struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    fn root() -> Self {
        Self { nodes: vec![Node { leaf: true, feature: 0, threshold: 0.0, left: 0, right: 0, depth: 0, mu: 0.0 }] }
    }
    /// Index of the leaf a point falls into.
    fn leaf_of(&self, x_row: &[f64]) -> usize {
        let mut n = 0;
        loop {
            let nd = &self.nodes[n];
            if nd.leaf {
                return n;
            }
            n = if x_row[nd.feature] <= nd.threshold { nd.left } else { nd.right };
        }
    }
    fn predict(&self, x_row: &[f64]) -> f64 {
        self.nodes[self.leaf_of(x_row)].mu
    }
    fn leaves(&self) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].leaf).collect()
    }
    /// "nog" nodes: internal nodes whose both children are leaves (prunable).
    fn nog_nodes(&self) -> Vec<usize> {
        (0..self.nodes.len())
            .filter(|&i| !self.nodes[i].leaf && self.nodes[self.nodes[i].left].leaf && self.nodes[self.nodes[i].right].leaf)
            .collect()
    }
}

struct Rng(u64);
impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn below(&mut self, b: usize) -> usize {
        (self.unit() * b as f64) as usize % b.max(1)
    }
    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
    /// Gamma(shape a > 0, scale 1) via Marsaglia-Tsang.
    fn gamma(&mut self, a: f64) -> f64 {
        if a < 1.0 {
            return self.gamma(a + 1.0) * self.unit().max(1e-12).powf(1.0 / a);
        }
        let d = a - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();
        loop {
            let x = self.gaussian();
            let v = (1.0 + c * x).powi(3);
            if v <= 0.0 {
                continue;
            }
            let u = self.unit();
            if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
                return d * v;
            }
        }
    }
}

/// A fitted BART model: the retained posterior draws (each a forest of trees).
#[derive(Debug, Clone)]
pub struct Bart {
    draws: Vec<Vec<Tree>>, // retained MCMC samples, each a Vec of m trees
    y_center: f64,
    y_scale: f64,
    p: usize,
}

/// Leaf marginal log-likelihood up to terms common across a tree partition: with
/// residual sum `s` over `n` points, leaf prior `N(0, σμ²)`, noise `σ²`.
fn leaf_loglik(s: f64, n: f64, sigma2: f64, sigma_mu2: f64) -> f64 {
    let denom = sigma2 + n * sigma_mu2;
    0.5 * (sigma2 / denom).ln() + (sigma_mu2 * s * s) / (2.0 * sigma2 * denom)
}

impl Bart {
    /// Fit BART. `m` trees, `n_iter` MCMC sweeps with `burn_in` discarded. `k`
    /// controls the leaf prior (≈ 2). Fails closed on shape mismatch / degenerate y.
    #[allow(clippy::too_many_arguments)]
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        p: usize,
        m: usize,
        n_iter: usize,
        burn_in: usize,
        k: f64,
        seed: u64,
    ) -> Result<Self, LearningError> {
        if n < 2 || p == 0 || x.len() != n * p || y.len() != n || m == 0 || n_iter <= burn_in {
            return Err(LearningError::InvalidDimension);
        }
        // Scale y to [-0.5, 0.5].
        let ymin = y.iter().cloned().fold(f64::INFINITY, f64::min);
        let ymax = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_center = 0.5 * (ymin + ymax);
        let y_scale = (ymax - ymin).max(1e-9);
        let ys: Vec<f64> = y.iter().map(|&v| (v - y_center) / y_scale).collect();

        let sigma_mu = 0.5 / (k * (m as f64).sqrt());
        let sigma_mu2 = sigma_mu * sigma_mu;
        // Noise prior InvGamma(ν/2, νλ/2); ν=3, λ from the scaled-y variance.
        let nu = 3.0;
        let ybar = ys.iter().sum::<f64>() / n as f64;
        let var_y = ys.iter().map(|&v| (v - ybar).powi(2)).sum::<f64>() / n as f64;
        let lambda = var_y.max(1e-6);
        let mut sigma2 = var_y.max(1e-6);

        let alpha = 0.95;
        let beta = 2.0;
        let p_split = |d: usize| alpha * (1.0 + d as f64).powf(-beta);

        let mut rng = Rng(seed ^ 0x9E3779B97F4A7C15);
        let mut trees: Vec<Tree> = (0..m).map(|_| Tree::root()).collect();
        // Running fitted values per tree at the training points.
        let mut tree_fit = vec![vec![0.0; n]; m];
        let mut draws: Vec<Vec<Tree>> = Vec::new();

        // Precompute per-point membership lazily each tree update.
        for it in 0..n_iter {
            for j in 0..m {
                // Partial residual R = ys − Σ_{k≠j} fit_k.
                let mut r = ys.clone();
                for (kk, fitk) in tree_fit.iter().enumerate() {
                    if kk != j {
                        for i in 0..n {
                            r[i] -= fitk[i];
                        }
                    }
                }
                // One MH step (grow or prune) on tree j against R.
                mh_step(&mut trees[j], x, &r, n, p, sigma2, sigma_mu2, &p_split, &mut rng);
                // Sample leaf μ's (conjugate normal) and recompute this tree's fit.
                sample_leaves(&mut trees[j], x, &r, n, sigma2, sigma_mu2, &mut rng);
                for i in 0..n {
                    tree_fit[j][i] = trees[j].predict(&x[i * p..(i + 1) * p]);
                }
            }
            // Sample σ² from its inverse-gamma full conditional.
            let mut sse = 0.0;
            for i in 0..n {
                let f: f64 = (0..m).map(|j| tree_fit[j][i]).sum();
                sse += (ys[i] - f).powi(2);
            }
            let shape = (nu + n as f64) / 2.0;
            let rate = (nu * lambda + sse) / 2.0;
            sigma2 = (rate / rng.gamma(shape)).max(1e-9);

            if it >= burn_in {
                draws.push(trees.clone());
            }
        }

        Ok(Self { draws, y_center, y_scale, p })
    }

    /// Posterior-mean prediction for one row (in the original y units).
    pub fn predict_row(&self, x_row: &[f64]) -> f64 {
        let mut acc = 0.0;
        for forest in &self.draws {
            let f: f64 = forest.iter().map(|t| t.predict(x_row)).sum();
            acc += f;
        }
        let mean_scaled = acc / self.draws.len().max(1) as f64;
        mean_scaled * self.y_scale + self.y_center
    }

    pub fn predict(&self, x: &[f64], n: usize) -> Vec<f64> {
        (0..n).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }

    pub fn n_draws(&self) -> usize {
        self.draws.len()
    }
}

/// Residual sum + count of points falling into each leaf of `tree`.
fn leaf_stats(tree: &Tree, x: &[f64], r: &[f64], n: usize, p: usize) -> std::collections::HashMap<usize, (f64, f64)> {
    let mut m: std::collections::HashMap<usize, (f64, f64)> = std::collections::HashMap::new();
    for i in 0..n {
        let l = tree.leaf_of(&x[i * p..(i + 1) * p]);
        let e = m.entry(l).or_insert((0.0, 0.0));
        e.0 += r[i];
        e.1 += 1.0;
    }
    m
}

#[allow(clippy::too_many_arguments)]
fn mh_step(
    tree: &mut Tree,
    x: &[f64],
    r: &[f64],
    n: usize,
    p: usize,
    sigma2: f64,
    sigma_mu2: f64,
    p_split: &impl Fn(usize) -> f64,
    rng: &mut Rng,
) {
    let nogs = tree.nog_nodes();
    let do_grow = nogs.is_empty() || rng.unit() < 0.5;

    if do_grow {
        let leaves = tree.leaves();
        if leaves.is_empty() {
            return;
        }
        let leaf = leaves[rng.below(leaves.len())];
        // Gather this leaf's point indices.
        let idx: Vec<usize> = (0..n).filter(|&i| tree.leaf_of(&x[i * p..(i + 1) * p]) == leaf).collect();
        if idx.len() < 2 {
            return;
        }
        // Pick a feature with ≥2 distinct values, and a threshold between two values.
        let feature = rng.below(p);
        let mut vals: Vec<f64> = idx.iter().map(|&i| x[i * p + feature]).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        vals.dedup();
        if vals.len() < 2 {
            return;
        }
        let cut = vals[1 + rng.below(vals.len() - 1) - 1]; // a value with something above it
        let threshold = cut;
        // Split sums.
        let (mut sl, mut nl, mut sr, mut nr) = (0.0, 0.0, 0.0, 0.0);
        let mut s = 0.0;
        for &i in &idx {
            let v = r[i];
            s += v;
            if x[i * p + feature] <= threshold {
                sl += v;
                nl += 1.0;
            } else {
                sr += v;
                nr += 1.0;
            }
        }
        if nl < 1.0 || nr < 1.0 {
            return;
        }
        let d = tree.nodes[leaf].depth;
        let loglik_ratio = leaf_loglik(sl, nl, sigma2, sigma_mu2) + leaf_loglik(sr, nr, sigma2, sigma_mu2)
            - leaf_loglik(s, nl + nr, sigma2, sigma_mu2);
        let ps = p_split(d);
        let prior_ratio = (ps * (1.0 - p_split(d + 1)).powi(2) / (1.0 - ps)).max(1e-300);
        let n_grow = leaves.len() as f64;
        let n_nog_new = (tree.nog_nodes().len() + 1) as f64; // this leaf becomes a nog
        let log_trans = (n_grow / n_nog_new).ln(); // P_prune=P_grow
        let log_alpha = loglik_ratio + prior_ratio.ln() + log_trans;
        if log_alpha >= 0.0 || rng.unit() < log_alpha.exp() {
            // Perform the grow.
            let li = tree.nodes.len();
            let ri = li + 1;
            let depth = d + 1;
            tree.nodes.push(Node { leaf: true, feature: 0, threshold: 0.0, left: 0, right: 0, depth, mu: 0.0 });
            tree.nodes.push(Node { leaf: true, feature: 0, threshold: 0.0, left: 0, right: 0, depth, mu: 0.0 });
            let nd = &mut tree.nodes[leaf];
            nd.leaf = false;
            nd.feature = feature;
            nd.threshold = threshold;
            nd.left = li;
            nd.right = ri;
        }
    } else {
        // Prune a random nog node.
        let nog = nogs[rng.below(nogs.len())];
        let (lc, rc) = (tree.nodes[nog].left, tree.nodes[nog].right);
        let idx: Vec<usize> = (0..n).filter(|&i| {
            // points reaching `nog` then either child
            let leaf = tree.leaf_of(&x[i * p..(i + 1) * p]);
            leaf == lc || leaf == rc
        }).collect();
        let (mut sl, mut nl, mut sr, mut nr) = (0.0, 0.0, 0.0, 0.0);
        for &i in &idx {
            let leaf = tree.leaf_of(&x[i * p..(i + 1) * p]);
            if leaf == lc {
                sl += r[i];
                nl += 1.0;
            } else {
                sr += r[i];
                nr += 1.0;
            }
        }
        if nl < 1.0 || nr < 1.0 {
            return;
        }
        let d = tree.nodes[nog].depth;
        // Reverse of grow: ratio is the reciprocal.
        let loglik_ratio = leaf_loglik(sl + sr, nl + nr, sigma2, sigma_mu2)
            - leaf_loglik(sl, nl, sigma2, sigma_mu2) - leaf_loglik(sr, nr, sigma2, sigma_mu2);
        let ps = p_split(d);
        let prior_ratio = ((1.0 - ps) / (ps * (1.0 - p_split(d + 1)).powi(2))).max(1e-300);
        let n_nog = nogs.len() as f64;
        let n_leaves_after = (tree.leaves().len() - 1) as f64; // two leaves → one
        let log_trans = (n_nog / n_leaves_after).ln();
        let log_alpha = loglik_ratio + prior_ratio.ln() + log_trans;
        if log_alpha >= 0.0 || rng.unit() < log_alpha.exp() {
            // Collapse `nog` to a leaf (children become dead but harmless; we
            // rebuild a compact tree to keep indices valid).
            tree.nodes[nog].leaf = true;
            compact(tree);
        }
    }
}

/// Rebuild the tree dropping unreachable nodes (after a prune) so child indices
/// stay valid.
fn compact(tree: &mut Tree) {
    let mut new_nodes = Vec::new();
    let mut map = std::collections::HashMap::new();
    fn visit(old: usize, src: &[Node], new_nodes: &mut Vec<Node>, map: &mut std::collections::HashMap<usize, usize>) -> usize {
        let id = new_nodes.len();
        map.insert(old, id);
        let mut nd = src[old].clone();
        new_nodes.push(nd.clone());
        if !src[old].leaf {
            let l = visit(src[old].left, src, new_nodes, map);
            let r = visit(src[old].right, src, new_nodes, map);
            new_nodes[id].left = l;
            new_nodes[id].right = r;
        }
        let _ = &mut nd;
        id
    }
    visit(0, &tree.nodes, &mut new_nodes, &mut map);
    tree.nodes = new_nodes;
}

fn sample_leaves(tree: &mut Tree, x: &[f64], r: &[f64], n: usize, sigma2: f64, sigma_mu2: f64, rng: &mut Rng) {
    let p = x.len() / n;
    let stats = leaf_stats(tree, x, r, n, p);
    for (leaf, (s, cnt)) in stats {
        let prec = cnt / sigma2 + 1.0 / sigma_mu2;
        let mean = (s / sigma2) / prec;
        let sd = (1.0 / prec).sqrt();
        tree.nodes[leaf].mu = mean + sd * rng.gaussian();
    }
    // Leaves with no points keep μ = 0.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::r2_score;

    #[test]
    fn fits_a_nonlinear_function() {
        // y = sin(x) over [0, 2π]; BART's sum-of-trees should track it well.
        let n = 60;
        let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();
        let bart = Bart::fit(&x, &y, n, 1, 40, 250, 100, 2.0, 1).unwrap();
        let preds = bart.predict(&x, n);
        let r2 = r2_score(&y, &preds).unwrap();
        assert!(r2 > 0.85, "BART R^2 too low: {r2}");
        assert!(bart.n_draws() == 150);
    }

    #[test]
    fn fits_a_two_feature_surface() {
        // y depends on both features (a step in each).
        let n = 80;
        let mut x = vec![0.0; n * 2];
        let mut y = vec![0.0; n];
        for i in 0..n {
            let a = (i % 8) as f64;
            let b = (i / 8) as f64;
            x[i * 2] = a;
            x[i * 2 + 1] = b;
            y[i] = if a > 3.5 { 2.0 } else { 0.0 } + if b > 4.5 { 1.0 } else { 0.0 };
        }
        let bart = Bart::fit(&x, &y, n, 2, 40, 200, 80, 2.0, 7).unwrap();
        let r2 = r2_score(&y, &bart.predict(&x, n)).unwrap();
        assert!(r2 > 0.8, "BART R^2 {r2}");
    }

    #[test]
    fn guards() {
        assert_eq!(Bart::fit(&[1.0], &[1.0], 1, 1, 10, 20, 10, 2.0, 0).unwrap_err(), LearningError::InvalidDimension);
    }
}
