pub use etalon_inference::runtime::kv::prefix::radix::*;

#[cfg(test)]
use super::super::paged::{BlockPool, SequenceBlockTable};

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
