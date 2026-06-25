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

// ─── Moral responsibility: blameworthiness / praiseworthiness ─────────────────────

/// The moral appraisal of an agent's act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoralAppraisal {
    /// Culpable for a bad outcome — degree scales with causal share.
    Blameworthy(u8),
    /// Creditable for a good outcome.
    Praiseworthy(u8),
    /// No moral weight (no causal contribution).
    Neutral,
    /// Caused the outcome but EXCUSED (lacked capacity, involuntary, or no culpable mind).
    Excused,
}

/// Appraise moral responsibility. Blame/praise require a non-zero `causal_degree`, a `voluntary`
/// act, and `has_capacity` (the §18 capacity gate carries into moral responsibility — an agent
/// who lacked capacity or acted involuntarily is EXCUSED). `good_outcome` sets the valence;
/// blame additionally requires a `culpable_mind` (intent or recklessness — a mere accident with a
/// bad outcome is excused). Degree scales with the agent's causal share.
pub fn appraise(
    causal_degree: u8,
    good_outcome: bool,
    voluntary: bool,
    has_capacity: bool,
    culpable_mind: bool,
) -> MoralAppraisal {
    if causal_degree == 0 {
        return MoralAppraisal::Neutral;
    }
    if !voluntary || !has_capacity {
        return MoralAppraisal::Excused;
    }
    if good_outcome {
        MoralAppraisal::Praiseworthy(causal_degree)
    } else if culpable_mind {
        MoralAppraisal::Blameworthy(causal_degree)
    } else {
        MoralAppraisal::Excused // bad outcome but no culpable mind → a mere accident
    }
}

// ─── Causal contribution vectors (degree of responsibility) ───────────────────────

/// Degree of responsibility = an agent's causal `contribution` as a share of the `total` causal
/// weight of all contributors, in `[0,1]`. `0.0` if `total` is 0.
pub fn degree_of_responsibility(contribution: u32, total: u32) -> f32 {
    if total == 0 {
        0.0
    } else {
        contribution as f32 / total as f32
    }
}

/// Normalise a vector of causal `contributions` into responsibility shares, written into `out`
/// (parallel; sums to 1.0). Returns `false` if all-zero or `out` is too small. Zero-heap.
pub fn responsibility_shares(contributions: &[u32], out: &mut [f32]) -> bool {
    let total: u32 = contributions.iter().sum();
    if total == 0 || out.len() < contributions.len() {
        return false;
    }
    for (i, &c) in contributions.iter().enumerate() {
        out[i] = c as f32 / total as f32;
    }
    true
}

// ─── Doctrine of double effect (intention vs foresight) ───────────────────────────

/// **Doctrine of Double Effect**: an act with a foreseen-but-unintended bad side effect is
/// permissible iff ALL four conditions hold:
///  1. the act itself is not wrong (`act_permissible`);
///  2. the bad effect is NOT intended, only foreseen (`!bad_intended`);
///  3. the bad effect is NOT the means to the good (`!bad_is_means`);
///  4. proportionality — the good is not outweighed by the bad (`proportionate`).
pub fn double_effect_permissible(
    act_permissible: bool,
    bad_intended: bool,
    bad_is_means: bool,
    proportionate: bool,
) -> bool {
    act_permissible && !bad_intended && !bad_is_means && proportionate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moral_appraisal_blame_praise_excuse() {
        // Good outcome, voluntary, capable → praiseworthy (degree = causal share).
        assert_eq!(appraise(200, true, true, true, false), MoralAppraisal::Praiseworthy(200));
        // Bad outcome + culpable mind, voluntary, capable → blameworthy.
        assert_eq!(appraise(150, false, true, true, true), MoralAppraisal::Blameworthy(150));
        // Bad outcome but NO culpable mind → mere accident → excused.
        assert_eq!(appraise(150, false, true, true, false), MoralAppraisal::Excused);
        // No capacity, or involuntary → excused regardless of outcome.
        assert_eq!(appraise(150, false, true, false, true), MoralAppraisal::Excused);
        assert_eq!(appraise(150, false, false, true, true), MoralAppraisal::Excused);
        // No causal contribution → neutral.
        assert_eq!(appraise(0, false, true, true, true), MoralAppraisal::Neutral);
    }

    #[test]
    fn causal_contribution_vectors() {
        assert!((degree_of_responsibility(3, 12) - 0.25).abs() < 1e-6);
        assert_eq!(degree_of_responsibility(1, 0), 0.0);
        let mut out = [0.0f32; 3];
        assert!(responsibility_shares(&[1, 2, 1], &mut out));
        assert!((out[0] - 0.25).abs() < 1e-6 && (out[1] - 0.5).abs() < 1e-6);
        assert!((out[0] + out[1] + out[2] - 1.0).abs() < 1e-6);
        assert!(!responsibility_shares(&[0, 0], &mut out)); // all-zero refuses
    }

    #[test]
    fn double_effect_requires_all_four_conditions() {
        // Permissible act, bad foreseen-not-intended, not a means, proportionate → permissible.
        assert!(double_effect_permissible(true, false, false, true));
        // Bad effect intended → impermissible.
        assert!(!double_effect_permissible(true, true, false, true));
        // Bad effect is the means to the good → impermissible.
        assert!(!double_effect_permissible(true, false, true, true));
        // Disproportionate → impermissible.
        assert!(!double_effect_permissible(true, false, false, false));
        // The act itself wrong → impermissible.
        assert!(!double_effect_permissible(false, false, false, true));
    }

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
