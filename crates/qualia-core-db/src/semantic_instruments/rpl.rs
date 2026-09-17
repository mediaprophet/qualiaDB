//! Recognition of Prior Learning over the existing capability-gap kernel.
//!
//! Formal, prior-learning, experiential and peer-attested evidence are first-class
//! held capabilities. Unauthorised equivalence never closes a gap. Unavailable
//! evidence is unresolved, not unmet. An evaluator pass is not an award.

use crate::modalities::capability_gap::{
    capability_gap, learning_path_cost, requirements_met, MAX_CAP_NODES,
};
use crate::q_hash;
use crate::semantic_instruments::attestation::{
    issue_capability_award, CapabilityAward, InstrumentAttestation,
};
use crate::semantic_instruments::errors::InstrumentError;

/// How a held capability was evidenced. All four bases count as held when available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceBasis {
    Formal,
    PriorLearning,
    Experiential,
    PeerAttested,
}

/// One evidence row. `capability` is an IRI hashed with [`q_hash`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceItem {
    pub capability: String,
    pub basis: EvidenceBasis,
    pub available: bool,
    pub private: bool,
}

/// Honest gap report. `equivalent` is authorised closeMatch, never unauthorised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapResult {
    pub satisfied: Vec<String>,
    pub equivalent: Vec<String>,
    pub unresolved: Vec<String>,
    pub unmet: Vec<String>,
    pub learning_path_cost: Option<u32>,
}

/// Classify required IRIs against RPL evidence. `out_scratch` is the caller
/// buffer for [`capability_gap`].
pub fn evaluate_rpl(
    required: &[String],
    evidence: &[EvidenceItem],
    authorised_equivalences: &[(String, String)],
    unauthorised_equivalences: &[(String, String)],
    prereq_edges: &[(String, String, u32)],
    goal: Option<&str>,
    out_scratch: &mut [u64],
) -> Result<GapResult, InstrumentError> {
    let req_h: Vec<u64> = required.iter().map(|iri| q_hash(iri)).collect();
    let mut held_h = Vec::new();
    let mut unresolved_h = Vec::new();
    for item in evidence {
        let h = q_hash(&item.capability);
        if item.available {
            push_unique(&mut held_h, h);
        } else {
            push_unique(&mut unresolved_h, h);
        }
    }
    let auth_eq: Vec<(u64, u64)> = authorised_equivalences
        .iter()
        .map(|(r, h)| (q_hash(r), q_hash(h)))
        .collect();

    for (req_iri, held_iri) in unauthorised_equivalences {
        let r = q_hash(req_iri);
        let h = q_hash(held_iri);
        if req_h.contains(&r) && held_h.contains(&h) && !is_held(r, &held_h, &auth_eq) {
            return Err(InstrumentError::UnauthorisedEquivalence);
        }
    }

    let _gap_n = capability_gap(&req_h, &held_h, &auth_eq, out_scratch);
    let closed = requirements_met(&req_h, &held_h, &auth_eq);

    let mut satisfied = Vec::new();
    let mut equivalent = Vec::new();
    let mut unresolved = Vec::new();
    let mut unmet = Vec::new();
    for (iri, &h) in required.iter().zip(req_h.iter()) {
        if satisfied.contains(iri)
            || equivalent.contains(iri)
            || unresolved.contains(iri)
            || unmet.contains(iri)
        {
            continue;
        }
        if held_h.contains(&h) {
            satisfied.push(iri.clone());
        } else if is_held(h, &held_h, &auth_eq) {
            equivalent.push(iri.clone());
        } else if unresolved_h.contains(&h) {
            unresolved.push(iri.clone());
        } else {
            unmet.push(iri.clone());
        }
    }

    let learning_path_cost = if prereq_edges.is_empty() {
        None
    } else {
        path_cost(
            required,
            evidence,
            prereq_edges,
            goal,
            &held_h,
            closed,
            &unmet,
            &unresolved,
        )
    };

    Ok(GapResult {
        satisfied,
        equivalent,
        unresolved,
        unmet,
        learning_path_cost,
    })
}

/// Issue only when an authorised issuer acts. A runner pass is not issuance.
pub fn maybe_issue_award(
    proposed_pass: bool,
    issuer_authorised: bool,
    award: CapabilityAward,
    public_omit_private: bool,
    evidence: &[EvidenceItem],
) -> Result<InstrumentAttestation, InstrumentError> {
    if proposed_pass && !issuer_authorised {
        return Err(InstrumentError::EvaluatorPassIsNotIssuance);
    }
    if public_omit_private && evidence.iter().any(|item| item.private) {
        return Err(InstrumentError::PrivateEvidence);
    }
    issue_capability_award(award)
}

fn is_held(cap: u64, held: &[u64], equivalences: &[(u64, u64)]) -> bool {
    held.contains(&cap)
        || equivalences
            .iter()
            .any(|&(req, h)| req == cap && held.contains(&h))
}

fn push_unique(ids: &mut Vec<u64>, id: u64) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

fn path_cost(
    required: &[String],
    evidence: &[EvidenceItem],
    prereq_edges: &[(String, String, u32)],
    goal: Option<&str>,
    held_h: &[u64],
    closed: bool,
    unmet: &[String],
    unresolved: &[String],
) -> Option<u32> {
    let goal_iri = goal
        .map(str::to_owned)
        .or_else(|| unmet.first().cloned())
        .or_else(|| unresolved.first().cloned())
        .or_else(|| required.first().cloned())?;
    let goal_h = q_hash(&goal_iri);
    if closed || held_h.contains(&goal_h) {
        return learning_path_cost_for(required, evidence, prereq_edges, goal_h, held_h)
            .or(Some(0));
    }
    learning_path_cost_for(required, evidence, prereq_edges, goal_h, held_h)
}

fn learning_path_cost_for(
    required: &[String],
    evidence: &[EvidenceItem],
    prereq_edges: &[(String, String, u32)],
    goal_h: u64,
    held_h: &[u64],
) -> Option<u32> {
    let mut nodes = Vec::new();
    for iri in required {
        push_unique(&mut nodes, q_hash(iri));
    }
    for item in evidence {
        push_unique(&mut nodes, q_hash(&item.capability));
    }
    let mut edges = Vec::with_capacity(prereq_edges.len());
    for (from, to, cost) in prereq_edges {
        let f = q_hash(from);
        let t = q_hash(to);
        push_unique(&mut nodes, f);
        push_unique(&mut nodes, t);
        edges.push((f, t, *cost));
    }
    push_unique(&mut nodes, goal_h);
    if nodes.len() > MAX_CAP_NODES {
        return None;
    }
    learning_path_cost(&nodes, &edges, held_h, goal_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMAL: &str = "cap:formalDegree";
    const EXPERIENTIAL: &str = "cap:apprenticeship";
    const WELDING: &str = "cap:welding";
    const FAB: &str = "cap:fabrication";
    const ROBOTICS: &str = "cap:robotics";
    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn item(cap: &str, basis: EvidenceBasis, available: bool) -> EvidenceItem {
        EvidenceItem {
            capability: cap.into(),
            basis,
            available,
            private: false,
        }
    }

    fn sample_award() -> CapabilityAward {
        CapabilityAward {
            learner: "did:example:learner".into(),
            assessment_release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0"
                .into(),
            content_digest: DIGEST.into(),
            issuer: "did:example:issuer".into(),
            capability: "https://ns.webizen.org/demo/concepts/units".into(),
            issued_at: 1_700_000_000,
            valid_until: 1_800_000_000,
        }
    }

    #[test]
    fn formal_evidence_satisfies() {
        let required = vec![FORMAL.into()];
        let evidence = [item(FORMAL, EvidenceBasis::Formal, true)];
        let got = evaluate_rpl(
            &required,
            &evidence,
            &[],
            &[],
            &[],
            None,
            &mut [0u64; 8],
        )
        .expect("rpl");
        assert_eq!(got.satisfied, vec![FORMAL.to_string()]);
        assert!(got.equivalent.is_empty());
        assert!(got.unresolved.is_empty());
        assert!(got.unmet.is_empty());
        assert_eq!(got.learning_path_cost, None);
    }

    #[test]
    fn experiential_authorised_equivalence_satisfies_required_formal() {
        let required = vec![FORMAL.into()];
        let evidence = [item(EXPERIENTIAL, EvidenceBasis::Experiential, true)];
        let auth = [(FORMAL.into(), EXPERIENTIAL.into())];
        let got = evaluate_rpl(
            &required,
            &evidence,
            &auth,
            &[],
            &[],
            None,
            &mut [0u64; 8],
        )
        .expect("rpl");
        assert!(got.satisfied.is_empty());
        assert_eq!(got.equivalent, vec![FORMAL.to_string()]);
        assert!(got.unmet.is_empty());
        assert!(got.unresolved.is_empty());
    }

    #[test]
    fn unauthorised_equivalence_errors() {
        let required = vec![FORMAL.into()];
        let evidence = [item(EXPERIENTIAL, EvidenceBasis::Experiential, true)];
        let unauth = [(FORMAL.into(), EXPERIENTIAL.into())];
        let err = evaluate_rpl(
            &required,
            &evidence,
            &[],
            &unauth,
            &[],
            None,
            &mut [0u64; 8],
        )
        .expect_err("must fail closed");
        assert_eq!(err, InstrumentError::UnauthorisedEquivalence);
    }

    #[test]
    fn unavailable_evidence_is_unresolved_not_unmet() {
        let required = vec![FORMAL.into()];
        let evidence = [item(FORMAL, EvidenceBasis::PriorLearning, false)];
        let got = evaluate_rpl(
            &required,
            &evidence,
            &[],
            &[],
            &[],
            None,
            &mut [0u64; 8],
        )
        .expect("rpl");
        assert_eq!(got.unresolved, vec![FORMAL.to_string()]);
        assert!(!got.unmet.contains(&FORMAL.to_string()));
        assert!(got.satisfied.is_empty());
    }

    #[test]
    fn learning_path_cost_some_when_edges_provided() {
        let required = vec![ROBOTICS.into()];
        let evidence = [item(WELDING, EvidenceBasis::PeerAttested, true)];
        let edges = [
            (WELDING.into(), FAB.into(), 2u32),
            (FAB.into(), ROBOTICS.into(), 3u32),
            (WELDING.into(), ROBOTICS.into(), 10u32),
        ];
        let got = evaluate_rpl(
            &required,
            &evidence,
            &[],
            &[],
            &edges,
            Some(ROBOTICS),
            &mut [0u64; 8],
        )
        .expect("rpl");
        assert_eq!(got.learning_path_cost, Some(5));
        assert_eq!(got.unmet, vec![ROBOTICS.to_string()]);
    }

    #[test]
    fn evaluator_pass_without_issuer_is_not_issuance() {
        let err = maybe_issue_award(true, false, sample_award(), false, &[])
            .expect_err("pass is not issuance");
        assert_eq!(err, InstrumentError::EvaluatorPassIsNotIssuance);
        let issued = maybe_issue_award(true, true, sample_award(), false, &[]).expect("issue");
        assert_ne!(issued.award_subject, issued.assessment_release_id);
    }

    #[test]
    fn private_evidence_blocked_on_public_award() {
        let mut private = item(FORMAL, EvidenceBasis::Formal, true);
        private.private = true;
        let err = maybe_issue_award(true, true, sample_award(), true, &[private])
            .expect_err("must omit private");
        assert_eq!(err, InstrumentError::PrivateEvidence);
    }
}
