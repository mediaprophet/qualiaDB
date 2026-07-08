//! Mechanism design: individual rationality, budget balance, VCG payments,
//! and strategy-proofness checks.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Quasilinear utility: `u_i = v_i(allocation) - payment_i`.
//! - Private values (each agent knows their own valuation).
//! - Risk-neutral agents.

/// Maximum agents in a bounded mechanism.
pub const MAX_AGENTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanismError {
    InvalidInput,
    NonFinite,
    BufferTooSmall,
    PropertyViolated,
}

/// Mechanism property report.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MechanismReport {
    pub individual_rationality: bool,
    pub budget_balance: bool,
    pub strategy_proof: bool,
    pub total_payment: f64,
    pub total_surplus: f64,
}

fn require_finite(x: f64) -> Result<(), MechanismError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(MechanismError::NonFinite)
    }
}

/// Check individual rationality: every agent's payment must be <= their
/// valuation (no agent loses by participating).
pub fn check_individual_rationality(
    valuations: &[f64],
    payments: &[f64],
) -> Result<bool, MechanismError> {
    if valuations.is_empty() || valuations.len() != payments.len() {
        return Err(MechanismError::InvalidInput);
    }
    if valuations.len() > MAX_AGENTS {
        return Err(MechanismError::BufferTooSmall);
    }
    for i in 0..valuations.len() {
        require_finite(valuations[i])?;
        require_finite(payments[i])?;
        if payments[i] > valuations[i] {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Check budget balance: returns `(is_balanced, net_transfer)`.
///
/// Convention: balanced if `sum(payments) == 0` (budget balanced); no deficit
/// if `sum(payments) >= 0`. Returns `is_balanced = (net_transfer >= 0.0)`.
pub fn check_budget_balance(
    payments: &[f64],
) -> Result<(bool, f64), MechanismError> {
    if payments.is_empty() {
        return Err(MechanismError::InvalidInput);
    }
    if payments.len() > MAX_AGENTS {
        return Err(MechanismError::BufferTooSmall);
    }
    let mut total = 0.0;
    for p in payments {
        require_finite(*p)?;
        total += p;
    }
    Ok((total >= -1e-10, total))
}

/// VCG (Clarke pivot) payment for single-item allocation to highest bidder.
///
/// The winner pays the second-highest valuation (Vickrey price). Writes
/// payments into `out[..n]` (0 for non-winners). Returns total revenue.
pub fn vickrey_clarke_groves_payment_into(
    valuations: &[f64],
    out: &mut [f64],
) -> Result<f64, MechanismError> {
    if valuations.is_empty() || out.len() < valuations.len() {
        return Err(MechanismError::BufferTooSmall);
    }
    if valuations.len() > MAX_AGENTS {
        return Err(MechanismError::BufferTooSmall);
    }
    for v in valuations {
        require_finite(*v)?;
        if *v < 0.0 {
            return Err(MechanismError::InvalidInput);
        }
    }
    let n = valuations.len();
    // Find winner (highest valuation, ties by lowest index).
    let mut winner = 0;
    let mut highest = valuations[0];
    for i in 1..n {
        if valuations[i] > highest {
            highest = valuations[i];
            winner = i;
        }
    }
    // Find second-highest.
    let mut second = 0.0;
    for i in 0..n {
        if i == winner {
            continue;
        }
        if valuations[i] > second {
            second = valuations[i];
        }
    }
    for i in 0..n {
        out[i] = if i == winner { second } else { 0.0 };
    }
    Ok(second)
}

/// Check strategy-proofness for a 2-agent, 2-type mechanism using precomputed
/// allocation and payment tables.
///
/// `valuation_matrix[agent][type]` = agent's valuation for their type.
/// `allocation_rule[type_i][type_j]` = true if agent 0 gets the item when
/// agent 0 reports `type_i` and agent 1 reports `type_j`.
/// `payment_rule[type_i][type_j]` = payment by agent 0.
///
/// Checks that truthful reporting weakly dominates misreporting for agent 0.
/// (Agent 1's check is symmetric and omitted for brevity; full check requires
/// both agents.)
pub fn check_strategy_proofness_2x2(
    valuation_matrix: &[f64], // 2x2: [agent][type]
    allocation_rule: &[bool], // 2x2: [type_i][type_j] → agent 0 gets item?
    payment_rule: &[f64],     // 2x2: [type_i][type_j] → payment by agent 0
) -> Result<bool, MechanismError> {
    if valuation_matrix.len() < 4 || allocation_rule.len() < 4 || payment_rule.len() < 4 {
        return Err(MechanismError::InvalidInput);
    }
    for v in valuation_matrix {
        require_finite(*v)?;
    }
    for p in payment_rule {
        require_finite(*p)?;
    }
    // For agent 0 with true type t, check:
    // utility(truthful) >= utility(misreport) for all opponent types.
    for true_type in 0..2 {
        for opponent_type in 0..2 {
            // Truthful utility.
            let truthful_alloc = allocation_rule[true_type * 2 + opponent_type];
            let truthful_payment = payment_rule[true_type * 2 + opponent_type];
            let truthful_utility = if truthful_alloc {
                valuation_matrix[0 * 2 + true_type] - truthful_payment
            } else {
                -truthful_payment
            };
            // Misreport utility (report the other type).
            let misreport_type = 1 - true_type;
            let misreport_alloc = allocation_rule[misreport_type * 2 + opponent_type];
            let misreport_payment = payment_rule[misreport_type * 2 + opponent_type];
            let misreport_utility = if misreport_alloc {
                valuation_matrix[0 * 2 + true_type] - misreport_payment
            } else {
                -misreport_payment
            };
            if misreport_utility > truthful_utility + 1e-10 {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Compute a mechanism report: IR, budget balance, total payment, and surplus.
///
/// `valuations` are the agents' actual valuations; `payments` are what they
/// pay. `allocations` (bool slice) indicates who received the item.
/// `strategy_proof` is set to `false` (not checkable generically here).
pub fn mechanism_report(
    valuations: &[f64],
    payments: &[f64],
    allocations: &[bool],
) -> Result<MechanismReport, MechanismError> {
    if valuations.is_empty() || valuations.len() != payments.len() || valuations.len() != allocations.len() {
        return Err(MechanismError::InvalidInput);
    }
    if valuations.len() > MAX_AGENTS {
        return Err(MechanismError::BufferTooSmall);
    }
    let ir = check_individual_rationality(valuations, payments)?;
    let (bb, total_payment) = check_budget_balance(payments)?;
    let mut total_surplus = 0.0;
    for i in 0..valuations.len() {
        if allocations[i] {
            total_surplus += valuations[i];
        }
        total_surplus -= payments[i];
    }
    Ok(MechanismReport {
        individual_rationality: ir,
        budget_balance: bb,
        strategy_proof: false,
        total_payment,
        total_surplus,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn ir_holds_when_payments_leq_valuations() {
        let v = [10.0, 20.0, 15.0];
        let p = [5.0, 15.0, 10.0];
        assert!(check_individual_rationality(&v, &p).unwrap());
    }

    #[test]
    fn ir_violated_when_payment_exceeds_valuation() {
        let v = [10.0, 20.0, 15.0];
        let p = [5.0, 25.0, 10.0]; // agent 1 pays 25 > 20
        assert!(!check_individual_rationality(&v, &p).unwrap());
    }

    #[test]
    fn budget_balance_balanced() {
        // Payments sum to 0 (one pays, one receives).
        let p = [10.0, -10.0];
        let (balanced, net) = check_budget_balance(&p).unwrap();
        assert!(balanced);
        assert!(approx(net, 0.0, 1e-9));
    }

    #[test]
    fn budget_balance_no_deficit() {
        let p = [15.0, 0.0, 0.0]; // sum = 15 >= 0
        let (balanced, net) = check_budget_balance(&p).unwrap();
        assert!(balanced);
        assert!(approx(net, 15.0, 1e-9));
    }

    #[test]
    fn budget_balance_deficit() {
        let p = [-5.0, -5.0]; // sum = -10 < 0
        let (balanced, net) = check_budget_balance(&p).unwrap();
        assert!(!balanced);
        assert!(approx(net, -10.0, 1e-9));
    }

    #[test]
    fn vcg_second_price() {
        // Bids [10, 20, 15] → winner = 1, payment = 15 (second highest)
        let v = [10.0, 20.0, 15.0];
        let mut p = [0.0f64; 3];
        let revenue = vickrey_clarke_groves_payment_into(&v, &mut p).unwrap();
        assert!(approx(p[1], 15.0, 1e-9));
        assert!(approx(p[0], 0.0, 1e-9));
        assert!(approx(p[2], 0.0, 1e-9));
        assert!(approx(revenue, 15.0, 1e-9));
    }

    #[test]
    fn vcg_two_bidders() {
        let v = [10.0, 20.0];
        let mut p = [0.0f64; 2];
        let revenue = vickrey_clarke_groves_payment_into(&v, &mut p).unwrap();
        assert!(approx(p[1], 10.0, 1e-9));
        assert!(approx(revenue, 10.0, 1e-9));
    }

    #[test]
    fn strategy_proof_vickrey() {
        // Vickrey (2nd-price) is strategy-proof.
        // Agent 0 valuations: type 0 → 10, type 1 → 20.
        // Allocation: highest bidder wins. Payment = second highest.
        // Agent 1 always has valuation 15.
        // type_i=0 (agent 0 reports 10): opponent=15 → agent 0 loses, payment=0
        // type_i=1 (agent 0 reports 20): opponent=15 → agent 0 wins, payment=15
        let val_matrix = [10.0, 20.0, 15.0, 15.0]; // [agent0_type0, agent0_type1, agent1_type0, agent1_type1]
        // allocation_rule[type_i][type_j] for agent 0:
        // [0][0]: report 10, opp 15 → lose → false
        // [0][1]: report 10, opp 15 → lose → false
        // [1][0]: report 20, opp 15 → win → true
        // [1][1]: report 20, opp 15 → win → true
        let alloc = [false, false, true, true];
        // payment_rule[type_i][type_j] for agent 0:
        // [0][*]: lose → 0
        // [1][*]: win → pay 15
        let payment = [0.0, 0.0, 15.0, 15.0];
        let sp = check_strategy_proofness_2x2(&val_matrix, &alloc, &payment).unwrap();
        assert!(sp, "Vickrey should be strategy-proof");
    }

    #[test]
    fn strategy_proof_first_price_not() {
        // First-price: winner pays their bid → not strategy-proof.
        // Construct a case where misreporting strictly helps:
        // Agent 0 type 0 (val 10), type 1 (val 20). Opponent always has val 5.
        let val_matrix = [10.0, 20.0, 5.0, 5.0];
        // type 0 (val 10): report 10 → win (opp 5) → pay 10, util = 0
        //   misreport type 1 (report 20): win → pay 20, util = -10. Worse.
        // type 1 (val 20): report 20 → win → pay 20, util = 0
        //   misreport type 0 (report 10): win → pay 10, util = 10. Better! → not SP.
        let alloc = [true, true, true, true]; // always wins (bid > 5)
        let payment = [10.0, 10.0, 20.0, 20.0]; // first-price: pays their report
        let sp = check_strategy_proofness_2x2(&val_matrix, &alloc, &payment).unwrap();
        assert!(!sp, "First-price should not be strategy-proof");
    }

    #[test]
    fn mechanism_report_vickrey() {
        let v = [10.0, 20.0, 15.0];
        let mut p = [0.0f64; 3];
        vickrey_clarke_groves_payment_into(&v, &mut p).unwrap();
        let alloc = [false, true, false]; // agent 1 wins
        let report = mechanism_report(&v, &p, &alloc).unwrap();
        assert!(report.individual_rationality); // 15 <= 20
        assert!(report.budget_balance); // revenue = 15 >= 0
        assert!(approx(report.total_payment, 15.0, 1e-9));
        assert!(approx(report.total_surplus, 20.0 - 15.0, 1e-9)); // winner val - payment
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(
            check_individual_rationality(&[], &[]).unwrap_err(),
            MechanismError::InvalidInput
        );
    }
}