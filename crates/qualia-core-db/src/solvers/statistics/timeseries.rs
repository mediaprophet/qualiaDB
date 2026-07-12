//! Time-series kernels — autocorrelation, moving average, exponential smoothing.
//!
//! The canonical home for the elementary time-series transforms used by the
//! domain libraries (`specialized_libs::statistical_computing`). Like the rest
//! of `solvers::statistics`, these operate over caller-owned slices; the
//! series-producing transforms write into a caller-provided `out` slice and
//! return the number of elements written (mirroring `histogram_into`), so no
//! allocation is imposed by this layer.

use super::descriptive::mean;

/// Sample autocorrelation at `lag`, using the standard biased estimator
/// (normalised by the total sum of squares, mean-centred):
///
/// ```text
/// r_k = Σ_{t=k}^{n-1} (x_t − x̄)(x_{t−k} − x̄)  /  Σ_{t=0}^{n-1} (x_t − x̄)²
/// ```
///
/// `r_0` is always `1.0` for non-constant data. Returns `None` if the slice is
/// empty, `lag >= n`, or the series is constant (zero variance → undefined).
pub fn autocorrelation(values: &[f64], lag: usize) -> Option<f64> {
    let n = values.len();
    if n == 0 || lag >= n {
        return None;
    }
    let m = mean(values)?;
    let mut denom = 0.0;
    let mut i = 0;
    while i < n {
        let d = values[i] - m;
        denom += d * d;
        i += 1;
    }
    if denom == 0.0 {
        return None; // constant series — autocorrelation undefined
    }
    let mut num = 0.0;
    let mut t = lag;
    while t < n {
        num += (values[t] - m) * (values[t - lag] - m);
        t += 1;
    }
    Some(num / denom)
}

/// Simple moving average with the given `window`, written into `out`.
///
/// Produces `n − window + 1` values, where `out[i]` is the mean of
/// `values[i..i+window]`. Returns the number of values written, or `None` if
/// `window == 0`, `window > n`, or `out` is too small to hold the result.
/// Uses a running-sum sweep (O(n), not O(n·window)).
pub fn moving_average_into(values: &[f64], window: usize, out: &mut [f64]) -> Option<usize> {
    let n = values.len();
    if window == 0 || window > n {
        return None;
    }
    let count = n - window + 1;
    if out.len() < count {
        return None;
    }
    let mut acc = 0.0;
    let mut i = 0;
    while i < window {
        acc += values[i];
        i += 1;
    }
    let w = window as f64;
    out[0] = acc / w;
    let mut j = 1;
    while j < count {
        acc += values[j + window - 1] - values[j - 1];
        out[j] = acc / w;
        j += 1;
    }
    Some(count)
}

/// Single (Brown's) exponential smoothing with factor `alpha ∈ (0, 1]`,
/// written into `out` (same length as `values`).
///
/// `s_0 = x_0`; `s_t = alpha·x_t + (1 − alpha)·s_{t−1}`. Returns the number of
/// values written, or `None` if the series is empty, `out` is too small, or
/// `alpha` is not in `(0, 1]`.
pub fn exponential_smoothing_into(values: &[f64], alpha: f64, out: &mut [f64]) -> Option<usize> {
    let n = values.len();
    if n == 0 || out.len() < n || !(alpha > 0.0 && alpha <= 1.0) {
        return None;
    }
    out[0] = values[0];
    let mut t = 1;
    while t < n {
        out[t] = alpha * values[t] + (1.0 - alpha) * out[t - 1];
        t += 1;
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn autocorr_lag0_is_one() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((autocorrelation(&v, 0).unwrap() - 1.0).abs() < EPS);
    }

    #[test]
    fn autocorr_lag1_known_value() {
        // mean=3; deviations [-2,-1,0,1,2]; num = 2+0+0+2 = 4; denom = 10 → 0.4
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((autocorrelation(&v, 1).unwrap() - 0.4).abs() < EPS);
    }

    #[test]
    fn autocorr_constant_is_none() {
        let v = [7.0, 7.0, 7.0];
        assert_eq!(autocorrelation(&v, 1), None);
    }

    #[test]
    fn moving_average_window2() {
        let v = [1.0, 2.0, 3.0, 4.0];
        let mut out = [0.0; 3];
        assert_eq!(moving_average_into(&v, 2, &mut out), Some(3));
        assert!((out[0] - 1.5).abs() < EPS);
        assert!((out[1] - 2.5).abs() < EPS);
        assert!((out[2] - 3.5).abs() < EPS);
    }

    #[test]
    fn moving_average_rejects_bad_window() {
        let v = [1.0, 2.0];
        let mut out = [0.0; 2];
        assert_eq!(moving_average_into(&v, 0, &mut out), None);
        assert_eq!(moving_average_into(&v, 3, &mut out), None);
    }

    #[test]
    fn exponential_smoothing_known_value() {
        // alpha=0.5: s0=1, s1=0.5*2+0.5*1=1.5, s2=0.5*3+0.5*1.5=2.25
        let v = [1.0, 2.0, 3.0];
        let mut out = [0.0; 3];
        assert_eq!(exponential_smoothing_into(&v, 0.5, &mut out), Some(3));
        assert!((out[0] - 1.0).abs() < EPS);
        assert!((out[1] - 1.5).abs() < EPS);
        assert!((out[2] - 2.25).abs() < EPS);
    }

    #[test]
    fn exponential_smoothing_rejects_bad_alpha() {
        let v = [1.0, 2.0];
        let mut out = [0.0; 2];
        assert_eq!(exponential_smoothing_into(&v, 0.0, &mut out), None);
        assert_eq!(exponential_smoothing_into(&v, 1.5, &mut out), None);
    }
}
