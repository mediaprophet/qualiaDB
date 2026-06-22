//! Value-flow & compensation (§23, legal_logic.md) — the Permissive Commons.
//!
//! Shifts economic obligation from infinite linear consumption to **threshold-based discharge**
//! to prevent extraction: a work's cost is its audited production cost plus a *legally capped*
//! ROI; usage by an agent triggers a royalty scaled by the agent's category; payments accumulate
//! into a pool; once the pool meets the cost, the obligation is **discharged and the use is freed
//! globally**. Integer arithmetic throughout (deterministic, zero-heap) — units are abstract
//! minor units (e.g. cents).

/// Total economic obligation for a work: `production_cost × (1 + roi_cap)`, with the ROI margin
/// **capped** (the `sh:maxInclusive` cap — extraction guard). `roi_cap_percent` is clamped to
/// `max_roi_percent` before applying. Saturating.
pub fn commons_cost(production_cost: u64, roi_cap_percent: u64, max_roi_percent: u64) -> u64 {
    let roi = roi_cap_percent.min(max_roi_percent);
    let margin = production_cost.saturating_mul(roi) / 100;
    production_cost.saturating_add(margin)
}

/// The royalty a use incurs: `base × multiplier%`, where the multiplier scales by agent
/// category (e.g. a corporate user pays a higher multiple than a non-profit). Saturating.
#[inline]
pub fn royalty(base: u64, agent_multiplier_percent: u64) -> u64 {
    base.saturating_mul(agent_multiplier_percent) / 100
}

/// Accumulate a payment into the compensation pool (saturating).
#[inline]
pub fn pool_after(pool: u64, payment: u64) -> u64 {
    pool.saturating_add(payment)
}

/// The obligation is **discharged** once accumulated compensation meets the cost — the use is
/// then obligation-free (freed globally). `Active(Outstanding) → Discharged(ObligationFree)`.
#[inline]
pub fn is_commons_discharged(pool: u64, cost: u64) -> bool {
    pool >= cost && cost > 0
}

/// Outstanding balance still owed before discharge (0 once met). Saturating.
#[inline]
pub fn outstanding(pool: u64, cost: u64) -> u64 {
    cost.saturating_sub(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roi_is_capped() {
        // 1000 cost, asked-for 50% ROI but cap is 20% → cost = 1000 + 200 = 1200.
        assert_eq!(commons_cost(1000, 50, 20), 1200);
        // Within cap → applied as-is.
        assert_eq!(commons_cost(1000, 10, 20), 1100);
    }

    #[test]
    fn royalty_scales_by_agent_category() {
        // corporate 300% vs non-profit 50% of the same base.
        assert_eq!(royalty(100, 300), 300);
        assert_eq!(royalty(100, 50), 50);
    }

    #[test]
    fn pool_discharges_at_threshold() {
        let cost = commons_cost(1000, 20, 20); // 1200
        let mut pool = 0u64;
        pool = pool_after(pool, royalty(400, 300)); // corporate use: 1200
        assert!(is_commons_discharged(pool, cost), "pool met cost → discharged + freed globally");
        assert_eq!(outstanding(pool, cost), 0);
        // Before that payment the obligation was outstanding.
        assert!(!is_commons_discharged(500, cost));
        assert_eq!(outstanding(500, cost), 700);
    }
}
