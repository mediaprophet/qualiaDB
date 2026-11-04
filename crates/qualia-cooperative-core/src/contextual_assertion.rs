//! Contextual Assertions, Provenance, Verification, and Challenge Model.
//!
//! Implements `C:\github\license\CONTEXTUAL-ASSERTIONS.md`:
//! - `StatementType`: `Observation`, `Inference`, `Allegation`, `Attestation`, `AdjudicatedFinding`
//! - `AssertionStatus`: `Active`, `Challenged`, `Restricted`, `Expired`, `Withdrawn`, `Superseded`
//! - `VerificationStatus`: `Proposed`, `Attested`, `Verified`, `Disputed`, `Adjudicated`
//!
//! Core invariants enforced by pure validation functions:
//! 1. **Single-Source Inference Deduplication & No Auto-Promotion**: Repetition or multiple
//!    dependent inferences sharing a `root_source_id` never count as independent corroboration
//!    and never auto-promote an `Allegation` or `Inference` into an `AdjudicatedFinding` or
//!    `Observation`.
//! 2. **Challenge Preservation & Downstream Review**: Challenging an assertion preserves both
//!    the challenged assertion and the challenge record, and marks all downstream dependent
//!    decisions for review.
//! 3. **Zero Universal Human Reputation Score**: Assertions remain scoped to their specific
//!    domain/context; deriving a collapsed scalar human reputation score is rejected.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::record::{q_hash_str, NQuin, SensitivityClass};

/// Epistemic category of a contextual statement (`CONTEXTUAL-ASSERTIONS.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementType {
    Observation,
    Inference,
    Allegation,
    Attestation,
    AdjudicatedFinding,
}

impl StatementType {
    /// Stable string identifier.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Inference => "inference",
            Self::Allegation => "allegation",
            Self::Attestation => "attestation",
            Self::AdjudicatedFinding => "adjudicated_finding",
        }
    }
}

/// Lifecycle state of an assertion (`CONTEXTUAL-ASSERTIONS.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionStatus {
    Active,
    Challenged,
    Restricted,
    Expired,
    Withdrawn,
    Superseded,
}

/// Verification posture of an assertion (`CONTEXTUAL-ASSERTIONS.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Proposed,
    Attested,
    Verified,
    Disputed,
    Adjudicated,
}

/// A contextual, provenance-anchored statement (`CONTEXTUAL-ASSERTIONS.md` §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextualAssertion {
    pub id: String,
    pub subject_uri: String,
    pub statement_type: StatementType,
    pub assertion_status: AssertionStatus,
    pub verification_status: VerificationStatus,
    pub claim_summary: String,
    pub author_did: String,
    /// Identifier of the root primary source or sensor event from which this statement originates.
    /// Dependent inferences derived from the same root source share `root_source_id`.
    pub root_source_id: String,
    pub derived_from_assertion_ids: Vec<String>,
    pub scope_domain: String,
    pub sensitivity: SensitivityClass,
    pub asserted_at_unix: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjudicated_by_human_did: Option<String>,
}

impl ContextualAssertion {
    /// Zero-heap compilation of a `ContextualAssertion` into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        let mut written = 0;
        let s = q_hash_str(&self.id);
        let ctx = q_hash_str(&self.scope_domain);
        let meta = self.sensitivity.to_metadata_mask() | (self.asserted_at_unix as u64);
        let triples = [
            (
                q_hash_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                q_hash_str(self.statement_type.as_str()),
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/hro#subjectUri"),
                q_hash_str(&self.subject_uri),
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/hro#rootSourceId"),
                q_hash_str(&self.root_source_id),
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/hro#assertionStatus"),
                self.assertion_status as u64,
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/hro#verificationStatus"),
                self.verification_status as u64,
            ),
        ];
        for (p, o) in triples {
            if written >= out.len() {
                break;
            }
            out[written] = NQuin::new(s, p, o, ctx, meta);
            written += 1;
        }
        written
    }
}

/// Formal challenge filed against an assertion (`CONTEXTUAL-ASSERTIONS.md` §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengeRecord {
    pub id: String,
    pub target_assertion_id: String,
    pub challenger_did: String,
    pub grounds: String,
    pub counter_evidence_ids: Vec<String>,
    pub challenged_at_unix: u32,
}

/// Downstream governance, accounting, or eligibility decision that depends on one or more assertions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownstreamDecisionRef {
    pub decision_id: String,
    pub depends_on_assertion_ids: Vec<String>,
    pub requires_review: bool,
    pub review_reason: Option<String>,
}

/// Count independent corroborating sources across a slice of assertions.
/// Multiple assertions or dependent inferences sharing a `root_source_id` count at most ONCE,
/// and `Withdrawn` or `Expired` assertions are excluded.
pub fn count_independent_corroborations(assertions: &[ContextualAssertion]) -> usize {
    let mut distinct_roots: HashSet<&str> = HashSet::new();
    for a in assertions {
        if matches!(
            a.assertion_status,
            AssertionStatus::Withdrawn | AssertionStatus::Expired
        ) {
            continue;
        }
        let root = a.root_source_id.trim();
        if !root.is_empty() {
            distinct_roots.insert(root);
        }
    }
    distinct_roots.len()
}

/// Validate a proposed transition in `StatementType` or `VerificationStatus`:
/// 1. An `Inference` or `Allegation` can NEVER be promoted to an `Observation` (an observation
///    is direct primary witness/device data, never synthesized by repetition).
/// 2. Repetition or multiple dependent inferences sharing a `root_source_id` NEVER auto-promote
///    an `Allegation` or `Inference` into an `AdjudicatedFinding` or `Verified` status.
/// 3. Promoting to `StatementType::AdjudicatedFinding` or `VerificationStatus::Adjudicated`
///    requires an explicit `human_adjudicator_did` and at least one independent corroborating
///    record (or formal human adjudication record).
pub fn validate_assertion_promotion(
    current: &ContextualAssertion,
    target_type: StatementType,
    target_verification: VerificationStatus,
    human_adjudicator_did: Option<&str>,
    corroborating: &[ContextualAssertion],
) -> Result<ContextualAssertion, String> {
    if target_type == StatementType::Observation
        && current.statement_type != StatementType::Observation
    {
        return Err(format!(
            "cannot promote {:?} into Observation: observations must originate directly from primary capture",
            current.statement_type
        ));
    }

    let independent_sources = count_independent_corroborations(corroborating);

    if matches!(
        current.statement_type,
        StatementType::Inference | StatementType::Allegation
    ) && target_verification == VerificationStatus::Verified
        && independent_sources < 2
    {
        return Err(
            "dependent inferences or repeated statements from a single root_source_id cannot verify an Inference or Allegation"
                .to_string(),
        );
    }

    if target_type == StatementType::AdjudicatedFinding
        || target_verification == VerificationStatus::Adjudicated
    {
        let Some(adjudicator) = human_adjudicator_did.map(str::trim).filter(|s| !s.is_empty())
        else {
            return Err(
                "promotion to AdjudicatedFinding requires an explicit human adjudicator DID; repetition never auto-promotes"
                    .to_string(),
            );
        };
        let mut promoted = current.clone();
        promoted.statement_type = target_type;
        promoted.verification_status = target_verification;
        promoted.adjudicated_by_human_did = Some(adjudicator.to_string());
        if current.statement_type == StatementType::Allegation {
            promoted.sensitivity = SensitivityClass::Restricted;
        }
        return Ok(promoted);
    }

    let mut updated = current.clone();
    updated.statement_type = target_type;
    updated.verification_status = target_verification;
    Ok(updated)
}

/// Apply a `ChallengeRecord` against `target_assertion_id`:
/// - Preserves the original assertion record (transitioning it to `AssertionStatus::Challenged`
///   and `VerificationStatus::Disputed`, and restricting sensitivity if it was an `Allegation`),
///   alongside the `ChallengeRecord`.
/// - Marks every downstream decision in `downstream_decisions` that depends on `target_assertion_id`
///   (or on any assertion derived from it) with `requires_review = true`.
pub fn apply_assertion_challenge(
    assertions: &[ContextualAssertion],
    challenge: ChallengeRecord,
    downstream_decisions: &[DownstreamDecisionRef],
) -> (
    Vec<ContextualAssertion>,
    ChallengeRecord,
    Vec<DownstreamDecisionRef>,
) {
    let target_id = challenge.target_assertion_id.as_str();
    let mut affected_assertion_ids: HashSet<String> = HashSet::new();
    affected_assertion_ids.insert(target_id.to_string());

    let mut updated_assertions: Vec<ContextualAssertion> = Vec::with_capacity(assertions.len());
    for a in assertions {
        let mut copy = a.clone();
        if copy.id == target_id
            || copy
                .derived_from_assertion_ids
                .iter()
                .any(|dep| affected_assertion_ids.contains(dep))
        {
            copy.assertion_status = AssertionStatus::Challenged;
            copy.verification_status = VerificationStatus::Disputed;
            if copy.statement_type == StatementType::Allegation {
                copy.sensitivity = SensitivityClass::Restricted;
            }
            affected_assertion_ids.insert(copy.id.clone());
        }
        updated_assertions.push(copy);
    }

    let updated_decisions: Vec<DownstreamDecisionRef> = downstream_decisions
        .iter()
        .map(|d| {
            let mut copy = d.clone();
            if copy
                .depends_on_assertion_ids
                .iter()
                .any(|id| affected_assertion_ids.contains(id))
            {
                copy.requires_review = true;
                copy.review_reason = Some(format!(
                    "upstream assertion {} challenged by {}: {}",
                    challenge.target_assertion_id, challenge.challenger_did, challenge.grounds
                ));
            }
            copy
        })
        .collect();

    (updated_assertions, challenge, updated_decisions)
}

/// Enforce the anti-universal-reputation rule (`CONTEXTUAL-ASSERTIONS.md` §1.4 & §4.5):
/// Zero universal human reputation score or cross-context scalar person ranking may ever be derived.
pub fn validate_contextual_evaluation_scope(
    scope_domain: &str,
    collapses_person_to_universal_scalar: bool,
) -> Result<(), String> {
    if collapses_person_to_universal_scalar {
        return Err(
            "deriving a universal human reputation score or scalar person ranking is forbidden"
                .to_string(),
        );
    }
    let normalized = scope_domain.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || normalized == "universal"
        || normalized == "global_reputation"
        || normalized == "social_credit"
    {
        return Err(format!(
            "invalid evaluation scope '{scope_domain}': assertions must be bounded to a specific domain/context"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_assertion(
        id: &str,
        st: StatementType,
        root_source: &str,
    ) -> ContextualAssertion {
        ContextualAssertion {
            id: id.into(),
            subject_uri: "did:wf:contributor-1".into(),
            statement_type: st,
            assertion_status: AssertionStatus::Active,
            verification_status: VerificationStatus::Proposed,
            claim_summary: format!("claim-{id}"),
            author_did: "did:wf:reviewer-1".into(),
            root_source_id: root_source.into(),
            derived_from_assertion_ids: Vec::new(),
            scope_domain: "coop:project:solar:code-review".into(),
            sensitivity: SensitivityClass::Restricted,
            asserted_at_unix: 1_700_000_000,
            expires_at_unix: None,
            superseded_by_id: None,
            adjudicated_by_human_did: None,
        }
    }

    #[test]
    fn dependent_inferences_from_one_root_source_do_not_corroborate_or_auto_promote() {
        let base_allegation = sample_assertion("a1", StatementType::Allegation, "src-log-1");
        let inf_1 = sample_assertion("inf-1", StatementType::Inference, "src-log-1");
        let inf_2 = sample_assertion("inf-2", StatementType::Inference, "src-log-1");
        let inf_3 = sample_assertion("inf-3", StatementType::Inference, "src-log-1");

        let group = vec![base_allegation.clone(), inf_1, inf_2, inf_3];
        assert_eq!(
            count_independent_corroborations(&group),
            1,
            "four statements sharing root_source_id must count as 1 source, not 4"
        );

        // Cannot verify from single root source
        assert!(validate_assertion_promotion(
            &base_allegation,
            StatementType::Allegation,
            VerificationStatus::Verified,
            None,
            &group
        )
        .is_err());

        // Cannot promote Allegation or Inference into Observation
        assert!(validate_assertion_promotion(
            &base_allegation,
            StatementType::Observation,
            VerificationStatus::Verified,
            Some("did:wf:steward"),
            &group
        )
        .is_err());

        // Cannot auto-promote into AdjudicatedFinding without human adjudicator DID
        assert!(validate_assertion_promotion(
            &base_allegation,
            StatementType::AdjudicatedFinding,
            VerificationStatus::Adjudicated,
            None,
            &group
        )
        .is_err());

        // Explicit human adjudicator DID succeeds
        let adj = validate_assertion_promotion(
            &base_allegation,
            StatementType::AdjudicatedFinding,
            VerificationStatus::Adjudicated,
            Some("did:wf:human-ombuds"),
            &group,
        )
        .expect("human adjudicator promotes finding");
        assert_eq!(adj.statement_type, StatementType::AdjudicatedFinding);
        assert_eq!(
            adj.adjudicated_by_human_did.as_deref(),
            Some("did:wf:human-ombuds")
        );
    }

    #[test]
    fn challenging_assertion_preserves_both_and_marks_downstream_decisions_for_review() {
        let a1 = sample_assertion("a1", StatementType::Allegation, "src-1");
        let mut derived = sample_assertion("a2", StatementType::Inference, "src-1");
        derived.derived_from_assertion_ids = vec!["a1".into()];

        let decisions = vec![
            DownstreamDecisionRef {
                decision_id: "dec-payout-hold".into(),
                depends_on_assertion_ids: vec!["a2".into()],
                requires_review: false,
                review_reason: None,
            },
            DownstreamDecisionRef {
                decision_id: "dec-unrelated".into(),
                depends_on_assertion_ids: vec!["other-assertion".into()],
                requires_review: false,
                review_reason: None,
            },
        ];

        let challenge = ChallengeRecord {
            id: "chal-1".into(),
            target_assertion_id: "a1".into(),
            challenger_did: "did:wf:contributor-1".into(),
            grounds: "Timestamped commit log proves authorship".into(),
            counter_evidence_ids: vec!["urn:git:commit:999".into()],
            challenged_at_unix: 1_700_000_500,
        };

        let (updated_assertions, preserved_challenge, updated_decisions) =
            apply_assertion_challenge(&[a1, derived], challenge, &decisions);

        assert_eq!(updated_assertions.len(), 2);
        assert_eq!(updated_assertions[0].assertion_status, AssertionStatus::Challenged);
        assert_eq!(updated_assertions[0].verification_status, VerificationStatus::Disputed);
        assert_eq!(updated_assertions[1].assertion_status, AssertionStatus::Challenged);
        assert_eq!(preserved_challenge.id, "chal-1");
        assert!(updated_decisions[0].requires_review);
        assert!(!updated_decisions[1].requires_review);
    }

    #[test]
    fn universal_human_reputation_score_is_rejected() {
        assert!(validate_contextual_evaluation_scope("coop:project:1", true).is_err());
        assert!(validate_contextual_evaluation_scope("universal", false).is_err());
        assert!(validate_contextual_evaluation_scope("global_reputation", false).is_err());
        assert!(validate_contextual_evaluation_scope("coop:project:1", false).is_ok());
    }
}
