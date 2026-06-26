//! Student's t-distribution — pdf / cdf / quantile + the two-sided p-value the
//! t-tests use. The CDF is exact via the regularized incomplete beta
//! ([`super::special::betai`]); the quantile inverts it numerically.

use super::special::{betai, ln_gamma};

/// pdf of the t-distribution with `nu > 0` degrees of freedom.
pub fn pdf(t: f64, nu: f64) -> f64 {
    debug_assert!(nu > 0.0);
    let c = (ln_gamma((nu + 1.0) / 2.0) - ln_gamma(nu / 2.0)).exp()
        / (nu * std::f64::consts::PI).sqrt();
    c * (1.0 + t * t / nu).powf(-(nu + 1.0) / 2.0)
}

/// cdf `P(T ≤ t)` with `nu` degrees of freedom. Uses
/// `I_x(ν/2, ½)` with `x = ν/(ν+t²)` and the sign of `t`.
pub fn cdf(t: f64, nu: f64) -> f64 {
    debug_assert!(nu > 0.0);
    let x = nu / (nu + t * t);
    let ib = 0.5 * betai(nu / 2.0, 0.5, x);
    if t >= 0.0 {
        1.0 - ib
    } else {
        ib
    }
}

/// Two-sided p-value for a t statistic: `2·(1 − P(T ≤ |t|)) = I_x(ν/2, ½)`.
pub fn two_sided_p(t: f64, nu: f64) -> f64 {
    let x = nu / (nu + t * t);
    betai(nu / 2.0, 0.5, x)
}

/// One-sided upper-tail p-value `P(T ≥ t)`.
pub fn upper_p(t: f64, nu: f64) -> f64 {
    1.0 - cdf(t, nu)
}

/// Inverse cdf (quantile) for `0 < p < 1`.
pub fn quantile(p: f64, nu: f64) -> f64 {
    super::invert_cdf(p, None, |t| cdf(t, nu))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdf_symmetry_and_center() {
        assert!((cdf(0.0, 5.0) - 0.5).abs() < 1e-12);
        // Symmetry: F(-t) = 1 - F(t).
        for &(t, nu) in &[(1.3, 7.0), (2.5, 12.0), (0.8, 3.0)] {
            assert!((cdf(-t, nu) - (1.0 - cdf(t, nu))).abs() < 1e-10);
        }
    }

    #[test]
    fn known_critical_values() {
        // Two-sided 95% critical t: df=10 → 2.228; df=∞ → 1.96.
        assert!((quantile(0.975, 10.0) - 2.228_138_851).abs() < 1e-4);
        assert!((quantile(0.975, 1.0) - 12.706_204_736).abs() < 1e-3); // Cauchy
        assert!((quantile(0.975, 1_000_000.0) - 1.959_963_98).abs() < 1e-3);
    }

    #[test]
    fn two_sided_p_matches_tail() {
        // p-value of t=2.228 at df=10 ≈ 0.05.
        assert!((two_sided_p(2.228_138_851, 10.0) - 0.05).abs() < 1e-4);
        assert!((two_sided_p(0.0, 5.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn quantile_inverts_cdf() {
        for &p in &[0.01, 0.05, 0.5, 0.9, 0.99] {
            let t = quantile(p, 8.0);
            assert!((cdf(t, 8.0) - p).abs() < 1e-8, "p={p}");
        }
    }
}
