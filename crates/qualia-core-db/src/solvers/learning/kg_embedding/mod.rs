//! Knowledge-graph embedding (TransE / DistMult / ComplEx / RotatE) — score a
//! triple `(head, relation, tail)` for plausibility, and rank candidate entities
//! for **link prediction** over the semantic graph.
//!
//! ## Affordability gating (PROJECT RULE — the honest-scope test)
//!
//! KG embedding has two halves with wildly different cost:
//!
//! * **Scoring / ranking** ([`score`], [`predict`]) — a few dot products per triple.
//!   Trivially cheap, always present, runs on any device. This is the path a *user*
//!   exercises: given an already-trained [`EmbeddingTable`], score and rank.
//! * **Training** ([`train`]) — gradient descent over many epochs and negatives. This
//!   is the **heavy, run-once** pass: it is structured as an artifact producer that
//!   runs on capable hardware and is then *distributed* (the trained table), never on
//!   a user's critical path. It is dispatch-ready (§13): the per-triple score/gradient
//!   batch is kernel-class `DenseLinear`, with the CPU reference here always present.
//!
//! So nothing here forces a user into food-vs-compute: they consume a table; they do
//! not have to train one.
//!
//! ## Honesty
//!
//! Every public entry fails closed ([`KgEmbeddingError`]) on a dimension/index
//! mismatch rather than returning a fabricated score. A score is only ever produced
//! from real embedding arithmetic.

pub mod predict;
pub mod score;
pub mod train;

pub use predict::{hits_at_k, mean_rank, mean_reciprocal_rank, rank_tail, RankFilter};
pub use score::ScoreModel;
pub use train::{train, TrainConfig};

/// Fail-closed errors for the embedding library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KgEmbeddingError {
    /// A vector length did not match the model's expected entity/relation dim.
    InvalidDimension,
    /// An entity or relation index was out of range for the table.
    IndexOutOfRange,
    /// Not enough triples / rank to fit (e.g. empty training set).
    InsufficientData,
    /// A configuration was inconsistent (e.g. rank 0, zero epochs with no table).
    InvalidParameters,
}

impl core::fmt::Display for KgEmbeddingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KgEmbeddingError::InvalidDimension => write!(f, "embedding vector length mismatch"),
            KgEmbeddingError::IndexOutOfRange => write!(f, "entity/relation index out of range"),
            KgEmbeddingError::InsufficientData => write!(f, "insufficient data to fit"),
            KgEmbeddingError::InvalidParameters => write!(f, "invalid embedding configuration"),
        }
    }
}
impl std::error::Error for KgEmbeddingError {}

/// A trained (or freshly-initialised) embedding table: one vector per entity and one
/// per relation. The storage length per entity/relation is fixed by the model and the
/// rank `k` (see [`ScoreModel::dims`]).
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingTable {
    pub model: ScoreModel,
    /// Rank (the conceptual embedding dimension). Storage may be `k` or `2k` per
    /// vector depending on the model.
    pub rank: usize,
    pub ent_dim: usize,
    pub rel_dim: usize,
    pub n_entities: usize,
    pub n_relations: usize,
    /// `n_entities * ent_dim`, row-major.
    pub entities: Vec<f64>,
    /// `n_relations * rel_dim`, row-major.
    pub relations: Vec<f64>,
}

impl EmbeddingTable {
    /// Allocate a zeroed table sized for `model` at rank `k`.
    pub fn zeros(
        model: ScoreModel,
        k: usize,
        n_entities: usize,
        n_relations: usize,
    ) -> Result<Self, KgEmbeddingError> {
        if k == 0 || n_entities == 0 || n_relations == 0 {
            return Err(KgEmbeddingError::InvalidParameters);
        }
        let (ent_dim, rel_dim) = model.dims(k);
        Ok(Self {
            model,
            rank: k,
            ent_dim,
            rel_dim,
            n_entities,
            n_relations,
            entities: vec![0.0; n_entities * ent_dim],
            relations: vec![0.0; n_relations * rel_dim],
        })
    }

    #[inline]
    pub fn entity(&self, i: usize) -> Result<&[f64], KgEmbeddingError> {
        if i >= self.n_entities {
            return Err(KgEmbeddingError::IndexOutOfRange);
        }
        Ok(&self.entities[i * self.ent_dim..(i + 1) * self.ent_dim])
    }

    #[inline]
    pub fn relation(&self, i: usize) -> Result<&[f64], KgEmbeddingError> {
        if i >= self.n_relations {
            return Err(KgEmbeddingError::IndexOutOfRange);
        }
        Ok(&self.relations[i * self.rel_dim..(i + 1) * self.rel_dim])
    }

    #[inline]
    pub fn entity_mut(&mut self, i: usize) -> &mut [f64] {
        let d = self.ent_dim;
        &mut self.entities[i * d..(i + 1) * d]
    }

    #[inline]
    pub fn relation_mut(&mut self, i: usize) -> &mut [f64] {
        let d = self.rel_dim;
        &mut self.relations[i * d..(i + 1) * d]
    }

    /// Plausibility score for triple `(h, r, t)` (entity/relation indices). Higher =
    /// more plausible. Fails closed on out-of-range indices.
    pub fn score(&self, h: usize, r: usize, t: usize) -> Result<f64, KgEmbeddingError> {
        let hv = self.entity(h)?;
        let rv = self.relation(r)?;
        let tv = self.entity(t)?;
        self.model.score(hv, rv, tv, self.rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_dims_and_accessors() {
        let t = EmbeddingTable::zeros(ScoreModel::ComplEx, 4, 3, 2).unwrap();
        assert_eq!(t.ent_dim, 8); // 2k
        assert_eq!(t.rel_dim, 8);
        assert_eq!(t.entities.len(), 24);
        assert!(t.entity(2).is_ok());
        assert_eq!(t.entity(3).unwrap_err(), KgEmbeddingError::IndexOutOfRange);
    }

    #[test]
    fn rotate_table_relation_is_angles_only() {
        let t = EmbeddingTable::zeros(ScoreModel::RotatE, 5, 2, 2).unwrap();
        assert_eq!(t.ent_dim, 10); // 2k
        assert_eq!(t.rel_dim, 5); // k angles
    }

    #[test]
    fn zeros_fails_closed_on_degenerate() {
        assert_eq!(
            EmbeddingTable::zeros(ScoreModel::TransE { p: 2 }, 0, 1, 1).unwrap_err(),
            KgEmbeddingError::InvalidParameters
        );
    }
}
