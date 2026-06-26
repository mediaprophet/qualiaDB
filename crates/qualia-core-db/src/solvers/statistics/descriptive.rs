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

/// Index of the maximum value (the first on a tie) — the **argmax** selection used for greedy
/// token decoding (choose the highest-scoring logit). `None` for an empty slice. Non-finite
/// values compare by the usual `>` (a `NaN` never wins).
pub fn argmax(values: &[f64]) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    let mut best = 0;
    for i in 1..values.len() {
        if values[i] > values[best] {
            best = i;
        }
    }
    Some(best)
}

/// Covariance of two equal-length series. `sample == true` divides by `n-1`
/// (Bessel), else by `n`. `None` if the lengths differ or are empty.
pub fn covariance(x: &[f64], y: &[f64], sample: bool) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n == 0 {
        return None;
    }
    let mx = mean(x)?;
    let my = mean(y)?;
    let mut acc = 0.0;
    let mut i = 0;
    while i < n {
        acc += (x[i] - mx) * (y[i] - my);
        i += 1;
    }
    let denom = if sample { (n - 1) as f64 } else { n as f64 };
    Some(acc / denom)
}

/// The `k`-th central moment about the mean, `Σ(xᵢ−m)^k / n`. `None` if empty.
#[inline]
fn central_moment(values: &[f64], k: i32) -> Option<f64> {
    let m = mean(values)?;
    let mut acc = 0.0;
    for &v in values {
        acc += (v - m).powi(k);
    }
    Some(acc / values.len() as f64)
}

/// Sample skewness (Fisher–Pearson, `g1 = m₃ / m₂^{3/2}`), the standardised third
/// moment. `None` if empty; `Some(0.0)` for a constant series (zero spread).
pub fn skewness(values: &[f64]) -> Option<f64> {
    let m2 = central_moment(values, 2)?;
    let m3 = central_moment(values, 3)?;
    if m2 <= 0.0 {
        return Some(0.0);
    }
    Some(m3 / m2.powf(1.5))
}

/// Excess kurtosis (`g2 = m₄ / m₂² − 3`); 0 for a normal distribution. `None` if
/// empty; `Some(0.0)` for a constant series.
pub fn kurtosis(values: &[f64]) -> Option<f64> {
    let m2 = central_moment(values, 2)?;
    let m4 = central_moment(values, 4)?;
    if m2 <= 0.0 {
        return Some(0.0);
    }
    Some(m4 / (m2 * m2) - 3.0)
}

/// Linear-interpolated quantile of an **already-sorted-ascending** slice (the
/// numpy "linear" / R type-7 convention). `q` is clamped to `[0,1]`. `None` if empty.
pub fn quantile_sorted(sorted: &[f64], q: f64) -> Option<f64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(sorted[0]);
    }
    let q = q.clamp(0.0, 1.0);
    let pos = q * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    Some(sorted[lo] + (sorted[hi] - sorted[lo]) * frac)
}

/// Quantile, sorting the caller's buffer in place (no allocation). `None` if empty.
pub fn quantile_in_place(values: &mut [f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    quantile_sorted(values, q)
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
        assert_eq!(argmax(&empty), None);
    }

    #[test]
    fn argmax_selects_highest_index() {
        assert_eq!(argmax(&[0.1, 0.7, 0.2]), Some(1));
        assert_eq!(argmax(&[3.0, 3.0, 1.0]), Some(0)); // first on a tie
        assert_eq!(argmax(&[-5.0, -2.0, -9.0]), Some(1));
        let _ = EPS;
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

    #[test]
    fn covariance_matches_definition() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [2.0, 4.0, 6.0, 8.0]; // y = 2x → cov(sample) = 2·var(x,sample)
        let cov = covariance(&x, &y, true).unwrap();
        let vx = variance(&x, true).unwrap();
        assert!((cov - 2.0 * vx).abs() < 1e-9);
        // cov(x,x) == var(x).
        assert!((covariance(&x, &x, true).unwrap() - vx).abs() < 1e-9);
        assert_eq!(covariance(&x, &[1.0], true), None);
    }

    #[test]
    fn skewness_sign_and_symmetry() {
        // Symmetric data → ~0 skew.
        assert!(skewness(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap().abs() < 1e-9);
        // Right-tailed data → positive skew.
        assert!(skewness(&[1.0, 1.0, 1.0, 2.0, 10.0]).unwrap() > 0.0);
        // Constant → 0 (no spread), not NaN.
        assert_eq!(skewness(&[7.0, 7.0, 7.0]), Some(0.0));
    }

    #[test]
    fn kurtosis_excess() {
        // A near-uniform set has negative excess kurtosis (platykurtic).
        assert!(kurtosis(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() < 0.0);
        assert_eq!(kurtosis(&[3.0, 3.0]), Some(0.0));
    }

    #[test]
    fn quantile_interpolates() {
        let sorted = [1.0, 2.0, 3.0, 4.0]; // n=4
        assert!((quantile_sorted(&sorted, 0.0).unwrap() - 1.0).abs() < EPS);
        assert!((quantile_sorted(&sorted, 1.0).unwrap() - 4.0).abs() < EPS);
        // Median (q=0.5) of even count interpolates the two centre values.
        assert!((quantile_sorted(&sorted, 0.5).unwrap() - 2.5).abs() < EPS);
        // q=0.25 → pos=0.75 → 1 + 0.75·(2-1) = 1.75.
        assert!((quantile_sorted(&sorted, 0.25).unwrap() - 1.75).abs() < EPS);
        let mut unsorted = [4.0, 1.0, 3.0, 2.0];
        assert!((quantile_in_place(&mut unsorted, 0.5).unwrap() - 2.5).abs() < EPS);
    }
}
