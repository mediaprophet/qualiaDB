//! Multivariate normal distribution (PRML ch 2.3) — log-density, sampling, and MLE
//! for a `p`-dimensional Gaussian. The covariance inverse / log-determinant and the
//! sampling transform reuse `linear_algebra::cholesky` (no new solver). This is the
//! foundation of the Bayesian spine (Bayesian linear regression, Gaussian processes,
//! mixture/EM with full covariance).

use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::statistics::descriptive::mean as scalar_mean;

const LN_2PI: f64 = 1.837_877_066_409_345_6;

/// Log-density `ln N(x | μ, Σ)` for length-`p` `x`/`mean` and row-major `p×p`
/// covariance `cov`. `None` if shapes mismatch or `Σ` is not positive-definite.
pub fn log_pdf(x: &[f64], mean: &[f64], cov: &[f64], p: usize) -> Option<f64> {
    if x.len() != p || mean.len() != p || cov.len() != p * p || p == 0 {
        return None;
    }
    let mut l = vec![0.0; p * p];
    cholesky_factor(p, cov, &mut l).ok()?;
    // log|Σ| = 2·Σ ln L_ii.
    let mut log_det = 0.0;
    for i in 0..p {
        let d = l[i * p + i];
        if d <= 0.0 {
            return None;
        }
        log_det += d.ln();
    }
    log_det *= 2.0;
    // Mahalanobis² = (x−μ)ᵀ Σ⁻¹ (x−μ) via the Cholesky factor.
    let diff: Vec<f64> = x.iter().zip(mean).map(|(a, b)| a - b).collect();
    let mut sol = vec![0.0; p];
    cholesky_solve(p, &l, &diff, &mut sol).ok()?;
    let maha: f64 = diff.iter().zip(sol.iter()).map(|(d, s)| d * s).sum();
    Some(-0.5 * (p as f64 * LN_2PI + log_det + maha))
}

/// Density `N(x | μ, Σ)`.
pub fn pdf(x: &[f64], mean: &[f64], cov: &[f64], p: usize) -> Option<f64> {
    log_pdf(x, mean, cov, p).map(f64::exp)
}

/// Deterministic LCG + Box–Muller standard-normal generator.
struct Rng(u64);
impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Draw one sample `μ + L·z`, `z ~ N(0, I)`, `L` the Cholesky factor of `Σ`.
/// Deterministic given `seed`. `None` on shape mismatch / non-PD covariance.
pub fn sample(mean: &[f64], cov: &[f64], p: usize, seed: u64) -> Option<Vec<f64>> {
    if mean.len() != p || cov.len() != p * p || p == 0 {
        return None;
    }
    let mut l = vec![0.0; p * p];
    cholesky_factor(p, cov, &mut l).ok()?;
    let mut rng = Rng(seed ^ 0x9E3779B97F4A7C15);
    let z: Vec<f64> = (0..p).map(|_| rng.gaussian()).collect();
    // y_i = μ_i + Σ_{j≤i} L_ij z_j  (L lower-triangular).
    let mut y = vec![0.0; p];
    for i in 0..p {
        let mut s = mean[i];
        for j in 0..=i {
            s += l[i * p + j] * z[j];
        }
        y[i] = s;
    }
    Some(y)
}

/// Maximum-likelihood mean and (population) covariance of a row-major `n × p`
/// sample. Returns `(mean, cov)`; `None` if `n < 2` or shapes mismatch.
pub fn mle(data: &[f64], n: usize, p: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    if n < 2 || p == 0 || data.len() != n * p {
        return None;
    }
    let mut mean = vec![0.0; p];
    let mut col = vec![0.0; n];
    for j in 0..p {
        for i in 0..n {
            col[i] = data[i * p + j];
        }
        mean[j] = scalar_mean(&col)?;
    }
    let mut cov = vec![0.0; p * p];
    for i in 0..n {
        for a in 0..p {
            let da = data[i * p + a] - mean[a];
            for b in 0..p {
                cov[a * p + b] += da * (data[i * p + b] - mean[b]);
            }
        }
    }
    for v in cov.iter_mut() {
        *v /= n as f64; // MLE (population) covariance
    }
    Some((mean, cov))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::distributions::normal;

    #[test]
    fn reduces_to_univariate_normal() {
        // p=1 MVN density equals the scalar normal pdf.
        for &x in &[0.0, 0.5, 1.5, -2.0] {
            let mv = pdf(&[x], &[0.0], &[1.0], 1).unwrap();
            assert!((mv - normal::standard_pdf(x)).abs() < 1e-12, "x={x}");
        }
    }

    #[test]
    fn independent_factorises() {
        // Diagonal Σ ⇒ joint density = product of marginals.
        let x = [1.0, -0.5];
        let mean = [0.0, 0.0];
        let cov = [2.0, 0.0, 0.0, 0.5];
        let joint = pdf(&x, &mean, &cov, 2).unwrap();
        let m0 = normal::pdf(x[0], 0.0, 2.0_f64.sqrt());
        let m1 = normal::pdf(x[1], 0.0, 0.5_f64.sqrt());
        assert!((joint - m0 * m1).abs() < 1e-12);
    }

    #[test]
    fn log_pdf_peaks_at_the_mean() {
        let mean = [1.0, 2.0];
        let cov = [1.0, 0.3, 0.3, 1.0];
        let at_mean = log_pdf(&mean, &mean, &cov, 2).unwrap();
        let away = log_pdf(&[3.0, -1.0], &mean, &cov, 2).unwrap();
        assert!(at_mean > away);
    }

    #[test]
    fn mle_recovers_planted_parameters() {
        // Sample from N(μ, Σ) and check the MLE is close.
        let mean = [3.0, -1.0];
        let cov = [1.0, 0.5, 0.5, 2.0];
        let n = 4000;
        let mut data = vec![0.0; n * 2];
        for i in 0..n {
            let s = sample(&mean, &cov, 2, i as u64 + 1).unwrap();
            data[i * 2] = s[0];
            data[i * 2 + 1] = s[1];
        }
        let (m, c) = mle(&data, n, 2).unwrap();
        assert!((m[0] - 3.0).abs() < 0.1 && (m[1] + 1.0).abs() < 0.1, "mean {m:?}");
        assert!((c[0] - 1.0).abs() < 0.15 && (c[3] - 2.0).abs() < 0.2, "var {c:?}");
        assert!((c[1] - 0.5).abs() < 0.15, "cov {}", c[1]);
    }

    #[test]
    fn singular_covariance_is_none() {
        // A rank-deficient covariance is rejected (not a fabricated density).
        assert!(log_pdf(&[1.0, 1.0], &[0.0, 0.0], &[1.0, 1.0, 1.0, 1.0], 2).is_none());
    }
}
