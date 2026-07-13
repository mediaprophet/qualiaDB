//! Uncertainty sampling — query the points where a *single* model is least sure.
//! Operates on a predicted class-probability matrix (`n_samples × n_classes`, each row
//! a distribution); returns per-sample informativeness and a ranking.

use super::{argsort_desc, ActiveError};
use crate::solvers::statistics::information::entropy;

/// Which uncertainty measure to rank by. All are oriented so **higher = more
/// informative** (more worth a human's label).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// `1 − max_c p(c)` — least confidence in the top prediction.
    LeastConfidence,
    /// `1 − (p₁ − p₂)` — small margin between the top two classes ⇒ informative.
    Margin,
    /// Shannon entropy of the row — diffuse predictions ⇒ informative.
    Entropy,
}

fn top_two(row: &[f64]) -> (f64, f64) {
    let mut a = f64::NEG_INFINITY; // largest
    let mut b = f64::NEG_INFINITY; // second
    for &p in row {
        if p > a {
            b = a;
            a = p;
        } else if p > b {
            b = p;
        }
    }
    if b.is_infinite() {
        b = 0.0;
    }
    (a, b)
}

/// Informativeness of one predicted distribution under `strategy`.
pub fn row_score(row: &[f64], strategy: Strategy) -> Result<f64, ActiveError> {
    if row.is_empty() {
        return Err(ActiveError::InvalidDimension);
    }
    Ok(match strategy {
        Strategy::LeastConfidence => {
            let (top, _) = top_two(row);
            1.0 - top
        }
        Strategy::Margin => {
            let (p1, p2) = top_two(row);
            1.0 - (p1 - p2)
        }
        Strategy::Entropy => entropy(row).ok_or(ActiveError::InvalidDimension)?,
    })
}

/// Per-sample informativeness for a probability matrix (each inner slice a row).
pub fn score(probs: &[Vec<f64>], strategy: Strategy) -> Result<Vec<f64>, ActiveError> {
    if probs.is_empty() {
        return Err(ActiveError::InsufficientData);
    }
    let n_classes = probs[0].len();
    if n_classes == 0 || probs.iter().any(|r| r.len() != n_classes) {
        return Err(ActiveError::InvalidDimension);
    }
    probs.iter().map(|r| row_score(r, strategy)).collect()
}

/// Rank pool indices most-informative first under `strategy`.
pub fn rank_informative(probs: &[Vec<f64>], strategy: Strategy) -> Result<Vec<usize>, ActiveError> {
    Ok(argsort_desc(&score(probs, strategy)?))
}

/// The single most-informative pool index (the next item to ask a human about).
pub fn most_informative(probs: &[Vec<f64>], strategy: Strategy) -> Result<usize, ActiveError> {
    rank_informative(probs, strategy)?
        .first()
        .copied()
        .ok_or(ActiveError::InsufficientData)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn least_confidence_picks_the_flat_distribution() {
        // Sample 0 confident (0.9), sample 1 uniform (0.5/0.5) → 1 more informative.
        let probs = vec![vec![0.9, 0.1], vec![0.5, 0.5]];
        let s = score(&probs, Strategy::LeastConfidence).unwrap();
        assert!((s[0] - 0.1).abs() < EPS);
        assert!((s[1] - 0.5).abs() < EPS);
        assert_eq!(
            most_informative(&probs, Strategy::LeastConfidence).unwrap(),
            1
        );
    }

    #[test]
    fn margin_uses_top_two_gap() {
        // Three classes: [0.5,0.3,0.2] margin .2 → score .8; [0.4,0.4,0.2] margin 0 → 1.0
        let probs = vec![vec![0.5, 0.3, 0.2], vec![0.4, 0.4, 0.2]];
        let s = score(&probs, Strategy::Margin).unwrap();
        assert!((s[0] - 0.8).abs() < EPS);
        assert!((s[1] - 1.0).abs() < EPS);
        assert_eq!(most_informative(&probs, Strategy::Margin).unwrap(), 1);
    }

    #[test]
    fn entropy_ranks_diffuse_highest() {
        let probs = vec![vec![1.0, 0.0], vec![0.5, 0.5]];
        let r = rank_informative(&probs, Strategy::Entropy).unwrap();
        assert_eq!(r[0], 1); // uniform row has max entropy
        let s = score(&probs, Strategy::Entropy).unwrap();
        assert!((s[0]).abs() < EPS); // entropy of a one-hot is 0
    }

    #[test]
    fn fails_closed_on_empty_and_ragged() {
        assert_eq!(
            score(&[], Strategy::Entropy).unwrap_err(),
            ActiveError::InsufficientData
        );
        let ragged = vec![vec![0.5, 0.5], vec![1.0]];
        assert_eq!(
            score(&ragged, Strategy::Entropy).unwrap_err(),
            ActiveError::InvalidDimension
        );
    }
}
