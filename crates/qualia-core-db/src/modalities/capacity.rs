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

// ─── Jurisdiction-specific capacity thresholds ──────────────────────────────────────

/// Does `age_years` meet a jurisdiction's `majority_age`? The threshold is **supplied by the
/// caller** (jurisdiction-specific — 18 in most, 21 in some, mental-health-act variations) so
/// the engine never bakes one jurisdiction's law in as universal.
#[inline]
pub fn meets_age_of_majority(age_years: u32, majority_age: u32) -> bool {
    age_years >= majority_age
}

/// Derive capacity from age against a jurisdiction threshold: below majority → `Impaired`
/// (minority); at/above → `Intact`. Coercion is layered separately ([`capacity_under_pressure`]).
pub fn capacity_from_age(age_years: u32, majority_age: u32) -> CapacityStatus {
    if meets_age_of_majority(age_years, majority_age) {
        CapacityStatus::Intact
    } else {
        CapacityStatus::Impaired
    }
}

// ─── Coercion / duress detection (relational imbalance → voidable) ──────────────────

/// Map a relational power-imbalance signal to a duress finding. `imbalance` is a normalised
/// `[0,1]` measure of relational asymmetry (dependency / authority / economic capture); an
/// `explicit_threat` forces duress regardless. At/above `threshold`, or under an explicit
/// threat, the act is coerced → the resulting stipulation is *voidable* (never auto-void; the
/// election stays with the victim — see module header).
pub fn detect_duress(imbalance: f32, explicit_threat: bool, threshold: f32) -> bool {
    explicit_threat || imbalance >= threshold
}

/// Capacity under relational pressure: an otherwise-`Intact` agent whose act is coerced becomes
/// `UnderDuress` (voidable). Duress never upgrades an already-`Impaired` capacity.
pub fn capacity_under_pressure(
    base: CapacityStatus,
    imbalance: f32,
    explicit_threat: bool,
    threshold: f32,
) -> CapacityStatus {
    if base == CapacityStatus::Intact && detect_duress(imbalance, explicit_threat, threshold) {
        CapacityStatus::UnderDuress
    } else {
        base
    }
}

// ─── Temporary impairment with time-decay (e.g. intoxication clearance) ─────────────

/// A transient impairment decaying linearly toward zero — a conservative clearance model
/// (e.g. intoxication). `initial` is the level in `[0,1]` at t0; `rate` is decay per elapsed
/// time unit. Returns the clamped residual level at `elapsed` units.
pub fn decayed_impairment(initial: f32, elapsed: f32, rate: f32) -> f32 {
    (initial - rate * elapsed).clamp(0.0, 1.0)
}

/// Capacity under a *transient* impairment: while the decayed level is at/above `threshold` the
/// agent is `Impaired`; once it decays below, capacity self-clears to `Intact`. Distinct from
/// durable incapacity (which does not decay).
pub fn transient_capacity(initial: f32, elapsed: f32, rate: f32, threshold: f32) -> CapacityStatus {
    if decayed_impairment(initial, elapsed, rate) >= threshold {
        CapacityStatus::Impaired
    } else {
        CapacityStatus::Intact
    }
}

// ─── Selective right-delegation (guardianship mechanism) ────────────────────────────
//
// NOTE: the *vocabulary* of guardianship domains (the 17+ domains of agency in Timothy's
// CopyOfGuardianShipRelations design) is his to coin. This module deliberately operates over
// OPAQUE caller-supplied domain identifiers (u64 hashes) — wiring the *mechanism* of selective
// delegation without inventing sensitive guardianship vocabulary.

/// Selective delegation: `authorized_domains` enumerate exactly the domains of agency delegated
/// to a guardian. The guardian may act in `requested_domain` iff it is among them — no domain
/// ⇒ no authority (selective, never plenary).
pub fn guardianship_authorized(authorized_domains: &[u64], requested_domain: u64) -> bool {
    authorized_domains.contains(&requested_domain)
}

/// Domain-scoped effective principal: a guardian's act carries the dependent's weight ONLY
/// within a delegated domain; outside the delegated set the guardian cannot bind the dependent
/// (the act falls back to the actor's own weight).
pub fn effective_principal_scoped(
    actor: u64,
    dependent: u64,
    authorized_domains: &[u64],
    requested_domain: u64,
) -> u64 {
    if guardianship_authorized(authorized_domains, requested_domain) {
        dependent
    } else {
        actor
    }
}

// ─── Delegation chains: attenuation + cascading revocation (ZCAP/Macaroon-style) ────
//
// Still mechanism-only over opaque domain ids — no guardianship vocabulary coined here. A
// delegatee can never gain MORE authority than the delegator (attenuation), and revoking a
// domain withdraws it immediately wherever it appears (cascading revocation).

/// **Attenuation:** a sub-delegation's `child_domains` are valid only if a SUBSET of the
/// delegator's `parent_domains` — a delegatee never receives more authority than the delegator
/// holds. (Empty child set trivially attenuates.)
pub fn delegation_attenuates(parent_domains: &[u64], child_domains: &[u64]) -> bool {
    child_domains.iter().all(|d| parent_domains.contains(d))
}

/// Authority after **cascading revocation**: a guardian may act in `requested_domain` iff it is
/// authorized AND not present in the `revoked_domains` set (revocation withdraws it immediately).
pub fn authorized_after_revocation(authorized: &[u64], revoked: &[u64], requested: u64) -> bool {
    guardianship_authorized(authorized, requested) && !revoked.contains(&requested)
}

/// A multi-link delegation **chain** authorizes `requested_domain` iff: the root holds it, every
/// link attenuates its predecessor (subset), and the domain survives at every level (no link
/// silently re-broadens authority). `chain[0]` is the root delegation; each later link is a
/// sub-delegation. Zero-heap (slice of slices).
pub fn chain_authorizes(chain: &[&[u64]], requested_domain: u64) -> bool {
    if chain.is_empty() || !chain[0].contains(&requested_domain) {
        return false;
    }
    for w in chain.windows(2) {
        if !delegation_attenuates(w[0], w[1]) || !w[1].contains(&requested_domain) {
            return false;
        }
    }
    true
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

    #[test]
    fn age_of_majority_is_jurisdiction_parametric() {
        // 18-majority jurisdiction.
        assert_eq!(capacity_from_age(17, 18), CapacityStatus::Impaired);
        assert_eq!(capacity_from_age(18, 18), CapacityStatus::Intact);
        // 21-majority jurisdiction: the same 18-year-old is a minor.
        assert_eq!(capacity_from_age(18, 21), CapacityStatus::Impaired);
        assert!(!stipulation_binding(capacity_from_age(17, 18)));
    }

    #[test]
    fn relational_imbalance_and_threats_yield_voidable_duress() {
        // High relational imbalance → duress → voidable (not auto-void, not binding).
        let c = capacity_under_pressure(CapacityStatus::Intact, 0.9, false, 0.7);
        assert_eq!(c, CapacityStatus::UnderDuress);
        assert!(stipulation_voidable(c));
        assert!(!stipulation_binding(c));
        // An explicit threat forces duress regardless of measured imbalance.
        assert!(detect_duress(0.0, true, 0.7));
        // Below threshold, no threat → capacity unchanged.
        assert_eq!(
            capacity_under_pressure(CapacityStatus::Intact, 0.3, false, 0.7),
            CapacityStatus::Intact
        );
        // Duress never "upgrades" an already-impaired (minor) agent.
        assert_eq!(
            capacity_under_pressure(CapacityStatus::Impaired, 0.9, true, 0.7),
            CapacityStatus::Impaired
        );
    }

    #[test]
    fn transient_impairment_decays_and_self_clears() {
        // Fully impaired at t0, decaying 0.1/unit, threshold 0.5.
        assert!(decayed_impairment(1.0, 0.0, 0.1) > 0.99);
        assert_eq!(
            transient_capacity(1.0, 0.0, 0.1, 0.5),
            CapacityStatus::Impaired
        );
        // After 6 units → level 0.4 < 0.5 → self-cleared.
        assert!((decayed_impairment(1.0, 6.0, 0.1) - 0.4).abs() < 1e-6);
        assert_eq!(
            transient_capacity(1.0, 6.0, 0.1, 0.5),
            CapacityStatus::Intact
        );
        // Never goes negative.
        assert_eq!(decayed_impairment(0.2, 100.0, 0.1), 0.0);
    }

    #[test]
    fn guardianship_delegation_is_selective_not_plenary() {
        let (guardian, dependent) = (0xA1u64, 0xB2u64);
        let medical = crate::q_hash("domain:medical");
        let financial = crate::q_hash("domain:financial");
        let legal = crate::q_hash("domain:legal");
        let delegated = [medical, financial]; // legal NOT delegated

        assert!(guardianship_authorized(&delegated, medical));
        assert!(!guardianship_authorized(&delegated, legal));
        // In a delegated domain the guardian carries the dependent's weight…
        assert_eq!(
            effective_principal_scoped(guardian, dependent, &delegated, financial),
            dependent
        );
        // …but outside the delegated set they cannot bind the dependent.
        assert_eq!(
            effective_principal_scoped(guardian, dependent, &delegated, legal),
            guardian
        );
    }

    #[test]
    fn delegation_attenuates_revokes_and_chains() {
        let medical = crate::q_hash("domain:medical");
        let financial = crate::q_hash("domain:financial");
        let legal = crate::q_hash("domain:legal");

        // Attenuation: a sub-delegation must be a subset of the parent's authority.
        assert!(delegation_attenuates(
            &[medical, financial, legal],
            &[medical, financial]
        ));
        assert!(
            !delegation_attenuates(&[medical], &[medical, legal]),
            "cannot broaden authority"
        );
        assert!(delegation_attenuates(&[medical], &[]));

        // Cascading revocation withdraws a domain immediately.
        let authorized = [medical, financial];
        assert!(authorized_after_revocation(&authorized, &[], medical));
        assert!(
            !authorized_after_revocation(&authorized, &[medical], medical),
            "revoked → withdrawn"
        );
        assert!(authorized_after_revocation(
            &authorized,
            &[medical],
            financial
        ));

        // A delegation chain: root{med,fin,legal} → sub{med,fin} → subsub{med}.
        let root: &[u64] = &[medical, financial, legal];
        let sub: &[u64] = &[medical, financial];
        let subsub: &[u64] = &[medical];
        let chain = [root, sub, subsub];
        assert!(
            chain_authorizes(&chain, medical),
            "medical survives the whole chain"
        );
        assert!(
            !chain_authorizes(&chain, financial),
            "financial dropped at the last link"
        );
        assert!(
            !chain_authorizes(&chain, legal),
            "legal dropped after the root"
        );
        // A chain that tries to RE-BROADEN (sub adds legal the parent lacks) fails attenuation.
        let bad_sub: &[u64] = &[medical, legal];
        assert!(!chain_authorizes(&[sub, bad_sub], legal));
        assert!(!chain_authorizes(&[], medical));
    }
}
