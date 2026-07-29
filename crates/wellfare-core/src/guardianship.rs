//! Guardianship approval escrow — M-of-N co-signature for proxy actions.
//!
//! **Supported agency, not warden control.** When an agent acting *on behalf of* a principal
//! (a proxy / supporter) writes a protected record, the write does not auto-commit. It is held
//! in escrow as a [`GuardianshipProposal`]; designated guardians co-sign with immutable
//! [`GuardianshipVote`] records. The current status is a **derived projection** over the votes
//! (latest vote per guardian) — replay-safe, so duplicated / reordered / replayed co-signatures
//! never change the outcome. On reaching the approval threshold the *actual* escrowed record
//! commits through the normal signed vault path.
//!
//! The escrow protects the **principal** from an ill-considered or erring supporter — a
//! co-guardian objection halts the escrow (a protective veto). That guards the principal's
//! agency against a rogue proxy; it is not a warden overriding the principal, who is not the
//! actor here.
//!
//! Merge discipline mirrors the rest of `wellfare-core`: proposals and votes are immutable
//! append-only records merged by stable id; status is *derived*, never a mutated field.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

/// A proxy write held pending M-of-N guardian co-signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianshipProposal {
    pub id: String,
    /// The principal on whose behalf the proxy acted (the escrowed record's owner).
    pub principal_did: String,
    /// The agent that proposed the write (the proxy / supporter).
    pub proxy_did: String,
    /// Number of distinct guardian approvals required to ratify (always ≥ 1).
    pub threshold: u8,
    /// Journal kind of the escrowed record (for the tray label / filtering).
    pub escrowed_kind: String,
    /// The escrowed [`RecordEnvelope`], serialized — committed verbatim on ratification.
    pub escrowed_envelope_json: String,
    /// The escrowed record's UI summary, re-attached on commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escrowed_summary: Option<String>,
    /// Human-readable reason the write needs co-signature.
    pub reason: String,
    pub created_unix: u32,
}

impl GuardianshipProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal_did: impl Into<String>,
        proxy_did: impl Into<String>,
        threshold: u8,
        escrowed: &RecordEnvelope,
        escrowed_summary: Option<String>,
        reason: impl Into<String>,
        created_unix: u32,
    ) -> Self {
        let escrowed_kind = crate::conditions::journal_kind_for_record_id(&escrowed.id).to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            principal_did: principal_did.into(),
            proxy_did: proxy_did.into(),
            threshold: threshold.max(1),
            escrowed_kind,
            escrowed_envelope_json: serde_json::to_string(escrowed).unwrap_or_default(),
            escrowed_summary,
            reason: reason.into(),
            created_unix,
        }
    }

    /// Deserialize the escrowed envelope (committed verbatim on ratification).
    pub fn escrowed_envelope(&self) -> Option<RecordEnvelope> {
        serde_json::from_str(&self.escrowed_envelope_json).ok()
    }

    /// The escrowed record id — the stable key used for idempotent commit-on-ratify.
    pub fn escrowed_record_id(&self) -> Option<String> {
        self.escrowed_envelope().map(|e| e.id)
    }
}

/// A guardian's immutable co-signature (approve) or objection (deny) on a proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianshipVote {
    pub id: String,
    pub proposal_id: String,
    pub guardian_did: String,
    pub approve: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub voted_unix: u32,
}

impl GuardianshipVote {
    pub fn new(
        proposal_id: impl Into<String>,
        guardian_did: impl Into<String>,
        approve: bool,
        reason: Option<String>,
        voted_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            proposal_id: proposal_id.into(),
            guardian_did: guardian_did.into(),
            approve,
            reason,
            voted_unix,
        }
    }
}

/// Terminal / in-flight state of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Pending,
    Ratified,
    Denied,
}

/// Derived status of a proposal — a pure projection over the votes, never a stored field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalStatus {
    pub state: ProposalState,
    pub approvals: u8,
    pub threshold: u8,
    /// Set when a guardian objection halted the escrow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denied_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denial_reason: Option<String>,
}

/// Derive the status of a proposal from its votes.
///
/// Replay-safe: only the **latest vote per guardian** counts (by `voted_unix`, later-seen wins on
/// tie), so duplicated or reordered co-signatures converge to the same result. A guardian's latest
/// vote being an objection halts the escrow (protective veto); otherwise the proposal ratifies once
/// `approvals ≥ threshold`.
pub fn derive_status(
    proposal: &GuardianshipProposal,
    votes: &[GuardianshipVote],
) -> ProposalStatus {
    // Latest vote per guardian for this proposal.
    let mut latest: Vec<&GuardianshipVote> = Vec::new();
    for v in votes.iter().filter(|v| v.proposal_id == proposal.id) {
        if let Some(slot) = latest.iter_mut().find(|s| s.guardian_did == v.guardian_did) {
            if v.voted_unix >= slot.voted_unix {
                *slot = v;
            }
        } else {
            latest.push(v);
        }
    }

    let approvals = latest.iter().filter(|v| v.approve).count() as u8;

    // A standing objection halts the escrow.
    if let Some(deny) = latest.iter().find(|v| !v.approve) {
        return ProposalStatus {
            state: ProposalState::Denied,
            approvals,
            threshold: proposal.threshold,
            denied_by: Some(deny.guardian_did.clone()),
            denial_reason: deny.reason.clone(),
        };
    }

    let state = if approvals >= proposal.threshold {
        ProposalState::Ratified
    } else {
        ProposalState::Pending
    };
    ProposalStatus {
        state,
        approvals,
        threshold: proposal.threshold,
        denied_by: None,
        denial_reason: None,
    }
}

pub fn proposal_record_id(uuid: &str) -> String {
    format!("urn:wellfair:guardianship_proposal:{uuid}")
}

pub fn vote_record_id(uuid: &str) -> String {
    format!("urn:wellfair:guardianship_vote:{uuid}")
}

/// Governance envelope for a proposal/vote record.
///
/// `proxy_did` is intentionally `None`: these governance records must not themselves be proxy
/// actions, or writing them would recurse back into the escrow. They inherit the escrowed
/// record's sensitivity so the Sanctuary projection guards them identically.
fn governance_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    sensitivity: SensitivityClass,
    asserted_unix: u32,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(asserted_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    }
}

pub fn build_proposal_envelope(
    proposal: &GuardianshipProposal,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    let sensitivity = proposal
        .escrowed_envelope()
        .map(|e| e.sensitivity)
        .unwrap_or(SensitivityClass::Restricted);
    let id = proposal_record_id(&proposal.id);
    governance_envelope(&id, owner_did, author_did, sensitivity, asserted_unix)
}

pub fn build_vote_envelope(
    vote: &GuardianshipVote,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    let id = vote_record_id(&vote.id);
    governance_envelope(
        &id,
        owner_did,
        author_did,
        SensitivityClass::Restricted,
        asserted_unix,
    )
}

pub fn proposal_summary(proposal: &GuardianshipProposal) -> String {
    serde_json::to_string(proposal).unwrap_or_default()
}

pub fn vote_summary(vote: &GuardianshipVote) -> String {
    serde_json::to_string(vote).unwrap_or_default()
}

pub fn parse_proposal_summary(summary: &str) -> Option<GuardianshipProposal> {
    serde_json::from_str(summary).ok()
}

pub fn parse_vote_summary(summary: &str) -> Option<GuardianshipVote> {
    serde_json::from_str(summary).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escrowed(owner: &str, proxy: &str) -> RecordEnvelope {
        RecordEnvelope {
            id: "urn:wellfair:condition:e1".into(),
            owner_did: owner.into(),
            author_did: proxy.into(),
            proxy_did: Some(proxy.into()),
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: Some(1_700_000_000),
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        }
    }

    fn proposal() -> GuardianshipProposal {
        let env = escrowed("did:wf:principal", "did:wf:proxy");
        GuardianshipProposal::new(
            "did:wf:principal",
            "did:wf:proxy",
            2,
            &env,
            Some("{\"label\":\"Asthma\"}".into()),
            "Proxy write on protected record",
            1_700_000_000,
        )
    }

    #[test]
    fn new_proposal_infers_kind_and_escrows_envelope() {
        let p = proposal();
        assert_eq!(p.escrowed_kind, "condition");
        assert_eq!(p.threshold, 2);
        let env = p.escrowed_envelope().unwrap();
        assert_eq!(env.id, "urn:wellfair:condition:e1");
        assert_eq!(
            p.escrowed_record_id().as_deref(),
            Some("urn:wellfair:condition:e1")
        );
    }

    #[test]
    fn threshold_is_floored_to_one() {
        let env = escrowed("o", "p");
        let p = GuardianshipProposal::new("o", "p", 0, &env, None, "r", 1);
        assert_eq!(p.threshold, 1);
    }

    #[test]
    fn pending_until_threshold_reached() {
        let p = proposal();
        let votes = vec![GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 10)];
        let s = derive_status(&p, &votes);
        assert_eq!(s.state, ProposalState::Pending);
        assert_eq!(s.approvals, 1);
        assert_eq!(s.threshold, 2);
    }

    #[test]
    fn ratifies_at_threshold_with_distinct_guardians() {
        let p = proposal();
        let votes = vec![
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 10),
            GuardianshipVote::new(&p.id, "did:wf:g2", true, None, 11),
        ];
        let s = derive_status(&p, &votes);
        assert_eq!(s.state, ProposalState::Ratified);
        assert_eq!(s.approvals, 2);
    }

    #[test]
    fn duplicate_and_replayed_votes_do_not_inflate_approvals() {
        let p = proposal();
        // g1 votes three times (duplicate + replay); still one distinct approval → never ratifies.
        let votes = vec![
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 10),
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 12),
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 11),
        ];
        let s = derive_status(&p, &votes);
        assert_eq!(s.approvals, 1);
        assert_eq!(s.state, ProposalState::Pending);
    }

    #[test]
    fn guardian_may_change_mind_latest_vote_wins() {
        let p = proposal();
        // g1 approves then later objects; the standing objection halts.
        let votes = vec![
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 10),
            GuardianshipVote::new(&p.id, "did:wf:g2", true, None, 10),
            GuardianshipVote::new(&p.id, "did:wf:g1", false, Some("reconsidered".into()), 20),
        ];
        let s = derive_status(&p, &votes);
        assert_eq!(s.state, ProposalState::Denied);
        assert_eq!(s.denied_by.as_deref(), Some("did:wf:g1"));
        assert_eq!(s.denial_reason.as_deref(), Some("reconsidered"));
    }

    #[test]
    fn objection_halts_even_past_threshold() {
        let p = proposal();
        // Two approvals would ratify, but a third guardian objects → protective veto.
        let votes = vec![
            GuardianshipVote::new(&p.id, "did:wf:g1", true, None, 10),
            GuardianshipVote::new(&p.id, "did:wf:g2", true, None, 11),
            GuardianshipVote::new(
                &p.id,
                "did:wf:g3",
                false,
                Some("not in her interest".into()),
                12,
            ),
        ];
        let s = derive_status(&p, &votes);
        assert_eq!(s.state, ProposalState::Denied);
    }

    #[test]
    fn votes_for_other_proposals_are_ignored() {
        let p = proposal();
        let votes = vec![
            GuardianshipVote::new("other-proposal", "did:wf:g1", true, None, 10),
            GuardianshipVote::new("other-proposal", "did:wf:g2", true, None, 11),
        ];
        let s = derive_status(&p, &votes);
        assert_eq!(s.approvals, 0);
        assert_eq!(s.state, ProposalState::Pending);
    }

    #[test]
    fn summary_round_trips() {
        let p = proposal();
        let parsed = parse_proposal_summary(&proposal_summary(&p)).unwrap();
        assert_eq!(parsed, p);

        let v = GuardianshipVote::new(&p.id, "did:wf:g1", true, Some("ok".into()), 10);
        let pv = parse_vote_summary(&vote_summary(&v)).unwrap();
        assert_eq!(pv, v);
    }

    #[test]
    fn proposal_envelope_inherits_escrowed_sensitivity_and_is_not_proxy() {
        let p = proposal();
        let env = build_proposal_envelope(&p, "did:wf:principal", "did:wf:proxy", 1_700_000_000);
        assert!(env.id.contains(":guardianship_proposal:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert!(
            env.proxy_did.is_none(),
            "governance records must not recurse into escrow"
        );
    }
}
