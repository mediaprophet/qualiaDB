//! k-Nearest-Neighbours classifier (ISL ch 2/4) — a lazy learner: store the
//! training set, classify a query by majority vote of its `k` nearest neighbours
//! (squared Euclidean). Kernel-class `AllPairs` (query↔train distances).

use crate::solvers::learning::LearningError;

/// A fitted (stored) k-NN classifier.
#[derive(Debug, Clone)]
pub struct KnnClassifier {
    x: Vec<f64>,
    y: Vec<usize>,
    n: usize,
    p: usize,
    k: usize,
}

impl KnnClassifier {
    /// Store the training data. Fails closed on shape mismatch or `k` out of range.
    pub fn fit(
        x: &[f64],
        y: &[usize],
        n: usize,
        p: usize,
        k: usize,
    ) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
            return Err(LearningError::InvalidDimension);
        }
        if k == 0 || k > n {
            return Err(LearningError::InsufficientData);
        }
        Ok(Self {
            x: x.to_vec(),
            y: y.to_vec(),
            n,
            p,
            k,
        })
    }

    /// Predict the class of one query row by majority vote of the `k` nearest
    /// training points (ties broken toward the lower class label).
    pub fn predict_row(&self, q: &[f64]) -> usize {
        // Indices of all training rows, sorted by distance to q, take k.
        let mut idx: Vec<usize> = (0..self.n).collect();
        idx.sort_by(|&a, &b| {
            let da = self.sq_dist(a, q);
            let db = self.sq_dist(b, q);
            da.partial_cmp(&db).unwrap_or(core::cmp::Ordering::Equal)
        });
        // Tally votes among the k nearest.
        let max_label = *self.y.iter().max().unwrap_or(&0);
        let mut votes = vec![0usize; max_label + 1];
        for &i in idx.iter().take(self.k) {
            votes[self.y[i]] += 1;
        }
        // argmax votes, lowest label on a tie.
        let mut best = 0;
        let mut best_v = 0;
        for (label, &v) in votes.iter().enumerate() {
            if v > best_v {
                best_v = v;
                best = label;
            }
        }
        best
    }

    /// Predict classes for a row-major `m × p` query matrix.
    pub fn predict(&self, x: &[f64], m: usize) -> Vec<usize> {
        (0..m)
            .map(|i| self.predict_row(&x[i * self.p..(i + 1) * self.p]))
            .collect()
    }

    #[inline]
    fn sq_dist(&self, train_row: usize, q: &[f64]) -> f64 {
        let row = &self.x[train_row * self.p..(train_row + 1) * self.p];
        row.iter().zip(q).map(|(a, b)| (a - b) * (a - b)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_nearest_neighbours() {
        // Two clusters: class 0 near origin, class 1 near (10,10).
        let x = [
            0.0, 0.0, 0.5, 0.3, -0.2, 0.1, 10.0, 10.0, 9.7, 10.2, 10.3, 9.8,
        ];
        let y = [0, 0, 0, 1, 1, 1];
        let knn = KnnClassifier::fit(&x, &y, 6, 2, 3).unwrap();
        assert_eq!(knn.predict_row(&[0.1, 0.1]), 0);
        assert_eq!(knn.predict_row(&[10.1, 9.9]), 1);
    }

    #[test]
    fn k_one_is_the_single_nearest() {
        let x = [0.0, 0.0, 5.0, 5.0];
        let y = [7, 3];
        let knn = KnnClassifier::fit(&x, &y, 2, 2, 1).unwrap();
        assert_eq!(knn.predict_row(&[0.4, 0.4]), 7);
        assert_eq!(knn.predict_row(&[4.6, 4.6]), 3);
    }

    #[test]
    fn guards() {
        assert_eq!(
            KnnClassifier::fit(&[1.0, 2.0], &[0], 1, 2, 5).unwrap_err(),
            LearningError::InsufficientData
        );
        assert_eq!(
            KnnClassifier::fit(&[1.0, 2.0], &[0, 1], 2, 2, 1).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
