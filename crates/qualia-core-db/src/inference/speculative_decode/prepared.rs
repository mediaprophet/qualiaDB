//! Private evolving grammar with a frozen initial grammar/trie identity.
#![allow(dead_code)]
use super::*;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub struct DominoPreparationBudget {
    pub owned_bytes: usize,
    pub text_bytes: usize,
    pub stack_depth: usize,
}
pub struct PreparedDomino {
    masker: DominoMasker,
    digest: [u8; 32],
    text_limit: usize,
    stack_limit: usize,
    valid: bool,
    admitted_text: usize,
    admitted_stack: usize,
}
impl DominoMasker {
    /// Cold clone admits every trie node, map/vector capacity and grammar buffer
    /// before cloning. Trie depth is structurally capped at MAX_TRIE_DEPTH.
    pub fn prepare_owned(
        &self,
        budget: DominoPreparationBudget,
    ) -> Result<PreparedDomino, &'static str> {
        let mut charged = std::mem::size_of::<Self>();
        let mut hash = Sha256::new();
        hash.update(b"qualia:prepared-domino-trie:v1\0");
        hash.update((self.trie.vocab_size as u64).to_le_bytes());
        hash.update([u8::from(self.active)]);
        let mut stack: [Option<(&TrieNode, u16)>; MAX_TRIE_DEPTH + 1] = [None; MAX_TRIE_DEPTH + 1];
        stack[0] = Some((&self.trie.root, 0));
        let mut depth = 0usize;
        loop {
            let (node, next) = stack[depth].ok_or("invalid DOMINO traversal")?;
            if next == 0 {
                charged = charged
                    .checked_add(
                        node.children
                            .capacity()
                            .checked_mul(std::mem::size_of::<(u8, TrieNode)>() * 2)
                            .ok_or("DOMINO budget overflow")?,
                    )
                    .and_then(|value| value.checked_add(node.token_ids.capacity().checked_mul(4)?))
                    .ok_or("DOMINO budget overflow")?;
                if charged > budget.owned_bytes {
                    return Err("DOMINO owned budget exceeded");
                }
                hash.update((node.token_ids.len() as u64).to_le_bytes());
                for token in &node.token_ids {
                    hash.update(token.to_le_bytes());
                }
            }
            if let Some((key, child)) = (next..256)
                .find_map(|key| node.children.get(&(key as u8)).map(|child| (key, child)))
            {
                if depth == MAX_TRIE_DEPTH {
                    return Err("DOMINO trie depth exceeds contract");
                }
                hash.update([1, key as u8]);
                stack[depth] = Some((node, key + 1));
                depth += 1;
                stack[depth] = Some((child, 0));
            } else {
                hash.update([0]);
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
        }
        let scratch = budget
            .text_bytes
            .checked_add(
                budget
                    .stack_depth
                    .checked_mul(std::mem::size_of::<GrammarState>())
                    .ok_or("DOMINO budget overflow")?,
            )
            .and_then(|value| value.checked_add(self.trie.vocab_size.checked_mul(4)?))
            .ok_or("DOMINO budget overflow")?;
        // Clone scratch/transient metadata is charged too, before allocation.
        if charged
            .checked_add(scratch)
            .and_then(|value| value.checked_add(self.grammar.text.capacity()))
            .and_then(|value| {
                value.checked_add(
                    self.grammar
                        .stack
                        .capacity()
                        .checked_mul(std::mem::size_of::<GrammarState>())?,
                )
            })
            .and_then(|value| value.checked_add(self.valid_buf.capacity().checked_mul(4)?))
            .is_none_or(|value| value > budget.owned_bytes)
        {
            return Err("DOMINO preparation budget exceeded");
        }
        let mut masker = self.clone();
        masker.reset();
        masker
            .grammar
            .text
            .reserve_exact(budget.text_bytes.saturating_sub(masker.grammar.text.len()));
        masker.grammar.stack.reserve_exact(
            budget
                .stack_depth
                .saturating_sub(masker.grammar.stack.len()),
        );
        masker.valid_buf.reserve_exact(
            masker
                .trie
                .vocab_size
                .saturating_sub(masker.valid_buf.len()),
        );
        // Initial grammar is reset by the actual prepared object before hashing.
        hash.update((masker.grammar.state as u32).to_le_bytes());
        hash.update((masker.grammar.whitespace_parent as u32).to_le_bytes());
        hash.update((budget.text_bytes as u64).to_le_bytes());
        hash.update((budget.stack_depth as u64).to_le_bytes());
        Ok(PreparedDomino {
            masker,
            digest: hash.finalize().into(),
            text_limit: budget.text_bytes,
            stack_limit: budget.stack_depth,
            valid: true,
            admitted_text: 0,
            admitted_stack: 0,
        })
    }
}
impl PreparedDomino {
    /// Reserve before publication, including display UTF-8 replacement and the
    /// legacy byte-to-char grammar expansion. Reservations are cumulative over
    /// speculative batches whose feeds run after their decisions.
    pub(crate) fn admit_raw_piece(&mut self, bytes: usize) -> bool {
        if !self.masker.active {
            return self.valid;
        }
        let text = self.admitted_text.checked_add(bytes.saturating_mul(6));
        let stack = self.admitted_stack.checked_add(bytes.saturating_mul(3));
        if text.is_none_or(|n| n > self.text_limit) || stack.is_none_or(|n| n > self.stack_limit) {
            self.valid = false;
            return false;
        }
        self.admitted_text = text.unwrap();
        self.admitted_stack = stack.unwrap();
        self.valid
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn valid(&self) -> bool {
        self.valid
    }
    pub(crate) fn active(&self) -> bool {
        self.valid && self.masker.is_active()
    }
    pub(crate) fn sample(
        &mut self,
        sampler: &mut crate::sampler::SamplerState,
        logits: &mut [f32],
        ctx: &[u32],
    ) -> Option<u32> {
        if !self.active() {
            return None;
        }
        if logits.len() > self.masker.valid_buf.capacity() {
            self.valid = false;
            return None;
        }
        Some(sampler.sample_constrained(logits, ctx, &mut self.masker))
    }
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        if !self.valid {
            return;
        }
        let text_bytes = bytes
            .iter()
            .map(|&byte| (byte as char).len_utf8())
            .sum::<usize>();
        if self
            .masker
            .grammar
            .text
            .len()
            .checked_add(text_bytes)
            .is_none_or(|n| n > self.text_limit)
            || self
                .masker
                .grammar
                .stack
                .len()
                .checked_add(bytes.len())
                .is_none_or(|n| n > self.stack_limit)
        {
            self.valid = false;
            return;
        }
        self.masker.feed_token(bytes);
    }
    pub(crate) fn reset(&mut self) {
        self.masker.reset();
        self.admitted_text = 0;
        self.admitted_stack = 0;
    }
}
