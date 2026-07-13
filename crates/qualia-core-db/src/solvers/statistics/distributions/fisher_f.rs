//! Fisher–Snedecor F-distribution — pdf / cdf / quantile + upper-tail p-value, used
//! by ANOVA and variance-ratio tests. CDF is exact via the regularized incomplete
//! beta `I_x(d₁/2, d₂/2)` with `x = d₁f/(d₁f + d₂)`.

use super::special::{betai, ln_gamma};

/// pdf with numerator/denominator dof `d1, d2 > 0`, `x ≥ 0`.
pub fn pdf(x: f64, d1: f64, d2: f64) -> f64 {
    debug_assert!(d1 > 0.0 && d2 > 0.0);
    if x <= 0.0 {
        return 0.0;
    }
    // ln of (d1/d2)^{d1/2} x^{d1/2-1} (1+d1 x/d2)^{-(d1+d2)/2} / B(d1/2,d2/2)
    let ln_b = ln_gamma(d1 / 2.0) + ln_gamma(d2 / 2.0) - ln_gamma((d1 + d2) / 2.0);
    let ln_num = (d1 / 2.0) * (d1 / d2).ln() + (d1 / 2.0 - 1.0) * x.ln()
        - ((d1 + d2) / 2.0) * (1.0 + d1 * x / d2).ln();
    (ln_num - ln_b).exp()
}

/// cdf `P(X ≤ x)` = `I_x(d₁/2, d₂/2)`, `x = d₁f/(d₁f + d₂)`.
pub fn cdf(f: f64, d1: f64, d2: f64) -> f64 {
    debug_assert!(d1 > 0.0 && d2 > 0.0);
    if f <= 0.0 {
        return 0.0;
    }
    let x = d1 * f / (d1 * f + d2);
    betai(d1 / 2.0, d2 / 2.0, x)
}

/// Upper-tail p-value `P(X ≥ f)` — the ANOVA / variance-ratio p-value.
pub fn upper_p(f: f64, d1: f64, d2: f64) -> f64 {
    1.0 - cdf(f, d1, d2)
}

/// Inverse cdf (quantile) for `0 < p < 1`.
pub fn quantile(p: f64, d1: f64, d2: f64) -> f64 {
    super::invert_cdf(p, Some(0.0), |f| cdf(f, d1, d2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_critical_values() {
        // F_{0.95}(5, 10) = 3.3258; F_{0.95}(1, 10) = 4.9646.
        assert!((quantile(0.95, 5.0, 10.0) - 3.325_835).abs() < 1e-3);
        assert!((quantile(0.95, 1.0, 10.0) - 4.964_603).abs() < 1e-3);
        // Upper-tail p of that critical value ≈ 0.05.
        assert!((upper_p(3.325_835, 5.0, 10.0) - 0.05).abs() < 1e-4);
    }

    #[test]
    fn cdf_monotone_and_bounds() {
        assert_eq!(cdf(0.0, 3.0, 8.0), 0.0);
        assert!(cdf(1.0, 3.0, 8.0) < cdf(5.0, 3.0, 8.0));
        assert!(cdf(1e6, 3.0, 8.0) > 0.999);
    }

    #[test]
    fn quantile_inverts_cdf() {
        for &p in &[0.1, 0.5, 0.9, 0.99] {
            let f = quantile(p, 4.0, 20.0);
            assert!((cdf(f, 4.0, 20.0) - p).abs() < 1e-7, "p={p}");
        }
    }
}
