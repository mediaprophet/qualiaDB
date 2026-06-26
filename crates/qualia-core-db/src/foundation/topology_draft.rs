//! Topological speculative decoding — concept hash → token id (B3.1c).

use crate::compute_universe::{TopologyDraftBatch, MAX_DRAFT_LEN};
use crate::gguf_sharder::GgufTokenizer;

/// Cold-path vocabulary bridge: 10D concept hashes → GGUF token ids.
pub struct TopologyDraftMapper<'a> {
    tokenizer: &'a GgufTokenizer,
}

impl<'a> TopologyDraftMapper<'a> {
    pub fn new(tokenizer: &'a GgufTokenizer) -> Self {
        Self { tokenizer }
    }

    /// Map a concept fingerprint to a draft token id (stable across runs for a given vocab).
    pub fn concept_to_token_id(&self, concept_hash: u64) -> u32 {
        let probe = format!("q42:{:016x}", concept_hash);
        let ids = self.tokenizer.encode(&probe);
        if let Some(&id) = ids.first() {
            return id;
        }
        (concept_hash as u32) % self.tokenizer.vocab_len().max(1)
    }

    /// Fill a draft batch from concept hashes (γ ≤ `MAX_DRAFT_LEN`).
    pub fn fill_draft_batch(&self, concept_hashes: &[u64], gamma: usize) -> TopologyDraftBatch {
        let gamma = gamma.clamp(1, MAX_DRAFT_LEN).min(concept_hashes.len());
        let mut batch = TopologyDraftBatch::empty();
        for i in 0..gamma {
            batch.concept_hashes[i] = concept_hashes[i];
            batch.draft_ids[i] = self.concept_to_token_id(concept_hashes[i]);
        }
        batch.draft_len = gamma as u8;
        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapper_is_stable_for_same_hash() {
        let tok = GgufTokenizer::default();
        let mapper = TopologyDraftMapper::new(&tok);
        let a = mapper.concept_to_token_id(0xDEAD_BEEF);
        let b = mapper.concept_to_token_id(0xDEAD_BEEF);
        assert_eq!(a, b);
    }

    #[test]
    fn fill_draft_batch_respects_gamma() {
        let tok = GgufTokenizer::default();
        let mapper = TopologyDraftMapper::new(&tok);
        let batch = mapper.fill_draft_batch(&[1, 2, 3, 4], 3);
        assert_eq!(batch.draft_len, 3);
        assert_ne!(batch.draft_ids[0], 0);
    }
}