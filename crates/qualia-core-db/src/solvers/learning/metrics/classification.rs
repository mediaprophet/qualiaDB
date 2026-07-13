//! Classification metrics — accuracy, the binary confusion matrix and its derived
//! rates, ROC AUC (rank form, reusing the statistics ranker), and log-loss.

use crate::solvers::statistics::correlation::rank_into;

/// Fraction of exact matches between predicted and true class labels. `None` if
/// lengths differ or are empty.
pub fn accuracy(y_true: &[usize], y_pred: &[usize]) -> Option<f64> {
    let n = y_true.len();
    if n == 0 || n != y_pred.len() {
        return None;
    }
    let correct = y_true.iter().zip(y_pred).filter(|(a, b)| a == b).count();
    Some(correct as f64 / n as f64)
}

/// Binary confusion matrix (positive class = `true`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfusionBinary {
    pub tp: u64,
    pub fp: u64,
    pub tn: u64,
    pub fn_: u64,
}

impl ConfusionBinary {
    pub fn total(&self) -> u64 {
        self.tp + self.fp + self.tn + self.fn_
    }
    pub fn accuracy(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            return 0.0;
        }
        (self.tp + self.tn) as f64 / t as f64
    }
    /// TP/(TP+FP); 0 when no positives are predicted.
    pub fn precision(&self) -> f64 {
        let d = self.tp + self.fp;
        if d == 0 {
            0.0
        } else {
            self.tp as f64 / d as f64
        }
    }
    /// TP/(TP+FN) (sensitivity / true-positive rate).
    pub fn recall(&self) -> f64 {
        let d = self.tp + self.fn_;
        if d == 0 {
            0.0
        } else {
            self.tp as f64 / d as f64
        }
    }
    /// TN/(TN+FP) (specificity).
    pub fn specificity(&self) -> f64 {
        let d = self.tn + self.fp;
        if d == 0 {
            0.0
        } else {
            self.tn as f64 / d as f64
        }
    }
    /// Harmonic mean of precision and recall.
    pub fn f1(&self) -> f64 {
        let (p, r) = (self.precision(), self.recall());
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

/// Build the binary confusion matrix from predicted/true boolean labels.
pub fn confusion_binary(y_true: &[bool], y_pred: &[bool]) -> Option<ConfusionBinary> {
    let n = y_true.len();
    if n == 0 || n != y_pred.len() {
        return None;
    }
    let mut c = ConfusionBinary {
        tp: 0,
        fp: 0,
        tn: 0,
        fn_: 0,
    };
    for (&t, &p) in y_true.iter().zip(y_pred) {
        match (t, p) {
            (true, true) => c.tp += 1,
            (false, true) => c.fp += 1,
            (false, false) => c.tn += 1,
            (true, false) => c.fn_ += 1,
        }
    }
    Some(c)
}

/// ROC AUC via the Mann–Whitney rank statistic:
/// `AUC = (R₊ − n₊(n₊+1)/2) / (n₊·n₋)`, where `R₊` is the sum of the (tie-averaged)
/// ranks of the positive-class scores. Reuses the statistics ranker. `None` if the
/// inputs mismatch or a class is empty.
pub fn roc_auc(scores: &[f64], labels: &[bool]) -> Option<f64> {
    let n = scores.len();
    if n == 0 || n != labels.len() {
        return None;
    }
    let n_pos = labels.iter().filter(|&&l| l).count();
    let n_neg = n - n_pos;
    if n_pos == 0 || n_neg == 0 {
        return None; // AUC undefined with only one class present
    }
    let mut idx = vec![0usize; n];
    let mut ranks = vec![0.0f64; n];
    rank_into(scores, &mut idx, &mut ranks)?;
    let sum_pos_ranks: f64 = labels
        .iter()
        .zip(ranks.iter())
        .filter(|(&l, _)| l)
        .map(|(_, &r)| r)
        .sum();
    let n_pos_f = n_pos as f64;
    Some((sum_pos_ranks - n_pos_f * (n_pos_f + 1.0) / 2.0) / (n_pos_f * n_neg as f64))
}

/// Binary cross-entropy (log-loss): `−(1/n)Σ[yᵢln pᵢ + (1−yᵢ)ln(1−pᵢ)]`, with `p`
/// clamped away from 0/1 for numerical safety. `None` on a length mismatch.
pub fn log_loss(probs: &[f64], labels: &[bool]) -> Option<f64> {
    let n = probs.len();
    if n == 0 || n != labels.len() {
        return None;
    }
    const EPS: f64 = 1e-15;
    let mut s = 0.0;
    for (&p, &y) in probs.iter().zip(labels) {
        let p = p.clamp(EPS, 1.0 - EPS);
        s += if y { -p.ln() } else { -(1.0 - p).ln() };
    }
    Some(s / n as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accuracy_basic() {
        assert!((accuracy(&[0, 1, 2, 1], &[0, 1, 1, 1]).unwrap() - 0.75).abs() < 1e-12);
        assert_eq!(accuracy(&[], &[]), None);
    }

    #[test]
    fn confusion_rates() {
        // 2 TP, 1 FP, 2 TN, 1 FN.
        let t = [true, true, false, false, false, true];
        let p = [true, true, true, false, false, false];
        let c = confusion_binary(&t, &p).unwrap();
        assert_eq!((c.tp, c.fp, c.tn, c.fn_), (2, 1, 2, 1));
        assert!((c.precision() - 2.0 / 3.0).abs() < 1e-12);
        assert!((c.recall() - 2.0 / 3.0).abs() < 1e-12);
        assert!((c.f1() - 2.0 / 3.0).abs() < 1e-12);
        assert!((c.accuracy() - 4.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn auc_perfect_and_random() {
        // Perfectly separable: all positives score above all negatives → AUC 1.
        let scores = [0.1, 0.2, 0.3, 0.8, 0.9, 1.0];
        let labels = [false, false, false, true, true, true];
        assert!((roc_auc(&scores, &labels).unwrap() - 1.0).abs() < 1e-12);
        // Reversed → AUC 0.
        let rev = [true, true, true, false, false, false];
        assert!(roc_auc(&scores, &rev).unwrap().abs() < 1e-12);
        // Single class → undefined.
        assert_eq!(roc_auc(&scores, &[true; 6]), None);
    }

    #[test]
    fn auc_known_value() {
        // scores/labels with a known AUC of 0.75.
        let scores = [0.2, 0.4, 0.6, 0.8];
        let labels = [false, true, false, true];
        assert!((roc_auc(&scores, &labels).unwrap() - 0.75).abs() < 1e-12);
    }

    #[test]
    fn log_loss_rewards_confident_correct() {
        let confident = log_loss(&[0.99, 0.01], &[true, false]).unwrap();
        let unsure = log_loss(&[0.5, 0.5], &[true, false]).unwrap();
        assert!(confident < unsure);
    }
}
