//! Unit tests for joint memory budget arithmetic and admission.

#[cfg(test)]
mod tests {
    use super::super::model::ModelMemoryProfile;
    use super::super::pools::{BlockGeometry, MemoryPoolBudget};
    use super::super::BudgetError;

    fn fixture_budget(
        total_device_bytes: u64,
        static_bytes: u64,
        tokens_per_block: u32,
        bytes_per_block: u32,
        max_blocks: u32,
        max_cow_blocks: u32,
    ) -> MemoryPoolBudget {
        let profile = ModelMemoryProfile::new(
            static_bytes / 4,
            static_bytes / 4,
            static_bytes / 4,
            static_bytes / 4,
        )
        .expect("profile");
        let geometry = BlockGeometry::new(tokens_per_block, bytes_per_block).expect("geometry");
        MemoryPoolBudget::new(
            total_device_bytes,
            profile,
            geometry,
            max_blocks,
            max_cow_blocks,
        )
        .expect("pool budget")
    }

    #[test]
    fn test_exact_fit_admission() {
        // 1000 bytes total, 400 static -> 600 available KV
        // Block: 16 tokens, 100 bytes. Max 6 blocks (600 bytes).
        let budget = fixture_budget(1000, 400, 16, 100, 6, 2);
        assert_eq!(budget.available_kv_bytes(), 600);

        // 6 blocks * 100 = 600 bytes exactly
        // 16 * 6 = 96 tokens
        let res = budget.reserve(64, 32, 0, 0);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.required_blocks, 6);
        assert_eq!(res.reserved_bytes, 600);
    }

    #[test]
    fn test_one_byte_over_rejected() {
        // 1000 bytes total, 400 static -> 600 available KV
        // Block: 16 tokens, 100 bytes. Max 6 blocks.
        let budget = fixture_budget(1000, 400, 16, 100, 6, 2);

        // 5 blocks = 500 bytes. If already 101 bytes active, 500 + 101 = 601 > 600.
        let err = budget.reserve(80, 0, 0, 101);
        assert!(matches!(
            err,
            Err(BudgetError::InsufficientBudget {
                required_bytes: 601,
                available_bytes: 600
            })
        ));
    }

    #[test]
    fn test_integer_overflow_protection() {
        let budget = fixture_budget(1000, 400, 16, 100, 6, 2);

        // Token addition overflow
        let err = budget.reserve(u32::MAX, 1, 0, 0);
        assert!(matches!(err, Err(BudgetError::IntegerOverflow)));

        // Active bytes addition overflow
        let err = budget.reserve(16, 0, 0, u64::MAX);
        assert!(matches!(err, Err(BudgetError::IntegerOverflow)));
    }

    #[test]
    fn test_fragmented_page_ceiling_rounding() {
        let geometry = BlockGeometry::new(16, 128).unwrap();
        assert_eq!(geometry.blocks_for_tokens(0).unwrap(), 0);
        assert_eq!(geometry.blocks_for_tokens(1).unwrap(), 1);
        assert_eq!(geometry.blocks_for_tokens(15).unwrap(), 1);
        assert_eq!(geometry.blocks_for_tokens(16).unwrap(), 1);
        assert_eq!(geometry.blocks_for_tokens(17).unwrap(), 2);
        assert_eq!(geometry.blocks_for_tokens(32).unwrap(), 2);
        assert_eq!(geometry.blocks_for_tokens(33).unwrap(), 3);
    }

    #[test]
    fn test_cow_transient_headroom_reservation() {
        let budget = fixture_budget(1000, 400, 16, 100, 6, 2);

        // Requesting 3 COW blocks when max is 2 -> rejected
        let err = budget.reserve(16, 0, 3, 0);
        assert!(matches!(
            err,
            Err(BudgetError::InsufficientCowHeadroom {
                requested_blocks: 3,
                max_cow_blocks: 2
            })
        ));

        // Requesting 2 COW blocks when max is 2 -> accepted if blocks fit
        let res = budget.reserve(16, 0, 2, 0);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.required_blocks, 1);
        assert_eq!(res.cow_transient_blocks, 2);
        assert_eq!(res.reserved_bytes, 300); // (1 + 2) * 100
    }

    #[test]
    fn test_graph_capture_headroom_reduces_kv_capacity() {
        let profile_no_graph = ModelMemoryProfile::new(200, 100, 100, 0).unwrap();
        assert_eq!(profile_no_graph.static_device_bytes(), 400);

        let profile_with_graph = ModelMemoryProfile::new(200, 100, 100, 250).unwrap();
        assert_eq!(profile_with_graph.static_device_bytes(), 650);

        let geom = BlockGeometry::new(16, 50).unwrap();
        let b1 = MemoryPoolBudget::new(1000, profile_no_graph, geom, 10, 2).unwrap();
        let b2 = MemoryPoolBudget::new(1000, profile_with_graph, geom, 7, 2).unwrap();

        assert_eq!(b1.available_kv_bytes(), 600);
        assert_eq!(b2.available_kv_bytes(), 350);

        // 8 blocks = 400 bytes, 650 + 400 = 1050 > 1000 -> rejected at pool creation
        let b_over = MemoryPoolBudget::new(1000, profile_with_graph, geom, 8, 2);
        assert!(matches!(
            b_over,
            Err(BudgetError::InsufficientBudget {
                required_bytes: 1050,
                available_bytes: 1000
            })
        ));
    }

    #[test]
    fn test_physical_block_pool_exhaustion() {
        let budget = fixture_budget(10000, 400, 16, 100, 5, 2);

        // Even though bytes would fit (6 * 100 = 600 < 9600), max_blocks is 5.
        // 96 tokens = 6 blocks.
        let err = budget.reserve(96, 0, 0, 0);
        assert!(matches!(
            err,
            Err(BudgetError::PoolExhausted {
                requested_blocks: 6,
                max_blocks: 5
            })
        ));
    }
}
