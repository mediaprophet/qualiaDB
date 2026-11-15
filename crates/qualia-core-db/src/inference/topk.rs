pub use etalon_inference::projections::topk::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_topk_orders_and_breaks_ties() {
        // Two 5.0s at ids 1 and 4 → lower id (1) wins the tie.
        let logits = [1.0, 5.0, 3.0, 2.0, 5.0, f32::NAN, -1.0];
        let top = topk_cpu(&logits, 3);
        assert_eq!(
            top[0],
            TopKItem {
                token_id: 1,
                logit: 5.0
            }
        );
        assert_eq!(
            top[1],
            TopKItem {
                token_id: 4,
                logit: 5.0
            }
        );
        assert_eq!(
            top[2],
            TopKItem {
                token_id: 2,
                logit: 3.0
            }
        );
    }

    #[test]
    fn cpu_topk_k1_is_argmax() {
        let logits = [0.1, -2.0, 9.9, 9.9, 1.0];
        let top = topk_cpu(&logits, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].token_id, 2); // first of the tied maxima
    }

    #[test]
    fn merge_drops_masked_and_neg_inf() {
        // Simulate 2 blocks × k=2 candidates.
        let cand_val = [9.0, 7.0, 8.0, f32::NEG_INFINITY];
        let cand_idx = [3u32, 10, 42, 99];
        // Mask id 3 (a governance veto) → next-best survives.
        let masked = |id: u32| id == 3;
        let merged = merge_block_candidates(&cand_val, &cand_idx, 2, Some(&masked));
        assert_eq!(
            merged[0],
            TopKItem {
                token_id: 42,
                logit: 8.0
            }
        );
        assert_eq!(
            merged[1],
            TopKItem {
                token_id: 10,
                logit: 7.0
            }
        );
    }
}
