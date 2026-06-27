//! Turn the support arithmetic into a verdict, and resolve provenance citations into
//! the facts the arithmetic needs.

use super::claim_support::report;
use super::{GroundingResolver, GroundingThresholds, GroundingVerdict};
use crate::NQuin;

/// Grounding score for `claim` against the resolved cited `facts`, as a verdict under
/// `thresholds`. Empty evidence ⇒ `Ungrounded(0.0)` (fail closed).
pub fn evaluate_grounding(
    claim: &NQuin,
    facts: &[NQuin],
    thresholds: GroundingThresholds,
) -> GroundingVerdict {
    let score = report(claim, facts).score;
    grounding_verdict(score, thresholds)
}

/// Partition a raw grounding score into a verdict.
pub fn grounding_verdict(score: f64, thresholds: GroundingThresholds) -> GroundingVerdict {
    if score >= thresholds.permit {
        GroundingVerdict::Grounded { score }
    } else if score >= thresholds.deny {
        GroundingVerdict::Weak { score }
    } else {
        GroundingVerdict::Ungrounded { score }
    }
}

/// Resolve a slice of provenance citation hashes to their fact quins via `resolver`,
/// dropping any that do not resolve. The output is what [`evaluate_grounding`] consumes.
pub fn resolve_citations(citations: &[u64], resolver: &dyn GroundingResolver) -> Vec<NQuin> {
    citations.iter().filter_map(|&h| resolver.resolve(h)).collect()
}

/// End-to-end gate input: resolve the citations and grade the claim in one call. If no
/// citation resolves (no evidence available), returns `Ungrounded(0.0)` — fail closed.
pub fn evaluate_output_grounding(
    claim: &NQuin,
    citations: &[u64],
    resolver: &dyn GroundingResolver,
    thresholds: GroundingThresholds,
) -> GroundingVerdict {
    let facts = resolve_citations(citations, resolver);
    if facts.is_empty() {
        return GroundingVerdict::Ungrounded { score: 0.0 };
    }
    evaluate_grounding(claim, &facts, thresholds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin { subject: s, predicate: p, object: o, context: 0, metadata: 0, parity: 0 }
    }

    /// A stub store mapping citation hash → fact quin.
    struct MapResolver(HashMap<u64, NQuin>);
    impl GroundingResolver for MapResolver {
        fn resolve(&self, h: u64) -> Option<NQuin> {
            self.0.get(&h).copied()
        }
    }

    #[test]
    fn exact_attestation_is_grounded() {
        let claim = quin(1, 2, 3);
        let facts = [quin(1, 2, 3)];
        let v = evaluate_grounding(&claim, &facts, GroundingThresholds::default());
        assert!(v.is_grounded());
    }

    #[test]
    fn endpoints_only_is_weak_review_band() {
        let claim = quin(1, 2, 3);
        let facts = [quin(1, 9, 9), quin(9, 9, 3)];
        let v = evaluate_grounding(&claim, &facts, GroundingThresholds::default());
        assert!(matches!(v, GroundingVerdict::Weak { .. }), "got {v:?}");
    }

    #[test]
    fn unrelated_is_ungrounded() {
        let claim = quin(1, 2, 3);
        let facts = [quin(4, 5, 6)];
        let v = evaluate_grounding(&claim, &facts, GroundingThresholds::default());
        assert!(matches!(v, GroundingVerdict::Ungrounded { .. }));
    }

    #[test]
    fn resolver_path_grounds_a_cited_claim() {
        let mut m = HashMap::new();
        m.insert(0xAA, quin(1, 2, 3)); // citation 0xAA resolves to the exact fact
        m.insert(0xBB, quin(9, 9, 9));
        let resolver = MapResolver(m);
        let claim = quin(1, 2, 3);
        let v = evaluate_output_grounding(&claim, &[0xAA, 0xBB], &resolver, GroundingThresholds::default());
        assert!(v.is_grounded());
    }

    #[test]
    fn unresolvable_citations_fail_closed() {
        let resolver = MapResolver(HashMap::new());
        let claim = quin(1, 2, 3);
        // Citation hash present but nothing resolves it → no evidence → ungrounded.
        let v = evaluate_output_grounding(&claim, &[0x123], &resolver, GroundingThresholds::default());
        assert!(matches!(v, GroundingVerdict::Ungrounded { score } if score == 0.0));
    }
}
