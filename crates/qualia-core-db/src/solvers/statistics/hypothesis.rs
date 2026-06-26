//! Hypothesis tests — zero-allocation over caller-owned slices.
//!
//! Canonical home for t-tests. Reuses the descriptive kernels for mean/variance
//! rather than recomputing them. Specialized libraries map this result onto
//! their own domain result types.

use super::descriptive::{mean, variance};

/// One-sample t-test result. Fixed-size, `Copy`, no allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TTest {
    pub t_statistic: f64,
    pub p_value: f64,
    pub degrees_of_freedom: u32,
    /// (lower, upper) of the 95% confidence interval around the sample mean.
    pub confidence_interval: (f64, f64),
}

/// One-sample t-test of the sample mean against `mu`. `None` if n < 2.
///
/// Uses the sample (Bessel-corrected) variance. The p-value is a coarse
/// 1.96 threshold (the historical behaviour at the call sites this replaced) —
/// a precise t-distribution CDF is a future refinement, flagged not hidden.
pub fn one_sample_t(values: &[f64], mu: f64) -> Option<TTest> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let m = mean(values)?;
    let var = variance(values, true)?;
    let std_error = (var / n as f64).sqrt();
    let t_statistic = (m - mu) / std_error;
    let p_value = if t_statistic.abs() > 1.96 { 0.05 } else { 0.1 };
    let margin = 1.96 * std_error;
    Some(TTest {
        t_statistic,
        p_value,
        degrees_of_freedom: (n - 1) as u32,
        confidence_interval: (m - margin, m + margin),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn guards_small_sample() {
        assert_eq!(one_sample_t(&[1.0], 0.0), None);
    }

    #[test]
    fn zero_mean_data_against_zero_gives_zero_t() {
        let v = [-1.0, 1.0, -1.0, 1.0];
        let r = one_sample_t(&v, 0.0).unwrap();
        assert!(r.t_statistic.abs() < EPS);
        assert_eq!(r.degrees_of_freedom, 3);
    }

    #[test]
    fn shifted_data_has_large_t_and_small_p() {
        let v = [10.0, 10.1, 9.9, 10.05, 9.95];
        let r = one_sample_t(&v, 0.0).unwrap();
        assert!(r.t_statistic > 1.96);
        assert!((r.p_value - 0.05).abs() < EPS);
        // CI brackets the sample mean (~10.0)
        assert!(r.confidence_interval.0 < 10.0 && r.confidence_interval.1 > 10.0);
    }
}
