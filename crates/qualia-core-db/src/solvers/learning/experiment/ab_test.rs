//! A/B testing (Practical Statistics ch 3) — compare two variants' conversion rates
//! with a two-proportion z-test and a confidence interval on the lift. The honest
//! output is "B beat A by 2.1pp [0.4pp, 3.8pp], p = 0.01" — effect size *with*
//! uncertainty, not a bare "B wins". Reuses the Normal CDF/quantile.

use crate::solvers::statistics::distributions::normal;

/// Result of comparing two conversion rates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbResult {
    pub rate_a: f64,
    pub rate_b: f64,
    /// `rate_b − rate_a` (positive ⇒ B converts better).
    pub difference: f64,
    pub z_statistic: f64,
    /// Two-sided p-value of "the rates are equal".
    pub p_value: f64,
    /// Confidence interval for the difference `rate_b − rate_a`.
    pub ci: (f64, f64),
    pub confidence: f64,
}

/// Two-proportion z-test. `conv_*` are conversions (successes), `n_*` the totals.
/// `alpha` sets the CI/p-value (two-sided). `None` on invalid counts.
pub fn ab_test(conv_a: u64, n_a: u64, conv_b: u64, n_b: u64, alpha: f64) -> Option<AbResult> {
    if n_a == 0 || n_b == 0 || conv_a > n_a || conv_b > n_b || !(0.0..1.0).contains(&alpha) || alpha <= 0.0 {
        return None;
    }
    let (na, nb) = (n_a as f64, n_b as f64);
    let pa = conv_a as f64 / na;
    let pb = conv_b as f64 / nb;
    let diff = pb - pa;

    // Pooled proportion for the test statistic (under H0: pa == pb).
    let pooled = (conv_a + conv_b) as f64 / (na + nb);
    let se_pooled = (pooled * (1.0 - pooled) * (1.0 / na + 1.0 / nb)).sqrt();
    let z = if se_pooled > 0.0 { diff / se_pooled } else { 0.0 };
    let p_value = 2.0 * (1.0 - normal::standard_cdf(z.abs()));

    // Unpooled SE for the confidence interval on the difference.
    let se_diff = (pa * (1.0 - pa) / na + pb * (1.0 - pb) / nb).sqrt();
    let zc = normal::standard_quantile(1.0 - alpha / 2.0);
    let margin = zc * se_diff;

    Some(AbResult {
        rate_a: pa,
        rate_b: pb,
        difference: diff,
        z_statistic: z,
        p_value,
        ci: (diff - margin, diff + margin),
        confidence: 1.0 - alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_real_lift() {
        // B (12%) clearly beats A (10%) at large n → significant, CI excludes 0.
        let r = ab_test(1000, 10_000, 1200, 10_000, 0.05).unwrap();
        assert!((r.rate_a - 0.10).abs() < 1e-9 && (r.rate_b - 0.12).abs() < 1e-9);
        assert!(r.difference > 0.0);
        assert!(r.p_value < 0.01, "p={}", r.p_value);
        assert!(r.ci.0 > 0.0, "CI lower {} should exclude 0", r.ci.0);
    }

    #[test]
    fn no_real_difference_is_not_significant() {
        let r = ab_test(500, 5000, 505, 5000, 0.05).unwrap();
        assert!(r.p_value > 0.2, "p={}", r.p_value);
        assert!(r.ci.0 < 0.0 && r.ci.1 > 0.0, "CI should straddle 0");
    }

    #[test]
    fn small_sample_is_underpowered() {
        // Same rates as the significant case but tiny n → not significant.
        let r = ab_test(10, 100, 12, 100, 0.05).unwrap();
        assert!(r.p_value > 0.05);
    }

    #[test]
    fn guards() {
        assert_eq!(ab_test(10, 0, 5, 100, 0.05), None);
        assert_eq!(ab_test(150, 100, 5, 100, 0.05), None); // conv > n
    }
}
