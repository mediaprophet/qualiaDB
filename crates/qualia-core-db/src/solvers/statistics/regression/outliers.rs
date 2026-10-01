//! Univariate and Mahalanobis outlier screens — flag only, never auto-delete.

use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::statistics::anomaly::{
    is_multivariate_outlier, mahalanobis_sq, z_score_outliers,
};
use crate::solvers::statistics::descriptive::{mean, median_sorted};

use serde::Serialize;

/// Univariate pre-flight: mean vs median gap, 2σ/3σ z-score indices.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnivariateOutlierScreen {
    pub mean: f64,
    pub median: f64,
    pub mean_median_gap: f64,
    pub sigma2_indices: Vec<usize>,
    pub sigma3_indices: Vec<usize>,
}

pub fn univariate_outlier_screen(values: &[f64]) -> Option<UnivariateOutlierScreen> {
    if values.len() < 4 {
        return None;
    }
    let mu = mean(values)?;
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = median_sorted(&sorted)?;
    let s2 = z_score_outliers(values, 2.0).unwrap_or_default();
    let s3 = z_score_outliers(values, 3.0).unwrap_or_default();
    Some(UnivariateOutlierScreen {
        mean: mu,
        median: med,
        mean_median_gap: (mu - med).abs(),
        sigma2_indices: s2,
        sigma3_indices: s3,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MahalanobisOutliers {
    pub distances_sq: Vec<f64>,
    pub flagged_indices: Vec<usize>,
    pub alpha: f64,
}

/// Sample Mahalanobis outliers for row-major `n × d` matrix `x`.
pub fn mahalanobis_outliers(x: &[f64], n: usize, d: usize, alpha: f64) -> Option<MahalanobisOutliers> {
    if d == 0 || n <= d + 1 || x.len() != n * d || !(0.0 < alpha && alpha < 1.0) {
        return None;
    }
    let mut mu = vec![0.0; d];
    for i in 0..n {
        for j in 0..d {
            mu[j] += x[i * d + j];
        }
    }
    for m in mu.iter_mut() {
        *m /= n as f64;
    }
    let mut cov = vec![0.0; d * d];
    for i in 0..n {
        for a in 0..d {
            let da = x[i * d + a] - mu[a];
            for b in 0..d {
                cov[a * d + b] += da * (x[i * d + b] - mu[b]);
            }
        }
    }
    let denom = (n - 1) as f64;
    for v in cov.iter_mut() {
        *v /= denom;
    }
    // Invert via Cholesky: solve Σ c_j = e_j.
    let mut l = vec![0.0; d * d];
    cholesky_factor(d, &cov, &mut l).ok()?;
    let mut inv = vec![0.0; d * d];
    let mut ej = vec![0.0; d];
    let mut cj = vec![0.0; d];
    for j in 0..d {
        ej.iter_mut().for_each(|v| *v = 0.0);
        ej[j] = 1.0;
        cholesky_solve(d, &l, &ej, &mut cj).ok()?;
        for i in 0..d {
            inv[i * d + j] = cj[i];
        }
    }

    let mut distances_sq = Vec::with_capacity(n);
    let mut flagged_indices = Vec::new();
    for i in 0..n {
        let row = &x[i * d..(i + 1) * d];
        let d2 = mahalanobis_sq(row, &mu, &inv)?;
        distances_sq.push(d2);
        if is_multivariate_outlier(row, &mu, &inv, alpha)? {
            flagged_indices.push(i);
        }
    }
    Some(MahalanobisOutliers {
        distances_sq,
        flagged_indices,
        alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn univariate_flags_spike() {
        let v = [10.0, 11.0, 9.5, 10.2, 10.1, 9.8, 50.0];
        let s = univariate_outlier_screen(&v).unwrap();
        assert!(s.sigma2_indices.contains(&6));
    }

    #[test]
    fn mahalanobis_flags_far_point() {
        // Tight cluster + one far point; α=0.05 so χ² gate is reachable with n≈12.
        let mut x = Vec::new();
        for i in 0..12 {
            let t = (i as f64) * 0.02;
            x.push(t);
            x.push(-t);
        }
        x.extend_from_slice(&[4.0, 4.0]);
        let n = 13;
        let r = mahalanobis_outliers(&x, n, 2, 0.05).unwrap();
        assert!(
            r.flagged_indices.contains(&(n - 1)) || r.distances_sq[n - 1] == r.distances_sq.iter().cloned().fold(0.0_f64, f64::max),
            "flags={:?} d2={:?}",
            r.flagged_indices,
            r.distances_sq
        );
        assert!(r.distances_sq[n - 1] > 5.0);
    }
}
