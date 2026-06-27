//! Embedding training — the **heavy, run-once** artifact producer (see the module
//! docs on affordability gating). Stochastic gradient descent with negative sampling:
//! a *margin ranking* loss for the translational models (TransE, RotatE) and a
//! *logistic* loss for the bilinear models (DistMult, ComplEx).
//!
//! This is the path that must NOT sit on a user's critical path: a capable machine
//! runs it once and distributes the resulting [`EmbeddingTable`]. The per-triple score
//! and gradient are kernel-class `DenseLinear`; this CPU reference is always present
//! and is what a future GPU batch path would be correctness-gated against (§13).
//!
//! The RNG is the deterministic LCG shared with the optimisation library, so a given
//! `seed` reproduces the same table — important for an auditable, distributable
//! artifact.

use super::score::ScoreModel;
use super::{EmbeddingTable, KgEmbeddingError};
use crate::solvers::optimization::metaheuristics::Rng;

/// Training hyper-parameters.
#[derive(Debug, Clone, Copy)]
pub struct TrainConfig {
    pub model: ScoreModel,
    pub rank: usize,
    pub epochs: usize,
    /// Learning rate.
    pub lr: f64,
    /// Margin γ for the ranking loss (translational models). Ignored for logistic.
    pub margin: f64,
    /// L2 regularisation coefficient (logistic models). Ignored for margin.
    pub reg: f64,
    /// Negative samples drawn per positive triple.
    pub neg_per_pos: usize,
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            model: ScoreModel::TransE { p: 2 },
            rank: 16,
            epochs: 100,
            lr: 0.05,
            margin: 1.0,
            reg: 1e-3,
            neg_per_pos: 2,
            seed: 1,
        }
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Train an embedding table on `triples` (entity/relation indices) with `n_entities`
/// distinct entities and `n_relations` relations. Returns the trained table, or fails
/// closed on an empty corpus / inconsistent config.
pub fn train(
    triples: &[(usize, usize, usize)],
    n_entities: usize,
    n_relations: usize,
    cfg: TrainConfig,
) -> Result<EmbeddingTable, KgEmbeddingError> {
    if triples.is_empty() {
        return Err(KgEmbeddingError::InsufficientData);
    }
    if cfg.rank == 0 || cfg.epochs == 0 {
        return Err(KgEmbeddingError::InvalidParameters);
    }
    // Validate indices up front (fail closed before any work).
    for &(h, r, t) in triples {
        if h >= n_entities || t >= n_entities || r >= n_relations {
            return Err(KgEmbeddingError::IndexOutOfRange);
        }
    }

    let mut table = EmbeddingTable::zeros(cfg.model, cfg.rank, n_entities, n_relations)?;
    let mut rng = Rng(cfg.seed ^ 0x4B47_4D42_4544_4447);

    // Xavier-ish small init in [-s, s].
    let s = (6.0_f64 / cfg.rank as f64).sqrt();
    for v in table.entities.iter_mut().chain(table.relations.iter_mut()) {
        *v = (rng.unit() * 2.0 - 1.0) * s;
    }

    let is_margin = matches!(cfg.model, ScoreModel::TransE { .. } | ScoreModel::RotatE);
    let (ed, rd) = cfg.model.dims(cfg.rank);

    // Reusable gradient buffers (no per-step heap churn in the inner loop).
    let mut ghp = vec![0.0; ed];
    let mut grp = vec![0.0; rd];
    let mut gtp = vec![0.0; ed];
    let mut ghn = vec![0.0; ed];
    let mut grn = vec![0.0; rd];
    let mut gtn = vec![0.0; ed];

    for _epoch in 0..cfg.epochs {
        for &(h, r, t) in triples {
            // Snapshot positive vectors.
            let hv = table.entity(h)?.to_vec();
            let rv = table.relation(r)?.to_vec();
            let tv = table.entity(t)?.to_vec();

            for _ in 0..cfg.neg_per_pos {
                // Corrupt head or tail with a random distinct entity.
                let corrupt_tail = rng.unit() < 0.5;
                let mut neg = rng.below(n_entities);
                let avoid = if corrupt_tail { t } else { h };
                if neg == avoid {
                    neg = (neg + 1) % n_entities;
                }
                let (nh, nt) = if corrupt_tail { (h, neg) } else { (neg, t) };
                let nhv = table.entity(nh)?.to_vec();
                let ntv = table.entity(nt)?.to_vec();

                let score_pos = cfg.model.score(&hv, &rv, &tv, cfg.rank)?;
                let score_neg = cfg.model.score(&nhv, &rv, &ntv, cfg.rank)?;

                if is_margin {
                    // L = max(0, margin - score_pos + score_neg). score = -distance.
                    let loss = cfg.margin - score_pos + score_neg;
                    if loss > 0.0 {
                        cfg.model.gradient(&hv, &rv, &tv, cfg.rank, &mut ghp, &mut grp, &mut gtp)?;
                        cfg.model.gradient(&nhv, &rv, &ntv, cfg.rank, &mut ghn, &mut grn, &mut gtn)?;
                        // Ascend score_pos, descend score_neg.
                        apply(table.entity_mut(h), &ghp, cfg.lr);
                        apply(table.entity_mut(t), &gtp, cfg.lr);
                        apply(table.relation_mut(r), &grp, cfg.lr);
                        apply(table.entity_mut(nh), &ghn, -cfg.lr);
                        apply(table.entity_mut(nt), &gtn, -cfg.lr);
                        apply(table.relation_mut(r), &grn, -cfg.lr);
                    }
                } else {
                    // Logistic: param -= lr*(∂L/∂s * grad + reg*param), ∂L/∂s = -y·σ(-y·s).
                    // Positive (y=+1).
                    cfg.model.gradient(&hv, &rv, &tv, cfg.rank, &mut ghp, &mut grp, &mut gtp)?;
                    let cp = -sigmoid(-score_pos);
                    apply_reg(table.entity_mut(h), &ghp, cfg.lr, cp, cfg.reg);
                    apply_reg(table.entity_mut(t), &gtp, cfg.lr, cp, cfg.reg);
                    apply_reg(table.relation_mut(r), &grp, cfg.lr, cp, cfg.reg);
                    // Negative (y=-1).
                    cfg.model.gradient(&nhv, &rv, &ntv, cfg.rank, &mut ghn, &mut grn, &mut gtn)?;
                    let cn = sigmoid(score_neg);
                    apply_reg(table.entity_mut(nh), &ghn, cfg.lr, cn, cfg.reg);
                    apply_reg(table.entity_mut(nt), &gtn, cfg.lr, cn, cfg.reg);
                    apply_reg(table.relation_mut(r), &grn, cfg.lr, cn, cfg.reg);
                }
            }

            // TransE/RotatE: renormalise entity embeddings to unit L2 (standard).
            if is_margin {
                normalise(table.entity_mut(h));
                normalise(table.entity_mut(t));
            }
        }
    }

    Ok(table)
}

/// `param += lr * grad` (gradient ascent on a score).
fn apply(param: &mut [f64], grad: &[f64], lr: f64) {
    for (p, &g) in param.iter_mut().zip(grad) {
        *p += lr * g;
    }
}

/// `param -= lr * (coeff*grad + reg*param)` (logistic descent with L2).
fn apply_reg(param: &mut [f64], grad: &[f64], lr: f64, coeff: f64, reg: f64) {
    for (p, &g) in param.iter_mut().zip(grad) {
        *p -= lr * (coeff * g + reg * *p);
    }
}

fn normalise(v: &mut [f64]) {
    let n: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::predict::{rank_tail, RankFilter};
    use super::*;

    /// A consistent chain under one relation: 0→1→2→3. A working translational model
    /// must learn to rank the true tail first.
    fn chain() -> Vec<(usize, usize, usize)> {
        vec![(0, 0, 1), (1, 0, 2), (2, 0, 3)]
    }

    fn mean_pos_minus_neg(
        table: &EmbeddingTable,
        pos: &[(usize, usize, usize)],
        neg: &[(usize, usize, usize)],
    ) -> f64 {
        let mp: f64 = pos.iter().map(|&(h, r, t)| table.score(h, r, t).unwrap()).sum::<f64>() / pos.len() as f64;
        let mn: f64 = neg.iter().map(|&(h, r, t)| table.score(h, r, t).unwrap()).sum::<f64>() / neg.len() as f64;
        mp - mn
    }

    #[test]
    fn transe_learns_to_rank_true_tail_first() {
        // Two disjoint edges under one relation — a pattern TransE *can* represent
        // under unit-norm entities (a single shared translation offset). A length-3
        // chain on the unit sphere is the classic TransE representability limit, so we
        // avoid it here.
        let triples = vec![(0, 0, 1), (2, 0, 3)];
        let cfg = TrainConfig {
            model: ScoreModel::TransE { p: 2 },
            rank: 8,
            epochs: 500,
            lr: 0.05,
            margin: 1.0,
            reg: 0.0,
            neg_per_pos: 4,
            seed: 7,
        };
        let table = train(&triples, 4, 1, cfg).unwrap();
        // Each edge's true tail should rank first among all entities.
        assert_eq!(rank_tail(&table, 0, 0, 1, &[0, 1, 2, 3], RankFilter::Raw).unwrap(), 1);
        assert_eq!(rank_tail(&table, 2, 0, 3, &[0, 1, 2, 3], RankFilter::Raw).unwrap(), 1);
    }

    #[test]
    fn distmult_separates_positives_from_negatives() {
        // Symmetric "sibling" relation — DistMult models symmetry well.
        let pos = vec![(0, 0, 1), (1, 0, 0), (2, 0, 3), (3, 0, 2)];
        let neg = vec![(0, 0, 2), (1, 0, 3), (0, 0, 3)];
        let cfg = TrainConfig {
            model: ScoreModel::DistMult,
            rank: 8,
            epochs: 400,
            lr: 0.1,
            margin: 0.0,
            reg: 1e-3,
            neg_per_pos: 4,
            seed: 3,
        };
        let table = train(&pos, 4, 1, cfg).unwrap();
        let gap = mean_pos_minus_neg(&table, &pos, &neg);
        assert!(gap > 0.5, "DistMult positives not separated from negatives (gap {gap})");
    }

    #[test]
    fn complex_separates_positives() {
        let pos = vec![(0, 0, 1), (1, 0, 2), (2, 0, 0)]; // cyclic
        let neg = vec![(0, 0, 2), (1, 0, 0), (2, 0, 1)];
        let cfg = TrainConfig {
            model: ScoreModel::ComplEx,
            rank: 8,
            epochs: 500,
            lr: 0.1,
            margin: 0.0,
            reg: 1e-3,
            neg_per_pos: 4,
            seed: 5,
        };
        let table = train(&pos, 3, 1, cfg).unwrap();
        let gap = mean_pos_minus_neg(&table, &pos, &neg);
        assert!(gap > 0.3, "ComplEx positives not separated (gap {gap})");
    }

    #[test]
    fn rotate_learns_a_chain() {
        let triples = chain();
        let cfg = TrainConfig {
            model: ScoreModel::RotatE,
            rank: 8,
            epochs: 500,
            lr: 0.05,
            margin: 1.0,
            reg: 0.0,
            neg_per_pos: 4,
            seed: 11,
        };
        let table = train(&triples, 4, 1, cfg).unwrap();
        // Positive (0,0,1) should outscore a corrupted (0,0,3).
        assert!(table.score(0, 0, 1).unwrap() > table.score(0, 0, 3).unwrap());
    }

    #[test]
    fn empty_corpus_fails_closed() {
        assert_eq!(train(&[], 4, 1, TrainConfig::default()).unwrap_err(), KgEmbeddingError::InsufficientData);
    }

    #[test]
    fn out_of_range_index_fails_closed() {
        let bad = vec![(0, 0, 9)];
        assert_eq!(train(&bad, 4, 1, TrainConfig::default()).unwrap_err(), KgEmbeddingError::IndexOutOfRange);
    }
}
