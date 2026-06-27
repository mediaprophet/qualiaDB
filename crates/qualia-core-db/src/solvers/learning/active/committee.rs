//! Query-by-committee — query the points where an *ensemble* disagrees. Disagreement
//! is the signal: where independently-trained models diverge, a human label resolves
//! the most uncertainty.
//!
//! Two soft measures (over each member's predicted distribution) and one hard measure
//! (over each member's single vote), all per-sample; plus a pool ranking.

use super::{argsort_desc, ActiveError};
use crate::solvers::statistics::information::{entropy, kl_divergence};

/// **Vote entropy** over hard votes: entropy of the distribution of committee votes
/// across `n_classes`. Maximal when the committee splits evenly. `votes` are class
/// indices, one per member.
pub fn vote_entropy(votes: &[usize], n_classes: usize) -> Result<f64, ActiveError> {
    if votes.is_empty() || n_classes == 0 {
        return Err(ActiveError::InsufficientData);
    }
    let mut counts = vec![0.0f64; n_classes];
    for &v in votes {
        if v >= n_classes {
            return Err(ActiveError::InvalidDimension);
        }
        counts[v] += 1.0;
    }
    let total = votes.len() as f64;
    for c in counts.iter_mut() {
        *c /= total;
    }
    entropy(&counts).ok_or(ActiveError::InvalidDimension)
}

/// The committee **consensus** distribution: the mean of the members' predicted
/// distributions for one sample. `members` is `n_members × n_classes`.
pub fn consensus(members: &[Vec<f64>]) -> Result<Vec<f64>, ActiveError> {
    if members.is_empty() {
        return Err(ActiveError::InsufficientData);
    }
    let n_classes = members[0].len();
    if n_classes == 0 || members.iter().any(|m| m.len() != n_classes) {
        return Err(ActiveError::InvalidDimension);
    }
    let mut mean = vec![0.0; n_classes];
    for m in members {
        for (acc, &p) in mean.iter_mut().zip(m) {
            *acc += p;
        }
    }
    let k = members.len() as f64;
    for v in mean.iter_mut() {
        *v /= k;
    }
    Ok(mean)
}

/// **Consensus entropy** (soft vote entropy): entropy of the consensus distribution.
/// Diffuse consensus ⇒ informative.
pub fn consensus_entropy(members: &[Vec<f64>]) -> Result<f64, ActiveError> {
    entropy(&consensus(members)?).ok_or(ActiveError::InvalidDimension)
}

/// **Average KL disagreement**: mean `KL(member ‖ consensus)` over the committee. This
/// is the canonical soft-QBC measure — it is large precisely when members are confident
/// but *about different classes* (genuine disagreement), not merely diffuse.
pub fn average_kl_disagreement(members: &[Vec<f64>]) -> Result<f64, ActiveError> {
    let cons = consensus(members)?;
    let mut sum = 0.0;
    for m in members {
        sum += kl_divergence(m, &cons).ok_or(ActiveError::InvalidDimension)?;
    }
    Ok(sum / members.len() as f64)
}

/// Rank pool indices by average-KL disagreement, most-informative first. `pool` is
/// `n_samples` committees, each `n_members × n_classes`.
pub fn rank_by_disagreement(pool: &[Vec<Vec<f64>>]) -> Result<Vec<usize>, ActiveError> {
    if pool.is_empty() {
        return Err(ActiveError::InsufficientData);
    }
    let scores: Result<Vec<f64>, ActiveError> =
        pool.iter().map(|c| average_kl_disagreement(c)).collect();
    Ok(argsort_desc(&scores?))
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn vote_entropy_max_on_even_split() {
        // 2 members vote different classes → even split → entropy = 1 bit (base-2).
        let v = vote_entropy(&[0, 1], 2).unwrap();
        assert!((v - 1.0).abs() < 1e-9);
        // Unanimous → entropy 0.
        assert!(vote_entropy(&[1, 1, 1], 2).unwrap().abs() < EPS);
    }

    #[test]
    fn disagreeing_committee_scores_above_agreeing() {
        // Agreeing: both confident class 0.
        let agree = vec![vec![0.9, 0.1], vec![0.85, 0.15]];
        // Disagreeing: one sure class 0, the other sure class 1.
        let disagree = vec![vec![0.95, 0.05], vec![0.05, 0.95]];
        let a = average_kl_disagreement(&agree).unwrap();
        let d = average_kl_disagreement(&disagree).unwrap();
        assert!(d > a, "disagreement {d} should exceed agreement {a}");
    }

    #[test]
    fn consensus_is_the_mean_distribution() {
        let m = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let c = consensus(&m).unwrap();
        assert!((c[0] - 0.5).abs() < EPS && (c[1] - 0.5).abs() < EPS);
        // Consensus entropy of an even split = 1 bit (base-2).
        assert!((consensus_entropy(&m).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pool_ranking_puts_the_split_sample_first() {
        let agree = vec![vec![0.9, 0.1], vec![0.88, 0.12]];
        let split = vec![vec![0.95, 0.05], vec![0.05, 0.95]];
        let pool = vec![agree, split];
        let r = rank_by_disagreement(&pool).unwrap();
        assert_eq!(r[0], 1);
    }

    #[test]
    fn fails_closed() {
        assert_eq!(consensus(&[]).unwrap_err(), ActiveError::InsufficientData);
        assert_eq!(vote_entropy(&[5], 2).unwrap_err(), ActiveError::InvalidDimension);
    }
}
