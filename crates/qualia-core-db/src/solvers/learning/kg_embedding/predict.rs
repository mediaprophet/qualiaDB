//! Link prediction: rank candidate entities for an incomplete triple, and the
//! standard ranking metrics (mean rank, MRR, Hits@k). This is the **cheap, always-on**
//! path — given a trained [`EmbeddingTable`], answer "which tail best completes
//! `(h, r, ?)`" by scoring candidates and ranking by plausibility.

use super::{EmbeddingTable, KgEmbeddingError};

/// Whether to use the *filtered* ranking protocol (Bordes et al. 2013): known-true
/// triples other than the target are removed from the candidate set before ranking,
/// so a model is not penalised for ranking another genuine answer above the target.
pub enum RankFilter<'a> {
    /// Raw ranking — score against all candidates.
    Raw,
    /// Filtered — exclude these (already-known-true) tail indices from competing.
    Known(&'a [usize]),
}

/// Rank of the true tail `t` among `candidates` for `(h, r, ?)`. Rank 1 is best.
/// The rank is `1 + (#candidates scoring strictly higher than the true tail)`; ties
/// are broken pessimistically by also counting equal-scoring *different* candidates
/// at half weight is avoided — we use the strict-greater convention (optimistic ties),
/// which is the common reporting choice. Fails closed on bad indices.
pub fn rank_tail(
    table: &EmbeddingTable,
    h: usize,
    r: usize,
    true_t: usize,
    candidates: &[usize],
    filter: RankFilter,
) -> Result<usize, KgEmbeddingError> {
    let target = table.score(h, r, true_t)?;
    let known: &[usize] = match filter {
        RankFilter::Raw => &[],
        RankFilter::Known(k) => k,
    };
    let mut rank = 1usize;
    for &c in candidates {
        if c == true_t {
            continue;
        }
        if known.contains(&c) {
            continue; // filtered out — a genuine answer, not a distractor
        }
        let s = table.score(h, r, c)?;
        if s > target {
            rank += 1;
        }
    }
    Ok(rank)
}

/// Mean rank over a set of `(h, r, t)` test triples, each ranked against `candidates`
/// (typically all entities). Lower is better.
pub fn mean_rank(
    table: &EmbeddingTable,
    triples: &[(usize, usize, usize)],
    candidates: &[usize],
) -> Result<f64, KgEmbeddingError> {
    if triples.is_empty() {
        return Err(KgEmbeddingError::InsufficientData);
    }
    let mut sum = 0.0;
    for &(h, r, t) in triples {
        sum += rank_tail(table, h, r, t, candidates, RankFilter::Raw)? as f64;
    }
    Ok(sum / triples.len() as f64)
}

/// Mean reciprocal rank (MRR) — `mean(1/rank)`. Higher is better, in `(0, 1]`.
pub fn mean_reciprocal_rank(
    table: &EmbeddingTable,
    triples: &[(usize, usize, usize)],
    candidates: &[usize],
) -> Result<f64, KgEmbeddingError> {
    if triples.is_empty() {
        return Err(KgEmbeddingError::InsufficientData);
    }
    let mut sum = 0.0;
    for &(h, r, t) in triples {
        let rk = rank_tail(table, h, r, t, candidates, RankFilter::Raw)?;
        sum += 1.0 / rk as f64;
    }
    Ok(sum / triples.len() as f64)
}

/// Hits@k — fraction of test triples whose true tail ranks within the top `k`.
pub fn hits_at_k(
    table: &EmbeddingTable,
    triples: &[(usize, usize, usize)],
    candidates: &[usize],
    k: usize,
) -> Result<f64, KgEmbeddingError> {
    if triples.is_empty() || k == 0 {
        return Err(KgEmbeddingError::InsufficientData);
    }
    let mut hits = 0usize;
    for &(h, r, t) in triples {
        if rank_tail(table, h, r, t, candidates, RankFilter::Raw)? <= k {
            hits += 1;
        }
    }
    Ok(hits as f64 / triples.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::super::score::ScoreModel;
    use super::*;

    /// Build a tiny TransE table by hand where the geometry is exactly right:
    /// entity 0 + relation 0 = entity 1. Then (0, 0, 1) must rank 1.
    fn hand_table() -> EmbeddingTable {
        let mut t = EmbeddingTable::zeros(ScoreModel::TransE { p: 2 }, 2, 3, 1).unwrap();
        t.entity_mut(0).copy_from_slice(&[0.0, 0.0]);
        t.entity_mut(1).copy_from_slice(&[1.0, 0.0]);
        t.entity_mut(2).copy_from_slice(&[5.0, 5.0]); // distractor, far away
        t.relation_mut(0).copy_from_slice(&[1.0, 0.0]);
        t
    }

    #[test]
    fn true_tail_ranks_first() {
        let t = hand_table();
        let rank = rank_tail(&t, 0, 0, 1, &[0, 1, 2], RankFilter::Raw).unwrap();
        assert_eq!(rank, 1);
    }

    #[test]
    fn metrics_on_a_perfect_table() {
        let t = hand_table();
        let test = [(0usize, 0usize, 1usize)];
        let cands = [0, 1, 2];
        assert!((mean_rank(&t, &test, &cands).unwrap() - 1.0).abs() < 1e-12);
        assert!((mean_reciprocal_rank(&t, &test, &cands).unwrap() - 1.0).abs() < 1e-12);
        assert!((hits_at_k(&t, &test, &cands, 1).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn filtered_protocol_excludes_known_true() {
        // Make entity 2 also a good answer; filtering it should keep rank 1.
        let mut t = hand_table();
        t.entity_mut(2).copy_from_slice(&[1.0, 0.0]); // now also exactly h+r
        let raw = rank_tail(&t, 0, 0, 1, &[0, 1, 2], RankFilter::Raw).unwrap();
        let filt = rank_tail(&t, 0, 0, 1, &[0, 1, 2], RankFilter::Known(&[2])).unwrap();
        assert!(raw >= 1 && filt == 1, "raw {raw} filt {filt}");
    }

    #[test]
    fn empty_test_set_fails_closed() {
        let t = hand_table();
        assert_eq!(mean_rank(&t, &[], &[0, 1, 2]).unwrap_err(), KgEmbeddingError::InsufficientData);
    }
}
