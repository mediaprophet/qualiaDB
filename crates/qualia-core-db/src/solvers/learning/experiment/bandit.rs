//! Multi-armed bandits (Practical Statistics ch 3) — sequential experimentation
//! that *adapts*: instead of a fixed-split A/B test, allocate more trials to the
//! arms that look better, trading exploration against exploitation. Three classic
//! policies: ε-greedy, UCB1, and Thompson sampling (Beta-Bernoulli). Kernel-class
//! `Divergent` (the sampling/branch logic).

/// Allocation policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Policy {
    /// With probability ε pick a random arm, else the empirically best.
    EpsilonGreedy(f64),
    /// Upper Confidence Bound (UCB1): optimism under uncertainty.
    Ucb1,
    /// Thompson sampling with a Beta-Bernoulli posterior per arm.
    ThompsonBernoulli,
}

/// A bandit over `k` arms.
#[derive(Debug, Clone)]
pub struct Bandit {
    policy: Policy,
    counts: Vec<u64>,
    values: Vec<f64>, // running mean reward
    alpha: Vec<f64>,  // Beta posterior (Thompson)
    beta: Vec<f64>,
    t: u64,
    rng: Lcg,
}

#[derive(Debug, Clone)]
struct Lcg(u64);
impl Lcg {
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
    fn beta(&mut self, a: f64, b: f64) -> f64 {
        let x = self.gamma(a);
        let y = self.gamma(b);
        if x + y > 0.0 {
            x / (x + y)
        } else {
            0.5
        }
    }
}

impl Bandit {
    pub fn new(k: usize, policy: Policy, seed: u64) -> Self {
        Self {
            policy,
            counts: vec![0; k],
            values: vec![0.0; k],
            alpha: vec![1.0; k], // Beta(1,1) uniform prior
            beta: vec![1.0; k],
            t: 0,
            rng: Lcg(seed ^ 0x9E3779B97F4A7C15),
        }
    }

    /// Choose the next arm to pull.
    pub fn select(&mut self) -> usize {
        let k = self.counts.len();
        match self.policy {
            Policy::EpsilonGreedy(eps) => {
                // Try each untried arm first, then ε-greedy.
                if let Some(u) = (0..k).find(|&i| self.counts[i] == 0) {
                    return u;
                }
                if self.rng.unit() < eps {
                    self.rng.below(k)
                } else {
                    argmax(&self.values)
                }
            }
            Policy::Ucb1 => {
                if let Some(u) = (0..k).find(|&i| self.counts[i] == 0) {
                    return u;
                }
                let t = self.t.max(1) as f64;
                let mut best = 0;
                let mut best_v = f64::NEG_INFINITY;
                for i in 0..k {
                    let bonus = (2.0 * t.ln() / self.counts[i] as f64).sqrt();
                    let v = self.values[i] + bonus;
                    if v > best_v {
                        best_v = v;
                        best = i;
                    }
                }
                best
            }
            Policy::ThompsonBernoulli => {
                let mut best = 0;
                let mut best_s = f64::NEG_INFINITY;
                for i in 0..k {
                    let s = self.rng.beta(self.alpha[i], self.beta[i]);
                    if s > best_s {
                        best_s = s;
                        best = i;
                    }
                }
                best
            }
        }
    }

    /// Record a `reward ∈ [0,1]` for `arm` (1 = success / 0 = failure for Bernoulli).
    pub fn update(&mut self, arm: usize, reward: f64) {
        self.t += 1;
        self.counts[arm] += 1;
        let n = self.counts[arm] as f64;
        self.values[arm] += (reward - self.values[arm]) / n; // running mean
        self.alpha[arm] += reward.clamp(0.0, 1.0);
        self.beta[arm] += 1.0 - reward.clamp(0.0, 1.0);
    }

    pub fn counts(&self) -> &[u64] {
        &self.counts
    }
    pub fn values(&self) -> &[f64] {
        &self.values
    }
    /// The arm with the most pulls — the bandit's current best guess.
    pub fn best_arm(&self) -> usize {
        argmax_u64(&self.counts)
    }
}

fn argmax(v: &[f64]) -> usize {
    let mut best = 0;
    for i in 1..v.len() {
        if v[i] > v[best] {
            best = i;
        }
    }
    best
}
fn argmax_u64(v: &[u64]) -> usize {
    let mut best = 0;
    for i in 1..v.len() {
        if v[i] > v[best] {
            best = i;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic Bernoulli reward source.
    struct Arms {
        p: Vec<f64>,
        rng: Lcg,
    }
    impl Arms {
        fn pull(&mut self, arm: usize) -> f64 {
            if self.rng.unit() < self.p[arm] {
                1.0
            } else {
                0.0
            }
        }
    }

    fn run(policy: Policy) -> (Bandit, usize) {
        let mut bandit = Bandit::new(3, policy, 1);
        let mut arms = Arms { p: vec![0.2, 0.5, 0.8], rng: Lcg(42) };
        for _ in 0..3000 {
            let a = bandit.select();
            let r = arms.pull(a);
            bandit.update(a, r);
        }
        let best = bandit.best_arm();
        (bandit, best)
    }

    #[test]
    fn thompson_converges_to_the_best_arm() {
        let (b, best) = run(Policy::ThompsonBernoulli);
        assert_eq!(best, 2, "should mostly pull the 0.8 arm; counts {:?}", b.counts());
        assert!(b.counts()[2] > b.counts()[0], "best arm pulled more than the worst");
    }

    #[test]
    fn ucb1_converges_to_the_best_arm() {
        let (b, best) = run(Policy::Ucb1);
        assert_eq!(best, 2, "counts {:?}", b.counts());
    }

    #[test]
    fn epsilon_greedy_favours_the_best_arm() {
        let (b, _) = run(Policy::EpsilonGreedy(0.1));
        // The best arm gets the lion's share of the non-exploration pulls.
        assert!(b.counts()[2] > b.counts()[0] + b.counts()[1], "counts {:?}", b.counts());
        // Its estimated value is near the true 0.8.
        assert!((b.values()[2] - 0.8).abs() < 0.1, "value {}", b.values()[2]);
    }
}
