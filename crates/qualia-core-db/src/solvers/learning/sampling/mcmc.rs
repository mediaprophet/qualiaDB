//! Markov-Chain Monte Carlo (PRML ch 11) — random-walk Metropolis-Hastings over an
//! arbitrary (unnormalized) log-density. This is the general inference engine the
//! Bayesian methods lean on: give it `log p(x)` (up to a constant) and it returns
//! samples from `p`. Kernel-class `Divergent` (the accept/reject branch).
//!
//! The target need only be specified up to a normalizing constant — the Metropolis
//! acceptance ratio `exp(log p(x') − log p(x))` cancels it.

/// Result of an MCMC run.
#[derive(Debug, Clone)]
pub struct McmcResult {
    /// Post-burn-in samples, row-major `n_samples × dim`.
    pub samples: Vec<f64>,
    pub n_samples: usize,
    pub dim: usize,
    /// Fraction of proposals accepted (tune `proposal_std` toward ~0.234–0.5).
    pub acceptance_rate: f64,
}

impl McmcResult {
    /// Per-dimension sample mean.
    pub fn mean(&self) -> Vec<f64> {
        let mut m = vec![0.0; self.dim];
        for i in 0..self.n_samples {
            for j in 0..self.dim {
                m[j] += self.samples[i * self.dim + j];
            }
        }
        for v in m.iter_mut() {
            *v /= self.n_samples.max(1) as f64;
        }
        m
    }

    /// Per-dimension sample variance (population).
    pub fn variance(&self) -> Vec<f64> {
        let m = self.mean();
        let mut v = vec![0.0; self.dim];
        for i in 0..self.n_samples {
            for j in 0..self.dim {
                let d = self.samples[i * self.dim + j] - m[j];
                v[j] += d * d;
            }
        }
        for x in v.iter_mut() {
            *x /= self.n_samples.max(1) as f64;
        }
        v
    }
}

struct Rng(u64);
impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Random-walk Metropolis-Hastings. `log_density(x)` returns the target's log-density
/// (up to a constant) at a `dim`-vector; `initial` is the starting point;
/// `proposal_std` the per-dimension Gaussian step; `n_samples` post-burn-in draws;
/// `burn_in` discarded warm-up steps. Deterministic given `seed`.
pub fn metropolis_hastings<F>(
    log_density: F,
    initial: &[f64],
    proposal_std: f64,
    n_samples: usize,
    burn_in: usize,
    seed: u64,
) -> McmcResult
where
    F: Fn(&[f64]) -> f64,
{
    let dim = initial.len();
    let mut rng = Rng(seed ^ 0x9E3779B97F4A7C15);
    let mut x = initial.to_vec();
    let mut lp = log_density(&x);
    let mut prop = vec![0.0; dim];
    let mut samples = Vec::with_capacity(n_samples * dim);
    let mut accepted = 0u64;
    let total = burn_in + n_samples;

    for step in 0..total {
        // Propose x' = x + N(0, proposal_std²).
        for d in 0..dim {
            prop[d] = x[d] + proposal_std * rng.gaussian();
        }
        let lp_prop = log_density(&prop);
        // Accept with probability min(1, exp(lp' − lp)).
        let accept = lp_prop >= lp || rng.unit() < (lp_prop - lp).exp();
        if accept {
            x.copy_from_slice(&prop);
            lp = lp_prop;
            if step >= burn_in {
                accepted += 1;
            }
        }
        if step >= burn_in {
            samples.extend_from_slice(&x);
        }
    }

    McmcResult {
        samples,
        n_samples,
        dim,
        acceptance_rate: if n_samples > 0 {
            accepted as f64 / n_samples as f64
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_a_1d_gaussian() {
        // Target N(3, 2²): log p(x) = −(x−3)²/(2·4).
        let target = |x: &[f64]| -(x[0] - 3.0) * (x[0] - 3.0) / 8.0;
        let r = metropolis_hastings(target, &[0.0], 2.0, 40_000, 5_000, 1);
        let m = r.mean();
        let v = r.variance();
        assert!((m[0] - 3.0).abs() < 0.15, "mean {}", m[0]);
        assert!((v[0] - 4.0).abs() < 0.6, "var {}", v[0]);
        assert!(r.acceptance_rate > 0.1 && r.acceptance_rate < 0.95);
    }

    #[test]
    fn samples_a_2d_gaussian_mean() {
        // Independent N(1,1)×N(-2,1).
        let target = |x: &[f64]| -(x[0] - 1.0).powi(2) / 2.0 - (x[1] + 2.0).powi(2) / 2.0;
        let r = metropolis_hastings(target, &[0.0, 0.0], 1.0, 40_000, 5_000, 7);
        let m = r.mean();
        assert!(
            (m[0] - 1.0).abs() < 0.15 && (m[1] + 2.0).abs() < 0.15,
            "mean {m:?}"
        );
    }

    #[test]
    fn unnormalized_target_is_fine() {
        // log p need not be normalized — a constant offset must not change samples.
        let target = |x: &[f64]| 12345.0 - (x[0]).powi(2) / 2.0; // N(0,1) + constant
        let r = metropolis_hastings(target, &[0.0], 1.5, 20_000, 3_000, 2);
        assert!(r.mean()[0].abs() < 0.15);
        assert!((r.variance()[0] - 1.0).abs() < 0.3);
    }
}
