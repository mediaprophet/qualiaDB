//! Multiclass SVM by one-vs-rest (ISL ch 9.4.2) — train one binary SVM per class
//! (that class vs. all others) and predict the class whose decision value is
//! largest. Reuses the binary [`super::svm`] (no duplicated SMO).

use crate::solvers::learning::classification::svm::{self, Kernel, Svm};
use crate::solvers::learning::LearningError;

/// A one-vs-rest multiclass SVM.
#[derive(Debug, Clone)]
pub struct MulticlassSvm {
    classes: Vec<usize>,
    svms: Vec<Svm>,
    p: usize,
}

impl MulticlassSvm {
    /// Fit one binary SVM per class (class vs rest). Fails closed if a class is
    /// absent or the binary fit fails (e.g. a degenerate split).
    pub fn fit_one_vs_rest(
        x: &[f64],
        y: &[usize],
        n: usize,
        p: usize,
        c: f64,
        kernel: Kernel,
        max_passes: usize,
        tol: f64,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        let mut classes: Vec<usize> = y.to_vec();
        classes.sort_unstable();
        classes.dedup();
        if classes.len() < 2 {
            return Err(LearningError::InsufficientData);
        }
        let mut svms = Vec::with_capacity(classes.len());
        for &cls in &classes {
            let binary: Vec<bool> = y.iter().map(|&yi| yi == cls).collect();
            let svm = svm::fit(x, &binary, n, p, c, kernel, max_passes, tol)?;
            svms.push(svm);
        }
        Ok(Self { classes, svms, p })
    }

    /// Predict the class whose one-vs-rest decision value is largest.
    pub fn predict_row(&self, q: &[f64]) -> usize {
        let mut best = 0;
        let mut best_d = f64::NEG_INFINITY;
        for (i, svm) in self.svms.iter().enumerate() {
            let d = svm.decision_row(q);
            if d > best_d {
                best_d = d;
                best = i;
            }
        }
        self.classes[best]
    }

    pub fn predict(&self, x: &[f64], m: usize) -> Vec<usize> {
        (0..m).map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p])).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_three_classes() {
        // Three clusters: 0 near (0,0), 1 near (10,0), 2 near (5,10).
        let mut x = Vec::new();
        let mut y = Vec::new();
        for &(cx, cy, lbl) in &[(0.0, 0.0, 0usize), (10.0, 0.0, 1), (5.0, 10.0, 2)] {
            for d in 0..5 {
                x.push(cx + (d as f64 - 2.0) * 0.2);
                x.push(cy + (d as f64 - 2.0) * 0.2);
                y.push(lbl);
            }
        }
        let n = 15;
        let m = MulticlassSvm::fit_one_vs_rest(&x, &y, n, 2, 1.0, Kernel::Linear, 30, 1e-3).unwrap();
        assert_eq!(m.predict_row(&[0.0, 0.0]), 0);
        assert_eq!(m.predict_row(&[10.0, 0.0]), 1);
        assert_eq!(m.predict_row(&[5.0, 10.0]), 2);
    }

    #[test]
    fn guards() {
        assert_eq!(
            MulticlassSvm::fit_one_vs_rest(&[0.0, 0.0, 1.0, 1.0], &[0, 0], 2, 2, 1.0, Kernel::Linear, 5, 1e-3).unwrap_err(),
            LearningError::InsufficientData
        );
    }
}
