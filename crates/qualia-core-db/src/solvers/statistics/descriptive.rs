//! Descriptive statistics — zero-allocation kernels over caller-owned slices.
//!
//! These are the *single source of truth* for descriptive statistics in the
//! engine. Domain/specialized libraries MUST call these rather than
//! re-implementing `mean`/`variance`/etc. inline (Modality-First Composition;
//! see `MODALITY_FIRST_CONSOLIDATION.md`).
//!
//! Every function operates on a slice the caller already owns — no `Vec`, no
//! allocation, no copy. `median_in_place` sorts the caller's buffer with the
//! non-allocating `sort_unstable_by`; the caller decides whether to clone first.
//! `None` is returned for an empty slice so callers can map it onto their own
//! error type without this layer inventing one.

/// Sum of all elements. Zero for an empty slice.
#[inline]
pub fn sum(values: &[f64]) -> f64 {
    let mut acc = 0.0;
    let mut i = 0;
    while i < values.len() {
        acc += values[i];
        i += 1;
    }
    acc
}

/// Arithmetic mean. `None` if empty.
#[inline]
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(sum(values) / values.len() as f64)
}

/// Variance. `sample == true` uses Bessel's correction (divide by n-1);
/// otherwise the population variance (divide by n). `None` if empty.
///
/// Note: sample variance of a single element divides by zero and yields `NaN`,
/// preserving the historical behaviour of the call sites this replaced.
#[inline]
pub fn variance(values: &[f64], sample: bool) -> Option<f64> {
    let m = mean(values)?;
    let mut ss = 0.0;
    let mut i = 0;
    while i < values.len() {
        let d = values[i] - m;
        ss += d * d;
        i += 1;
    }
    let denom = if sample {
        (values.len() - 1) as f64
    } else {
        values.len() as f64
    };
    Some(ss / denom)
}

/// Standard deviation = sqrt(variance). `None` if empty.
#[inline]
pub fn std_dev(values: &[f64], sample: bool) -> Option<f64> {
    variance(values, sample).map(|v| v.sqrt())
}

/// Median of a slice that is **already sorted ascending**. `None` if empty.
/// For an even count, returns the mean of the two central elements.
#[inline]
pub fn median_sorted(sorted: &[f64]) -> Option<f64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    if n % 2 == 0 {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    } else {
        Some(sorted[n / 2])
    }
}

/// Median, sorting the caller's buffer in place (no allocation). `None` if empty.
#[inline]
pub fn median_in_place(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    median_sorted(values)
}

/// Minimum element by total-order comparison. `None` if empty.
#[inline]
pub fn min(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .reduce(|a, b| if b < a { b } else { a })
}

/// Maximum element by total-order comparison. `None` if empty.
#[inline]
pub fn max(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .reduce(|a, b| if b > a { b } else { a })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn empty_returns_none() {
        let empty: [f64; 0] = [];
        assert_eq!(mean(&empty), None);
        assert_eq!(variance(&empty, true), None);
        assert_eq!(std_dev(&empty, false), None);
        assert_eq!(median_sorted(&empty), None);
        assert_eq!(min(&empty), None);
        assert_eq!(max(&empty), None);
    }

    #[test]
    fn mean_matches_inline_formula() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert!((mean(&v).unwrap() - 2.5).abs() < EPS);
        assert!((sum(&v) - 10.0).abs() < EPS);
    }

    #[test]
    fn variance_sample_and_population() {
        let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        // population variance = 4.0, sample variance = 32/7
        assert!((variance(&v, false).unwrap() - 4.0).abs() < EPS);
        assert!((variance(&v, true).unwrap() - (32.0 / 7.0)).abs() < EPS);
        assert!((std_dev(&v, false).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn sample_variance_of_one_is_nan_like_legacy() {
        let v = [42.0];
        assert!(variance(&v, true).unwrap().is_nan());
        assert!((variance(&v, false).unwrap() - 0.0).abs() < EPS);
    }

    #[test]
    fn median_odd_even_and_in_place() {
        assert!((median_sorted(&[1.0, 2.0, 3.0]).unwrap() - 2.0).abs() < EPS);
        assert!((median_sorted(&[1.0, 2.0, 3.0, 4.0]).unwrap() - 2.5).abs() < EPS);
        let mut unsorted = [3.0, 1.0, 4.0, 1.0, 5.0];
        assert!((median_in_place(&mut unsorted).unwrap() - 3.0).abs() < EPS);
    }

    #[test]
    fn min_max() {
        let v = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0];
        assert!((min(&v).unwrap() - 1.0).abs() < EPS);
        assert!((max(&v).unwrap() - 9.0).abs() < EPS);
    }
}
