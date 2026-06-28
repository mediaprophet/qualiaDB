//! Gaussian Mixture Models via EM (PRML ch 9.2, ISL ch 12) — diagonal-covariance
//! mixture, the standard robust GMM. Means are seeded by k-means (reusing
//! [`super::kmeans`]); the EM loop alternates responsibilities (E) and weighted
//! moment updates (M) and is guaranteed to increase the log-likelihood each step.
//!
//! The diagonal-covariance assumption (per-feature variance, no cross terms) is
//! stated explicitly, not hidden — it is the common, numerically stable GMM and
//! avoids singular full covariances on small data. A variance floor prevents
//! component collapse. Kernel-class `Reduction` (the per-point responsibilities).

use crate::solvers::learning::LearningError;

/// A fitted diagonal-covariance Gaussian mixture.
#[derive(Debug, Clone)]
pub struct GmmModel {
    /// Mixing weights `π_c` (sum to 1).
    pub weights: Vec<f64>,
    /// Component means, `k × p` row-major.
    pub means: Vec<f64>,
    /// Per-component diagonal variances, `k × p` row-major.
    pub variances: Vec<f64>,
    /// Hard assignment (argmax responsibility) per input row.
    pub labels: Vec<usize>,
    pub log_likelihood: f64,
    pub k: usize,
    pub p: usize,
    pub n_iter: usize,
    pub converged: bool,
}

const VAR_FLOOR: f64 = 1e-6;
const LN_2PI: f64 = 1.837_877_066_409_345_6; // ln(2π)

/// Log of a diagonal Gaussian density at `x` for mean/variance rows of length `p`.
fn log_gauss_diag(x: &[f64], mean: &[f64], var: &[f64], p: usize) -> f64 {
    let mut s = 0.0;
    for j in 0..p {
        let v = var[j].max(VAR_FLOOR);
        let d = x[j] - mean[j];
        s += LN_2PI + v.ln() + d * d / v;
    }
    -0.5 * s
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let m = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if m == f64::NEG_INFINITY {
        return m;
    }
    m + values.iter().map(|&v| (v - m).exp()).sum::<f64>().ln()
}

/// Fit a `k`-component diagonal GMM by EM. Fails closed: `InvalidDimension`,
/// `InsufficientData` (`k == 0` or `k > n`), `NotConverged`.
pub fn fit(
    x: &[f64],
    n: usize,
    p: usize,
    k: usize,
    max_iter: usize,
    tol: f64,
    seed: u64,
) -> Result<GmmModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p {
        return Err(LearningError::InvalidDimension);
    }
    if k == 0 || k > n {
        return Err(LearningError::InsufficientData);
    }

    // Initialise means with k-means; variances with the global per-feature
    // variance; weights uniform.
    let km = super::kmeans::fit(x, n, p, k, 50, seed)?;
    let mut means = km.centroids;
    let mut weights = vec![1.0 / k as f64; k];
    let mut variances = vec![0.0; k * p];
    {
        // Global per-feature variance as the starting spread.
        let mut gmean = vec![0.0; p];
        for i in 0..n {
            for j in 0..p {
                gmean[j] += x[i * p + j];
            }
        }
        for j in 0..p {
            gmean[j] /= n as f64;
        }
        let mut gvar = vec![0.0; p];
        for i in 0..n {
            for j in 0..p {
                let d = x[i * p + j] - gmean[j];
                gvar[j] += d * d;
            }
        }
        for j in 0..p {
            gvar[j] = (gvar[j] / n as f64).max(VAR_FLOOR);
        }
        for c in 0..k {
            variances[c * p..(c + 1) * p].copy_from_slice(&gvar);
        }
    }

    let mut resp = vec![0.0; n * k]; // responsibilities γ_ic
    let mut log_comp = vec![0.0; k];
    let mut prev_ll = f64::NEG_INFINITY;
    let mut converged = false;
    let mut iters = 0;

    for it in 1..=max_iter.max(1) {
        iters = it;
        // ── E-step: responsibilities + log-likelihood ──
        let mut ll = 0.0;
        for i in 0..n {
            let xi = &x[i * p..(i + 1) * p];
            for c in 0..k {
                log_comp[c] = weights[c].max(1e-300).ln()
                    + log_gauss_diag(
                        xi,
                        &means[c * p..(c + 1) * p],
                        &variances[c * p..(c + 1) * p],
                        p,
                    );
            }
            let lse = log_sum_exp(&log_comp);
            ll += lse;
            for c in 0..k {
                resp[i * k + c] = (log_comp[c] - lse).exp();
            }
        }

        // ── M-step: weighted weights / means / variances ──
        for c in 0..k {
            let mut nc = 0.0;
            for i in 0..n {
                nc += resp[i * k + c];
            }
            let nc_safe = nc.max(1e-300);
            weights[c] = nc / n as f64;
            // Mean.
            for j in 0..p {
                let mut s = 0.0;
                for i in 0..n {
                    s += resp[i * k + c] * x[i * p + j];
                }
                means[c * p + j] = s / nc_safe;
            }
            // Diagonal variance.
            for j in 0..p {
                let mut s = 0.0;
                for i in 0..n {
                    let d = x[i * p + j] - means[c * p + j];
                    s += resp[i * k + c] * d * d;
                }
                variances[c * p + j] = (s / nc_safe).max(VAR_FLOOR);
            }
        }

        if (ll - prev_ll).abs() < tol && it > 1 {
            prev_ll = ll;
            converged = true;
            break;
        }
        prev_ll = ll;
    }

    if !converged {
        return Err(LearningError::NotConverged);
    }

    // Hard labels = argmax responsibility (recomputed at the final parameters).
    let mut labels = vec![0usize; n];
    for i in 0..n {
        let xi = &x[i * p..(i + 1) * p];
        for c in 0..k {
            log_comp[c] = weights[c].max(1e-300).ln()
                + log_gauss_diag(
                    xi,
                    &means[c * p..(c + 1) * p],
                    &variances[c * p..(c + 1) * p],
                    p,
                );
        }
        let mut best = 0;
        for c in 1..k {
            if log_comp[c] > log_comp[best] {
                best = c;
            }
        }
        labels[i] = best;
    }

    Ok(GmmModel {
        weights,
        means,
        variances,
        labels,
        log_likelihood: prev_ll,
        k,
        p,
        n_iter: iters,
        converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_blobs() -> (Vec<f64>, usize) {
        // Two separated Gaussian-ish blobs around (0,0) and (8,8).
        let mut x = Vec::new();
        for d in 0..8 {
            let t = (d as f64 - 3.5) * 0.2;
            x.push(0.0 + t);
            x.push(0.0 - t);
        }
        for d in 0..8 {
            let t = (d as f64 - 3.5) * 0.2;
            x.push(8.0 + t);
            x.push(8.0 + t);
        }
        (x, 16)
    }

    #[test]
    fn recovers_two_components() {
        let (x, n) = two_blobs();
        let m = fit(&x, n, 2, 2, 200, 1e-8, 1).unwrap();
        assert!(m.converged);
        // Weights ~0.5 each.
        assert!((m.weights[0] - 0.5).abs() < 0.1 && (m.weights[1] - 0.5).abs() < 0.1);
        // The first 8 points share a label; the last 8 share the other.
        let l0 = m.labels[0];
        let l1 = m.labels[8];
        assert_ne!(l0, l1);
        assert!((0..8).all(|i| m.labels[i] == l0));
        assert!((8..16).all(|i| m.labels[i] == l1));
        // One mean near (0,0), the other near (8,8).
        let near = |c: usize, tx: f64, ty: f64| {
            (m.means[c * 2] - tx).abs() < 0.5 && (m.means[c * 2 + 1] - ty).abs() < 0.5
        };
        assert!(
            (near(0, 0.0, 0.0) && near(1, 8.0, 8.0)) || (near(1, 0.0, 0.0) && near(0, 8.0, 8.0))
        );
    }

    #[test]
    fn log_likelihood_is_finite_and_weights_normalised() {
        let (x, n) = two_blobs();
        let m = fit(&x, n, 2, 2, 200, 1e-8, 3).unwrap();
        assert!(m.log_likelihood.is_finite());
        assert!((m.weights.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn guards() {
        assert_eq!(
            fit(&[1.0, 2.0], 1, 2, 2, 10, 1e-6, 0).unwrap_err(),
            LearningError::InsufficientData
        );
    }
}
