//! The support arithmetic: how strongly a claim triple is backed by a set of cited
//! fact quins. Pure over [`NQuin`]s — no I/O, fully testable.
//!
//! A claim is the triple `(subject, predicate, object)` of `AgentOutput::semantic_quin`.
//! Two complementary, real signals are computed over the cited facts:
//!
//! * **Role support** — the strongest single fact that matches the claim's components
//!   *in the same role*: an exact triple match scores `1.0`; a predicate+object or
//!   subject+predicate match `2/3`; a single role `1/3`. This is "is this exact
//!   relation attested?".
//! * **Entity grounding** — the fraction of the claim's two *endpoints* (subject,
//!   object) that appear *anywhere* in the evidence. This catches a claim that is a
//!   legitimate multi-hop consequence of cited facts even when no single fact is the
//!   whole triple: "are the things I'm talking about even in the evidence?".
//!
//! The combined score takes the stronger of role support and a capped half-weight of
//! entity grounding, so an exact attestation always dominates while a claim whose
//! endpoints are at least cited gets partial (review-band) credit.

use crate::NQuin;

/// Per-claim grounding signals plus the combined score in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundingReport {
    /// Strongest same-role overlap with a single cited fact (0, 1/3, 2/3, 1).
    pub role_support: f64,
    /// Fraction of `{subject, object}` appearing anywhere in the evidence (0, 0.5, 1).
    pub entity_grounding: f64,
    /// `true` iff some cited fact equals the claim triple exactly.
    pub exact: bool,
    /// Combined grounding score.
    pub score: f64,
}

/// Same-role overlap between a claim and one fact: count of matching `(s, p, o)`
/// positions, as a fraction of three.
fn role_overlap(claim: &NQuin, fact: &NQuin) -> f64 {
    let mut m = 0u8;
    if claim.subject == fact.subject {
        m += 1;
    }
    if claim.predicate == fact.predicate {
        m += 1;
    }
    if claim.object == fact.object {
        m += 1;
    }
    m as f64 / 3.0
}

/// Strongest same-role support for `claim` across `facts` (max role overlap). `0.0`
/// for empty evidence.
pub fn component_support(claim: &NQuin, facts: &[NQuin]) -> f64 {
    facts
        .iter()
        .fold(0.0_f64, |best, f| best.max(role_overlap(claim, f)))
}

/// Fraction of the claim's endpoints `{subject, object}` that occur in *any* role
/// (subject, predicate or object) of *any* cited fact.
pub fn entity_grounding(claim: &NQuin, facts: &[NQuin]) -> f64 {
    let present = |h: u64| {
        h != 0
            && facts
                .iter()
                .any(|f| f.subject == h || f.predicate == h || f.object == h)
    };
    let mut grounded = 0u8;
    if present(claim.subject) {
        grounded += 1;
    }
    if present(claim.object) {
        grounded += 1;
    }
    grounded as f64 / 2.0
}

/// Build the full grounding report for `claim` over `facts`.
pub fn report(claim: &NQuin, facts: &[NQuin]) -> GroundingReport {
    let role_support = component_support(claim, facts);
    let eg = entity_grounding(claim, facts);
    let exact = facts.iter().any(|f| {
        f.subject == claim.subject && f.predicate == claim.predicate && f.object == claim.object
    });
    // Exact attestation dominates; otherwise the stronger of role support and a capped
    // half-weight of endpoint grounding.
    let score = if exact {
        1.0
    } else {
        role_support.max(0.5 * eg)
    };
    GroundingReport {
        role_support,
        entity_grounding: eg,
        exact,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn exact_match_scores_full() {
        let claim = quin(1, 2, 3);
        let facts = [quin(9, 9, 9), quin(1, 2, 3)];
        let r = report(&claim, &facts);
        assert!(r.exact);
        assert!((r.score - 1.0).abs() < 1e-12);
    }

    #[test]
    fn two_role_match_clears_two_thirds() {
        // predicate + object match, subject differs.
        let claim = quin(1, 2, 3);
        let facts = [quin(7, 2, 3)];
        let r = report(&claim, &facts);
        assert!(!r.exact);
        assert!((r.role_support - 2.0 / 3.0).abs() < 1e-12);
        assert!((r.score - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn both_endpoints_cited_elsewhere_is_review_band() {
        // No single fact matches a role of the claim, but both endpoints (1 and 3)
        // appear across the evidence in *other* roles (1 as a predicate, 3 as a
        // subject) → entity grounding 1.0, role support 0 → score 0.5.
        let claim = quin(1, 2, 3);
        let facts = [quin(50, 1, 60), quin(3, 70, 80)];
        let r = report(&claim, &facts);
        assert_eq!(r.role_support, 0.0);
        assert!((r.entity_grounding - 1.0).abs() < 1e-12);
        assert!((r.score - 0.5).abs() < 1e-12);
    }

    #[test]
    fn unrelated_evidence_scores_zero() {
        let claim = quin(1, 2, 3);
        let facts = [quin(4, 5, 6)];
        let r = report(&claim, &facts);
        assert!(r.score.abs() < 1e-12);
    }

    #[test]
    fn empty_evidence_scores_zero() {
        let claim = quin(1, 2, 3);
        let r = report(&claim, &[]);
        assert!(r.score.abs() < 1e-12);
        assert!(!r.exact);
    }

    #[test]
    fn zero_hash_endpoints_do_not_ground() {
        // A claim with a zero (unset) subject must not be credited just because some
        // fact also has structure — entity_grounding ignores zero hashes.
        let claim = quin(0, 2, 3);
        let facts = [quin(0, 0, 0)];
        assert_eq!(entity_grounding(&claim, &facts), 0.0);
    }
}
