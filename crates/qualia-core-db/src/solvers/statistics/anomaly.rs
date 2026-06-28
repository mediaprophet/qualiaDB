//! Anomaly / outlier detection — folded into the statistics foundation (it is just
//! statistics with a decision rule). Univariate detectors (z-score, robust modified
//! z-score, Tukey fences, Grubbs' test) plus a multivariate Mahalanobis gate. Each
//! reuses the existing descriptive / robust / distribution primitives and follows the
//! module's `Option` idiom (`None` on degenerate input — empty, zero spread).
//!
//! Mission note: outliers are flagged as **candidates for human attention**, never
//! auto-acted-on — a deviation is a signal, not a verdict.

use super::descriptive::{mean, quantile_in_place, std_dev};
use super::distributions::{chi_squared, students_t};
use super::robust::median_abs_deviation;

/// Indices whose standard score `|x − μ| / σ` exceeds `threshold` (e.g. 3.0). `None`
/// if there are fewer than 2 points or the spread is zero.
pub fn z_score_outliers(values: &[f64], threshold: f64) -> Option<Vec<usize>> {
    if values.len() < 2 {
        return None;
    }
    let mu = mean(values)?;
    let sd = std_dev(values, true)?;
    if sd <= 0.0 {
        return None;
    }
    Some(
        values
            .iter()
            .enumerate()
            .filter(|(_, &x)| ((x - mu) / sd).abs() > threshold)
            .map(|(i, _)| i)
            .collect(),
    )
}

/// Robust outliers via the **modified z-score** (Iglewicz–Hoaglin):
/// `0.6745 · (x − median) / MAD`. Resistant to the very outliers it detects — the mean
/// and SD are not. Flags indices exceeding `threshold` (3.5 is the standard choice).
pub fn modified_z_score_outliers(values: &[f64], threshold: f64) -> Option<Vec<usize>> {
    if values.len() < 2 {
        return None;
    }
    // Median.
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = super::descriptive::median_sorted(&sorted)?;
    // Scaled MAD (already ×1.4826 → consistent with σ for normal data).
    let mad = median_abs_deviation(values, true)?;
    if mad <= 0.0 {
        return None;
    }
    Some(
        values
            .iter()
            .enumerate()
            // 0.6745·(x−med)/MAD; with the *scaled* MAD the constant folds in, so we
            // compare (x−med)/MAD directly against the threshold.
            .filter(|(_, &x)| ((x - med) / mad).abs() > threshold)
            .map(|(i, _)| i)
            .collect(),
    )
}

/// Tukey fences `[Q1 − k·IQR, Q3 + k·IQR]` (`k = 1.5` mild, `3.0` extreme). `None` if
/// fewer than 4 points.
pub fn tukey_fences(values: &[f64], k: f64) -> Option<(f64, f64)> {
    if values.len() < 4 {
        return None;
    }
    let mut v = values.to_vec();
    let q1 = quantile_in_place(&mut v, 0.25)?;
    let q3 = quantile_in_place(&mut v, 0.75)?;
    let iqr = q3 - q1;
    Some((q1 - k * iqr, q3 + k * iqr))
}

/// Indices outside the Tukey fences.
pub fn iqr_outliers(values: &[f64], k: f64) -> Option<Vec<usize>> {
    let (lo, hi) = tukey_fences(values, k)?;
    Some(
        values
            .iter()
            .enumerate()
            .filter(|(_, &x)| x < lo || x > hi)
            .map(|(i, _)| i)
            .collect(),
    )
}

/// The result of Grubbs' test for a single outlier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrubbsResult {
    /// Index of the most extreme point.
    pub index: usize,
    /// Grubbs statistic `G = max|x − μ| / σ`.
    pub statistic: f64,
    /// Two-sided critical value at the requested `alpha`.
    pub critical: f64,
    /// `true` iff `G > critical` — the point is a statistically significant outlier.
    pub is_outlier: bool,
}

/// **Grubbs' test** for the single most extreme value (assumes approximate normality).
/// `alpha` is the significance level (e.g. 0.05). Reuses the Student-t quantile for the
/// critical value. `None` if fewer than 3 points or zero spread.
pub fn grubbs_test(values: &[f64], alpha: f64) -> Option<GrubbsResult> {
    let n = values.len();
    if n < 3 {
        return None;
    }
    let mu = mean(values)?;
    let sd = std_dev(values, true)?;
    if sd <= 0.0 {
        return None;
    }
    let (index, statistic) = values
        .iter()
        .enumerate()
        .map(|(i, &x)| (i, (x - mu).abs() / sd))
        .fold((0usize, f64::NEG_INFINITY), |best, cur| {
            if cur.1 > best.1 {
                cur
            } else {
                best
            }
        });

    // Two-sided critical value: G_crit = ((n-1)/√n)·√( t² / (n-2 + t²) ),
    // t = t-quantile(1 − alpha/(2n)) with n-2 d.f.
    let nf = n as f64;
    let t = students_t::quantile(1.0 - alpha / (2.0 * nf), nf - 2.0);
    let t2 = t * t;
    let critical = ((nf - 1.0) / nf.sqrt()) * (t2 / (nf - 2.0 + t2)).sqrt();

    Some(GrubbsResult {
        index,
        statistic,
        critical,
        is_outlier: statistic > critical,
    })
}

/// Squared **Mahalanobis distance** `(x − μ)ᵀ Σ⁻¹ (x − μ)`. The caller supplies the
/// inverse covariance `inv_cov` (row-major `d×d`, obtained from the linear-algebra
/// substrate) — this keeps the metric self-contained. `None` on a dimension mismatch.
pub fn mahalanobis_sq(x: &[f64], mean_vec: &[f64], inv_cov: &[f64]) -> Option<f64> {
    let d = x.len();
    if d == 0 || mean_vec.len() != d || inv_cov.len() != d * d {
        return None;
    }
    // diff = x − μ ; then diffᵀ · inv_cov · diff.
    let diff: Vec<f64> = x.iter().zip(mean_vec).map(|(&xi, &mi)| xi - mi).collect();
    let mut acc = 0.0;
    for i in 0..d {
        let mut row = 0.0;
        for j in 0..d {
            row += inv_cov[i * d + j] * diff[j];
        }
        acc += diff[i] * row;
    }
    Some(acc)
}

/// Multivariate outlier gate: a point is flagged when its squared Mahalanobis distance
/// exceeds the χ²(d) upper-`alpha` quantile (the standard `d`-dimensional rule). `None`
/// on a dimension mismatch.
pub fn is_multivariate_outlier(
    x: &[f64],
    mean_vec: &[f64],
    inv_cov: &[f64],
    alpha: f64,
) -> Option<bool> {
    let d2 = mahalanobis_sq(x, mean_vec, inv_cov)?;
    let threshold = chi_squared::quantile(1.0 - alpha, x.len() as f64);
    Some(d2 > threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_score_flags_the_obvious_spike() {
        let data = [10.0, 11.0, 9.0, 10.5, 9.5, 10.2, 100.0];
        let out = z_score_outliers(&data, 2.0).unwrap();
        assert_eq!(out, vec![6]); // the 100 is the spike
    }

    #[test]
    fn modified_z_is_robust_to_masking() {
        // Two large outliers can inflate SD enough to *mask* themselves under plain z;
        // the MAD-based rule still catches them.
        let data = [1.0, 2.0, 1.5, 1.8, 2.2, 50.0, 52.0];
        let out = modified_z_score_outliers(&data, 3.5).unwrap();
        assert!(
            out.contains(&5) && out.contains(&6),
            "both spikes flagged: {out:?}"
        );
    }

    #[test]
    fn tukey_fences_and_iqr_outliers() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 100.0];
        let (lo, hi) = tukey_fences(&data, 1.5).unwrap();
        assert!(hi < 100.0 && lo < 1.0);
        let out = iqr_outliers(&data, 1.5).unwrap();
        assert!(out.contains(&8));
    }

    #[test]
    fn grubbs_detects_and_gates() {
        let data = [2.0, 3.0, 2.5, 2.8, 3.1, 2.9, 12.0];
        let g = grubbs_test(&data, 0.05).unwrap();
        assert_eq!(g.index, 6);
        assert!(g.is_outlier, "G {} vs crit {}", g.statistic, g.critical);
        // A clean sample yields no outlier.
        let clean = [2.0, 3.0, 2.5, 2.8, 3.1, 2.9, 2.7];
        assert!(!grubbs_test(&clean, 0.05).unwrap().is_outlier);
    }

    #[test]
    fn mahalanobis_reduces_to_scaled_distance_for_identity() {
        // inv_cov = I → Mahalanobis² = Euclidean².
        let x = [3.0, 4.0];
        let mu = [0.0, 0.0];
        let inv_cov = [1.0, 0.0, 0.0, 1.0];
        assert!((mahalanobis_sq(&x, &mu, &inv_cov).unwrap() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn multivariate_gate_flags_a_far_point() {
        // Unit covariance; a point 5σ out in 2-D is well past the χ²(2) 0.99 quantile.
        let inv_cov = [1.0, 0.0, 0.0, 1.0];
        let mu = [0.0, 0.0];
        assert_eq!(
            is_multivariate_outlier(&[5.0, 5.0], &mu, &inv_cov, 0.01),
            Some(true)
        );
        assert_eq!(
            is_multivariate_outlier(&[0.1, -0.1], &mu, &inv_cov, 0.01),
            Some(false)
        );
    }

    #[test]
    fn fails_closed_on_degenerate() {
        assert!(z_score_outliers(&[5.0], 3.0).is_none());
        assert!(z_score_outliers(&[2.0, 2.0, 2.0], 3.0).is_none()); // zero spread
        assert!(mahalanobis_sq(&[1.0], &[0.0, 0.0], &[1.0]).is_none()); // dim mismatch
    }
}
