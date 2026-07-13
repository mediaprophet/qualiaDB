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

// ─── Thermodynamic cost caps (E-ROI) ──────────────────────────────────────────────

/// **Energy Return On Investment** = `energy_returned / energy_invested`. `0.0` if nothing was
/// invested. The physics-bound viability ratio of a value flow.
pub fn eroi(energy_returned: u64, energy_invested: u64) -> f32 {
    if energy_invested == 0 {
        0.0
    } else {
        energy_returned as f32 / energy_invested as f32
    }
}

/// Is a value flow thermodynamically viable — E-ROI at or above `min_ratio`? Below this floor the
/// flow is net-extractive (spends more energy than it recovers) and is refused — the physics-bound
/// cost cap.
#[inline]
pub fn eroi_viable(energy_returned: u64, energy_invested: u64, min_ratio: f32) -> bool {
    eroi(energy_returned, energy_invested) >= min_ratio
}

// ─── Recursive royalty trees (derivative-work attribution) ────────────────────────

/// The royalty owed to an ancestor `generation` levels up a derivation chain: a geometric split
/// where each level takes `share_percent` of what reaches it —
/// `total × (share_percent/100)^(generation+1)`. Generation 0 = the immediate parent. Saturating
/// integer arithmetic; zero-heap. (Recursive commons attribution for derivative works.)
pub fn ancestor_royalty(total_royalty: u64, generation: u32, share_percent: u64) -> u64 {
    let mut amount = total_royalty;
    for _ in 0..=generation {
        amount = amount.saturating_mul(share_percent) / 100;
    }
    amount
}

/// Total royalty distributed up a chain of `generations` ancestors (the sum of each generation's
/// geometric share) — what leaves the deriving work as upstream attribution.
pub fn royalty_tree_total(total_royalty: u64, generations: u32, share_percent: u64) -> u64 {
    let mut sum = 0u64;
    for g in 0..generations {
        sum = sum.saturating_add(ancestor_royalty(total_royalty, g, share_percent));
    }
    sum
}

// ─── Multi-currency & cross-jurisdictional tax shunting ───────────────────────────

/// Convert `amount` at `rate_micros` (target units per source unit, in millionths — e.g.
/// `1_500_000` = ×1.5). Saturating (u128 intermediate).
pub fn convert_currency(amount: u64, rate_micros: u64) -> u64 {
    ((amount as u128).saturating_mul(rate_micros as u128) / 1_000_000u128) as u64
}

/// The tax owed on `amount` at `tax_basis_points` (1 bp = 0.01%) — the automated cross-
/// jurisdictional tax-schema shunt. Saturating (u128 intermediate).
pub fn apply_tax(amount: u64, tax_basis_points: u64) -> u64 {
    ((amount as u128).saturating_mul(tax_basis_points as u128) / 10_000u128) as u64
}

// ─── Liquidity-pool ODE (drainage + replenishment) ────────────────────────────────

/// One discrete Euler step of the liquidity ODE `dL/dt = inflow − drain·L`: constant `inflow`
/// replenishment minus drainage proportional to the current pool (`drain_percent`% per step).
/// Saturating; zero-heap. The steady state is `L* = inflow / drain`.
pub fn liquidity_step(pool: u64, inflow: u64, drain_percent: u64) -> u64 {
    let drained = pool.saturating_mul(drain_percent.min(100)) / 100;
    pool.saturating_sub(drained).saturating_add(inflow)
}

/// Evolve liquidity over `steps` of the ODE (constant `inflow`, proportional `drain_percent`) —
/// converges toward the steady state `inflow / (drain_percent/100)`.
pub fn liquidity_after(initial: u64, inflow: u64, drain_percent: u64, steps: u32) -> u64 {
    let mut pool = initial;
    for _ in 0..steps {
        pool = liquidity_step(pool, inflow, drain_percent);
    }
    pool
}

// ─── Usury circuit-breaker (multi-agent token ceiling) ────────────────────────────
//
// MULTI_AGENT_PROTOCOL.md's resource-governance guard: an agent's projected spend may
// run up to — but not past — an agreed budget plus a small overage; breaching the
// ceiling is *usurious* and the operation is refused (`ERROR_USURY_LIMIT_EXCEEDED`).
// A hard anti-extraction cap in the same family as the capped ROI and the E-ROI floor
// above, not a soft warning. Deterministic saturating integer arithmetic.

/// Default permitted overage above an agreed budget, in percent — the spec's **110%**
/// token ceiling is `budget × (1 + 10/100)`.
pub const USURY_OVERAGE_PERCENT_DEFAULT: u64 = 10;

/// Maximum spend permitted before the usury breaker trips: `budget × (1 + overage/100)`
/// (saturating). With the default 10% overage this is the 110% ceiling.
#[inline]
pub fn usury_ceiling(budget: u64, overage_percent: u64) -> u64 {
    let margin = budget.saturating_mul(overage_percent) / 100;
    budget.saturating_add(margin)
}

/// Has `projected_spend` breached the usury ceiling for `budget`? Spending *up to and
/// including* the ceiling is permitted; only a strictly greater spend is usurious.
#[inline]
pub fn is_usurious(projected_spend: u64, budget: u64, overage_percent: u64) -> bool {
    projected_spend > usury_ceiling(budget, overage_percent)
}

/// `ERROR_USURY_LIMIT_EXCEEDED` — a projected spend breached the agreed budget's
/// ceiling. Carries the `budget`, the computed `ceiling`, and the offending
/// `projected` spend so the caller can write a faithful conduct-violation record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsuryError {
    pub budget: u64,
    pub ceiling: u64,
    pub projected: u64,
}

/// Gate a projected spend against the usury ceiling — `Ok(())` while at/under the
/// ceiling, `Err(UsuryError)` once it is breached. The fiduciary circuit-breaker an
/// agent's resource declaration is checked through before the spend is admitted.
#[inline]
pub fn check_usury(
    projected_spend: u64,
    budget: u64,
    overage_percent: u64,
) -> Result<(), UsuryError> {
    let ceiling = usury_ceiling(budget, overage_percent);
    if projected_spend > ceiling {
        Err(UsuryError {
            budget,
            ceiling,
            projected: projected_spend,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eroi_gates_thermodynamic_viability() {
        assert!((eroi(300, 100) - 3.0).abs() < 1e-6);
        assert_eq!(eroi(5, 0), 0.0);
        // Viable iff E-ROI ≥ floor.
        assert!(eroi_viable(300, 100, 2.0));
        assert!(!eroi_viable(150, 100, 2.0), "net-extractive → refused");
    }

    #[test]
    fn recursive_royalty_tree() {
        // 50% per generation: parent gets 50, grandparent 25, great-grandparent 12.
        assert_eq!(ancestor_royalty(100, 0, 50), 50);
        assert_eq!(ancestor_royalty(100, 1, 50), 25);
        assert_eq!(ancestor_royalty(100, 2, 50), 12); // 100*.5*.5*.5 = 12.5 → 12 (integer)
                                                      // The chain total over 3 generations: 50 + 25 + 12 = 87.
        assert_eq!(royalty_tree_total(100, 3, 50), 87);
    }

    #[test]
    fn multi_currency_and_tax() {
        assert_eq!(convert_currency(100, 1_500_000), 150); // ×1.5
        assert_eq!(convert_currency(100, 500_000), 50); // ×0.5
        assert_eq!(apply_tax(10_000, 250), 250); // 2.5% of 10000
    }

    #[test]
    fn liquidity_ode_converges_to_steady_state() {
        // inflow 100, drain 10%/step → steady state L* = 100 / 0.10 = 1000.
        let one = liquidity_step(0, 100, 10);
        assert_eq!(one, 100); // 0 drained + 100 inflow
        let settled = liquidity_after(0, 100, 10, 500);
        assert!(
            (settled as i64 - 1000).abs() <= 1,
            "converges to inflow/drain = 1000, got {settled}"
        );
        // Draining a full pool with no inflow shrinks it.
        assert!(liquidity_after(1000, 0, 50, 5) < 1000);
    }

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
    fn usury_breaker_trips_past_the_110_percent_ceiling() {
        // A 1000-token budget admits spend up to the 110% ceiling (1100); past it is usurious.
        assert_eq!(usury_ceiling(1000, USURY_OVERAGE_PERCENT_DEFAULT), 1100);
        assert!(check_usury(1000, 1000, USURY_OVERAGE_PERCENT_DEFAULT).is_ok());
        assert!(
            check_usury(1100, 1000, USURY_OVERAGE_PERCENT_DEFAULT).is_ok(),
            "exactly at ceiling is permitted"
        );
        assert!(!is_usurious(1100, 1000, USURY_OVERAGE_PERCENT_DEFAULT));
        let err = check_usury(1101, 1000, USURY_OVERAGE_PERCENT_DEFAULT).unwrap_err();
        assert_eq!(err.ceiling, 1100);
        assert_eq!(err.projected, 1101);
        assert!(is_usurious(1101, 1000, USURY_OVERAGE_PERCENT_DEFAULT));
        // A zero budget permits no positive spend.
        assert!(check_usury(1, 0, USURY_OVERAGE_PERCENT_DEFAULT).is_err());
        assert!(check_usury(0, 0, USURY_OVERAGE_PERCENT_DEFAULT).is_ok());
        // The overage is a policy knob: a stricter 0% overage caps exactly at budget.
        assert_eq!(usury_ceiling(1000, 0), 1000);
        assert!(check_usury(1001, 1000, 0).is_err());
    }

    #[test]
    fn pool_discharges_at_threshold() {
        let cost = commons_cost(1000, 20, 20); // 1200
        let mut pool = 0u64;
        pool = pool_after(pool, royalty(400, 300)); // corporate use: 1200
        assert!(
            is_commons_discharged(pool, cost),
            "pool met cost → discharged + freed globally"
        );
        assert_eq!(outstanding(pool, cost), 0);
        // Before that payment the obligation was outstanding.
        assert!(!is_commons_discharged(500, cost));
        assert_eq!(outstanding(500, cost), 700);
    }
}
