//! t-tests — one-sample, paired, and two-sample (pooled + Welch). Real Student-t
//! p-values and t-based confidence intervals.

use super::super::descriptive::{mean, variance};
use super::super::distributions::students_t;

/// One-sample / paired t-test result (integer degrees of freedom). `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TTest {
    pub t_statistic: f64,
    /// Two-sided p-value from the exact Student-t CDF.
    pub p_value: f64,
    pub degrees_of_freedom: u32,
    /// (lower, upper) of the 95% confidence interval around the sample mean, using
    /// the t critical value for these degrees of freedom (not a fixed 1.96).
    pub confidence_interval: (f64, f64),
}

/// One-sample t-test of the sample mean against `mu`. `None` if `n < 2`.
pub fn one_sample_t(values: &[f64], mu: f64) -> Option<TTest> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let m = mean(values)?;
    let var = variance(values, true)?;
    let df = (n - 1) as f64;
    let std_error = (var / n as f64).sqrt();
    if std_error == 0.0 {
        // Degenerate (zero variance): t is ±∞ unless the mean equals mu exactly.
        let t = if m == mu {
            0.0
        } else {
            f64::INFINITY.copysign(m - mu)
        };
        return Some(TTest {
            t_statistic: t,
            p_value: if m == mu { 1.0 } else { 0.0 },
            degrees_of_freedom: (n - 1) as u32,
            confidence_interval: (m, m),
        });
    }
    let t = (m - mu) / std_error;
    let p = students_t::two_sided_p(t, df);
    let t_crit = students_t::quantile(0.975, df);
    let margin = t_crit * std_error;
    Some(TTest {
        t_statistic: t,
        p_value: p,
        degrees_of_freedom: (n - 1) as u32,
        confidence_interval: (m - margin, m + margin),
    })
}

/// Paired t-test: the one-sample t-test of the paired differences against 0.
/// `None` if the samples differ in length or `n < 2`.
pub fn paired_t(a: &[f64], b: &[f64]) -> Option<TTest> {
    if a.len() != b.len() || a.len() < 2 {
        return None;
    }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
    one_sample_t(&diffs, 0.0)
}

/// Two-sample t-test result (degrees of freedom may be fractional, for Welch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoSampleTTest {
    pub t_statistic: f64,
    pub p_value: f64,
    pub degrees_of_freedom: f64,
    /// Difference of sample means (`mean(a) − mean(b)`).
    pub mean_difference: f64,
    /// 95% CI for the difference of means.
    pub confidence_interval: (f64, f64),
}

/// Two-sample t-test of `mean(a) − mean(b) = 0`.
///
/// `equal_var = true` → the pooled-variance (Student) test; `false` → the
/// Welch test (unequal variances, the safer default). `None` if either sample has
/// `n < 2`.
pub fn two_sample_t(a: &[f64], b: &[f64], equal_var: bool) -> Option<TwoSampleTTest> {
    let (na, nb) = (a.len(), b.len());
    if na < 2 || nb < 2 {
        return None;
    }
    let (ma, mb) = (mean(a)?, mean(b)?);
    let (va, vb) = (variance(a, true)?, variance(b, true)?);
    let (na_f, nb_f) = (na as f64, nb as f64);
    let diff = ma - mb;

    let (se, df) = if equal_var {
        // Pooled variance.
        let sp2 = ((na_f - 1.0) * va + (nb_f - 1.0) * vb) / (na_f + nb_f - 2.0);
        let se = (sp2 * (1.0 / na_f + 1.0 / nb_f)).sqrt();
        (se, na_f + nb_f - 2.0)
    } else {
        // Welch–Satterthwaite.
        let se2 = va / na_f + vb / nb_f;
        let se = se2.sqrt();
        let df =
            se2 * se2 / ((va / na_f).powi(2) / (na_f - 1.0) + (vb / nb_f).powi(2) / (nb_f - 1.0));
        (se, df)
    };

    if se == 0.0 {
        return Some(TwoSampleTTest {
            t_statistic: if diff == 0.0 {
                0.0
            } else {
                f64::INFINITY.copysign(diff)
            },
            p_value: if diff == 0.0 { 1.0 } else { 0.0 },
            degrees_of_freedom: df,
            mean_difference: diff,
            confidence_interval: (diff, diff),
        });
    }
    let t = diff / se;
    let p = students_t::two_sided_p(t, df);
    let t_crit = students_t::quantile(0.975, df);
    let margin = t_crit * se;
    Some(TwoSampleTTest {
        t_statistic: t,
        p_value: p,
        degrees_of_freedom: df,
        mean_difference: diff,
        confidence_interval: (diff - margin, diff + margin),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sample_real_p_value_not_a_threshold() {
        // Classic worked example: data with mean ~5.4, test against 5.0.
        let v = [5.1, 4.9, 5.6, 5.2, 5.8, 5.3, 4.7, 5.5];
        let r = one_sample_t(&v, 5.0).unwrap();
        assert_eq!(r.degrees_of_freedom, 7);
        // p-value is a real number strictly inside (0,1), not 0.05/0.1.
        assert!(r.p_value > 0.0 && r.p_value < 1.0);
        assert_ne!(r.p_value, 0.05);
        assert_ne!(r.p_value, 0.1);
        // CI brackets the sample mean.
        let m = v.iter().sum::<f64>() / v.len() as f64;
        assert!(r.confidence_interval.0 < m && r.confidence_interval.1 > m);
    }

    #[test]
    fn one_sample_matches_known_statistic() {
        // [1,2,3,4,5] vs mu=3: mean=3 → t=0, p=1.
        let r = one_sample_t(&[1.0, 2.0, 3.0, 4.0, 5.0], 3.0).unwrap();
        assert!(r.t_statistic.abs() < 1e-12);
        assert!((r.p_value - 1.0).abs() < 1e-9);
        // vs mu=0: t = mean/(s/√n) = 3/(√2.5/√5) = 3/0.7071 = 4.2426; p≈0.0133.
        let r2 = one_sample_t(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.0).unwrap();
        assert!((r2.t_statistic - 4.242_640_687).abs() < 1e-6);
        assert!((r2.p_value - 0.013_31).abs() < 1e-4);
    }

    #[test]
    fn paired_is_one_sample_of_differences() {
        let before = [10.0, 12.0, 9.0, 11.0, 13.0];
        let after = [11.0, 14.0, 10.0, 12.0, 15.0];
        let r = paired_t(&before, &after).unwrap();
        // Differences are all -1 or -2 → mean negative, significant.
        assert!(r.t_statistic < 0.0);
        assert!(r.p_value < 0.05);
        assert_eq!(paired_t(&before, &after[..4]), None); // length mismatch
    }

    #[test]
    fn welch_vs_pooled_two_sample() {
        let a = [20.0, 22.0, 19.0, 24.0, 25.0, 21.0];
        let b = [28.0, 31.0, 26.0, 30.0, 29.0, 27.0];
        let welch = two_sample_t(&a, &b, false).unwrap();
        let pooled = two_sample_t(&a, &b, true).unwrap();
        // Group b is clearly higher → diff negative, both highly significant.
        assert!(welch.mean_difference < 0.0);
        assert!(welch.p_value < 0.01 && pooled.p_value < 0.01);
        // Welch df ≤ pooled df (= n1+n2-2 = 10).
        assert!(welch.degrees_of_freedom <= 10.0 + 1e-9);
        assert!((pooled.degrees_of_freedom - 10.0).abs() < 1e-9);
        // CI for the difference excludes 0 (significant).
        assert!(welch.confidence_interval.1 < 0.0);
    }

    #[test]
    fn identical_groups_are_not_significant() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let r = two_sample_t(&a, &a, false).unwrap();
        assert!(r.t_statistic.abs() < 1e-9);
        assert!((r.p_value - 1.0).abs() < 1e-9);
    }
}
