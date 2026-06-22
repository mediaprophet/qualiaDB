//! Responsibility & systemic meta-guard (§25 + §30, legal_logic.md).
//!
//! **§25 — allegation → adjudication.** The engine must reason *about* claims of harm without
//! adopting them as truth. An alleged act is held as an allegation (RDF-star: a quoted
//! statement) and only an authority's adjudication promotes it to an enforceable fact. This
//! keeps the accusation machinery from becoming an accusation *weapon*.
//!
//! **§30 — systemic meta-guard.** The person must be protected from the system itself. If the
//! engine acts as enforcer it is bound by the same baselines it enforces: it may not grant an
//! institution power while denying the affected person a remedy (rule-of-law asymmetry), block
//! an action with no appeal path (enforcer overreach), or let harm occur with no accountable
//! natural person behind it (accountability vacuum). These are deliberately simple, total
//! predicates — guardrails, not heuristics.

/// The adjudicative state of a claim about conduct (§25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponsibilityStatus {
    /// A quoted/reported claim of conduct — NOT a fact. The default for an incoming report.
    #[default]
    Alleged,
    /// A recognised authority has confirmed the claim → it is an enforceable fact.
    Adjudicated,
    /// The claim was rejected on adjudication → it carries no enforcement weight.
    Dismissed,
}

/// Promote an allegation per an adjudication outcome. `Adjudicated` and `Dismissed` are
/// terminal; an undecided claim stays `Alleged`.
pub fn adjudicate(confirmed: bool, dismissed: bool) -> ResponsibilityStatus {
    match (confirmed, dismissed) {
        (true, _) => ResponsibilityStatus::Adjudicated,
        (false, true) => ResponsibilityStatus::Dismissed,
        _ => ResponsibilityStatus::Alleged,
    }
}

/// Only an **adjudicated** claim is an enforceable fact (allegations and dismissals are not).
/// This is the gate that stops an allegation from triggering contrary-to-duty penalties before
/// due process.
#[inline]
pub fn is_enforceable_fact(status: ResponsibilityStatus) -> bool {
    matches!(status, ResponsibilityStatus::Adjudicated)
}

// ─── §30 Systemic meta-guard flags ───────────────────────────────────────────────

/// **Rule-of-law asymmetry**: the system grants an institution access/power while denying the
/// affected person a remedy / due process. (J-asymmetry — a defining marker of capture.)
#[inline]
pub fn rule_of_law_asymmetry(institution_granted_power: bool, person_denied_remedy: bool) -> bool {
    institution_granted_power && person_denied_remedy
}

/// **Enforcer overreach**: the system blocks an action but provides the person no grounded
/// path to appeal the block. (The enforcer is bound by due process too.)
#[inline]
pub fn enforcer_overreach(system_blocked: bool, has_appeal_path: bool) -> bool {
    system_blocked && !has_appeal_path
}

/// **Accountability vacuum**: harm occurred via an autonomous process, but no natural person is
/// accountable (the corporate veil / architecture shields everyone). Routes to SanctionableSubject
/// review. (Ties to agency.n3 G1' — an artificial agent must have a human Principal behind it.)
#[inline]
pub fn accountability_vacuum(harm_occurred: bool, has_accountable_natural_person: bool) -> bool {
    harm_occurred && !has_accountable_natural_person
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allegation_is_not_a_fact_until_adjudicated() {
        // Default and undecided → Alleged, not enforceable.
        assert_eq!(ResponsibilityStatus::default(), ResponsibilityStatus::Alleged);
        assert!(!is_enforceable_fact(ResponsibilityStatus::Alleged));
        assert!(!is_enforceable_fact(adjudicate(false, false)));
        // Confirmed → Adjudicated → enforceable fact.
        let s = adjudicate(true, false);
        assert_eq!(s, ResponsibilityStatus::Adjudicated);
        assert!(is_enforceable_fact(s));
        // Dismissed → not enforceable.
        assert!(!is_enforceable_fact(adjudicate(false, true)));
    }

    #[test]
    fn meta_guards_protect_the_person_from_the_system() {
        // Institution gets power, person denied remedy → asymmetry flagged.
        assert!(rule_of_law_asymmetry(true, true));
        assert!(!rule_of_law_asymmetry(true, false)); // remedy available → ok
        // Blocked with no appeal → overreach; blocked WITH an appeal path → ok.
        assert!(enforcer_overreach(true, false));
        assert!(!enforcer_overreach(true, true));
        // Harm with no accountable natural person → accountability vacuum.
        assert!(accountability_vacuum(true, false));
        assert!(!accountability_vacuum(true, true));
        assert!(!accountability_vacuum(false, false)); // no harm → nothing to flag
    }
}
