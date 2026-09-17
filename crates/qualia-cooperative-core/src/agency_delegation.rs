//! `AgencyDelegation` — the integrating record for **supported agency** (ADR §7–§10).
//!
//! Binds a principal to their agent(s) for a *domain of agency*, under an [`AuthorityProfile`]
//! (modality × trigger × accountability), gated by a [`Trigger`], anchored to a shared-values
//! credential (default UN-HR / UNCRC), scoped by ABAC attributes + jurisdiction + precedence
//! (the backpacker case: family Primary, a local housemate LocalTemporary), with validity,
//! consent, revocation, an evidence-chain reference, and — for the child-maturation case — a
//! monotonic developmental transfer schedule.
//!
//! [`delegation_permits`] is the fail-closed ABAC evaluator. Two invariants are load-bearing:
//! **selfhood is never delegated by default** (a Selfhood-sphere request is denied unless the
//! delegation carries an explicit selfhood grant), and **consequential-domain judgements require
//! declared provenance + an epistemic horizon**.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wellfare_core::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

use crate::agency_domain::is_consequential;
use crate::authority_type::AuthorityProfile;
use crate::provenance::JudgementProvenance;
use crate::taxonomy::{Sphere, Taxonomy, TermId};
use crate::trigger::{evaluate as evaluate_trigger, Trigger, TriggerContext};

/// Precedence among agents/delegations. The backpacker case: family back home is `Primary`, a
/// local housemate who can help in an emergency is `LocalTemporary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Precedence {
    #[default]
    Primary,
    Secondary,
    LocalTemporary,
}

/// Consent state of the principal. Fail-closed: `Pending` is not effective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConsentState {
    #[default]
    Pending,
    Granted,
    Withdrawn,
    /// The delegation's origination doesn't require the principal's consent (e.g. adjudicated).
    NotRequired,
}

impl ConsentState {
    fn is_effective(self) -> bool {
        matches!(self, ConsentState::Granted | ConsentState::NotRequired)
    }
}

/// Control stage of a domain under a developmental (child-maturation) transfer. Monotonic:
/// authority only ever flows toward the principal (ADR §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlStage {
    GuardianSole,
    CoSigned,
    PrincipalSole,
}

/// One monotonic transfer step: at `trigger` (age milestone / capacity attestation / the child's
/// declarative claim), `domain` advances to `to_stage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStage {
    pub domain: TermId,
    pub to_stage: ControlStage,
    pub trigger: Trigger,
}

/// A supported-agency delegation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgencyDelegation {
    pub id: String,
    pub principal_did: String,
    pub agent_dids: Vec<String>,
    /// The domain of agency this delegation covers (an `agency_domain` term id).
    pub domain: TermId,
    #[serde(default)]
    pub authority: AuthorityProfile,
    /// When the authority activates. `None` = active whenever consent + validity hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    /// Shared-values credential anchor (a credential/instrument id). REQUIRED (ADR decision;
    /// default UN-HR / UNCRC), so no delegation is values-unanchored.
    pub values_anchor: String,
    /// ABAC scope attributes, e.g. ("data_class","medication"), ("action","read"),
    /// ("purpose","emergency-care"), ("selfhood_grant","explicit").
    #[serde(default)]
    pub scope: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<TermId>,
    #[serde(default)]
    pub precedence: Precedence,
    pub valid_from_unix: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to_unix: Option<u32>,
    #[serde(default)]
    pub consent: ConsentState,
    #[serde(default)]
    pub revoked: bool,
    /// Evidence-chain reference (a provenance / receipt record id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    /// Developmental transfer schedule (empty unless a developmental/scaffolding delegation).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transfer_schedule: Vec<TransferStage>,
}

impl AgencyDelegation {
    pub fn new(
        principal_did: impl Into<String>,
        domain: impl Into<TermId>,
        values_anchor: impl Into<String>,
        valid_from_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            principal_did: principal_did.into(),
            agent_dids: Vec::new(),
            domain: domain.into(),
            authority: AuthorityProfile::default(),
            trigger: None,
            values_anchor: values_anchor.into(),
            scope: Vec::new(),
            jurisdiction: None,
            precedence: Precedence::Primary,
            valid_from_unix,
            valid_to_unix: None,
            consent: ConsentState::Pending,
            revoked: false,
            evidence_ref: None,
            transfer_schedule: Vec::new(),
        }
    }

    fn scope_has(&self, key: &str, value: &str) -> bool {
        self.scope.iter().any(|(k, v)| k == key && v == value)
    }
}

pub fn agency_delegation_record_id(uuid: &str) -> String {
    format!("urn:qualia:agency-delegation:{uuid}")
}

pub fn build_agency_delegation_envelope(
    delegation: &AgencyDelegation,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    RecordEnvelope {
        id: agency_delegation_record_id(&delegation.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(delegation.valid_from_unix),
        valid_time_start_instant: None,
        valid_time_end_unix: delegation.valid_to_unix,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    }
}

pub fn agency_delegation_summary(delegation: &AgencyDelegation) -> String {
    serde_json::json!({
        "id": delegation.id,
        "principal_did": delegation.principal_did,
        "agent_dids": delegation.agent_dids,
        "domain": delegation.domain,
        "precedence": delegation.precedence,
        "consent": delegation.consent,
        "revoked": delegation.revoked,
        "jurisdiction": delegation.jurisdiction,
    })
    .to_string()
}

/// The **lossless** JSON of a delegation — every field. Stored as the journal record's summary so
/// the full delegation (authority, trigger, scope, transfer schedule, …) reconstructs on read; the
/// lossy [`agency_delegation_summary`] is for compact projections only.
pub fn agency_delegation_full_json(delegation: &AgencyDelegation) -> String {
    serde_json::to_string(delegation).unwrap_or_default()
}

/// Reconstruct a delegation from its lossless JSON (as stored by [`agency_delegation_full_json`]).
pub fn parse_agency_delegation(json: &str) -> Option<AgencyDelegation> {
    serde_json::from_str(json).ok()
}

// ---------------------------------------------------------------------------
// ABAC evaluation
// ---------------------------------------------------------------------------

/// A request to act under a delegation. `provenance` carries the declared judgement provenance
/// (required for consequential-domain judgements).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessRequest {
    pub domain: TermId,
    #[serde(default)]
    pub data_class: String,
    /// "read" | "write" | "decide" — write/decide are *judgements* for the consequential rule.
    pub action: String,
    #[serde(default)]
    pub sphere: Sphere,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<TermId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<JudgementProvenance>,
}

impl AccessRequest {
    fn is_judgement(&self) -> bool {
        matches!(self.action.as_str(), "write" | "decide")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDecision {
    Permit,
    Deny(String),
}

impl AccessDecision {
    pub fn is_permit(&self) -> bool {
        matches!(self, AccessDecision::Permit)
    }
}

/// Fail-closed, bounded, deterministic ABAC. Order matters: revocation → consent → validity →
/// trigger → domain → **selfhood default-deny** → **consequential-provenance** → jurisdiction.
pub fn delegation_permits(
    delegation: &AgencyDelegation,
    domains: &Taxonomy,
    request: &AccessRequest,
    ctx: &TriggerContext,
) -> AccessDecision {
    if delegation.revoked {
        return AccessDecision::Deny("delegation revoked".into());
    }
    if !delegation.consent.is_effective() {
        return AccessDecision::Deny(format!("consent not effective ({:?})", delegation.consent));
    }
    if ctx.now_unix < delegation.valid_from_unix {
        return AccessDecision::Deny("delegation not yet valid".into());
    }
    if let Some(end) = delegation.valid_to_unix {
        if ctx.now_unix > end {
            return AccessDecision::Deny("delegation expired".into());
        }
    }
    if let Some(trigger) = &delegation.trigger {
        if !evaluate_trigger(trigger, ctx) {
            return AccessDecision::Deny("delegation trigger is not active".into());
        }
    }
    if request.domain != delegation.domain {
        return AccessDecision::Deny(format!(
            "delegation does not cover domain '{}'",
            request.domain
        ));
    }
    // Selfhood is never delegated by default. A selfhood-sphere request is denied unless the
    // delegation carries an explicit selfhood grant (ADR §7 invariant).
    if request.sphere == Sphere::Selfhood && !delegation.scope_has("selfhood_grant", "explicit") {
        return AccessDecision::Deny(
            "selfhood is not delegable without an explicit selfhood grant".into(),
        );
    }
    // Consequential-domain judgements require declared provenance + an epistemic horizon.
    if request.is_judgement() && is_consequential(domains, &request.domain) {
        match &request.provenance {
            Some(p) if p.epistemic_horizon.is_some() => {}
            _ => return AccessDecision::Deny(
                "consequential judgement requires declared provenance with an epistemic horizon"
                    .into(),
            ),
        }
    }
    // Jurisdiction, if both are specified, must match (the backpacker case: a LocalTemporary
    // delegation in the foreign jurisdiction permits foreign-jurisdiction requests).
    if let (Some(dj), Some(rj)) = (&delegation.jurisdiction, &request.jurisdiction) {
        if dj != rj {
            return AccessDecision::Deny("jurisdiction mismatch".into());
        }
    }
    AccessDecision::Permit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agency_domain::{agency_domain_taxonomy, ids as dom};
    use crate::provenance::{AgentRef, JudgementProvenance};
    use crate::trigger::{Attestation, Trigger, TriggerContext};

    fn ctx(now: u32) -> TriggerContext {
        TriggerContext::at(now)
    }

    fn granted(domain: &str) -> AgencyDelegation {
        let mut d = AgencyDelegation::new("did:wf:alice", domain, "urn:un:hr:udhr", 100);
        d.consent = ConsentState::Granted;
        d
    }

    fn read_req(domain: &str) -> AccessRequest {
        AccessRequest {
            domain: domain.into(),
            data_class: "x".into(),
            action: "read".into(),
            sphere: Sphere::Personhood,
            jurisdiction: None,
            provenance: None,
        }
    }

    #[test]
    fn permits_within_scope_when_valid_and_consented() {
        let tax = agency_domain_taxonomy();
        let d = granted(dom::PERSONAL_WELFARE);
        assert!(
            delegation_permits(&d, &tax, &read_req(dom::PERSONAL_WELFARE), &ctx(200)).is_permit()
        );
    }

    #[test]
    fn denies_revoked_pending_withdrawn_expired_and_not_yet_valid() {
        let tax = agency_domain_taxonomy();
        let base = granted(dom::PERSONAL_WELFARE);
        let req = read_req(dom::PERSONAL_WELFARE);

        let mut revoked = base.clone();
        revoked.revoked = true;
        assert!(!delegation_permits(&revoked, &tax, &req, &ctx(200)).is_permit());

        let mut pending = base.clone();
        pending.consent = ConsentState::Pending;
        assert!(!delegation_permits(&pending, &tax, &req, &ctx(200)).is_permit());

        let mut withdrawn = base.clone();
        withdrawn.consent = ConsentState::Withdrawn;
        assert!(!delegation_permits(&withdrawn, &tax, &req, &ctx(200)).is_permit());

        let mut expiring = base.clone();
        expiring.valid_to_unix = Some(150);
        assert!(!delegation_permits(&expiring, &tax, &req, &ctx(200)).is_permit());

        // now < valid_from (100)
        assert!(!delegation_permits(&base, &tax, &req, &ctx(50)).is_permit());
    }

    #[test]
    fn wrong_domain_is_denied() {
        let tax = agency_domain_taxonomy();
        let d = granted(dom::FINANCIAL);
        assert!(!delegation_permits(&d, &tax, &read_req(dom::MEDICAL), &ctx(200)).is_permit());
    }

    #[test]
    fn trigger_gates_activation() {
        let tax = agency_domain_taxonomy();
        let mut d = granted(dom::MEDICAL);
        // Crisis: needs an ER-admission event AND 2 signed physician attestations.
        d.trigger = Some(Trigger::All(vec![
            Trigger::VerifiableEvent {
                event_id: "er".into(),
            },
            Trigger::HumanConsensus {
                required_capacity: Some("physician".into()),
                m: 2,
                n: 5,
            },
        ]));
        // A read on a consequential domain still needs the trigger active.
        let req = read_req(dom::MEDICAL);

        // Trigger inactive → deny.
        assert!(!delegation_permits(&d, &tax, &req, &ctx(200)).is_permit());

        // Trigger satisfied → the read is permitted (read is not a judgement, so no provenance needed).
        let active = ctx(200)
            .with_event("er")
            .with_attestation(Attestation::signed("physician"))
            .with_attestation(Attestation::signed("physician"));
        assert!(delegation_permits(&d, &tax, &req, &active).is_permit());
    }

    #[test]
    fn selfhood_is_denied_without_explicit_grant() {
        let tax = agency_domain_taxonomy();
        let d = granted(dom::REPRODUCTIVE_BIOMETRIC_GENETIC);
        let mut req = read_req(dom::REPRODUCTIVE_BIOMETRIC_GENETIC);
        req.sphere = Sphere::Selfhood;
        // Default-deny.
        assert!(!delegation_permits(&d, &tax, &req, &ctx(200)).is_permit());

        // Only an explicit selfhood grant permits it.
        let mut explicit = d.clone();
        explicit
            .scope
            .push(("selfhood_grant".into(), "explicit".into()));
        assert!(delegation_permits(&explicit, &tax, &req, &ctx(200)).is_permit());
    }

    #[test]
    fn consequential_judgement_requires_provenance_with_horizon() {
        let tax = agency_domain_taxonomy();
        let d = granted(dom::MEDICAL);
        let mut decide = read_req(dom::MEDICAL);
        decide.action = "decide".into();

        // A consequential judgement with no provenance is denied.
        assert!(!delegation_permits(&d, &tax, &decide, &ctx(200)).is_permit());

        // Provenance without a horizon is still denied.
        let mut no_horizon = decide.clone();
        no_horizon.provenance = Some(JudgementProvenance::new(AgentRef::natural_person("did:dr")));
        assert!(!delegation_permits(&d, &tax, &no_horizon, &ctx(200)).is_permit());

        // Provenance WITH a horizon is permitted.
        let mut with_horizon = decide.clone();
        with_horizon.provenance = Some(
            JudgementProvenance::new(AgentRef::natural_person("did:dr")).with_horizon("merkle:abc"),
        );
        assert!(delegation_permits(&d, &tax, &with_horizon, &ctx(200)).is_permit());
    }

    #[test]
    fn jurisdiction_mismatch_denied_but_local_temporary_matches_its_own() {
        let tax = agency_domain_taxonomy();
        let mut d = granted(dom::MEDICAL);
        d.jurisdiction = Some("urn:jur:au-vic".into());
        let mut req = read_req(dom::MEDICAL);
        req.jurisdiction = Some("urn:jur:th".into());
        assert!(!delegation_permits(&d, &tax, &req, &ctx(200)).is_permit());

        // A LocalTemporary delegation in the foreign jurisdiction permits foreign requests.
        let mut local = granted(dom::MEDICAL);
        local.precedence = Precedence::LocalTemporary;
        local.jurisdiction = Some("urn:jur:th".into());
        assert!(delegation_permits(&local, &tax, &req, &ctx(200)).is_permit());
    }

    #[test]
    fn envelope_kind_and_summary_round_trip() {
        let d = granted(dom::FINANCIAL);
        let env = build_agency_delegation_envelope(&d, "did:wf:alice", "did:wf:alice", 100);
        assert!(env.id.contains(":agency-delegation:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        let summary = agency_delegation_summary(&d);
        assert!(summary.contains(dom::FINANCIAL));
    }

    #[test]
    fn full_json_round_trips_losslessly() {
        // The full-JSON form (stored as the journal summary) must reconstruct every field, not the
        // lossy projection.
        let mut d = granted(dom::MEDICAL);
        d.agent_dids = vec!["did:wf:carer".into()];
        d.scope = vec![("data_class".into(), "medication".into())];
        d.trigger = Some(Trigger::TemporalWindow {
            from_unix: 10,
            to_unix: Some(20),
        });
        d.transfer_schedule = vec![TransferStage {
            domain: dom::MEDICAL.into(),
            to_stage: ControlStage::CoSigned,
            trigger: Trigger::TemporalWindow {
                from_unix: 1_000,
                to_unix: None,
            },
        }];
        let json = agency_delegation_full_json(&d);
        let back = parse_agency_delegation(&json).expect("reconstructs");
        assert_eq!(d, back);
        // The lossy summary, by contrast, drops fields like the trigger and scope.
        assert!(!agency_delegation_summary(&d).contains("temporal_window"));
    }

    #[test]
    fn developmental_transfer_stages_are_ordered_and_serde_round_trip() {
        let mut d = granted(dom::COMMUNICATION);
        d.transfer_schedule = vec![
            TransferStage {
                domain: dom::COMMUNICATION.into(),
                to_stage: ControlStage::CoSigned,
                trigger: Trigger::TemporalWindow {
                    from_unix: 1_000,
                    to_unix: None,
                },
            },
            TransferStage {
                domain: dom::COMMUNICATION.into(),
                to_stage: ControlStage::PrincipalSole,
                trigger: Trigger::TemporalWindow {
                    from_unix: 2_000,
                    to_unix: None,
                },
            },
        ];
        // Monotonic toward principal autonomy.
        assert!(d.transfer_schedule[0].to_stage < d.transfer_schedule[1].to_stage);
        let json = serde_json::to_string(&d).unwrap();
        let back: AgencyDelegation = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
