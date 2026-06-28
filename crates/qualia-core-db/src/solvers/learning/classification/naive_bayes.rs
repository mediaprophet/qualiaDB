//! Gaussian Naive Bayes (ISL ch 4.4.4) — a generative classifier assuming the
//! features are conditionally independent Gaussians given the class. Fit per-class
//! priors and per-feature mean/variance (reusing `statistics::descriptive`);
//! classify by the argmax log-posterior. Kernel-class `Reduction`.

use crate::solvers::learning::LearningError;
use crate::solvers::statistics::descriptive::{mean, variance};

const VAR_FLOOR: f64 = 1e-9;
const LN_2PI: f64 = 1.837_877_066_409_345_6;

/// A fitted Gaussian naive-Bayes classifier.
#[derive(Debug, Clone)]
pub struct GaussianNb {
    /// Distinct class labels, in ascending order.
    pub classes: Vec<usize>,
    /// Log class priors, aligned with `classes`.
    log_priors: Vec<f64>,
    /// Per-class per-feature means / variances, `n_classes × p` row-major.
    means: Vec<f64>,
    variances: Vec<f64>,
    p: usize,
}

impl GaussianNb {
    /// Fit from a row-major `n × p` matrix and integer labels.
    pub fn fit(x: &[f64], y: &[usize], n: usize, p: usize) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        let mut classes: Vec<usize> = y.to_vec();
        classes.sort_unstable();
        classes.dedup();
        let c = classes.len();

        let mut log_priors = vec![0.0; c];
        let mut means = vec![0.0; c * p];
        let mut variances = vec![0.0; c * p];

        for (ci, &cls) in classes.iter().enumerate() {
            let rows: Vec<usize> = (0..n).filter(|&i| y[i] == cls).collect();
            if rows.is_empty() {
                return Err(LearningError::InsufficientData);
            }
            log_priors[ci] = (rows.len() as f64 / n as f64).ln();
            let mut colbuf = vec![0.0; rows.len()];
            for j in 0..p {
                for (t, &i) in rows.iter().enumerate() {
                    colbuf[t] = x[i * p + j];
                }
                means[ci * p + j] = mean(&colbuf).ok_or(LearningError::InsufficientData)?;
                // Population variance per class (NaN-safe via floor for n_c==1).
                let v = variance(&colbuf, false).unwrap_or(0.0);
                variances[ci * p + j] = v.max(VAR_FLOOR);
            }
        }

        Ok(Self {
            classes,
            log_priors,
            means,
            variances,
            p,
        })
    }

    /// Log-posterior (up to the shared evidence constant) of `q` for class index `ci`.
    fn log_score(&self, q: &[f64], ci: usize) -> f64 {
        let mut s = self.log_priors[ci];
        for j in 0..self.p {
            let v = self.variances[ci * self.p + j];
            let d = q[j] - self.means[ci * self.p + j];
            s += -0.5 * (LN_2PI + v.ln() + d * d / v);
        }
        s
    }

    /// Predict the most probable class label for one query row.
    pub fn predict_row(&self, q: &[f64]) -> usize {
        let mut best = 0;
        let mut best_s = f64::NEG_INFINITY;
        for ci in 0..self.classes.len() {
            let s = self.log_score(q, ci);
            if s > best_s {
                best_s = s;
                best = ci;
            }
        }
        self.classes[best]
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<usize> {
        (0..m)
            .map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_two_gaussian_classes() {
        // Class 0 around (0,0), class 1 around (5,5).
        let x = [0.0, 0.1, 0.2, -0.1, -0.1, 0.0, 5.0, 5.1, 4.9, 5.0, 5.2, 4.8];
        let y = [0, 0, 0, 1, 1, 1];
        let nb = GaussianNb::fit(&x, &y, 6, 2).unwrap();
        assert_eq!(nb.classes, vec![0, 1]);
        assert_eq!(nb.predict_row(&[0.05, 0.05]), 0);
        assert_eq!(nb.predict_row(&[5.05, 4.95]), 1);
    }

    #[test]
    fn respects_priors() {
        // Heavily imbalanced classes; a borderline point leans to the majority.
        let x = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0]; // 3 of class 0 (origin) ... reuse coords
        let y = [0, 0, 1];
        let nb = GaussianNb::fit(&x, &y, 3, 2).unwrap();
        assert_eq!(nb.classes.len(), 2);
        // Prior for class 0 (2/3) > class 1 (1/3).
        assert!(nb.log_priors[0] > nb.log_priors[1]);
    }

    #[test]
    fn guards() {
        assert_eq!(
            GaussianNb::fit(&[1.0, 2.0, 3.0], &[0, 1], 2, 2).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
