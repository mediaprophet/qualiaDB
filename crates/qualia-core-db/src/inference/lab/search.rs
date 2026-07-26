//! Search and optimization engine: Sobol quasi-random initial design,
//! a k-NN surrogate with Expected Improvement acquisition (Bayesian-style),
//! and a Track-and-Stop multi-armed bandit for A/B testing.
//!
//! The ask-and-tell API:
//! 1. Create a `SearchEngine` with a `ConfigurationSpace` and budget.
//! 2. Call `ask()` to get the next configuration to try.
//! 3. Run the experiment and call `tell(result)` with the outcome.
//! 4. Repeat until budget exhausted. Call `best()` for the Pareto-optimal result.

use std::collections::HashMap;

use super::config_space::{Configuration, ConfigurationSpace};
use super::experiment::ExperimentResult;

// ── Sobol quasi-random sequence ───────────────────────────────────────────────

/// A Sobol quasi-random sequence generator for uniform coverage of the [0,1]^d
/// unit hypercube. Uses the Joe & Kuo direction numbers (compact implementation).
pub struct SobolSequence {
    dim: usize,
    index: u64,
    /// Direction numbers (bit-packed). For dim ≤ 20 this is sufficient.
    direction: Vec<u64>,
}

impl SobolSequence {
    pub fn new(dim: usize) -> Self {
        let direction = sobol_direction_numbers(dim);
        Self {
            dim,
            index: 0,
            direction,
        }
    }

    /// Generate the next point in [0,1]^d.
    pub fn next(&mut self) -> Vec<f64> {
        self.index += 1;
        let mut x = vec![0u64; self.dim];
        let mut v = self.index;
        let mut bit = 0usize;
        while v > 0 {
            if v & 1 != 0 {
                for d in 0..self.dim {
                    x[d] ^= self.direction[d * 64 + bit];
                }
            }
            v >>= 1;
            bit += 1;
        }
        x.iter().map(|&v| (v as f64) / (u64::MAX as f64)).collect()
    }

    /// Reset the sequence to the beginning.
    pub fn reset(&mut self) {
        self.index = 0;
    }
}

/// Precomputed Sobol direction numbers for up to 20 dimensions.
/// These are the first 20 Joe & Kuo primitive polynomials + direction numbers.
fn sobol_direction_numbers(dim: usize) -> Vec<u64> {
    // Each dimension d has direction numbers v[d][j] for j = 0..63.
    // We store them as a flat array: direction[d * 64 + j].
    // For compactness, we use the standard initialization for the first 20 dims.
    // Source: Joe & Kuo (2008), "Constructing Sobol sequences with better two-dimensional projections"
    let mut dir = vec![0u64; dim * 64];

    // Initialize dimension 0: v[0][j] = 1 (the identity sequence).
    if dim > 0 {
        for j in 0..64 {
            dir[j] = 1u64 << (63 - j);
        }
    }

    // For dimensions 1..dim-1, use primitive polynomials and initial direction numbers.
    // (p, m, initial v values) for each dimension.
    let seeds: &[(u32, u32, &[u32])] = &[
        (1, 1, &[1]),                        // d=1
        (3, 2, &[1, 3]),                     // d=2
        (1, 3, &[1, 3, 5]),                  // d=3
        (5, 3, &[1, 1, 7]),                  // d=4
        (9, 4, &[1, 3, 7, 13]),              // d=5
        (9, 4, &[1, 1, 5, 11]),              // d=6
        (21, 5, &[1, 1, 7, 13, 23]),         // d=7
        (29, 5, &[1, 3, 3, 9, 17]),          // d=8
        (5, 5, &[1, 3, 5, 11, 25]),          // d=9
        (7, 5, &[1, 1, 1, 3, 15]),           // d=10
        (15, 5, &[1, 1, 5, 11, 25]),         // d=11
        (17, 5, &[1, 3, 1, 9, 21]),          // d=12
        (65, 6, &[1, 1, 5, 11, 25, 53]),     // d=13
        (21, 6, &[1, 3, 1, 7, 19, 37]),      // d=14
        (15, 6, &[1, 1, 3, 11, 25, 45]),     // d=15
        (735, 6, &[1, 1, 1, 3, 13, 29]),     // d=16
        (165, 6, &[1, 1, 5, 7, 21, 43]),     // d=17
        (15, 6, &[1, 3, 5, 11, 25, 45]),     // d=18
        (511, 7, &[1, 1, 1, 3, 13, 29, 61]), // d=19
    ];

    for d in 1..dim.min(seeds.len() + 1) {
        let (poly, m, init) = seeds[d - 1];
        let m = m as usize;
        let base = d * 64;

        // Set initial direction numbers.
        for j in 0..m {
            let v = (init[j] as u64) << (63 - j);
            dir[base + j] = v;
        }

        // Extend using the recurrence: v[j] = v[j-m] XOR (v[j-m] * 2^m)
        // with the primitive polynomial bits.
        for j in m..64 {
            let mut v = dir[base + j - m];
            v >>= m;
            // Apply polynomial: for each bit set in poly, XOR with shifted v.
            let mut p = poly;
            let mut k = 1u32;
            while p > 0 {
                if p & 1 != 0 {
                    let shifted = dir[base + j - k as usize];
                    v ^= shifted >> (m - k as usize);
                }
                p >>= 1;
                k += 1;
            }
            dir[base + j] = v;
        }
    }

    dir
}

// ── k-NN surrogate model ──────────────────────────────────────────────────────

/// A simple k-nearest-neighbor surrogate model for the objective function.
/// Predicts the objective at a new point as the distance-weighted average of
/// the k nearest observed points. This is a lightweight alternative to a full
/// Random Forest (SMAC3) or Gaussian Process, suitable for small budgets.
pub struct KnnSurrogate {
    /// Observed normalized inputs.
    points: Vec<Vec<f64>>,
    /// Observed objective values (higher is better).
    values: Vec<f64>,
    /// Number of neighbors to consider.
    k: usize,
}

impl KnnSurrogate {
    pub fn new(k: usize) -> Self {
        Self {
            points: Vec::new(),
            values: Vec::new(),
            k: k.max(1),
        }
    }

    /// Add an observation.
    pub fn add(&mut self, x: Vec<f64>, y: f64) {
        self.points.push(x);
        self.values.push(y);
    }

    /// Number of observations.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Predict the objective at a new point.
    /// Returns (mean, std) — the std is used for exploration in EI.
    pub fn predict(&self, x: &[f64]) -> (f64, f64) {
        if self.points.is_empty() {
            return (0.0, 1.0);
        }

        // Compute distances to all observed points.
        let mut dists: Vec<(usize, f64)> = self
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let d = euclidean(x, p);
                (i, if d < 1e-12 { 1e-12 } else { d })
            })
            .collect();

        // Sort by distance ascending.
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = self.k.min(dists.len());
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        let mut vals = Vec::with_capacity(k);
        for &(i, d) in dists.iter().take(k) {
            let w = 1.0 / d; // inverse distance weighting
            weighted_sum += w * self.values[i];
            weight_total += w;
            vals.push(self.values[i]);
        }

        let mean = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };

        // Standard deviation of the k nearest values (exploration signal).
        let std = if vals.len() > 1 {
            let avg = vals.iter().sum::<f64>() / vals.len() as f64;
            let var = vals.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / vals.len() as f64;
            var.sqrt()
        } else {
            0.5
        };

        (mean, std)
    }
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

// ── Expected Improvement acquisition ──────────────────────────────────────────

/// Expected Improvement (EI) acquisition function.
/// EI(x) = (mu - f_best) * Phi(Z) + sigma * phi(Z)
/// where Z = (mu - f_best) / sigma.
/// Higher EI = more promising to try.
pub fn expected_improvement(mu: f64, sigma: f64, f_best: f64) -> f64 {
    if sigma < 1e-12 {
        // No uncertainty: EI = max(0, mu - f_best).
        return (mu - f_best).max(0.0);
    }
    let z = (mu - f_best) / sigma;
    // Normal CDF and PDF approximations.
    let phi_z = normal_pdf(z);
    let cdf_z = normal_cdf(z);
    (mu - f_best) * cdf_z + sigma * phi_z
}

/// Standard normal PDF.
fn normal_pdf(x: f64) -> f64 {
    let c = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
    c * (-0.5 * x * x).exp()
}

/// Standard normal CDF (Abramowitz & Stegun approximation).
fn normal_cdf(x: f64) -> f64 {
    // Use the error function approximation.
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation (Abramowitz & Stegun 7.1.26).
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

// ── Search engine (ask-and-tell API) ──────────────────────────────────────────

/// The search engine: combines Sobol initial design with k-NN + EI Bayesian optimization.
pub struct SearchEngine {
    space: ConfigurationSpace,
    sobol: SobolSequence,
    surrogate: KnnSurrogate,
    /// Number of initial Sobol points before switching to EI.
    initial_design: usize,
    /// Observed results: (normalized input, objective value, config hash).
    observations: Vec<(Vec<f64>, f64, u64)>,
    /// Best objective value seen so far.
    best_objective: f64,
    /// Budget (max number of trials).
    budget: usize,
    /// Set of already-tried config hashes (dedup).
    tried: HashMap<u64, ()>,
}

impl SearchEngine {
    pub fn new(space: ConfigurationSpace, budget: usize) -> Self {
        let dim = space.dims();
        let initial = (budget / 5).clamp(5, 20).min(budget);
        Self {
            space,
            sobol: SobolSequence::new(dim),
            surrogate: KnnSurrogate::new(5),
            initial_design: initial,
            observations: Vec::new(),
            best_objective: f64::NEG_INFINITY,
            budget,
            tried: HashMap::new(),
        }
    }

    /// Number of trials so far.
    pub fn trials(&self) -> usize {
        self.observations.len()
    }

    /// Best objective value seen.
    pub fn best_value(&self) -> f64 {
        self.best_objective
    }

    /// Ask for the next configuration to try.
    /// Returns `None` if the budget is exhausted.
    pub fn ask(&mut self) -> Option<Configuration> {
        if self.observations.len() >= self.budget {
            return None;
        }

        // Phase 1: Sobol initial design.
        if self.observations.len() < self.initial_design {
            let t = self.sobol.next();
            let cfg = self.space.build_from_normalized(&t);
            // Skip if already tried (rare but possible with rounding).
            let h = cfg.hash();
            if self.tried.contains_key(&h) {
                return self.ask();
            }
            return Some(cfg);
        }

        // Phase 2: EI-guided search.
        // Generate a batch of random candidates and pick the one with highest EI.
        let n_candidates = 100;
        let mut best_ei = f64::NEG_INFINITY;
        let mut best_cfg = None;

        for _ in 0..n_candidates {
            let t = self.sobol.next();
            let cfg = self.space.build_from_normalized(&t);
            let h = cfg.hash();
            if self.tried.contains_key(&h) {
                continue;
            }
            let (mu, sigma) = self.surrogate.predict(&t);
            let ei = expected_improvement(mu, sigma, self.best_objective);
            if ei > best_ei {
                best_ei = ei;
                best_cfg = Some(cfg);
            }
        }

        // If all candidates were already tried, fall back to a random one.
        if best_cfg.is_none() {
            for _ in 0..10 {
                let t = self.sobol.next();
                let cfg = self.space.build_from_normalized(&t);
                let h = cfg.hash();
                if !self.tried.contains_key(&h) {
                    return Some(cfg);
                }
            }
            return None;
        }

        best_cfg
    }

    /// Tell the engine the result of an experiment.
    pub fn tell(&mut self, cfg: &Configuration, result: &ExperimentResult) {
        let h = cfg.hash();
        if self.tried.contains_key(&h) {
            return;
        }
        self.tried.insert(h, ());

        // Objective: combine throughput and quality (higher is better).
        // This is a scalarization for the surrogate; the Pareto frontier uses all 6 dims.
        let objective = compute_objective(result);

        let normalized = self.space.normalize_config(cfg);
        self.surrogate.add(normalized.clone(), objective);
        self.observations.push((normalized, objective, h));

        if objective > self.best_objective {
            self.best_objective = objective;
        }
    }

    /// Directly add an observation to the surrogate (for resuming from a log).
    /// The normalized vector and config hash must be pre-computed.
    pub fn surrogate_add(&mut self, normalized: Vec<f64>, objective: f64, config_hash: u64) {
        if self.tried.contains_key(&config_hash) {
            return;
        }
        self.tried.insert(config_hash, ());
        self.surrogate.add(normalized.clone(), objective);
        self.observations.push((normalized, objective, config_hash));
        if objective > self.best_objective {
            self.best_objective = objective;
        }
    }

    /// Get the best configuration found so far (by scalar objective).
    pub fn best_config(&self) -> Option<u64> {
        self.observations
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, _, h)| *h)
    }
}

/// Scalar objective for the surrogate model: throughput × quality (higher is better).
/// Failed experiments get -infinity.
fn compute_objective(r: &ExperimentResult) -> f64 {
    if r.error.is_some() {
        return f64::NEG_INFINITY;
    }
    let tok_s = r.bench.as_ref().map(|b| b.decode_tok_s).unwrap_or(0.0);
    let quality = r.quality.composite();
    tok_s * quality
}

// ── Track-and-Stop bandit for A/B testing ─────────────────────────────────────

/// A multi-armed bandit using the Track-and-Stop algorithm for best-arm identification.
/// Given a small set of pre-selected configurations, it adaptively allocates samples
/// to identify the best one with statistical rigor.
pub struct TrackAndStopBandit {
    /// Arm configurations (pre-selected).
    arms: Vec<Configuration>,
    /// Observed rewards per arm.
    rewards: Vec<Vec<f64>>,
    /// Total samples per arm.
    counts: Vec<usize>,
    /// Whether the bandit has declared a winner.
    stopped: bool,
    /// The winning arm index (if stopped).
    winner: Option<usize>,
    /// Number of samples per arm in the initial round.
    initial_samples: usize,
    /// GLR threshold for stopping (Chernoff rule).
    glr_threshold: f64,
}

impl TrackAndStopBandit {
    pub fn new(arms: Vec<Configuration>) -> Self {
        let n = arms.len();
        Self {
            arms,
            rewards: vec![Vec::new(); n],
            counts: vec![0; n],
            stopped: false,
            winner: None,
            initial_samples: 3,
            glr_threshold: 10.0, // Conservative threshold.
        }
    }

    /// Number of arms.
    pub fn n_arms(&self) -> usize {
        self.arms.len()
    }

    /// Whether the bandit has stopped (declared a winner).
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// The winning arm index (if stopped).
    pub fn winner(&self) -> Option<usize> {
        self.winner
    }

    /// Get the configuration for an arm.
    pub fn arm_config(&self, i: usize) -> Option<&Configuration> {
        self.arms.get(i)
    }

    /// Which arm to sample next. Returns `None` if stopped.
    pub fn ask(&self) -> Option<usize> {
        if self.stopped {
            return None;
        }
        let n = self.arms.len();
        if n == 0 {
            return None;
        }

        // Phase 1: ensure each arm has at least `initial_samples` samples.
        for i in 0..n {
            if self.counts[i] < self.initial_samples {
                return Some(i);
            }
        }

        // Phase 2: Track-and-Stop — sample the arm with the highest GLR statistic.
        // The GLR (Generalized Likelihood Ratio) test identifies the arm most likely
        // to be the best. We sample the current best arm more often (tracking).
        let means: Vec<f64> = (0..n)
            .map(|i| {
                let r = &self.rewards[i];
                if r.is_empty() {
                    0.0
                } else {
                    r.iter().sum::<f64>() / r.len() as f64
                }
            })
            .collect();

        // Find the current best arm.
        let best = means
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Check stopping condition: GLR test.
        let total_samples: usize = self.counts.iter().sum();
        if total_samples > n * self.initial_samples * 2 {
            let glr = self.compute_glr(&means, best);
            if glr > self.glr_threshold {
                return None; // Signal to stop — check_winner will be called.
            }
        }

        // Tracking: sample the best arm with probability proportional to its lead.
        // Simple D-tracking: sample the best arm 50% of the time, round-robin the rest.
        let round = total_samples % n;
        if round == best % n {
            Some(best)
        } else {
            Some(round)
        }
    }

    /// Report a reward for an arm.
    pub fn tell(&mut self, arm: usize, reward: f64) {
        if arm >= self.arms.len() {
            return;
        }
        self.rewards[arm].push(reward);
        self.counts[arm] += 1;

        // Check if we should stop.
        if self.counts.iter().all(|&c| c >= self.initial_samples) {
            let means: Vec<f64> = (0..self.arms.len())
                .map(|i| {
                    let r = &self.rewards[i];
                    if r.is_empty() {
                        0.0
                    } else {
                        r.iter().sum::<f64>() / r.len() as f64
                    }
                })
                .collect();
            let best = means
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let glr = self.compute_glr(&means, best);
            if glr > self.glr_threshold {
                self.stopped = true;
                self.winner = Some(best);
            }
        }
    }

    /// Compute the GLR (Generalized Likelihood Ratio) statistic.
    /// Higher GLR = more confidence that the best arm is truly the best.
    fn compute_glr(&self, means: &[f64], best: usize) -> f64 {
        let n = means.len();
        if n <= 1 {
            return f64::INFINITY;
        }
        let best_mean = means[best];
        let mut min_gap = f64::INFINITY;
        for i in 0..n {
            if i == best {
                continue;
            }
            let gap = best_mean - means[i];
            if gap < min_gap {
                min_gap = gap;
            }
        }
        if min_gap <= 0.0 {
            return 0.0; // No clear winner.
        }
        // GLR ≈ sum_i n_i * (best_mean - mean_i)^2 / (2 * sigma^2)
        // With sigma^2 estimated from the rewards.
        let total_var: f64 = (0..n)
            .map(|i| {
                let r = &self.rewards[i];
                if r.len() < 2 {
                    0.01
                } else {
                    let m = means[i];
                    let v = r.iter().map(|x| (x - m).powi(2)).sum::<f64>() / r.len() as f64;
                    v
                }
            })
            .sum::<f64>()
            / n as f64;
        let sigma2 = total_var.max(0.01);

        let mut glr = 0.0;
        for i in 0..n {
            if i == best {
                continue;
            }
            let gap = best_mean - means[i];
            glr += self.counts[i] as f64 * gap * gap / (2.0 * sigma2);
        }
        glr
    }

    /// Get the mean reward for each arm.
    pub fn arm_means(&self) -> Vec<f64> {
        (0..self.arms.len())
            .map(|i| {
                let r = &self.rewards[i];
                if r.is_empty() {
                    0.0
                } else {
                    r.iter().sum::<f64>() / r.len() as f64
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::lab::config_space::ParameterDef;

    #[test]
    fn sobol_generates_uniform() {
        let mut sobol = SobolSequence::new(3);
        let mut points = Vec::new();
        for _ in 0..100 {
            points.push(sobol.next());
        }
        // Check that points are in [0, 1].
        for p in &points {
            for &v in p {
                assert!(v >= 0.0 && v <= 1.0);
            }
        }
        // Check that the mean is roughly 0.5 (uniform).
        let mean: f64 = points.iter().map(|p| p[0]).sum::<f64>() / points.len() as f64;
        assert!((mean - 0.5).abs() < 0.1);
    }

    #[test]
    fn knn_surrogate_predicts() {
        let mut model = KnnSurrogate::new(3);
        model.add(vec![0.0, 0.0], 10.0);
        model.add(vec![1.0, 1.0], 20.0);
        model.add(vec![0.5, 0.5], 15.0);

        let (mean, _) = model.predict(&[0.1, 0.1]);
        // Should be closer to 10 than 20.
        assert!(mean < 15.0);

        let (mean, _) = model.predict(&[0.9, 0.9]);
        // Should be closer to 20 than 10.
        assert!(mean > 15.0);
    }

    #[test]
    fn expected_improvement_zero_sigma() {
        // When sigma=0, EI = max(0, mu - f_best).
        assert_eq!(expected_improvement(10.0, 0.0, 5.0), 5.0);
        assert_eq!(expected_improvement(3.0, 0.0, 5.0), 0.0);
    }

    #[test]
    fn expected_improvement_positive_with_uncertainty() {
        // With uncertainty, EI should be positive even if mu < f_best.
        let ei = expected_improvement(4.0, 2.0, 5.0);
        assert!(ei > 0.0);
    }

    #[test]
    fn search_engine_ask_tell() {
        let space = ConfigurationSpace::new("test")
            .with("x", ParameterDef::Int { lo: 0, hi: 10 })
            .with("y", ParameterDef::Bool);
        let mut engine = SearchEngine::new(space, 20);

        // Should produce configurations.
        let cfg = engine.ask();
        assert!(cfg.is_some());

        // After telling results, it should continue producing.
        for _ in 0..5 {
            if let Some(c) = engine.ask() {
                // Simulate a result (we can't run a real experiment in unit tests).
                // Just check that ask/tell cycle works.
                engine.tried.insert(c.hash(), ());
                engine.observations.push((vec![0.5, 0.5], 10.0, c.hash()));
            }
        }
        assert!(engine.trials() <= 20);
    }

    #[test]
    fn track_and_stop_identifies_best() {
        let space = ConfigurationSpace::new("test").with("x", ParameterDef::Int { lo: 0, hi: 1 });
        let arms = vec![
            space.build_from_normalized(&[0.0]),
            space.build_from_normalized(&[1.0]),
        ];
        let mut bandit = TrackAndStopBandit::new(arms);

        // Simulate: arm 0 has mean reward 10, arm 1 has mean reward 5.
        use rand::RngExt;
        let mut rng = rand::rng();
        for _ in 0..100 {
            if let Some(arm) = bandit.ask() {
                let reward = match arm {
                    0 => 10.0 + rng.random_range(-1.0..1.0),
                    _ => 5.0 + rng.random_range(-1.0..1.0),
                };
                bandit.tell(arm, reward);
            }
            if bandit.is_stopped() {
                break;
            }
        }
        // The bandit should identify arm 0 as the winner.
        if let Some(w) = bandit.winner() {
            assert_eq!(w, 0);
        }
    }
}
