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
pub mod normal;
pub mod special;
pub mod students_t;

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
