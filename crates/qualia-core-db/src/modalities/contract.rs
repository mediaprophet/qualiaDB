//! Contractual formation & agreement (§22, legal_logic.md) — private ordering.
//!
//! Beyond universal human rights, agents create binding private law through agreements. This
//! formalises the micro-states of formation (Offer → Assent → Binding) and composes **§18
//! capacity**: mutual assent only creates a binding obligation when *both* parties had the
//! juridical capacity to agree. A contract may also incorporate a larger normative corpus by
//! reference (e.g. the UN Guiding Principles).

use crate::modalities::capacity::{stipulation_binding, CapacityStatus};

/// The formation stage of an agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormationStage {
    /// Nothing stipulated yet.
    #[default]
    None,
    /// One party has stipulated an obligation as a condition of engagement (an offer).
    Offer,
    /// Both parties have assented — a binding, localised obligation exists.
    Binding,
}

/// The raw formation stage from the two acts (capacity aside): an offer needs a stipulation;
/// binding needs assent on top of it.
pub fn formation_stage(stipulated: bool, accepted: bool) -> FormationStage {
    match (stipulated, accepted) {
        (true, true) => FormationStage::Binding,
        (true, false) => FormationStage::Offer,
        _ => FormationStage::None,
    }
}

/// A contract is **binding** iff it was stipulated, assented to, AND *both* parties had intact
/// juridical capacity (composes `capacity::stipulation_binding` — an agreement assented to under
/// duress or by an incapacitated party does not bind).
pub fn is_binding_contract(
    stipulated: bool,
    accepted: bool,
    offeror: CapacityStatus,
    acceptor: CapacityStatus,
) -> bool {
    formation_stage(stipulated, accepted) == FormationStage::Binding
        && stipulation_binding(offeror)
        && stipulation_binding(acceptor)
}

/// Incorporation by reference: the agreement imports the clauses of `instrument` (a corpus URI
/// hash). A non-zero instrument means clauses are incorporated.
#[inline]
pub fn incorporates_by_reference(instrument: u64) -> bool {
    instrument != 0
}

// ─── Formal verification of terms against deontic / human-rights limits ───────────

/// A contract term that obligates a FORBIDDEN action (a deontic / human-rights limit — e.g. an
/// agreement to waive a non-derogable right) is VOID. The terms respect the limits iff none of
/// `obligated_actions` is in `forbidden`. (Private ordering cannot contract out of the baselines.)
pub fn terms_respect_limits(obligated_actions: &[u64], forbidden: &[u64]) -> bool {
    obligated_actions.iter().all(|a| !forbidden.contains(a))
}

// ─── Breach-detection state machine (conditions precedent / subsequent) ───────────

/// The lifecycle state of a binding contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractState {
    /// A condition PRECEDENT is unmet — the obligation has not yet arisen.
    Pending,
    /// In force, awaiting performance.
    Active,
    /// Performed — obligation satisfied.
    Performed,
    /// A condition SUBSEQUENT occurred — the obligation is terminated/discharged.
    Discharged,
    /// The deadline passed without performance — breach.
    Breached,
}

/// Contract lifecycle (precedence order): an unmet condition **precedent** → `Pending`; a
/// condition **subsequent** that occurred → `Discharged`; performance → `Performed`; a passed
/// deadline without performance → `Breached`; otherwise `Active`.
pub fn contract_state(
    precedent_met: bool,
    subsequent_occurred: bool,
    performed: bool,
    deadline_passed: bool,
) -> ContractState {
    if !precedent_met {
        ContractState::Pending
    } else if subsequent_occurred {
        ContractState::Discharged
    } else if performed {
        ContractState::Performed
    } else if deadline_passed {
        ContractState::Breached
    } else {
        ContractState::Active
    }
}

// ─── Computable performance metrics + oracle ──────────────────────────────────────

/// Performance ratio = `delivered / required` (clamped ≥ 0); `1.0` = fully performed. A
/// non-positive `required` is vacuously satisfied (`1.0`).
pub fn performance_ratio(delivered: f64, required: f64) -> f64 {
    if required <= 0.0 {
        1.0
    } else {
        (delivered / required).max(0.0)
    }
}

/// Performance is met iff an **oracle-trusted** measurement shows `delivered` reaching `required`
/// (`performance_ratio >= 1.0`). An untrusted oracle measurement is not admissible (fail closed).
pub fn performance_met(delivered: f64, required: f64, oracle_trusted: bool) -> bool {
    oracle_trusted && performance_ratio(delivered, required) >= 1.0
}

// ─── Multi-party splitting + sub-contract liability tracing ───────────────────────

/// Trace liability through a sub-contract `chain`: `chain[i]` is the party at depth `i` (the prime
/// contractor `chain[0]` sub-contracts to `chain[1]`, etc.). Liability for a breach at `depth`
/// rests on `chain[depth]`, capped at the performing sub-contractor at the chain's end. `None` for
/// an empty chain.
pub fn liable_party(chain: &[u64], depth: usize) -> Option<u64> {
    if chain.is_empty() {
        None
    } else {
        Some(chain[depth.min(chain.len() - 1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_cannot_contract_out_of_baselines() {
        let waive_dignity = crate::q_hash("act:waiveInherentDignity");
        let deliver_goods = crate::q_hash("act:deliverGoods");
        let forbidden = [waive_dignity];
        assert!(terms_respect_limits(&[deliver_goods], &forbidden));
        assert!(!terms_respect_limits(&[deliver_goods, waive_dignity], &forbidden), "a void term");
        assert!(terms_respect_limits(&[], &forbidden));
    }

    #[test]
    fn breach_state_machine() {
        // Precedent unmet → Pending.
        assert_eq!(contract_state(false, false, false, true), ContractState::Pending);
        // In force, nothing yet → Active.
        assert_eq!(contract_state(true, false, false, false), ContractState::Active);
        // Performed → Performed.
        assert_eq!(contract_state(true, false, true, false), ContractState::Performed);
        // Condition subsequent occurred → Discharged (even if deadline passed).
        assert_eq!(contract_state(true, true, false, true), ContractState::Discharged);
        // Deadline passed, no performance, no discharge → Breached.
        assert_eq!(contract_state(true, false, false, true), ContractState::Breached);
    }

    #[test]
    fn performance_metrics_and_oracle() {
        assert!((performance_ratio(8.0, 10.0) - 0.8).abs() < 1e-9);
        assert_eq!(performance_ratio(5.0, 0.0), 1.0); // nothing required → satisfied
        assert!(performance_met(10.0, 10.0, true));
        assert!(!performance_met(9.9, 10.0, true), "under-delivered");
        assert!(!performance_met(10.0, 10.0, false), "untrusted oracle → fail closed");
    }

    #[test]
    fn sub_contract_liability_tracing() {
        let prime = crate::q_hash("party:prime");
        let sub1 = crate::q_hash("party:sub1");
        let sub2 = crate::q_hash("party:sub2");
        let chain = [prime, sub1, sub2];
        assert_eq!(liable_party(&chain, 0), Some(prime));
        assert_eq!(liable_party(&chain, 2), Some(sub2));
        assert_eq!(liable_party(&chain, 9), Some(sub2), "capped at the performing sub-contractor");
        assert_eq!(liable_party(&[], 0), None);
    }

    #[test]
    fn formation_progresses_offer_to_binding() {
        assert_eq!(formation_stage(false, false), FormationStage::None);
        assert_eq!(formation_stage(true, false), FormationStage::Offer);
        assert_eq!(formation_stage(true, true), FormationStage::Binding);
        // Acceptance with no offer is not a contract.
        assert_eq!(formation_stage(false, true), FormationStage::None);
    }

    #[test]
    fn binding_requires_capacity_of_both_parties() {
        let intact = CapacityStatus::Intact;
        // Full assent + both intact → binding.
        assert!(is_binding_contract(true, true, intact, intact));
        // Acceptor under duress → not binding (the agreement is voidable, not binding).
        assert!(!is_binding_contract(true, true, intact, CapacityStatus::UnderDuress));
        // Offeror impaired → not binding.
        assert!(!is_binding_contract(true, true, CapacityStatus::Impaired, intact));
        // Mere offer (no assent) → not binding even with capacity.
        assert!(!is_binding_contract(true, false, intact, intact));
    }

    #[test]
    fn incorporation_by_reference() {
        assert!(incorporates_by_reference(crate::q_hash("instrument:ungp")));
        assert!(!incorporates_by_reference(0));
    }
}
