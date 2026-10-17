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

// ─── Graduated & Kinship Capacity (Gillick & Supported Decision-Making) ────────────

/// Graduated competence evaluation parameters for an agent facing a specific act.
/// Models Gillick competence where capacity is dynamic and decision-relative rather than
/// an absolute binary toggle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraduatedCompetence {
    /// Assessed comprehension and maturity score in `[0.0, 1.0]`.
    pub understanding_score: f32,
    /// Inherent cognitive/legal complexity of the decision in `[0.0, 1.0]`.
    pub decision_complexity: f32,
}

impl GraduatedCompetence {
    /// Pair an assessed understanding score with a decision's inherent
    /// complexity — both normalized to `[0.0, 1.0]` (not clamped here;
    /// callers supply normalized values).
    #[inline]
    pub const fn new(understanding_score: f32, decision_complexity: f32) -> Self {
        Self {
            understanding_score,
            decision_complexity,
        }
    }

    /// Whether self-standing competence is achieved without auxiliary support.
    #[inline]
    pub fn is_self_standing(&self) -> bool {
        self.understanding_score >= self.decision_complexity
    }
}

/// Endorsement from a kinship circle, elder, parent, or trusted decision-making advocate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinshipEndorsement {
    /// DID hash of the endorsing custodian / elder / supporter.
    pub supporter_did: u64,
    /// Cultural or legal role (e.g. `q_hash("role:elder")`, `q_hash("role:parent")`).
    pub role_hash: u64,
    /// Verified confidence / weight of the endorsement in `[0.0, 1.0]`.
    pub weight: f32,
}

/// Evaluate graduated capacity under Gillick competence and supported decision-making.
///
/// Rules:
/// 1. If an authorized elder or parent asserts an active veto (`elder_veto == true`),
///    capacity is `UnderDuress` (voidable / suspended) to safeguard the dependent.
/// 2. If the agent's individual `understanding_score >= decision_complexity`, they possess
///    autonomous Gillick competence (`CapacityStatus::Intact`).
/// 3. If individual competence is insufficient, supported decision-making is evaluated:
///    endorsers with `weight >= min_endorsement_weight` are counted. If the count reaches
///    `required_quorum` and the collective support bridges the gap, returns `CapacityStatus::Intact`.
/// 4. Otherwise returns `CapacityStatus::Impaired`.
pub fn eval_graduated_capacity(
    competence: GraduatedCompetence,
    endorsements: &[KinshipEndorsement],
    min_endorsement_weight: f32,
    required_quorum: usize,
    elder_veto: bool,
) -> CapacityStatus {
    if elder_veto {
        return CapacityStatus::UnderDuress;
    }

    if competence.is_self_standing() {
        return CapacityStatus::Intact;
    }

    let mut valid_count = 0usize;
    let mut total_supported_weight = 0.0f32;

    for e in endorsements {
        if e.weight >= min_endorsement_weight {
            valid_count += 1;
            total_supported_weight += e.weight;
        }
    }

    if valid_count >= required_quorum {
        // Supported capacity bridges up to 50% extra complexity deficit based on collective weight
        let support_boost = (total_supported_weight / valid_count.max(1) as f32) * 0.5;
        if (competence.understanding_score + support_boost) >= competence.decision_complexity {
            return CapacityStatus::Intact;
        }
    }

    CapacityStatus::Impaired
}

// ─── Digital Probate & Succession Semantics ────────────────────────────────────────

/// Testamentary delegation terms specifying successor rights upon mortality event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestamentaryClause<'a> {
    /// Deceased or testator entity DID hash.
    pub testator: u64,
    /// Designated heir or successor representative DID hash.
    pub successor: u64,
    /// Authorized domains of transmission (opaque u64 domain hashes).
    pub delegated_domains: &'a [u64],
    /// Whether this grant explicitly survives death (`survivesDeath`).
    pub survives_death: bool,
}

/// Attested succession/mortality event from a mortality oracle or civil registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuccessionEvent {
    /// Decedent DID hash.
    pub decedent: u64,
    /// Mortality oracle or registrar DID hash.
    pub oracle: u64,
    /// Attestation epoch timestamp.
    pub attestation_epoch: u32,
    /// Whether the death attestation has been cryptographically verified.
    pub verified: bool,
}

/// Predicate URI hash for claims surviving death (`values:survivesDeath`).
#[inline]
pub fn survives_death_predicate() -> u64 {
    crate::q_hash("values:survivesDeath")
}

/// Predicate URI hash for testamentary successor (`values:testamentarySuccessor`).
#[inline]
pub fn testamentary_successor_predicate() -> u64 {
    crate::q_hash("values:testamentarySuccessor")
}

/// Evaluates whether a successor holds standing to exercise authority in a requested domain
/// under a testamentary clause and verified succession event.
pub fn eval_succession_standing(
    clause: &TestamentaryClause,
    event: Option<&SuccessionEvent>,
    requested_domain: u64,
) -> bool {
    let Some(ev) = event else {
        return false;
    };

    if !ev.verified || ev.decedent != clause.testator {
        return false;
    }

    if !clause.survives_death {
        return false;
    }

    clause.delegated_domains.contains(&requested_domain)
}

/// Filters an input stream of Quins for claims that survive the decedent's death.
/// Matches Quins whose predicate equals `values:survivesDeath` or subject equals `decedent`.
pub fn filter_surviving_claims(
    claims: &[crate::NQuin],
    decedent: u64,
    out: &mut [crate::NQuin],
) -> usize {
    let surv_pred = survives_death_predicate();
    let mut count = 0usize;

    for &c in claims {
        if count >= out.len() {
            break;
        }
        if c.subject == decedent && (c.predicate == surv_pred || (c.metadata & 0x01) != 0) {
            out[count] = c;
            count += 1;
        }
    }

    count
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

    #[test]
    fn test_graduated_capacity_and_kinship_endorsement() {
        let comp = GraduatedCompetence::new(0.6, 0.5);
        assert!(comp.is_self_standing());
        assert_eq!(
            eval_graduated_capacity(comp, &[], 0.5, 1, false),
            CapacityStatus::Intact
        );

        // Insufficient individual competence: score 0.4 < complexity 0.7
        let minor_comp = GraduatedCompetence::new(0.4, 0.7);
        assert!(!minor_comp.is_self_standing());
        assert_eq!(
            eval_graduated_capacity(minor_comp, &[], 0.5, 1, false),
            CapacityStatus::Impaired
        );

        // With kinship support meeting quorum (2 elders/parents)
        let endorsements = [
            KinshipEndorsement {
                supporter_did: 0xC1,
                role_hash: crate::q_hash("role:parent"),
                weight: 0.8,
            },
            KinshipEndorsement {
                supporter_did: 0xC2,
                role_hash: crate::q_hash("role:elder"),
                weight: 0.9,
            },
        ];
        assert_eq!(
            eval_graduated_capacity(minor_comp, &endorsements, 0.7, 2, false),
            CapacityStatus::Intact
        );

        // Elder veto forces UnderDuress even with valid support
        assert_eq!(
            eval_graduated_capacity(minor_comp, &endorsements, 0.7, 2, true),
            CapacityStatus::UnderDuress
        );
    }

    #[test]
    fn test_digital_probate_and_succession_standing() {
        let testator = 0xD1;
        let heir = 0xD2;
        let domain_land = crate::q_hash("domain:custodial_land");
        let domain_finance = crate::q_hash("domain:finance");

        let clause = TestamentaryClause {
            testator,
            successor: heir,
            delegated_domains: &[domain_land],
            survives_death: true,
        };

        // Before death attested: no standing
        assert!(!eval_succession_standing(&clause, None, domain_land));

        // Unverified event: no standing
        let unverified_ev = SuccessionEvent {
            decedent: testator,
            oracle: 0x99,
            attestation_epoch: 1000,
            verified: false,
        };
        assert!(!eval_succession_standing(
            &clause,
            Some(&unverified_ev),
            domain_land
        ));

        // Verified event: standing in domain_land, but not finance
        let verified_ev = SuccessionEvent {
            decedent: testator,
            oracle: 0x99,
            attestation_epoch: 1000,
            verified: true,
        };
        assert!(eval_succession_standing(
            &clause,
            Some(&verified_ev),
            domain_land
        ));
        assert!(!eval_succession_standing(
            &clause,
            Some(&verified_ev),
            domain_finance
        ));

        // Surviving claims filtering
        let mut q1 = crate::NQuin {
            subject: testator,
            predicate: survives_death_predicate(),
            object: 0x123,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q1.parity = q1.subject ^ q1.predicate ^ q1.object ^ q1.context;

        let mut q2 = crate::NQuin {
            subject: testator,
            predicate: crate::q_hash("claims:ordinary"),
            object: 0x456,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q2.parity = q2.subject ^ q2.predicate ^ q2.object ^ q2.context;

        let mut out = [crate::NQuin::default(); 4];
        let n = filter_surviving_claims(&[q1, q2], testator, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].object, 0x123);
    }
}
