//! Fuzzy linguistic quantifiers (Ma, Li & Ma ch 5.4; Zadeh) — evaluate statements
//! like "*most* guardians concur" or "*few* objections" over graded evidence,
//! keeping governance legible in human terms without collapsing to brittle counts.
//!
//! **Scope (a §12-allowed deferral):** this provides the *machinery* — Zadeh's
//! relative quantifiers as monotone membership curves over the satisfied-proportion,
//! plus the sigma-count proportion. The **named set** of governance quantifiers and
//! their exact membership curves ("most", "almost all", and any sensitive ones) are
//! **Timothy's to coin/ratify** — this module deliberately ships only generic
//! constructors + the classic illustrative curves, not a governance vocabulary.
//! Kernel-class `Reduction`.

/// A relative fuzzy quantifier: a monotone non-decreasing map from a proportion in
/// `[0,1]` to a truth degree in `[0,1]`, represented as a linear ramp `0` below
/// `low`, `1` above `high`, linear between (`low ≤ high`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeQuantifier {
    pub low: f64,
    pub high: f64,
}

impl RelativeQuantifier {
    /// "At least a `low`..`high` fraction" ramp — the building block for relative
    /// quantifiers. `low == high` is a crisp threshold.
    pub fn ramp(low: f64, high: f64) -> Self {
        let l = low.clamp(0.0, 1.0);
        let h = high.clamp(l, 1.0);
        Self { low: l, high: h }
    }

    /// Zadeh's classic illustrative "most" (ramp 0.3 → 0.8). Provided as an *example*
    /// only — governance quantifiers are coined by Timothy, not assumed here.
    pub fn most_example() -> Self {
        Self::ramp(0.3, 0.8)
    }

    /// Truth degree for the given satisfied-proportion.
    pub fn apply(self, proportion: f64) -> f64 {
        let p = proportion.clamp(0.0, 1.0);
        // Check the upper bound first so the degenerate `low == high` (a crisp
        // threshold) maps `p == high` to full truth rather than 0.
        if p >= self.high {
            1.0
        } else if p <= self.low {
            0.0
        } else {
            (p - self.low) / (self.high - self.low)
        }
    }
}

/// Sigma-count proportion: the fuzzy "fraction satisfied" = `Σ degrees / n`. `None`
/// for an empty set.
pub fn fuzzy_proportion(degrees: &[f64]) -> Option<f64> {
    if degrees.is_empty() {
        return None;
    }
    let s: f64 = degrees.iter().map(|d| d.clamp(0.0, 1.0)).sum();
    Some(s / degrees.len() as f64)
}

/// Evaluate "Q elements satisfy P" — apply the quantifier to the sigma-count
/// proportion of the per-element satisfaction `degrees`. `None` for an empty set.
pub fn evaluate(degrees: &[f64], quantifier: RelativeQuantifier) -> Option<f64> {
    Some(quantifier.apply(fuzzy_proportion(degrees)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_endpoints_and_interpolation() {
        let q = RelativeQuantifier::ramp(0.3, 0.8);
        assert_eq!(q.apply(0.2), 0.0); // below low
        assert_eq!(q.apply(0.9), 1.0); // above high
        assert!((q.apply(0.55) - 0.5).abs() < 1e-9); // midpoint
    }

    #[test]
    fn most_of_a_concurring_group_is_true() {
        // 5 elements, mostly high satisfaction → "most" is largely true.
        let degrees = [0.9, 0.8, 0.85, 0.7, 0.95];
        let v = evaluate(&degrees, RelativeQuantifier::most_example()).unwrap();
        assert!(v > 0.8, "most should be ~true: {v}");
        // A divided group → "most" is low.
        let split = [0.9, 0.1, 0.8, 0.2, 0.1];
        let v2 = evaluate(&split, RelativeQuantifier::most_example()).unwrap();
        assert!(v2 < 0.5, "divided group: {v2}");
    }

    #[test]
    fn proportion_is_sigma_count() {
        assert!((fuzzy_proportion(&[1.0, 0.0, 0.5, 0.5]).unwrap() - 0.5).abs() < 1e-9);
        assert!(fuzzy_proportion(&[]).is_none());
    }

    #[test]
    fn crisp_threshold_quantifier() {
        // "all" ≈ ramp(1,1): only proportion 1.0 yields full truth.
        let all = RelativeQuantifier::ramp(1.0, 1.0);
        assert_eq!(all.apply(0.99), 0.0);
        assert_eq!(all.apply(1.0), 1.0);
    }
}
