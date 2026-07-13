//! Probability distributions — the canonical, full-precision pdf / cdf / quantile
//! for the whole engine, built on the shared special functions ([`special`]).
//!
//! This is what makes p-values and confidence intervals **honest**: hypothesis
//! tests ([`super::hypothesis`]) get their tail probabilities from a real
//! Student-t / χ² / F CDF here, not a `|t| > 1.96 ⇒ p = 0.05` placeholder, and
//! domain libraries reuse [`normal`] instead of copying a local `normal_cdf`.
//!
//! Submodules (one distribution each, PROJECT RULE §11): [`special`] (erf / incomplete
//! gamma & beta), [`normal`], [`students_t`], [`chi_squared`], [`fisher_f`].
//!
//! Everything here is scalar `f64` special-function evaluation — pointwise, not
//! GPU-amenable (CLAUDE.md §13: the CPU path is the right one; the *data-aggregate*
//! kernels that feed these, e.g. mean/variance, are the `Reduction`-class work that
//! routes through `ComputePolicy`).

pub mod chi_squared;
pub mod fisher_f;
pub mod multivariate_normal;
pub mod normal;
pub mod special;
pub mod students_t;

// Lightweight additional distributions for computational economics (5.1-A).
// These are scalar, no allocation. Full families can grow into dedicated files.

/// Binomial PMF: P(K = k | n, p).
#[inline]
pub fn binomial_pmf(k: u32, n: u32, p: f64) -> f64 {
    if p < 0.0 || p > 1.0 || k > n {
        return f64::NAN;
    }
    if n == 0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    let ln_c = ln_binom(n, k);
    let ln_p = (k as f64) * p.ln() + ((n - k) as f64) * (1.0 - p).ln();
    (ln_c + ln_p).exp()
}

/// Binomial CDF via direct sum (small n only; for large use normal approx in caller).
pub fn binomial_cdf(k: u32, n: u32, p: f64) -> f64 {
    if p < 0.0 || p > 1.0 {
        return f64::NAN;
    }
    let kk = k.min(n);
    let mut s = 0.0;
    for i in 0..=kk {
        s += binomial_pmf(i, n, p);
        if !s.is_finite() {
            return f64::NAN;
        }
    }
    s
}

fn ln_binom(n: u32, k: u32) -> f64 {
    // ln(n! / (k!(n-k)! )) using sum of logs
    if k > n {
        return f64::NEG_INFINITY;
    }
    let mut s = 0.0;
    for i in 0..k {
        s += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
    }
    s
}

/// Poisson PMF.
#[inline]
pub fn poisson_pmf(k: u32, lambda: f64) -> f64 {
    if lambda < 0.0 {
        return f64::NAN;
    }
    if k == 0 {
        return (-lambda).exp();
    }
    let mut pmf = (-lambda).exp();
    for i in 1..=k {
        pmf *= lambda / (i as f64);
    }
    pmf
}

/// Poisson CDF.
pub fn poisson_cdf(k: u32, lambda: f64) -> f64 {
    if lambda < 0.0 {
        return f64::NAN;
    }
    let mut s = 0.0;
    for i in 0..=k {
        s += poisson_pmf(i, lambda);
    }
    s
}

/// Lognormal PDF (mu, sigma>0).
#[inline]
pub fn lognormal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if x <= 0.0 || sigma <= 0.0 || !x.is_finite() || !mu.is_finite() || !sigma.is_finite() {
        return f64::NAN;
    }
    let z = (x.ln() - mu) / sigma;
    (1.0 / (x * sigma * core::f64::consts::TAU.sqrt())) * (-0.5 * z * z).exp()
}

/// Lognormal CDF via normal cdf of ln(x).
pub fn lognormal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if x <= 0.0 || sigma <= 0.0 {
        return if x <= 0.0 { 0.0 } else { f64::NAN };
    }
    normal::cdf((x.ln() - mu) / sigma, 0.0, 1.0)
}

/// Exponential PDF (rate > 0).
#[inline]
pub fn exponential_pdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 || rate <= 0.0 || !x.is_finite() || !rate.is_finite() {
        return f64::NAN;
    }
    rate * (-rate * x).exp()
}

/// Exponential CDF.
#[inline]
pub fn exponential_cdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 || rate <= 0.0 {
        return if x < 0.0 { 0.0 } else { f64::NAN };
    }
    1.0 - (-rate * x).exp()
}

/// Uniform PDF on [a, b].
#[inline]
pub fn uniform_pdf(x: f64, a: f64, b: f64) -> f64 {
    if a >= b || !x.is_finite() || !a.is_finite() || !b.is_finite() {
        return f64::NAN;
    }
    if x >= a && x <= b {
        1.0 / (b - a)
    } else {
        0.0
    }
}

/// Uniform CDF.
#[inline]
pub fn uniform_cdf(x: f64, a: f64, b: f64) -> f64 {
    if a >= b {
        return f64::NAN;
    }
    if x < a {
        0.0
    } else if x > b {
        1.0
    } else {
        (x - a) / (b - a)
    }
}

/// Laplace (double exponential) PDF.
#[inline]
pub fn laplace_pdf(x: f64, mu: f64, b: f64) -> f64 {
    if b <= 0.0 || !x.is_finite() || !mu.is_finite() || !b.is_finite() {
        return f64::NAN;
    }
    (1.0 / (2.0 * b)) * (-((x - mu).abs() / b)).exp()
}

/// Laplace CDF.
pub fn laplace_cdf(x: f64, mu: f64, b: f64) -> f64 {
    if b <= 0.0 {
        return f64::NAN;
    }
    let z = (x - mu) / b;
    if z < 0.0 {
        0.5 * (z).exp()
    } else {
        1.0 - 0.5 * (-z).exp()
    }
}

/// Gamma PDF (shape k>0, scale theta>0).
pub fn gamma_pdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x <= 0.0 || shape <= 0.0 || scale <= 0.0 {
        return if x <= 0.0 { 0.0 } else { f64::NAN };
    }
    let log_pdf = (shape - 1.0) * x.ln() - x / scale - shape * scale.ln() - special::ln_gamma(shape);
    log_pdf.exp()
}

/// Basic Beta PDF (alpha, beta >0) on (0,1).
pub fn beta_pdf(x: f64, alpha: f64, beta: f64) -> f64 {
    if x <= 0.0 || x >= 1.0 || alpha <= 0.0 || beta <= 0.0 {
        return 0.0;
    }
    let log_b = special::ln_gamma(alpha) + special::ln_gamma(beta) - special::ln_gamma(alpha + beta);
    ((alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln() - log_b).exp()
}

/// Weibull PDF (shape k>0, scale lambda>0).
#[inline]
pub fn weibull_pdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x < 0.0 || shape <= 0.0 || scale <= 0.0 {
        return 0.0;
    }
    (shape / scale) * (x / scale).powf(shape - 1.0) * (-(x / scale).powf(shape)).exp()
}

/// Empirical CDF from sorted samples (for caller-sorted data).
pub fn empirical_cdf(sorted_samples: &[f64], x: f64) -> f64 {
    if sorted_samples.is_empty() {
        return f64::NAN;
    }
    let mut count = 0usize;
    for &s in sorted_samples {
        if s <= x {
            count += 1;
        }
    }
    count as f64 / sorted_samples.len() as f64
}

/// Invert a monotone-increasing CDF: find `x` with `cdf(x) ≈ p` by adaptive
/// bracketing + bisection. `lower` bounds the support below (`Some(0.0)` for χ²/F,
/// `None` for a doubly-infinite support like Student-t). Used by the distribution
/// quantiles that have no closed-form inverse. Robust and ~machine-precision.
pub(crate) fn invert_cdf(p: f64, lower: Option<f64>, cdf: impl Fn(f64) -> f64) -> f64 {
    if p <= 0.0 {
        return lower.unwrap_or(f64::NEG_INFINITY);
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    // Establish a bracket [lo, hi] with cdf(lo) ≤ p ≤ cdf(hi).
    let mut lo = lower.unwrap_or(-1.0);
    let mut hi = lower.map(|l| l + 1.0).unwrap_or(1.0);
    if lower.is_none() {
        let mut guard = 0;
        while cdf(lo) > p && guard < 80 {
            lo *= 2.0;
            guard += 1;
        }
    }
    let mut guard = 0;
    while cdf(hi) < p && guard < 80 {
        hi *= 2.0;
        guard += 1;
    }
    // Bisection.
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_cdf_recovers_a_linear_cdf() {
        // CDF(x) = x on [0,1] → quantile(p) = p.
        let q = invert_cdf(0.37, Some(0.0), |x| x.clamp(0.0, 1.0));
        assert!((q - 0.37).abs() < 1e-9);
    }
}
