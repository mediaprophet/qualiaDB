//! Robust / exploratory estimators — location and spread measures that resist
//! outliers, the EDA complement to the mean/variance in [`super::descriptive`].
//! They reuse the descriptive median/quantile kernels (no re-implementation).

use super::descriptive::{mean, median_in_place, quantile_in_place};

/// Trimmed mean: drop a `proportion` (in `[0, 0.5)`) of the data from each end and
/// average the rest. `proportion = 0` is the ordinary mean. `None` if empty or
/// `proportion` is out of range.
pub fn trimmed_mean(values: &[f64], proportion: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 || !(0.0..0.5).contains(&proportion) {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let cut = (n as f64 * proportion).floor() as usize;
    if 2 * cut >= n {
        return median_in_place(&mut v); // everything trimmed → fall back to median
    }
    mean(&v[cut..n - cut])
}

/// Winsorized mean: clamp the lowest/highest `proportion` of the data to the
/// boundary values (rather than dropping them), then average. `None` if empty or
/// out of range.
pub fn winsorized_mean(values: &[f64], proportion: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 || !(0.0..0.5).contains(&proportion) {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let cut = (n as f64 * proportion).floor() as usize;
    if 2 * cut >= n {
        return median_in_place(&mut v);
    }
    let lo = v[cut];
    let hi = v[n - 1 - cut];
    for x in v.iter_mut() {
        if *x < lo {
            *x = lo;
        } else if *x > hi {
            *x = hi;
        }
    }
    mean(&v)
}

/// Median absolute deviation `median(|xᵢ − median(x)|)`. With `scaled = true`,
/// multiplied by 1.4826 so it is a consistent estimator of the standard deviation
/// for normal data. `None` if empty.
pub fn median_abs_deviation(values: &[f64], scaled: bool) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    let med = median_in_place(&mut v)?;
    let mut dev: Vec<f64> = values.iter().map(|&x| (x - med).abs()).collect();
    let mad = median_in_place(&mut dev)?;
    Some(if scaled { 1.482_602_218_505_602 * mad } else { mad })
}

/// Interquartile range `Q3 − Q1` (the 0.75 and 0.25 quantiles). `None` if empty.
pub fn iqr(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    let q3 = quantile_in_place(&mut v, 0.75)?;
    let q1 = quantile_in_place(&mut v, 0.25)?;
    Some(q3 - q1)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn trimmed_mean_ignores_outliers() {
        // A wild outlier wrecks the mean but not the 20%-trimmed mean.
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1000.0];
        assert!(mean(&data).unwrap() > 100.0);
        let tm = trimmed_mean(&data, 0.2).unwrap();
        assert!((tm - 5.5).abs() < 1.0, "trimmed mean {tm}"); // ~ middle of 3..8
        // proportion 0 == ordinary mean.
        assert!((trimmed_mean(&[1.0, 2.0, 3.0], 0.0).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn winsorized_mean_pulls_in_tails() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1000.0];
        let wm = winsorized_mean(&data, 0.1).unwrap();
        // The 1000 is clamped to 9 → mean far below the raw mean.
        assert!(wm < 10.0, "winsorized mean {wm}");
    }

    #[test]
    fn mad_and_iqr_measure_robust_spread() {
        // 0..=8 symmetric: median 4, |x-4| = 4,3,2,1,0,1,2,3,4 → MAD median = 2.
        let data: Vec<f64> = (0..=8).map(|i| i as f64).collect();
        assert!((median_abs_deviation(&data, false).unwrap() - 2.0).abs() < EPS);
        // Scaled MAD ≈ 1.4826·2.
        assert!((median_abs_deviation(&data, true).unwrap() - 2.965204).abs() < 1e-5);
        // IQR: Q1=2, Q3=6 → 4.
        assert!((iqr(&data).unwrap() - 4.0).abs() < EPS);
    }

    #[test]
    fn robust_resists_a_single_contaminant() {
        let clean: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let mut dirty = clean.clone();
        dirty[0] = 1e6;
        // MAD barely moves; std-dev explodes.
        let m_clean = median_abs_deviation(&clean, false).unwrap();
        let m_dirty = median_abs_deviation(&dirty, false).unwrap();
        assert!((m_clean - m_dirty).abs() < 2.0);
    }

    #[test]
    fn guards() {
        assert_eq!(trimmed_mean(&[], 0.1), None);
        assert_eq!(trimmed_mean(&[1.0], 0.6), None);
        assert_eq!(iqr(&[]), None);
        assert_eq!(median_abs_deviation(&[], false), None);
    }
}
