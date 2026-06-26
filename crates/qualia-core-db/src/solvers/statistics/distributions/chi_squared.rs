//! χ² (chi-squared) distribution — pdf / cdf / quantile + upper-tail p-value, used
//! by the goodness-of-fit and independence tests. CDF is exact via the regularized
//! lower incomplete gamma `P(k/2, x/2)` ([`super::special::gammp`]).

use super::special::{gammp, gammq, ln_gamma};

/// pdf with `k > 0` degrees of freedom, `x ≥ 0`.
pub fn pdf(x: f64, k: f64) -> f64 {
    debug_assert!(k > 0.0);
    if x < 0.0 {
        return 0.0;
    }
    if x == 0.0 {
        // Finite only for k = 2 (= 1/2); 0 for k > 2; +∞ for k < 2.
        return if k < 2.0 {
            f64::INFINITY
        } else if (k - 2.0).abs() < 1e-12 {
            0.5
        } else {
            0.0
        };
    }
    let kh = k / 2.0;
    ((kh - 1.0) * x.ln() - x / 2.0 - kh * std::f64::consts::LN_2 - ln_gamma(kh)).exp()
}

/// cdf `P(X ≤ x)` = `P(k/2, x/2)`.
pub fn cdf(x: f64, k: f64) -> f64 {
    debug_assert!(k > 0.0);
    if x <= 0.0 {
        return 0.0;
    }
    gammp(k / 2.0, x / 2.0)
}

/// Upper-tail p-value `P(X ≥ x)` — the usual χ² test p-value.
pub fn upper_p(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    gammq(k / 2.0, x / 2.0)
}

/// Inverse cdf (quantile) for `0 < p < 1`.
pub fn quantile(p: f64, k: f64) -> f64 {
    super::invert_cdf(p, Some(0.0), |x| cdf(x, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdf_is_exponential_for_k_two() {
        // χ²(2) = Exp with mean 2 → CDF(x) = 1 - e^{-x/2}.
        for &x in &[0.5, 2.0, 5.0] {
            assert!((cdf(x, 2.0) - (1.0 - (-x / 2.0).exp())).abs() < 1e-10, "x={x}");
        }
    }

    #[test]
    fn known_critical_values() {
        // 95th percentile: df=1 → 3.841; df=10 → 18.307.
        assert!((quantile(0.95, 1.0) - 3.841_458_82).abs() < 1e-4);
        assert!((quantile(0.95, 10.0) - 18.307_038_05).abs() < 1e-3);
        // Upper-tail p of 3.841 at df=1 ≈ 0.05.
        assert!((upper_p(3.841_458_82, 1.0) - 0.05).abs() < 1e-6);
    }

    #[test]
    fn quantile_inverts_cdf() {
        for &p in &[0.05, 0.5, 0.9, 0.99] {
            let x = quantile(p, 7.0);
            assert!((cdf(x, 7.0) - p).abs() < 1e-8, "p={p}");
        }
    }
}
