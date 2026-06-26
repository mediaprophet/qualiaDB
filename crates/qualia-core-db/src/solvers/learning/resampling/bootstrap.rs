//! The bootstrap (ISL ch 5.2) — resample-with-replacement to estimate the
//! sampling distribution (standard error / spread) of an arbitrary statistic.

use crate::solvers::statistics::descriptive::{mean, std_dev};

/// Deterministic LCG for reproducible resamples.
struct Lcg(u64);
impl Lcg {
    fn next_below(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

/// One bootstrap resample of `n` row indices, drawn with replacement.
pub fn bootstrap_indices(n: usize, seed: u64) -> Vec<usize> {
    let mut rng = Lcg(seed ^ 0xD1B54A32D192ED03);
    (0..n).map(|_| rng.next_below(n)).collect()
}

/// Bootstrap estimate of a scalar statistic's sampling distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapResult {
    /// The statistic evaluated on the original full sample.
    pub estimate: f64,
    /// Mean of the statistic across the `b` resamples.
    pub boot_mean: f64,
    /// Bootstrap standard error (std-dev of the resample statistics).
    pub std_error: f64,
    /// Bias estimate `boot_mean − estimate`.
    pub bias: f64,
}

/// Estimate the standard error (and bias) of `statistic` applied to `data`, over
/// `b` bootstrap resamples. `statistic` maps a sample slice to a scalar. `None`
/// if `data` is empty or `b < 2`.
pub fn bootstrap_estimate(
    data: &[f64],
    b: usize,
    seed: u64,
    statistic: impl Fn(&[f64]) -> f64,
) -> Option<BootstrapResult> {
    let n = data.len();
    if n == 0 || b < 2 {
        return None;
    }
    let estimate = statistic(data);
    let mut stats = Vec::with_capacity(b);
    let mut sample = vec![0.0; n];
    for r in 0..b {
        let idx = bootstrap_indices(n, seed.wrapping_add(r as u64));
        for (s, &i) in sample.iter_mut().zip(idx.iter()) {
            *s = data[i];
        }
        stats.push(statistic(&sample));
    }
    let boot_mean = mean(&stats)?;
    let std_error = std_dev(&stats, true).unwrap_or(0.0);
    Some(BootstrapResult {
        estimate,
        boot_mean,
        std_error,
        bias: boot_mean - estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::descriptive::{mean, std_dev};

    #[test]
    fn resample_indices_in_range_and_reproducible() {
        let a = bootstrap_indices(20, 123);
        let b = bootstrap_indices(20, 123);
        assert_eq!(a, b, "same seed → same resample");
        assert!(a.iter().all(|&i| i < 20));
        assert_eq!(a.len(), 20);
    }

    #[test]
    fn bootstrap_se_of_mean_matches_clt() {
        // For the sample mean, the bootstrap SE ≈ sample_std/√n.
        let data: Vec<f64> = (1..=50).map(|i| i as f64).collect();
        let r = bootstrap_estimate(&data, 2000, 7, |s| mean(s).unwrap()).unwrap();
        let analytic_se = std_dev(&data, true).unwrap() / (data.len() as f64).sqrt();
        assert!((r.estimate - mean(&data).unwrap()).abs() < 1e-12);
        // Within ~10% of the analytic SE (Monte-Carlo tolerance).
        assert!((r.std_error - analytic_se).abs() / analytic_se < 0.1,
            "boot SE {} vs analytic {}", r.std_error, analytic_se);
        // Bias of the mean is ~0.
        assert!(r.bias.abs() < 0.5);
    }

    #[test]
    fn guards() {
        assert!(bootstrap_estimate(&[], 100, 0, |_| 0.0).is_none());
        assert!(bootstrap_estimate(&[1.0, 2.0], 1, 0, |_| 0.0).is_none());
    }
}
