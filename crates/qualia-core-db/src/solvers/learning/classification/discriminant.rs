//! Discriminant analysis (ISL ch 4.4) — LDA (shared covariance ⇒ linear boundary)
//! and QDA (per-class covariance ⇒ quadratic boundary). Both are Gaussian
//! generative classifiers; the covariance inverse / log-determinant come from
//! `linear_algebra::cholesky` (no new solver). Kernel-class `DenseLinear`.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_determinant, cholesky_factor, cholesky_solve};

/// Per-class means + priors + the index map, shared setup for LDA and QDA.
struct ClassStats {
    classes: Vec<usize>,
    priors: Vec<f64>,
    means: Vec<f64>, // n_classes × p
    counts: Vec<usize>,
}

fn class_stats(x: &[f64], y: &[usize], n: usize, p: usize) -> Result<ClassStats, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    let mut classes: Vec<usize> = y.to_vec();
    classes.sort_unstable();
    classes.dedup();
    let c = classes.len();
    let mut priors = vec![0.0; c];
    let mut means = vec![0.0; c * p];
    let mut counts = vec![0usize; c];
    let cls_idx = |cls: usize| classes.iter().position(|&v| v == cls).unwrap();
    for i in 0..n {
        let ci = cls_idx(y[i]);
        counts[ci] += 1;
        for j in 0..p {
            means[ci * p + j] += x[i * p + j];
        }
    }
    for ci in 0..c {
        if counts[ci] == 0 {
            return Err(LearningError::InsufficientData);
        }
        priors[ci] = counts[ci] as f64 / n as f64;
        for j in 0..p {
            means[ci * p + j] /= counts[ci] as f64;
        }
    }
    Ok(ClassStats { classes, priors, means, counts })
}

// ── LDA ───────────────────────────────────────────────────────────────────────

/// Linear Discriminant Analysis: one shared within-class covariance. The
/// discriminant is linear, `δ_c(x) = xᵀwc + bc`.
#[derive(Debug, Clone)]
pub struct LdaModel {
    pub classes: Vec<usize>,
    w: Vec<f64>, // n_classes × p : Σ⁻¹ μ_c
    b: Vec<f64>, // n_classes      : −½ μcᵀ Σ⁻¹ μc + ln π_c
    p: usize,
}

impl LdaModel {
    /// Fit LDA. Needs `n > n_classes` for a non-degenerate pooled covariance.
    pub fn fit(x: &[f64], y: &[usize], n: usize, p: usize) -> Result<Self, LearningError> {
        let cs = class_stats(x, y, n, p)?;
        let c = cs.classes.len();
        if n <= c {
            return Err(LearningError::InsufficientData);
        }
        // Pooled within-class scatter → covariance Σ = S/(n−C).
        let mut cov = vec![0.0; p * p];
        let cls_idx = |cls: usize| cs.classes.iter().position(|&v| v == cls).unwrap();
        for i in 0..n {
            let ci = cls_idx(y[i]);
            for a in 0..p {
                let da = x[i * p + a] - cs.means[ci * p + a];
                for bb in 0..p {
                    let db = x[i * p + bb] - cs.means[ci * p + bb];
                    cov[a * p + bb] += da * db;
                }
            }
        }
        let denom = (n - c) as f64;
        for v in cov.iter_mut() {
            *v /= denom;
        }
        let l = cholesky_of(&cov, p)?;

        let mut w = vec![0.0; c * p];
        let mut b = vec![0.0; c];
        let mut mu = vec![0.0; p];
        let mut wc = vec![0.0; p];
        for ci in 0..c {
            mu.copy_from_slice(&cs.means[ci * p..(ci + 1) * p]);
            cholesky_solve(p, &l, &mu, &mut wc)?; // wc = Σ⁻¹ μ_c
            w[ci * p..(ci + 1) * p].copy_from_slice(&wc);
            let quad: f64 = mu.iter().zip(wc.iter()).map(|(m, w)| m * w).sum();
            b[ci] = -0.5 * quad + cs.priors[ci].ln();
        }
        Ok(Self { classes: cs.classes, w, b, p })
    }

    pub fn predict_row(&self, q: &[f64]) -> usize {
        let mut best = 0;
        let mut best_s = f64::NEG_INFINITY;
        for ci in 0..self.classes.len() {
            let s: f64 = q.iter().zip(&self.w[ci * self.p..(ci + 1) * self.p]).map(|(x, w)| x * w).sum::<f64>()
                + self.b[ci];
            if s > best_s {
                best_s = s;
                best = ci;
            }
        }
        self.classes[best]
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<usize> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }
}

// ── QDA ───────────────────────────────────────────────────────────────────────

/// Quadratic Discriminant Analysis: a separate covariance per class. The
/// discriminant `δ_c(x) = −½ ln|Σc| − ½ (x−μc)ᵀ Σc⁻¹ (x−μc) + ln π_c`.
#[derive(Debug, Clone)]
pub struct QdaModel {
    pub classes: Vec<usize>,
    means: Vec<f64>,            // n_classes × p
    chol: Vec<f64>,             // n_classes × p × p : Cholesky factors of Σ_c
    log_det: Vec<f64>,          // n_classes
    log_prior: Vec<f64>,
    p: usize,
}

impl QdaModel {
    /// Fit QDA. Needs each class to have `> p` samples for a non-singular covariance.
    pub fn fit(x: &[f64], y: &[usize], n: usize, p: usize) -> Result<Self, LearningError> {
        let cs = class_stats(x, y, n, p)?;
        let c = cs.classes.len();
        let cls_idx = |cls: usize| cs.classes.iter().position(|&v| v == cls).unwrap();
        let mut chol = vec![0.0; c * p * p];
        let mut log_det = vec![0.0; c];
        let mut log_prior = vec![0.0; c];
        for ci in 0..c {
            if cs.counts[ci] <= p {
                return Err(LearningError::InsufficientData); // covariance would be singular
            }
            // Per-class covariance Σ_c = S_c/(n_c−1).
            let mut cov = vec![0.0; p * p];
            for i in 0..n {
                if cls_idx(y[i]) != ci {
                    continue;
                }
                for a in 0..p {
                    let da = x[i * p + a] - cs.means[ci * p + a];
                    for bb in 0..p {
                        let db = x[i * p + bb] - cs.means[ci * p + bb];
                        cov[a * p + bb] += da * db;
                    }
                }
            }
            let denom = (cs.counts[ci] - 1) as f64;
            for v in cov.iter_mut() {
                *v /= denom;
            }
            let l = cholesky_of(&cov, p)?;
            log_det[ci] = cholesky_determinant(p, &l).max(1e-300).ln();
            chol[ci * p * p..(ci + 1) * p * p].copy_from_slice(&l);
            log_prior[ci] = cs.priors[ci].ln();
        }
        Ok(Self { classes: cs.classes, means: cs.means, chol, log_det, log_prior, p })
    }

    pub fn predict_row(&self, q: &[f64]) -> usize {
        let p = self.p;
        let mut diff = vec![0.0; p];
        let mut sol = vec![0.0; p];
        let mut best = 0;
        let mut best_s = f64::NEG_INFINITY;
        for ci in 0..self.classes.len() {
            for j in 0..p {
                diff[j] = q[j] - self.means[ci * p + j];
            }
            // Mahalanobis² = diffᵀ Σ⁻¹ diff via the Cholesky factor.
            let l = &self.chol[ci * p * p..(ci + 1) * p * p];
            if cholesky_solve(p, l, &diff, &mut sol).is_err() {
                continue;
            }
            let maha: f64 = diff.iter().zip(sol.iter()).map(|(d, s)| d * s).sum();
            let s = -0.5 * self.log_det[ci] - 0.5 * maha + self.log_prior[ci];
            if s > best_s {
                best_s = s;
                best = ci;
            }
        }
        self.classes[best]
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<usize> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }
}

/// Cholesky factor of a `p×p` covariance, mapping a solver failure to `Singular`.
fn cholesky_of(cov: &[f64], p: usize) -> Result<Vec<f64>, LearningError> {
    let mut l = vec![0.0; p * p];
    cholesky_factor(p, cov, &mut l).map_err(|_| LearningError::Singular)?;
    Ok(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two 2-D classes, well separated, enough points for per-class covariance.
    fn data() -> (Vec<f64>, Vec<usize>, usize) {
        let mut x = Vec::new();
        let mut y = Vec::new();
        // class 0 around (0,0)
        for &(a, b) in &[(0.0, 0.0), (0.5, -0.3), (-0.4, 0.2), (0.2, 0.4), (-0.3, -0.2)] {
            x.push(a);
            x.push(b);
            y.push(0);
        }
        // class 1 around (5,5)
        for &(a, b) in &[(5.0, 5.0), (5.4, 4.7), (4.6, 5.3), (5.1, 4.9), (4.8, 5.2)] {
            x.push(a);
            x.push(b);
            y.push(1);
        }
        (x, y, 10)
    }

    #[test]
    fn lda_separates_classes() {
        let (x, y, n) = data();
        let m = LdaModel::fit(&x, &y, n, 2).unwrap();
        assert_eq!(m.classes, vec![0, 1]);
        assert_eq!(m.predict_row(&[0.1, 0.1]), 0);
        assert_eq!(m.predict_row(&[5.0, 5.0]), 1);
        // Training accuracy is perfect on this separable set.
        let preds = m.predict(&x, n);
        assert!(preds.iter().zip(&y).all(|(a, b)| a == b));
    }

    #[test]
    fn qda_separates_classes() {
        let (x, y, n) = data();
        let m = QdaModel::fit(&x, &y, n, 2).unwrap();
        assert_eq!(m.predict_row(&[0.0, 0.0]), 0);
        assert_eq!(m.predict_row(&[5.2, 4.8]), 1);
        let preds = m.predict(&x, n);
        assert!(preds.iter().zip(&y).all(|(a, b)| a == b));
    }

    #[test]
    fn qda_fails_closed_on_too_few_per_class() {
        // class 0 has 3 non-collinear points (fine); class 1 has only 2, p=2 →
        // n_c ≤ p → its covariance would be singular → fail closed.
        let x = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 5.0, 5.0, 6.0, 5.0];
        let y = [0, 0, 0, 1, 1];
        assert_eq!(QdaModel::fit(&x, &y, 5, 2).unwrap_err(), LearningError::InsufficientData);
    }
}
