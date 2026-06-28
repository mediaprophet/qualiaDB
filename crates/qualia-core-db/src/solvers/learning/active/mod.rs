//! **Active learning** — spend the human's attestation budget wisely.
//!
//! The mission frame: machine carries Data→Knowledge, but *wisdom* (the final
//! judgement, the label, the ratification) stays with the human. Human attention is
//! the scarce, expensive resource. Active learning is the theory of **ranking which
//! few items are most worth a human's judgement** — so a model improves fastest per
//! label asked, and the person is never asked to grind through the obvious.
//!
//! This is the supply-side of frugality: instead of demanding mass labelling (which
//! burdens exactly the people this project protects), the engine surfaces the handful
//! of genuinely-informative cases and routes them for attestation — the same
//! `RequiresHumanReview` discipline as the rest of the stack.
//!
//! Three classic query strategies, each over the *predictions already produced* by the
//! existing estimators ([`crate::solvers::learning`]) — no new model, pure ranking:
//!
//! * [`uncertainty`] — query where one model is least sure (least-confidence, margin,
//!   entropy).
//! * [`committee`] — query where an ensemble *disagrees* (vote/consensus entropy, KL).
//! * [`density`] — weight uncertainty by how *representative* a point is, so the model
//!   is not lured into labelling unrepresentative outliers.
//!
//! Every entry fails closed ([`ActiveError`]); ranking reuses the engine's information
//! theory ([`crate::solvers::statistics::information`]). Kernel-class `Reduction`.

pub mod committee;
pub mod density;
pub mod uncertainty;

pub use committee::{average_kl_disagreement, consensus_entropy, vote_entropy};
pub use density::{cosine_similarity, information_density};
pub use uncertainty::{rank_informative, score, Strategy};

/// Fail-closed errors for active-learning ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveError {
    /// Inconsistent shapes (e.g. ragged probability rows, mismatched class counts).
    InvalidDimension,
    /// Not enough data to rank (empty pool / empty committee).
    InsufficientData,
}

impl core::fmt::Display for ActiveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ActiveError::InvalidDimension => write!(f, "inconsistent active-learning input shapes"),
            ActiveError::InsufficientData => write!(f, "insufficient data to rank a query"),
        }
    }
}
impl std::error::Error for ActiveError {}

/// Argsort `scores` descending (highest first), returning indices. Stable on ties.
pub(crate) fn argsort_desc(scores: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argsort_orders_high_to_low() {
        assert_eq!(argsort_desc(&[0.1, 0.9, 0.5]), vec![1, 2, 0]);
    }
}
