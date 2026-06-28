//! Multinomial logistic regression / softmax classifier (ISL ch 4.3.5, PRML ch 4).
//!
//! `P(y=c | x) = softmax(W_c·φ(x))`. Fit by gradient ascent on the regularized
//! log-likelihood (the multinomial cross-entropy is convex, so this converges to
//! the global optimum); a small L2 term keeps the solution finite under separation.
//! Kernel-class `DenseLinear` (the logits) — scalar fit loop is CPU.

use crate::solvers::learning::LearningError;

/// A fitted softmax classifier. `weights` is `n_classes × k` row-major
/// (`k = p + intercept`).
#[derive(Debug, Clone)]
pub struct MultinomialLogistic {
    pub classes: Vec<usize>,
    weights: Vec<f64>,
    fit_intercept: bool,
    p: usize,
}

fn design_row(x_row: &[f64], fit_intercept: bool, out: &mut [f64]) {
    if fit_intercept {
        out[0] = 1.0;
        out[1..].copy_from_slice(x_row);
    } else {
        out.copy_from_slice(x_row);
    }
}

/// Numerically-stable softmax of `logits` into `out`.
fn softmax(logits: &[f64], out: &mut [f64]) {
    let m = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut s = 0.0;
    for (o, &l) in out.iter_mut().zip(logits) {
        *o = (l - m).exp();
        s += *o;
    }
    if s > 0.0 {
        for o in out.iter_mut() {
            *o /= s;
        }
    }
}

impl MultinomialLogistic {
    /// Fit by gradient ascent. `lr` learning rate, `l2` ridge penalty (≥ 0),
    /// `max_iter` iterations. Fails closed on shape mismatch / a single class.
    pub fn fit(
        x: &[f64],
        y: &[usize],
        n: usize,
        p: usize,
        fit_intercept: bool,
        lr: f64,
        l2: f64,
        max_iter: usize,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        if !(lr > 0.0) || l2 < 0.0 {
            return Err(LearningError::InsufficientData);
        }
        let mut classes: Vec<usize> = y.to_vec();
        classes.sort_unstable();
        classes.dedup();
        let c = classes.len();
        if c < 2 {
            return Err(LearningError::InsufficientData);
        }
        let cls_idx = |label: usize| classes.iter().position(|&v| v == label).unwrap();
        let k = p + usize::from(fit_intercept);

        // Design matrix.
        let mut d = vec![0.0; n * k];
        for i in 0..n {
            design_row(
                &x[i * p..(i + 1) * p],
                fit_intercept,
                &mut d[i * k..(i + 1) * k],
            );
        }

        let mut w = vec![0.0; c * k];
        let mut logits = vec![0.0; c];
        let mut probs = vec![0.0; c];
        let mut grad = vec![0.0; c * k];

        for _ in 0..max_iter.max(1) {
            grad.iter_mut().for_each(|g| *g = 0.0);
            for i in 0..n {
                let di = &d[i * k..(i + 1) * k];
                for cc in 0..c {
                    logits[cc] = (0..k).map(|j| w[cc * k + j] * di[j]).sum();
                }
                softmax(&logits, &mut probs);
                let yi = cls_idx(y[i]);
                for cc in 0..c {
                    let err = (if cc == yi { 1.0 } else { 0.0 }) - probs[cc];
                    for j in 0..k {
                        grad[cc * k + j] += err * di[j];
                    }
                }
            }
            // Gradient step with L2 shrinkage (don't penalize the intercept column).
            for cc in 0..c {
                for j in 0..k {
                    let mut g = grad[cc * k + j] / n as f64;
                    if !(fit_intercept && j == 0) {
                        g -= l2 * w[cc * k + j];
                    }
                    w[cc * k + j] += lr * g;
                }
            }
        }

        Ok(Self {
            classes,
            weights: w,
            fit_intercept,
            p,
        })
    }

    /// Class probabilities for one row, aligned with `classes`.
    pub fn predict_proba_row(&self, x_row: &[f64]) -> Vec<f64> {
        let c = self.classes.len();
        let k = self.p + usize::from(self.fit_intercept);
        let mut di = vec![0.0; k];
        design_row(x_row, self.fit_intercept, &mut di);
        let logits: Vec<f64> = (0..c)
            .map(|cc| (0..k).map(|j| self.weights[cc * k + j] * di[j]).sum())
            .collect();
        let mut probs = vec![0.0; c];
        softmax(&logits, &mut probs);
        probs
    }

    /// Predicted class label (argmax probability).
    pub fn predict_row(&self, x_row: &[f64]) -> usize {
        let probs = self.predict_proba_row(x_row);
        let mut best = 0;
        for cc in 1..probs.len() {
            if probs[cc] > probs[best] {
                best = cc;
            }
        }
        self.classes[best]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_three_separated_clusters() {
        // class 0 near (0,0), 1 near (10,0), 2 near (0,10).
        let mut x = Vec::new();
        let mut y = Vec::new();
        for &(cx, cy, lbl) in &[(0.0, 0.0, 0usize), (10.0, 0.0, 1), (0.0, 10.0, 2)] {
            for d in 0..5 {
                x.push(cx + (d as f64 - 2.0) * 0.1);
                x.push(cy + (d as f64 - 2.0) * 0.1);
                y.push(lbl);
            }
        }
        let n = 15;
        let m = MultinomialLogistic::fit(&x, &y, n, 2, true, 0.5, 1e-4, 500).unwrap();
        assert_eq!(m.classes, vec![0, 1, 2]);
        assert_eq!(m.predict_row(&[0.0, 0.0]), 0);
        assert_eq!(m.predict_row(&[10.0, 0.0]), 1);
        assert_eq!(m.predict_row(&[0.0, 10.0]), 2);
        // Probabilities sum to 1.
        let s: f64 = m.predict_proba_row(&[0.0, 0.0]).iter().sum();
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn guards() {
        assert_eq!(
            MultinomialLogistic::fit(&[1.0, 2.0], &[0], 1, 2, true, 0.1, 0.0, 10).unwrap_err(),
            LearningError::InsufficientData
        );
    }
}
