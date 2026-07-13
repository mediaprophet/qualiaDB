//! The bootstrap (ISL ch 5.2) — resample-with-replacement to estimate the
//! sampling distribution (standard error / spread) of an arbitrary statistic.

use crate::solvers::statistics::descriptive::{mean, std_dev};

/// Deterministic LCG for reproducible resamples.
struct Lcg(u64);
impl Lcg {
    fn next_below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
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

/// Which bootstrap confidence-interval to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiMethod {
    /// The α/2 and 1−α/2 percentiles of the bootstrap distribution.
    Percentile,
    /// Bias-corrected and accelerated (BCa) — corrects for bias and skew via a
    /// jackknife acceleration; the gold-standard nonparametric interval.
    Bca,
}

/// An earned confidence interval — derived by resampling the data, not assumed
/// from a Gaussian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapCi {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence: f64,
    pub method: CiMethod,
}

/// Bootstrap confidence interval for `statistic` at confidence `1 − alpha`
/// (`alpha` in `(0,1)`, e.g. `0.05` → 95%). `None` if `data` is empty, `b < 2`,
/// or `alpha` is out of range.
pub fn bootstrap_ci(
    data: &[f64],
    b: usize,
    alpha: f64,
    seed: u64,
    method: CiMethod,
    statistic: impl Fn(&[f64]) -> f64,
) -> Option<BootstrapCi> {
    use crate::solvers::statistics::descriptive::quantile_sorted;
    use crate::solvers::statistics::distributions::normal;

    let n = data.len();
    if n < 2 || b < 2 || !(0.0..1.0).contains(&alpha) || alpha <= 0.0 {
        return None;
    }
    let estimate = statistic(data);

    // Bootstrap replicates.
    let mut boots = Vec::with_capacity(b);
    let mut sample = vec![0.0; n];
    for r in 0..b {
        let idx = bootstrap_indices(n, seed.wrapping_add(r as u64));
        for (s, &i) in sample.iter_mut().zip(idx.iter()) {
            *s = data[i];
        }
        boots.push(statistic(&sample));
    }
    boots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let (lo_q, hi_q) = match method {
        CiMethod::Percentile => (alpha / 2.0, 1.0 - alpha / 2.0),
        CiMethod::Bca => {
            // Bias correction z0 from the fraction of replicates below the estimate.
            let n_below = boots.iter().filter(|&&v| v < estimate).count();
            let frac = (n_below as f64 / b as f64).clamp(1e-9, 1.0 - 1e-9);
            let z0 = normal::standard_quantile(frac);
            // Acceleration from the jackknife distribution.
            let mut jack = vec![0.0; n];
            let mut loo = vec![0.0; n - 1];
            for i in 0..n {
                let mut k = 0;
                for (j, &v) in data.iter().enumerate() {
                    if j != i {
                        loo[k] = v;
                        k += 1;
                    }
                }
                jack[i] = statistic(&loo);
            }
            let jbar = jack.iter().sum::<f64>() / n as f64;
            let mut num = 0.0;
            let mut den = 0.0;
            for &j in &jack {
                let d = jbar - j;
                num += d * d * d;
                den += d * d;
            }
            let a = if den > 0.0 {
                num / (6.0 * den.powf(1.5))
            } else {
                0.0
            };
            // Adjusted percentiles.
            let adj = |z_alpha: f64| {
                let num = z0 + z_alpha;
                normal::standard_cdf(z0 + num / (1.0 - a * num))
            };
            let zlo = normal::standard_quantile(alpha / 2.0);
            let zhi = normal::standard_quantile(1.0 - alpha / 2.0);
            (adj(zlo).clamp(0.0, 1.0), adj(zhi).clamp(0.0, 1.0))
        }
    };

    Some(BootstrapCi {
        estimate,
        lower: quantile_sorted(&boots, lo_q)?,
        upper: quantile_sorted(&boots, hi_q)?,
        confidence: 1.0 - alpha,
        method,
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
        assert!(
            (r.std_error - analytic_se).abs() / analytic_se < 0.1,
            "boot SE {} vs analytic {}",
            r.std_error,
            analytic_se
        );
        // Bias of the mean is ~0.
        assert!(r.bias.abs() < 0.5);
    }

    #[test]
    fn percentile_ci_brackets_the_true_mean() {
        // Data centered at 10; the 95% bootstrap CI for the mean should bracket 10
        // and be ordered lower < estimate < upper.
        let data: Vec<f64> = (0..60)
            .map(|i| 10.0 + ((i * 17 % 40) as f64 - 20.0) / 7.0)
            .collect();
        let ci = bootstrap_ci(&data, 2000, 0.05, 1, CiMethod::Percentile, |s| {
            mean(s).unwrap()
        })
        .unwrap();
        assert!(ci.lower < ci.estimate && ci.estimate < ci.upper);
        assert!(
            ci.lower < 10.0 && ci.upper > 10.0,
            "CI [{}, {}] should bracket 10",
            ci.lower,
            ci.upper
        );
        assert!((ci.confidence - 0.95).abs() < 1e-12);
    }

    #[test]
    fn bca_runs_and_is_a_valid_interval() {
        let data: Vec<f64> = (1..=40).map(|i| i as f64).collect();
        let ci = bootstrap_ci(&data, 2000, 0.1, 3, CiMethod::Bca, |s| mean(s).unwrap()).unwrap();
        assert!(ci.lower < ci.upper);
        // Mean of 1..=40 is 20.5; the 90% CI brackets it.
        assert!(ci.lower < 20.5 && ci.upper > 20.5);
        assert_eq!(ci.method, CiMethod::Bca);
    }

    #[test]
    fn ci_works_for_a_nonlinear_statistic() {
        // The bootstrap earns a CI for the median too — no Gaussian assumption.
        use crate::solvers::statistics::descriptive::median_in_place;
        let data: Vec<f64> = (0..51).map(|i| i as f64).collect();
        let ci = bootstrap_ci(&data, 1500, 0.05, 5, CiMethod::Percentile, |s| {
            let mut v = s.to_vec();
            median_in_place(&mut v).unwrap()
        })
        .unwrap();
        assert!(ci.lower <= 25.0 && ci.upper >= 25.0); // true median is 25
    }

    #[test]
    fn guards() {
        assert!(bootstrap_estimate(&[], 100, 0, |_| 0.0).is_none());
        assert!(bootstrap_estimate(&[1.0, 2.0], 1, 0, |_| 0.0).is_none());
        assert!(bootstrap_ci(&[1.0], 100, 0.05, 0, CiMethod::Percentile, |_| 0.0).is_none());
        assert!(bootstrap_ci(&[1.0, 2.0], 100, 1.5, 0, CiMethod::Percentile, |_| 0.0).is_none());
    }
}
