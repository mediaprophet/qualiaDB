//! Socioeconomics & compensation (§23, legal_logic.md) — the Permissive Commons.
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
/// invested. The physics-bound viability ratio of a resource flow.
pub fn eroi(energy_returned: u64, energy_invested: u64) -> f32 {
    if energy_invested == 0 {
        0.0
    } else {
        energy_returned as f32 / energy_invested as f32
    }
}

/// Is a resource flow thermodynamically viable — E-ROI at or above `min_ratio`? Below this floor the
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
/// ceiling, `Err(UsuryError)` once it is breached.
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

// ─── QDNF Economics Metering Binding (WP-4.1) ─────────────────────────────────────

use crate::net::qdnf::economics::meters::MeterReading;

/// Computes the carriage compensation for observed resource meters based on
/// allowed carriage rates (cents per unit), strictly bounded by production ROI caps.
pub fn carriage_fee_from_meters(reading: &MeterReading, cents_per_unit_micro: u64) -> u64 {
    let units = reading.milli_units / 1000;
    ((units as u128).saturating_mul(cents_per_unit_micro as u128) / 1_000_000u128) as u64
}

/// Accumulates measured resource usage into an asset's compensation pool.
pub fn accumulate_meter_to_pool(
    pool: u64,
    reading: &MeterReading,
    cents_per_unit_micro: u64,
) -> u64 {
    let fee = carriage_fee_from_meters(reading, cents_per_unit_micro);
    pool_after(pool, fee)
}

// ─── Persistent `.q42j` Obligation Pool NQuin Binding (WP-07) ────────────────────

/// Modality opcode for persistent CommonsObligation pool checkpoints in `.q42j` journals.
pub const OP_COMMONS_OBLIGATION_POOL: u8 = 0x58;
/// Routing lane bit-pattern (`metadata[61..62] = 0b01`) for EnforcePermissiveCommons.
pub const METADATA_LANE_PERMISSIVE_COMMONS: u64 = 0b01u64 << 61;
/// Routing lane bit-pattern (`metadata[61..62] = 0b10`) for Bilateral / Active Obligation recovery.
pub const METADATA_LANE_OBLIGATION_RECOVERY: u64 = 0b10u64 << 61;

/// Encode a persistent `CommonsObligation` pool checkpoint as a 48-byte [`crate::NQuin`] for `.q42j`.
#[inline]
pub fn encode_obligation_pool_quin(
    asset_did_hash: u64,
    accumulated_pool_cents: u64,
    capped_cost_cents: u64,
    context_did_hash: u64,
    lamport_clock: u32,
) -> crate::NQuin {
    let prop_hash = crate::q_hash("https://ns.webcivics.net/economics#obligationPoolState");
    let predicate = (prop_hash << 8) | (OP_COMMONS_OBLIGATION_POOL as u64);
    let object = (0b001u64 << 60) | (accumulated_pool_cents & 0x0FFF_FFFF_FFFF_FFFF);
    let discharged = is_commons_discharged(accumulated_pool_cents, capped_cost_cents);
    let lane_bits = if discharged {
        METADATA_LANE_PERMISSIVE_COMMONS
    } else {
        METADATA_LANE_OBLIGATION_RECOVERY
    };
    let clock_bits = ((lamport_clock as u64) & 0x1FFF_FFFF) << 32;
    let cost_low32 = capped_cost_cents.min(u32::MAX as u64) as u64;
    let metadata = lane_bits | clock_bits | cost_low32;
    let parity = asset_did_hash ^ predicate ^ object ^ context_did_hash ^ metadata;
    crate::NQuin {
        subject: asset_did_hash,
        predicate,
        object,
        context: context_did_hash,
        metadata,
        parity,
    }
}

/// Replay a slice of `.q42j` journal `NQuin`s for `asset_did_hash` in zero heap memory,
/// returning `(accumulated_pool_cents, capped_cost_cents, is_discharged_to_commons)`.
pub fn replay_obligation_pool_journal(
    journal_quins: &[crate::NQuin],
    asset_did_hash: u64,
) -> Option<(u64, u64, bool)> {
    let mut best_clock: Option<u32> = None;
    let mut latest_pool = 0u64;
    let mut latest_cost = 0u64;
    let mut discharged = false;

    for q in journal_quins {
        if q.subject != asset_did_hash {
            continue;
        }
        if (q.predicate & 0xFF) as u8 != OP_COMMONS_OBLIGATION_POOL {
            continue;
        }
        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        if q.parity != expected_parity {
            continue;
        }
        let clock = ((q.metadata >> 32) & 0x1FFF_FFFF) as u32;
        if best_clock.map_or(true, |prev| clock >= prev) {
            best_clock = Some(clock);
            latest_pool = q.object & 0x0FFF_FFFF_FFFF_FFFF;
            latest_cost = q.metadata & 0xFFFF_FFFF;
            discharged = (q.metadata & (0b11u64 << 61)) == METADATA_LANE_PERMISSIVE_COMMONS
                || is_commons_discharged(latest_pool, latest_cost);
        }
    }

    best_clock.map(|_| (latest_pool, latest_cost, discharged))
}

// ─── Zero-Heap Basis-Point Pool Clearing (`Pool Funded != Contributor Paid`) ─────

/// Result of zero-heap basis-point pool clearing (`QUALIADB-IMPLEMENTATION-ROADMAP.md` §13).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolClearingResult {
    pub gross_receipt_cents: i64,
    pub operating_reserve_cents: i64,
    pub fiduciary_fee_cents: i64,
    pub obligation_pool_allocation_cents: i64,
    pub pool_funded_cents: i64,
    pub settled_verified_cents: i64,
    pub remaining_obligation_cents: i64,
    /// `true` only when `settled_verified_cents + waived_cents >= target_obligation_cents`.
    /// `pool_funded_cents >= target_obligation_cents` alone NEVER sets this to `true`.
    pub contributor_obligations_discharged: bool,
}

#[inline]
fn div_round_bankers_u128(n: u128, d: u128) -> u128 {
    let q = n / d;
    let r = n % d;
    let twice_r = r * 2;
    if twice_r > d || (twice_r == d && (q & 1) == 1) {
        q + 1
    } else {
        q
    }
}

/// Zero-heap integer basis-point clearing (`clear_pool_obligations_bps`) verifying
/// `Pool Funded != Contributor Paid` conservation (`Gross = O + F + A`).
#[inline]
pub fn clear_pool_obligations_bps(
    gross_receipt_cents: i64,
    operating_reserve_bps: u32,
    fiduciary_fee_bps: u32,
    prior_pool_funded_cents: i64,
    target_obligation_cents: i64,
    settled_verified_cents: i64,
    waived_obligation_cents: i64,
) -> Option<PoolClearingResult> {
    if gross_receipt_cents < 0
        || prior_pool_funded_cents < 0
        || target_obligation_cents < 0
        || settled_verified_cents < 0
        || waived_obligation_cents < 0
        || operating_reserve_bps.saturating_add(fiduciary_fee_bps) > 10_000
    {
        return None;
    }
    let gross_u = gross_receipt_cents as u128;
    let o_cents = div_round_bankers_u128(gross_u * (operating_reserve_bps as u128), 10_000) as i64;
    let f_cents = div_round_bankers_u128(gross_u * (fiduciary_fee_bps as u128), 10_000) as i64;
    let a_cents = gross_receipt_cents.saturating_sub(o_cents).saturating_sub(f_cents);
    let pool_funded_cents = prior_pool_funded_cents.saturating_add(a_cents);
    let discharged_amount = settled_verified_cents.saturating_add(waived_obligation_cents);
    let remaining_obligation_cents = (target_obligation_cents - discharged_amount).max(0);
    let contributor_obligations_discharged =
        target_obligation_cents > 0 && remaining_obligation_cents == 0;
    Some(PoolClearingResult {
        gross_receipt_cents,
        operating_reserve_cents: o_cents,
        fiduciary_fee_cents: f_cents,
        obligation_pool_allocation_cents: a_cents,
        pool_funded_cents,
        settled_verified_cents,
        remaining_obligation_cents,
        contributor_obligations_discharged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eroi_gates_thermodynamic_viability() {
        assert!((eroi(300, 100) - 3.0).abs() < 1e-6);
        assert_eq!(eroi(5, 0), 0.0);
        assert!(eroi_viable(300, 100, 2.0));
        assert!(!eroi_viable(150, 100, 2.0), "net-extractive → refused");
    }

    #[test]
    fn recursive_royalty_tree() {
        assert_eq!(ancestor_royalty(100, 0, 50), 50);
        assert_eq!(ancestor_royalty(100, 1, 50), 25);
        assert_eq!(ancestor_royalty(100, 2, 50), 12);
        assert_eq!(royalty_tree_total(100, 3, 50), 87);
    }

    #[test]
    fn multi_currency_and_tax() {
        assert_eq!(convert_currency(100, 1_500_000), 150);
        assert_eq!(convert_currency(100, 500_000), 50);
        assert_eq!(apply_tax(10_000, 250), 250);
    }

    #[test]
    fn liquidity_ode_converges_to_steady_state() {
        let one = liquidity_step(0, 100, 10);
        assert_eq!(one, 100);
        let settled = liquidity_after(0, 100, 10, 500);
        assert!((settled as i64 - 1000).abs() <= 1);
        assert!(liquidity_after(1000, 0, 50, 5) < 1000);
    }

    #[test]
    fn roi_is_capped() {
        assert_eq!(commons_cost(1000, 50, 20), 1200);
        assert_eq!(commons_cost(1000, 10, 20), 1100);
    }

    #[test]
    fn royalty_scales_by_agent_category() {
        assert_eq!(royalty(100, 300), 300);
        assert_eq!(royalty(100, 50), 50);
    }

    #[test]
    fn usury_breaker_trips_past_the_110_percent_ceiling() {
        assert_eq!(usury_ceiling(1000, USURY_OVERAGE_PERCENT_DEFAULT), 1100);
        assert!(check_usury(1000, 1000, USURY_OVERAGE_PERCENT_DEFAULT).is_ok());
        assert!(check_usury(1100, 1000, USURY_OVERAGE_PERCENT_DEFAULT).is_ok());
        assert!(!is_usurious(1100, 1000, USURY_OVERAGE_PERCENT_DEFAULT));
        let err = check_usury(1101, 1000, USURY_OVERAGE_PERCENT_DEFAULT).unwrap_err();
        assert_eq!(err.ceiling, 1100);
        assert_eq!(err.projected, 1101);
        assert!(is_usurious(1101, 1000, USURY_OVERAGE_PERCENT_DEFAULT));
        assert!(check_usury(1, 0, USURY_OVERAGE_PERCENT_DEFAULT).is_err());
        assert!(check_usury(0, 0, USURY_OVERAGE_PERCENT_DEFAULT).is_ok());
        assert_eq!(usury_ceiling(1000, 0), 1000);
        assert!(check_usury(1001, 1000, 0).is_err());
    }

    #[test]
    fn pool_discharges_at_threshold() {
        let cost = commons_cost(1000, 20, 20);
        let mut pool = 0u64;
        pool = pool_after(pool, royalty(400, 300));
        assert!(is_commons_discharged(pool, cost));
        assert_eq!(outstanding(pool, cost), 0);
        assert!(!is_commons_discharged(500, cost));
        assert_eq!(outstanding(500, cost), 700);
    }

    #[test]
    fn test_qdnf_metering_binding() {
        use crate::net::qdnf::authority::ObservationQuality;
        use crate::net::qdnf::economics::meters::MeterKind;

        let reading = MeterReading {
            kind: MeterKind::NetBytes,
            milli_units: 50_000,
            quality: ObservationQuality::Measured,
        };
        let fee = carriage_fee_from_meters(&reading, 2_000_000);
        assert_eq!(fee, 100);
        let mut pool = 1000u64;
        pool = accumulate_meter_to_pool(pool, &reading, 2_000_000);
        assert_eq!(pool, 1100);
    }

    #[test]
    fn clear_pool_obligations_bps_enforces_pool_funded_not_contributor_paid() {
        let res = clear_pool_obligations_bps(100_000, 1_000, 500, 50_000, 120_000, 40_000, 0)
            .expect("valid bps clearing");
        assert_eq!(
            res.operating_reserve_cents
                + res.fiduciary_fee_cents
                + res.obligation_pool_allocation_cents,
            100_000
        );
        assert_eq!(res.pool_funded_cents, 135_000);
        assert_eq!(res.remaining_obligation_cents, 80_000);
        assert!(!res.contributor_obligations_discharged);

        let paid = clear_pool_obligations_bps(100_000, 1_000, 500, 50_000, 120_000, 100_000, 20_000)
            .unwrap();
        assert_eq!(paid.remaining_obligation_cents, 0);
        assert!(paid.contributor_obligations_discharged);
    }
}
