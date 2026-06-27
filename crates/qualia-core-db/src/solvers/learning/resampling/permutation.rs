//! Permutation tests — assumption-free hypothesis testing by resampling.
//!
//! Under the null hypothesis that two groups are exchangeable, the labels carry no
//! information, so the sampling distribution of any test statistic is obtained by
//! **shuffling the pooled data** and recomputing it. The p-value is the fraction of
//! shuffles whose statistic is at least as extreme as the observed one. No
//! distributional assumption — the honest empirical twin of a parametric test.

/// Result of a two-sample permutation test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PermutationResult {
    /// The statistic on the original grouping.
    pub observed: f64,
    /// Two-sided p-value `(1 + #{|perm| ≥ |observed|}) / (n_perm + 1)`.
    pub p_value: f64,
    pub n_permutations: usize,
}

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

/// Two-sample permutation test of `statistic(a) − statistic(b)` (e.g. a difference
/// of means). Pools the two samples, repeatedly shuffles and re-splits into the
/// original sizes, and compares. `None` on an empty sample. The `statistic` closure
/// maps a group slice to a scalar.
pub fn two_sample_test(
    a: &[f64],
    b: &[f64],
    n_perm: usize,
    seed: u64,
    statistic: impl Fn(&[f64]) -> f64,
) -> Option<PermutationResult> {
    let (na, nb) = (a.len(), b.len());
    if na == 0 || nb == 0 || n_perm == 0 {
        return None;
    }
    let observed = statistic(a) - statistic(b);
    let obs_abs = observed.abs();

    // Pool the samples.
    let mut pool: Vec<f64> = Vec::with_capacity(na + nb);
    pool.extend_from_slice(a);
    pool.extend_from_slice(b);
    let n = pool.len();

    let mut rng = Lcg(seed ^ 0x9E3779B97F4A7C15);
    let mut count_extreme = 0usize;
    let mut ga = vec![0.0; na];
    let mut gb = vec![0.0; nb];
    for _ in 0..n_perm {
        // Fisher–Yates shuffle of the pool, then split.
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            pool.swap(i, j);
        }
        ga.copy_from_slice(&pool[..na]);
        gb.copy_from_slice(&pool[na..]);
        let stat = statistic(&ga) - statistic(&gb);
        if stat.abs() >= obs_abs {
            count_extreme += 1;
        }
    }
    // Add-one smoothing so the p-value is never exactly 0.
    let p_value = (1.0 + count_extreme as f64) / (n_perm as f64 + 1.0);
    Some(PermutationResult { observed, p_value, n_permutations: n_perm })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::descriptive::mean;

    #[test]
    fn detects_a_real_difference() {
        // Group b clearly higher than a → small p-value.
        let a = [1.0, 2.0, 1.5, 2.5, 1.8, 2.2];
        let b = [8.0, 9.0, 8.5, 9.5, 8.2, 9.1];
        let r = two_sample_test(&a, &b, 5000, 1, |s| mean(s).unwrap()).unwrap();
        assert!(r.observed < 0.0); // mean(a) - mean(b) < 0
        assert!(r.p_value < 0.01, "clear difference should be significant: p={}", r.p_value);
    }

    #[test]
    fn no_difference_is_not_significant() {
        // Two interleaved samples from the same distribution.
        let a = [5.0, 6.0, 4.0, 5.5, 4.5, 6.5];
        let b = [5.2, 5.8, 4.2, 5.6, 4.8, 6.2];
        let r = two_sample_test(&a, &b, 5000, 7, |s| mean(s).unwrap()).unwrap();
        assert!(r.p_value > 0.2, "similar groups should not be significant: p={}", r.p_value);
    }

    #[test]
    fn works_for_a_difference_of_medians() {
        use crate::solvers::statistics::descriptive::median_in_place;
        let a = [1.0, 2.0, 3.0, 4.0, 100.0]; // outlier — medians are robust
        let b = [10.0, 11.0, 12.0, 13.0, 14.0];
        let r = two_sample_test(&a, &b, 3000, 2, |s| {
            let mut v = s.to_vec();
            median_in_place(&mut v).unwrap()
        })
        .unwrap();
        // median(a)=3, median(b)=12 → observed difference negative.
        assert!(r.observed < 0.0);
    }

    #[test]
    fn guards() {
        assert!(two_sample_test(&[], &[1.0], 100, 0, |_| 0.0).is_none());
        assert!(two_sample_test(&[1.0], &[1.0], 0, 0, |_| 0.0).is_none());
    }
}
