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

#[cfg(test)]
mod tests {
    use super::*;

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
