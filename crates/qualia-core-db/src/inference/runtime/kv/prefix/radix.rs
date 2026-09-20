//! Iterative token-prefix radix index and page-table repointing (Work Package F5).
//!
//! Provides zero-heap longest prefix matching, partial-prompt hit detection,
//! OSRP final-token logit re-evaluation, deterministic LRU leaf eviction, and
//! canonical page table deduplication.

use super::super::paged::{BlockPool, PoolError, SequenceBlockTable};

pub const RADIX_PAGE_TOKENS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadixError {
    TreeFull,
    NodeNotFound,
    BufferTooSmall,
    InvalidTokenSequence,
    Pool(PoolError),
}

impl From<PoolError> for RadixError {
    fn from(err: PoolError) -> Self {
        Self::Pool(err)
    }
}

/// A node in the token-prefix radix tree representing one page block of tokens.
#[derive(Clone, Copy, Debug)]
pub struct RadixNode {
    pub tokens: [u32; RADIX_PAGE_TOKENS],
    pub token_count: u8,
    pub page_id: u32,
    pub parent: Option<u16>,
    pub first_child: Option<u16>,
    pub next_sibling: Option<u16>,
    pub last_access_epoch: u64,
    pub is_occupied: bool,
}

impl Default for RadixNode {
    fn default() -> Self {
        Self {
            tokens: [0; RADIX_PAGE_TOKENS],
            token_count: 0,
            page_id: u32::MAX,
            parent: None,
            first_child: None,
            next_sibling: None,
            last_access_epoch: 0,
            is_occupied: false,
        }
    }
}

/// Fixed-capacity, zero-heap Radix Prefix Index.
#[derive(Debug)]
pub struct RadixPrefixIndex<const MAX_NODES: usize> {
    nodes: [RadixNode; MAX_NODES],
    root: Option<u16>,
    node_count: usize,
}

impl<const MAX_NODES: usize> Default for RadixPrefixIndex<MAX_NODES> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_NODES: usize> RadixPrefixIndex<MAX_NODES> {
    pub const fn new() -> Self {
        Self {
            nodes: [RadixNode {
                tokens: [0; RADIX_PAGE_TOKENS],
                token_count: 0,
                page_id: u32::MAX,
                parent: None,
                first_child: None,
                next_sibling: None,
                last_access_epoch: 0,
                is_occupied: false,
            }; MAX_NODES],
            root: None,
            node_count: 0,
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Find the longest matching prefix for `prompt_tokens`.
    ///
    /// Applies OSRP rule: if all prompt tokens match, the last token is excluded
    /// from `reusable_tokens` to ensure decode computes fresh logits for generation.
    ///
    /// Writes physical page IDs into `out_pages`.
    /// Returns `(matched_tokens, reusable_tokens, page_count)`.
    pub fn match_longest_prefix(
        &mut self,
        prompt_tokens: &[u32],
        current_epoch: u64,
        out_pages: &mut [u32],
    ) -> Result<(u32, u32, usize), RadixError> {
        if prompt_tokens.is_empty() {
            return Ok((0, 0, 0));
        }

        let mut current_child = self.root;
        let mut prompt_offset = 0;
        let mut pages_matched = 0;

        while let Some(node_idx) = current_child {
            let node = &mut self.nodes[node_idx as usize];
            let count = node.token_count as usize;

            if prompt_offset + count <= prompt_tokens.len()
                && &prompt_tokens[prompt_offset..prompt_offset + count] == &node.tokens[..count]
            {
                if pages_matched >= out_pages.len() {
                    return Err(RadixError::BufferTooSmall);
                }
                out_pages[pages_matched] = node.page_id;
                pages_matched += 1;
                prompt_offset += count;
                node.last_access_epoch = current_epoch;
                current_child = node.first_child;
            } else {
                current_child = node.next_sibling;
            }
        }

        let matched_tokens = prompt_offset as u32;
        // OSRP rule: if full prompt matched, withhold final token for decode logits
        let reusable_tokens = if matched_tokens == prompt_tokens.len() as u32 && matched_tokens > 0
        {
            matched_tokens - 1
        } else {
            matched_tokens
        };

        Ok((matched_tokens, reusable_tokens, pages_matched))
    }

    /// Insert a page of tokens as a child of `parent` (or root if None).
    pub fn insert_child_page(
        &mut self,
        parent_idx: Option<u16>,
        tokens: &[u32],
        page_id: u32,
        current_epoch: u64,
    ) -> Result<u16, RadixError> {
        if tokens.is_empty() || tokens.len() > RADIX_PAGE_TOKENS {
            return Err(RadixError::InvalidTokenSequence);
        }

        // Allocate free slot
        let slot = self
            .nodes
            .iter()
            .position(|n| !n.is_occupied)
            .ok_or(RadixError::TreeFull)? as u16;

        let mut node = RadixNode::default();
        node.tokens[..tokens.len()].copy_from_slice(tokens);
        node.token_count = tokens.len() as u8;
        node.page_id = page_id;
        node.parent = parent_idx;
        node.last_access_epoch = current_epoch;
        node.is_occupied = true;

        match parent_idx {
            Some(p_idx) => {
                let parent = &mut self.nodes[p_idx as usize];
                node.next_sibling = parent.first_child;
                parent.first_child = Some(slot);
            }
            None => {
                node.next_sibling = self.root;
                self.root = Some(slot);
            }
        }

        self.nodes[slot as usize] = node;
        self.node_count += 1;
        Ok(slot)
    }

    /// Evict the least-recently-used leaf node and return its physical page to `pool`.
    pub fn evict_lru_leaf(&mut self, pool: &mut BlockPool) -> Result<Option<u32>, RadixError> {
        let mut lru_leaf: Option<usize> = None;
        let mut min_epoch = u64::MAX;

        for (idx, node) in self.nodes.iter().enumerate() {
            if node.is_occupied && node.first_child.is_none() && node.last_access_epoch < min_epoch
            {
                min_epoch = node.last_access_epoch;
                lru_leaf = Some(idx);
            }
        }

        let Some(leaf_idx) = lru_leaf else {
            return Ok(None);
        };

        let leaf = self.nodes[leaf_idx];
        let freed_page = leaf.page_id;

        // Unlink from parent / siblings
        if let Some(p_idx) = leaf.parent {
            let parent = &mut self.nodes[p_idx as usize];
            if parent.first_child == Some(leaf_idx as u16) {
                parent.first_child = leaf.next_sibling;
            } else {
                let mut sib = parent.first_child;
                while let Some(s_idx) = sib {
                    if self.nodes[s_idx as usize].next_sibling == Some(leaf_idx as u16) {
                        self.nodes[s_idx as usize].next_sibling = leaf.next_sibling;
                        break;
                    }
                    sib = self.nodes[s_idx as usize].next_sibling;
                }
            }
        } else if self.root == Some(leaf_idx as u16) {
            self.root = leaf.next_sibling;
        } else {
            let mut sib = self.root;
            while let Some(s_idx) = sib {
                if self.nodes[s_idx as usize].next_sibling == Some(leaf_idx as u16) {
                    self.nodes[s_idx as usize].next_sibling = leaf.next_sibling;
                    break;
                }
                sib = self.nodes[s_idx as usize].next_sibling;
            }
        }

        self.nodes[leaf_idx] = RadixNode::default();
        self.node_count -= 1;

        pool.release(freed_page)?;
        Ok(Some(freed_page))
    }

    /// Repoint occurrences of `duplicate_page` to `canonical_page` across live block tables.
    pub fn repoint_page_references(
        tables: &mut [&mut SequenceBlockTable],
        duplicate_page: u32,
        canonical_page: u32,
        pool: &mut BlockPool,
    ) -> Result<usize, RadixError> {
        let mut repointed = 0;
        for table in tables.iter_mut() {
            for entry in table.entries_mut() {
                if *entry == duplicate_page {
                    pool.retain(canonical_page)?;
                    pool.release(duplicate_page)?;
                    *entry = canonical_page;
                    repointed += 1;
                }
            }
        }
        Ok(repointed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_match_and_osrp_logit_withhold() {
        let mut index = RadixPrefixIndex::<16>::new();

        // Page 1: tokens 1..=16
        let p1_tokens: [u32; 16] = core::array::from_fn(|i| (i + 1) as u32);
        let n1 = index.insert_child_page(None, &p1_tokens, 100, 1).unwrap();

        // Page 2: tokens 17..=32
        let p2_tokens: [u32; 16] = core::array::from_fn(|i| (i + 17) as u32);
        index
            .insert_child_page(Some(n1), &p2_tokens, 200, 1)
            .unwrap();

        let mut out_pages = [0u32; 4];

        // 1. Partial query matching only Page 1 + 5 tokens of Page 2
        let mut query = Vec::new();
        query.extend_from_slice(&p1_tokens);
        query.extend_from_slice(&p2_tokens[..5]);

        let (matched, reusable, pages) = index
            .match_longest_prefix(&query, 2, &mut out_pages)
            .unwrap();
        // Page 1 matched completely; Page 2 was partial, so only Page 1 is matched
        assert_eq!(matched, 16);
        assert_eq!(reusable, 16);
        assert_eq!(pages, 1);
        assert_eq!(out_pages[0], 100);

        // 2. Exact match of all 32 tokens -> OSRP rule withholds 1 token (reusable = 31)
        let mut full_query = Vec::new();
        full_query.extend_from_slice(&p1_tokens);
        full_query.extend_from_slice(&p2_tokens);

        let (matched_full, reusable_full, pages_full) = index
            .match_longest_prefix(&full_query, 3, &mut out_pages)
            .unwrap();
        assert_eq!(matched_full, 32);
        assert_eq!(reusable_full, 31); // Withheld for decode logits!
        assert_eq!(pages_full, 2);
        assert_eq!(out_pages[0], 100);
        assert_eq!(out_pages[1], 200);
    }

    #[test]
    fn test_radix_lru_leaf_eviction() {
        let mut pool = BlockPool::new(8);
        let p0 = pool.allocate().unwrap();
        let p1 = pool.allocate().unwrap();

        let mut index = RadixPrefixIndex::<8>::new();
        let t1 = [1u32, 2, 3, 4];
        let t2 = [5u32, 6, 7, 8];

        let n1 = index.insert_child_page(None, &t1, p0, 10).unwrap();
        index.insert_child_page(Some(n1), &t2, p1, 5).unwrap();

        assert_eq!(index.node_count(), 2);
        // Leaf (p1, epoch 5) is evicted first
        let evicted = index.evict_lru_leaf(&mut pool).unwrap();
        assert_eq!(evicted, Some(p1));
        assert_eq!(index.node_count(), 1);

        // Now n1 (p0, epoch 10) is a leaf and can be evicted
        let evicted2 = index.evict_lru_leaf(&mut pool).unwrap();
        assert_eq!(evicted2, Some(p0));
        assert_eq!(index.node_count(), 0);
    }

    #[test]
    fn test_repoint_page_references() {
        let mut pool = BlockPool::new(8);
        let p_dup = pool.allocate().unwrap();
        let p_canon = pool.allocate().unwrap();

        let mut t1 = SequenceBlockTable::new(4);
        let mut t2 = SequenceBlockTable::new(4);
        t1.install_shared_prefix(&[p_dup], &mut pool).unwrap();
        t2.install_shared_prefix(&[p_dup], &mut pool).unwrap();

        let mut slice = [&mut t1, &mut t2];
        let repointed =
            RadixPrefixIndex::<8>::repoint_page_references(&mut slice, p_dup, p_canon, &mut pool)
                .unwrap();

        assert_eq!(repointed, 2);
        assert_eq!(t1.get(0), Some(p_canon));
        assert_eq!(t2.get(0), Some(p_canon));
    }
}
