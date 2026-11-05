//! Fingerprint the tokenizer actually consumed by a prepared request, including
//! resolved helper stop metadata. No subsequent sidecar filename lookup is used.
#![allow(dead_code)]
use super::*;
use sha2::{Digest, Sha256};

impl GgufTokenizer {
    pub(crate) fn retained_bytes(&self) -> Option<usize> {
        let mut bytes = std::mem::size_of::<Self>();
        for token in &self.vocab {
            bytes = bytes.checked_add(token.capacity())?;
        }
        for (a, b) in &self.merge_pairs {
            bytes = bytes.checked_add(a.capacity())?.checked_add(b.capacity())?;
        }
        for token in self.token_to_id_map.keys() {
            bytes = bytes.checked_add(token.capacity())?;
        }
        for (token, _) in self.special_tokens.iter().chain(&self.token_to_id) {
            bytes = bytes.checked_add(token.capacity())?;
        }
        bytes = bytes
            .checked_add(self.pre_type.capacity())?
            .checked_add(
                self.vocab
                    .capacity()
                    .checked_mul(std::mem::size_of::<String>())?,
            )?
            .checked_add(
                self.merge_pairs
                    .capacity()
                    .checked_mul(std::mem::size_of::<(String, String)>())?,
            )?
            .checked_add(self.merge_rank_index.capacity().checked_mul(64)?)?
            .checked_add(self.token_to_id_map.capacity().checked_mul(128)?)?
            .checked_add(self.special_tokens.capacity().checked_mul(64)?)?
            .checked_add(self.token_to_id.capacity().checked_mul(64)?)?;
        Some(bytes)
    }

    /// Caller admits cold sort scratch explicitly. All runtime lookup tables are
    /// included, so a stale table cannot share a fingerprint with different
    /// tokenizer behavior after a caller mutates the public vocabulary fields.
    pub(crate) fn execution_digest(&self, scratch_budget: usize) -> Option<[u8; 32]> {
        let scratch = self
            .merge_rank_index
            .len()
            .checked_mul(std::mem::size_of::<(&u64, &usize)>())?
            .checked_add(
                self.token_to_id_map
                    .len()
                    .checked_mul(std::mem::size_of::<(&String, &u32)>())?,
            )?;
        if scratch > scratch_budget {
            return None;
        }
        let mut hash = Sha256::new();
        hash.update(b"qualia:prepared-tokenizer:v1\0");
        hash.update(self.bos_token_id.to_le_bytes());
        hash.update(self.eos_token_id.to_le_bytes());
        hash.update([
            u8::from(self.add_bos_token),
            u8::from(self.merge_rank_collision),
            self.stop_token_count,
        ]);
        text(&mut hash, &self.pre_type);
        hash.update((self.vocab.len() as u64).to_le_bytes());
        for token in &self.vocab {
            text(&mut hash, token);
        }
        hash.update((self.merge_pairs.len() as u64).to_le_bytes());
        for (left, right) in &self.merge_pairs {
            text(&mut hash, left);
            text(&mut hash, right);
        }
        let mut ranks: Vec<_> = self.merge_rank_index.iter().collect();
        ranks.sort_unstable_by_key(|(key, _)| **key);
        hash.update((ranks.len() as u64).to_le_bytes());
        for (key, rank) in ranks {
            hash.update(key.to_le_bytes());
            hash.update((*rank as u64).to_le_bytes());
        }
        let mut lookup: Vec<_> = self.token_to_id_map.iter().collect();
        lookup.sort_unstable_by(|a, b| a.0.cmp(b.0));
        hash.update((lookup.len() as u64).to_le_bytes());
        for (token, id) in lookup {
            text(&mut hash, token);
            hash.update(id.to_le_bytes());
        }
        for table in [&self.special_tokens, &self.token_to_id] {
            hash.update((table.len() as u64).to_le_bytes());
            for (token, id) in table {
                text(&mut hash, token);
                hash.update(id.to_le_bytes());
            }
        }
        for id in self.stop_token_ids {
            hash.update(id.to_le_bytes());
        }
        Some(hash.finalize().into())
    }
}

fn text(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}
