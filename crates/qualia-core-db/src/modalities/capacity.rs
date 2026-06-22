//! Juridical capacity & state-transition (§18, legal_logic.md).
//!
//! Obligations and stipulations are only valid if the asserting agent had the legal/cognitive
//! capacity to form them. This module is a **conservative** engine over the *existing*
//! guardianship ontology vocabulary (`values:juridicalCapacity`, `CoercedConsentFlag`,
//! `VoidableStipulation`, `values:guardian`, `survivesDeath`) — it wires terms Timothy already
//! coined; it does not invent new sensitive vocabulary.
//!
//! Handled with gravity, two deliberate semantic choices (both from the spec, both the
//! legally-careful reading — flagged here, not silently assumed):
//!   * **Duress → VOIDABLE, not void.** A stipulation made under coercion is voidable *at the
//!     victim's election* — it is NOT automatically nullified. Auto-nullifying would strip the
//!     victim of the choice to keep or undo it. (`◇Void`, not `Void`.)
//!   * **Guardianship carries the dependent's weight, it does not replace the dependent.** A
//!     guardian acts *on behalf of* the dependent; the legal weight is the dependent's.

/// An agent's juridical capacity for an act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapacityStatus {
    /// Full legal/cognitive capacity — stipulations are binding.
    #[default]
    Intact,
    /// Capacity is impaired (e.g. minority, incapacity) — stipulations are not binding.
    Impaired,
    /// Capacity present but the act was coerced — the stipulation is *voidable*.
    UnderDuress,
}

/// A stipulation by the agent is **binding** only when capacity is `Intact`.
#[inline]
pub fn stipulation_binding(capacity: CapacityStatus) -> bool {
    matches!(capacity, CapacityStatus::Intact)
}

/// A stipulation made `UnderDuress` is **voidable at the victim's election** (CoercedConsentFlag
/// → VoidableStipulation). Returns true iff the agent may elect to void it — NOT that it is
/// already void (that choice stays with the victim).
#[inline]
pub fn stipulation_voidable(capacity: CapacityStatus) -> bool {
    matches!(capacity, CapacityStatus::UnderDuress)
}

/// Guardianship / delegation: when a guardianship relation holds, a guardian's act carries the
/// **dependent's** legal weight (`values:actsOnBehalfOf`). Returns the identity whose weight
/// the act bears — the dependent under guardianship, else the actor themselves.
#[inline]
pub fn effective_principal(actor: u64, dependent: u64, has_guardianship: bool) -> u64 {
    if has_guardianship {
        dependent
    } else {
        actor
    }
}

/// Posthumous standing: a representative may prosecute the **surviving** claims of a deceased
/// agent (`BreachRecord(survivesDeath)` + representative standing). True iff the agent is
/// deceased AND a representative stands for them.
#[inline]
pub fn posthumous_standing(deceased: bool, has_representative: bool) -> bool {
    deceased && has_representative
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_gates_binding() {
        assert!(stipulation_binding(CapacityStatus::Intact));
        assert!(!stipulation_binding(CapacityStatus::Impaired));
        assert!(!stipulation_binding(CapacityStatus::UnderDuress));
    }

    #[test]
    fn duress_is_voidable_not_void() {
        // Under duress: the victim MAY void — not auto-nullified, and not binding either.
        assert!(stipulation_voidable(CapacityStatus::UnderDuress));
        assert!(!stipulation_binding(CapacityStatus::UnderDuress));
        // Intact / impaired are not "voidable for duress".
        assert!(!stipulation_voidable(CapacityStatus::Intact));
        assert!(!stipulation_voidable(CapacityStatus::Impaired));
    }

    #[test]
    fn guardianship_carries_the_dependents_weight() {
        let guardian = 0xA1;
        let dependent = 0xB2;
        assert_eq!(effective_principal(guardian, dependent, true), dependent);
        assert_eq!(effective_principal(guardian, dependent, false), guardian);
    }

    #[test]
    fn posthumous_claims_need_a_representative() {
        assert!(posthumous_standing(true, true));
        assert!(!posthumous_standing(true, false)); // deceased, no representative → no standing
        assert!(!posthumous_standing(false, true)); // alive → they hold their own standing
    }
}
