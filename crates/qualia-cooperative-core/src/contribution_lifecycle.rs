//! Nine-Stage Contribution, Valuation, Obligation, and Licence-Transition Lifecycle (`CONTRIBUTION-LIFECYCLE.md`).

use serde::{Deserialize, Serialize};
use crate::finance::div_round_bankers;
use crate::record::{q_hash_str, InstantBridge, NQuin, SensitivityClass};

/// Governance authority under which work was requested or initiated (Stage 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgreementAuthority {
    Commissioned, OpenBounty, CooperativeMandate, StewardAuthorised, UnsolicitedNoAgreement,
}

/// Compensation model agreed prior to or at acceptance (Stage 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompensationClass {
    ContractedService, AcceptedBounty, ConditionalPoolShare, ReimbursableExpense, VoluntaryCommonsGift,
}

/// Stage 1 — Request & Agreement record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestAndAgreement {
    pub id: String, pub project_id: String, pub contributor_did: String,
    pub authorised_by_did: Option<String>, pub authority: AgreementAuthority,
    pub compensation_class: CompensationClass, pub max_authorised_effort_minutes: u32,
    pub max_authorised_expense_cents: i64, pub rate_cents_per_hour: i64, pub created_at_unix: u32,
}

impl RequestAndAgreement {
    /// True when the agreement was commissioned with guaranteed payment for authorised effort/expense.
    pub const fn is_commissioned_guarantee(&self) -> bool {
        matches!(self.authority, AgreementAuthority::Commissioned | AgreementAuthority::StewardAuthorised)
            && matches!(self.compensation_class, CompensationClass::ContractedService | CompensationClass::ReimbursableExpense)
            && self.authorised_by_did.is_some()
    }

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:RequestAndAgreement", self.rate_cents_per_hour as u64, self.created_at_unix, out)
    }
}

/// Stage 2 — Activity & Resources record (logs effort/expense; never directly creates payable debt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityAndResources {
    pub id: String,
    pub project_id: String,
    pub contributor_did: String,
    pub agreement_id: Option<String>,
    pub work_item_id: Option<String>,
    pub effort_minutes: u32,
    pub reimbursable_expense_cents: i64,
    pub capital_advanced_cents: i64,
    pub occurred_at_unix: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at_instant: Option<InstantBridge>,
}

impl ActivityAndResources {
    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:ActivityAndResources", self.effort_minutes as u64, self.occurred_at_unix, out)
    }
}

/// Stage 3 — Output or Service produced by activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputOrService {
    pub id: String,
    pub project_id: String,
    pub contributor_did: String,
    pub activity_ids: Vec<String>,
    pub artifact_uri: String,
    pub non_monetary_badges: Vec<String>,
    pub provenance_attestation_id: Option<String>,
    pub produced_at_unix: u32,
}

impl OutputOrService {
    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:OutputOrService", q_hash_str(&self.artifact_uri), self.produced_at_unix, out)
    }
}

/// Stage 4 — Submission or Claim for review/compensation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionOrClaim {
    pub id: String,
    pub project_id: String,
    pub contributor_did: String,
    pub agreement_id: Option<String>,
    pub output_id: String,
    pub claimed_effort_minutes: u32,
    pub claimed_expense_cents: i64,
    pub claimed_capital_cents: i64,
    pub submitted_at_unix: u32,
}

impl SubmissionOrClaim {
    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:SubmissionOrClaim", self.claimed_effort_minutes as u64, self.submitted_at_unix, out)
    }
}

/// Outcome of a Stage 5 `AcceptanceDecision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceOutcome {
    Accepted,
    AcceptedWithConditions,
    RevisionRequested,
    Rejected,
    Deferred,
    Disputed,
}

/// Type alias for callers referring to `AcceptanceStatus`.
pub type AcceptanceStatus = AcceptanceOutcome;

impl AcceptanceOutcome {
    /// True if the deliverable was accepted (unconditionally or with conditions).
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted | Self::AcceptedWithConditions)
    }
}

/// Stage 5 — Acceptance Decision record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    pub id: String,
    pub project_id: String,
    pub submission_id: String,
    pub agreement_id: Option<String>,
    pub outcome: AcceptanceOutcome,
    pub reviewer_did: String,
    pub conditions_or_rationale: Option<String>,
    pub decided_at_unix: u32,
}

#[allow(non_upper_case_globals)]
impl AcceptanceDecision {
    pub const Accepted: AcceptanceOutcome = AcceptanceOutcome::Accepted;
    pub const AcceptedWithConditions: AcceptanceOutcome = AcceptanceOutcome::AcceptedWithConditions;
    pub const RevisionRequested: AcceptanceOutcome = AcceptanceOutcome::RevisionRequested;
    pub const Rejected: AcceptanceOutcome = AcceptanceOutcome::Rejected;
    pub const Deferred: AcceptanceOutcome = AcceptanceOutcome::Deferred;
    pub const Disputed: AcceptanceOutcome = AcceptanceOutcome::Disputed;

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:AcceptanceDecision", self.outcome as u64, self.decided_at_unix, out)
    }
}

/// Stage 6 — Valuation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Valuation {
    pub id: String,
    pub project_id: String,
    pub submission_id: String,
    pub acceptance_id: String,
    pub valued_effort_cents: i64,
    pub valued_expense_cents: i64,
    pub valued_capital_cents: i64,
    pub risk_cap_bps: u32,
    pub approved_by_did: String,
    pub valued_at_unix: u32,
}

impl Valuation {
    /// Total valued obligation in integer minor units (cents) using banker's rounding on capital return.
    pub const fn total_valued_cents(&self) -> i64 {
        let cap_with_return = if self.risk_cap_bps == 0 {
            self.valued_capital_cents
        } else {
            div_round_bankers((self.valued_capital_cents as i128) * (self.risk_cap_bps as i128), 10_000)
        };
        self.valued_effort_cents.saturating_add(self.valued_expense_cents).saturating_add(cap_with_return)
    }

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:Valuation", self.total_valued_cents() as u64, self.valued_at_unix, out)
    }
}

/// Status of a Stage 7 `LifecycleObligation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    Active,
    PartiallySettled,
    Satisfied,
    Waived,
    Disputed,
}

/// Stage 7 — Lifecycle Obligation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleObligation {
    pub id: String,
    pub project_id: String,
    pub beneficiary_did: String,
    pub agreement_id: Option<String>,
    pub acceptance_id: Option<String>,
    pub valuation_id: Option<String>,
    pub principal_cents: i64,
    pub settled_cents: i64,
    pub outstanding_cents: i64,
    pub status: ObligationStatus,
    pub waiver_terms: Option<String>,
    pub waived_by_did: Option<String>,
    pub created_at_unix: u32,
}

#[allow(non_upper_case_globals)]
impl LifecycleObligation {
    pub const Active: ObligationStatus = ObligationStatus::Active;
    pub const PartiallySettled: ObligationStatus = ObligationStatus::PartiallySettled;
    pub const Satisfied: ObligationStatus = ObligationStatus::Satisfied;
    pub const Waived: ObligationStatus = ObligationStatus::Waived;
    pub const Disputed: ObligationStatus = ObligationStatus::Disputed;

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:LifecycleObligation", self.outstanding_cents as u64, self.created_at_unix, out)
    }
}

/// Stage 8 — Settlement Allocation record (maps pool funds to a specific beneficiary obligation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementAllocation {
    pub id: String,
    pub project_id: String,
    pub obligation_id: String,
    pub beneficiary_did: String,
    pub allocated_cents: i64,
    pub currency: String,
    pub source_pool_id: String,
    pub beneficiary_receipt_hash: Option<u64>,
    pub settled_at_unix: u32,
}

impl SettlementAllocation {
    /// True only when a positive allocation is backed by a verified beneficiary receipt hash.
    pub const fn has_verified_receipt(&self) -> bool {
        self.allocated_cents > 0 && self.beneficiary_receipt_hash.is_some()
    }

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:SettlementAllocation", self.beneficiary_receipt_hash.unwrap_or(0), self.settled_at_unix, out)
    }
}

/// Readiness evaluation status for Stage 9 (`TransitionDecision`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionReadiness {
    Satisfied,
    Held { reason: String },
    Failed { reason: String },
}

impl TransitionReadiness {
    /// True only when all transition preconditions are satisfied.
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

/// Stage 9 — Licence Transition Decision record (requires an authorised human steward DID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDecision {
    pub id: String,
    pub project_id: String,
    pub target_licence_uri: String,
    pub authorised_steward_did: String,
    pub steward_is_human: bool,
    pub pool_funded_cents: i64,
    pub total_obligation_cents: i64,
    pub settled_contributor_cents: i64,
    pub unresolved_material_challenges: u32,
    pub dependencies_compatible: bool,
    pub decided_at_unix: u32,
}

impl TransitionDecision {
    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        emit_stage_quins(&self.id, &self.project_id, "hro:TransitionDecision", q_hash_str(&self.target_licence_uri), self.decided_at_unix, out)
    }
}

/// Unified enum over all 9 contribution lifecycle stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum ContributionLifecycleStage {
    RequestAndAgreement(RequestAndAgreement),
    ActivityAndResources(ActivityAndResources),
    OutputOrService(OutputOrService),
    SubmissionOrClaim(SubmissionOrClaim),
    AcceptanceDecision(AcceptanceDecision),
    Valuation(Valuation),
    LifecycleObligation(LifecycleObligation),
    SettlementAllocation(SettlementAllocation),
    TransitionDecision(TransitionDecision),
}

impl ContributionLifecycleStage {
    /// Zero-heap compilation of any lifecycle stage into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        match self {
            Self::RequestAndAgreement(r) => r.compile_to_quins(out),
            Self::ActivityAndResources(r) => r.compile_to_quins(out),
            Self::OutputOrService(r) => r.compile_to_quins(out),
            Self::SubmissionOrClaim(r) => r.compile_to_quins(out),
            Self::AcceptanceDecision(r) => r.compile_to_quins(out),
            Self::Valuation(r) => r.compile_to_quins(out),
            Self::LifecycleObligation(r) => r.compile_to_quins(out),
            Self::SettlementAllocation(r) => r.compile_to_quins(out),
            Self::TransitionDecision(r) => r.compile_to_quins(out),
        }
    }
}

fn emit_stage_quins(id: &str, project_id: &str, rdf_type: &str, value_word: u64, at_unix: u32, out: &mut [NQuin]) -> usize {
    let mut written = 0;
    let (s, ctx) = (q_hash_str(id), q_hash_str(project_id));
    let meta = SensitivityClass::Restricted.to_metadata_mask() | (at_unix as u64);
    for (p, o) in [
        (q_hash_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"), q_hash_str(rdf_type)),
        (q_hash_str("https://vocab.Burst.net.au/hro#stageValue"), value_word),
    ] {
        if written < out.len() {
            out[written] = NQuin::new(s, p, o, ctx, meta);
            written += 1;
        }
    }
    written
}

/// Rule 1 (`Activity != Payable Debt`): Derive a `LifecycleObligation` from a submission,
/// optional agreement, acceptance decision, and optional valuation.
pub fn derive_obligation_from_lifecycle(
    obligation_id: impl Into<String>,
    submission: &SubmissionOrClaim,
    agreement: Option<&RequestAndAgreement>,
    acceptance: Option<&AcceptanceDecision>,
    valuation: Option<&Valuation>,
    now_unix: u32,
) -> Result<LifecycleObligation, String> {
    if agreement.is_some_and(|ag| ag.compensation_class == CompensationClass::VoluntaryCommonsGift) {
        return Err("voluntary commons gifts do not create payable obligations".to_string());
    }
    if acceptance.is_some_and(|a| a.outcome.is_accepted()) {
        let val = valuation.ok_or_else(|| "accepted submission requires a Valuation before creating a LifecycleObligation".to_string())?;
        let principal = val.total_valued_cents();
        if principal <= 0 {
            return Err("valuation must have positive total_valued_cents".to_string());
        }
        return Ok(LifecycleObligation {
            id: obligation_id.into(),
            project_id: submission.project_id.clone(),
            beneficiary_did: submission.contributor_did.clone(),
            agreement_id: submission.agreement_id.clone(),
            acceptance_id: acceptance.map(|a| a.id.clone()),
            valuation_id: Some(val.id.clone()),
            principal_cents: principal,
            settled_cents: 0,
            outstanding_cents: principal,
            status: ObligationStatus::Active,
            waiver_terms: None,
            waived_by_did: None,
            created_at_unix: now_unix,
        });
    }
    if let Some(ag) = agreement.filter(|a| a.is_commissioned_guarantee()) {
        let billable_minutes = submission.claimed_effort_minutes.min(ag.max_authorised_effort_minutes);
        let effort_cents = div_round_bankers((billable_minutes as i128) * (ag.rate_cents_per_hour as i128), 60);
        let expense_cents = submission.claimed_expense_cents.max(0).min(ag.max_authorised_expense_cents.max(0));
        let principal = effort_cents.saturating_add(expense_cents);
        if principal > 0 {
            return Ok(LifecycleObligation {
                id: obligation_id.into(),
                project_id: submission.project_id.clone(),
                beneficiary_did: submission.contributor_did.clone(),
                agreement_id: Some(ag.id.clone()),
                acceptance_id: acceptance.map(|a| a.id.clone()),
                valuation_id: valuation.map(|v| v.id.clone()),
                principal_cents: principal,
                settled_cents: 0,
                outstanding_cents: principal,
                status: ObligationStatus::Active,
                waiver_terms: None,
                waived_by_did: None,
                created_at_unix: now_unix,
            });
        }
    }
    Err("unsolicited or non-commissioned activity cannot create a payable obligation without AcceptanceDecision and Valuation".to_string())
}

/// Rule 3 (`Non-Monetary Recognition Never Discharges Debt`): Recompute an obligation's balance.
pub fn apply_settlements_and_recognition(
    obligation: &LifecycleObligation,
    allocations: &[SettlementAllocation],
    _outputs_with_badges: &[OutputOrService],
) -> LifecycleObligation {
    let mut updated = obligation.clone();
    if updated.status == ObligationStatus::Waived {
        if updated.waiver_terms.as_deref().is_some_and(|t| !t.trim().is_empty())
            && updated.waived_by_did.as_deref().is_some_and(|d| !d.trim().is_empty())
        {
            updated.outstanding_cents = 0;
            return updated;
        }
        updated.status = ObligationStatus::Active;
    }
    let verified_paid_cents: i64 = allocations
        .iter()
        .filter(|a| a.obligation_id == obligation.id && a.beneficiary_did == obligation.beneficiary_did && a.has_verified_receipt())
        .fold(0i64, |acc, a| acc.saturating_add(a.allocated_cents));
    updated.settled_cents = verified_paid_cents.min(updated.principal_cents);
    updated.outstanding_cents = (updated.principal_cents - updated.settled_cents).max(0);
    if updated.status != ObligationStatus::Disputed {
        updated.status = if updated.outstanding_cents == 0 {
            ObligationStatus::Satisfied
        } else if updated.settled_cents > 0 {
            ObligationStatus::PartiallySettled
        } else {
            ObligationStatus::Active
        };
    }
    updated
}

/// Rule 2 (`Pool Funded != Contributor Paid`): Evaluate whether a project is ready for licence transition.
pub fn evaluate_transition_readiness(
    decision: &TransitionDecision,
    obligations: &[LifecycleObligation],
    allocations: &[SettlementAllocation],
) -> TransitionReadiness {
    if !decision.steward_is_human || decision.authorised_steward_did.trim().is_empty() {
        return TransitionReadiness::Failed { reason: "transition decision requires an authorised human steward DID".to_string() };
    }
    if !decision.dependencies_compatible {
        return TransitionReadiness::Failed { reason: "incompatible dependency licences block transition".to_string() };
    }
    if decision.unresolved_material_challenges > 0 {
        return TransitionReadiness::Held { reason: format!("{} unresolved material challenge(s) block transition", decision.unresolved_material_challenges) };
    }
    for ob in obligations {
        let recomputed = apply_settlements_and_recognition(ob, allocations, &[]);
        if recomputed.status == ObligationStatus::Disputed {
            return TransitionReadiness::Held { reason: format!("obligation {} is disputed", ob.id) };
        }
        if recomputed.status != ObligationStatus::Waived && recomputed.outstanding_cents > 0 {
            return TransitionReadiness::Held {
                reason: format!(
                    "pool funded ({}) != contributor paid: obligation {} for {} has {} cents outstanding without verified beneficiary receipt",
                    decision.pool_funded_cents, ob.id, ob.beneficiary_did, recomputed.outstanding_cents
                ),
            };
        }
    }
    TransitionReadiness::Satisfied
}

/// Boolean helper returning `true` iff [`evaluate_transition_readiness`] returns `Satisfied`.
pub fn is_transition_ready(
    decision: &TransitionDecision,
    obligations: &[LifecycleObligation],
    allocations: &[SettlementAllocation],
) -> bool {
    evaluate_transition_readiness(decision, obligations, allocations).is_ready()
}

#[cfg(test)]
#[path = "contribution_lifecycle_tests.rs"]
mod tests;
