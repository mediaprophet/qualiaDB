use super::{policy::text, *};
use crate::projects::{Contribution, ContributionPrivacy};
use serde::{Deserialize, Serialize};

/// Actual compensation is independent of role/permission. A receipt reference
/// is retained, not verified or converted into money by this authoring slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Unknown,
    Unpaid { evidence_ref: String },
    SelfFunded { evidence_ref: String },
    PartiallyCompensated { evidence_ref: String },
    Compensated { evidence_ref: String },
    Disputed { evidence_ref: String },
}

impl WorkStatus {
    pub(super) fn evidence(&self) -> Option<&str> {
        match self {
            Self::Unknown => None,
            Self::Unpaid { evidence_ref }
            | Self::SelfFunded { evidence_ref }
            | Self::PartiallyCompensated { evidence_ref }
            | Self::Compensated { evidence_ref }
            | Self::Disputed { evidence_ref } => Some(evidence_ref),
        }
    }
}

/// Canonical contribution plus exact editable source binding. No valuation,
/// payment, permission, agreement adoption or human acceptance is implied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionRecord {
    pub contribution: Contribution,
    pub target: DocumentRevision,
    pub contributor_subject: String,
    pub principal_kind: PrincipalKind,
    pub authentication_ref: String,
    pub identifier_association: IdentifierAssociationEvidence,
    pub membership_id: String,
    pub admission_ref: String,
    pub authority_generation: u64,
    pub work_status: WorkStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Accepted,
    ChangesRequested,
    Rejected,
}

/// An actual identified reviewer's decision about this exact source state.
/// `SoftwareAgent` review cannot be presented as a human acceptance gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRecord {
    pub id: String,
    pub target: DocumentRevision,
    pub reviewer_subject: String,
    pub reviewer_identifier: String,
    pub principal_kind: PrincipalKind,
    pub authentication_ref: String,
    pub identifier_association: IdentifierAssociationEvidence,
    pub membership_id: String,
    pub admission_ref: String,
    pub review_grant_id: String,
    pub review_admission_ref: String,
    pub authority_generation: u64,
    pub occurred_at_unix: u64,
    pub decision: ReviewDecision,
    pub rationale: String,
    /// The review act, not a generated draft or unverified signing display.
    pub decision_evidence_ref: String,
}

/// Saved evidence of the exact association used; reopening it grants nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierAssociationEvidence {
    pub value: String,
    pub scheme: String,
    pub issuer: String,
    pub project_id: String,
    pub evidence_ref: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
}

impl From<&QualifiedIdentifier<'_>> for IdentifierAssociationEvidence {
    fn from(identifier: &QualifiedIdentifier<'_>) -> Self {
        Self {
            value: identifier.value.into(),
            scheme: identifier.scheme.into(),
            issuer: identifier.issuer.into(),
            project_id: identifier.project_id.into(),
            evidence_ref: identifier.evidence_ref.into(),
            valid_from_unix: identifier.valid_from_unix,
            valid_until_unix: identifier.valid_until_unix,
        }
    }
}

pub(super) fn reserve_record<'a>(
    values: impl IntoIterator<Item = &'a str>,
    limits: AuthoringLimits,
) -> Result<(), AuthoringError> {
    // JSON can escape a byte into six bytes. Include a fixed metadata reserve;
    // check BEFORE any owned output allocation. Runtime encoding must also use
    // a byte-limited writer because these are logical records, not a codec.
    let mut bytes = 2048usize;
    for value in values {
        text(value, limits)?;
        bytes = value
            .len()
            .checked_mul(6)
            .and_then(|n| bytes.checked_add(n))
            .ok_or(AuthoringError::BudgetExceeded)?;
    }
    if bytes > limits.max_record_bytes {
        return Err(AuthoringError::BudgetExceeded);
    }
    Ok(())
}

impl AuthoringPermit<'_> {
    /// Call only after authoring succeeded, while the host's transaction still
    /// owns this current revision. The owner persists/deduplicates this record
    /// atomically with its source journal. Caller cannot attribute another author.
    pub fn contribution_record(
        self,
        id: &str,
        description: &str,
        effort_minutes: u32,
        work_status: WorkStatus,
        predecessor: Option<&ContributionRecord>,
    ) -> Result<ContributionRecord, AuthoringError> {
        if self.action != AuthoringAction::ContributionRecord {
            return Err(AuthoringError::WrongAction);
        }
        let target = self.view.current_document;
        if target.author_subject.as_deref() != Some(self.principal.subject) {
            return Err(AuthoringError::InvalidBinding);
        }
        let member = self.membership.membership;
        let occurred_at_unix =
            u32::try_from(self.view.now_unix).map_err(|_| AuthoringError::UnsupportedTime)?;
        let values = [
            id,
            description,
            target.project_id.as_str(),
            target.project_id.as_str(), // canonical contribution and source binding
            target.document_id.as_str(),
            target.source_id.as_str(),
            self.principal.subject,
            self.principal.subject, // contributor referent and target author
            self.principal.authentication_ref,
            member.member_did.as_str(),
            member.id.as_str(),
            self.membership.admission_ref,
            self.identifier.value,
            self.identifier.scheme,
            self.identifier.issuer,
            self.identifier.project_id,
            self.identifier.evidence_ref,
        ];
        if let Some(previous) = predecessor {
            if previous.contribution.project_id != target.project_id
                || previous.contributor_subject != self.principal.subject
                || previous.contribution.contributor_did != member.member_did
                || previous.contribution.id == id
            {
                return Err(AuthoringError::InvalidBinding);
            }
        }
        reserve_record(
            values
                .into_iter()
                .chain(work_status.evidence())
                .chain(predecessor.map(|p| p.contribution.id.as_str())),
            self.view.limits,
        )?;
        Ok(ContributionRecord {
            contribution: Contribution {
                id: id.into(),
                project_id: target.project_id.clone(),
                contributor_did: member.member_did.clone(),
                description: description.into(),
                effort_minutes,
                // This API records effort only. No capital or monetary amount
                // is inferred, and no mixed-unit obligation score is evaluated.
                capital_cents: 0,
                roi_multiplier: 1.0,
                privacy_level: ContributionPrivacy::Private,
                occurred_at_unix,
                occurred_at_instant: None,
                predecessor_id: predecessor.map(|p| p.contribution.id.clone()),
            },
            target: target.clone(),
            contributor_subject: self.principal.subject.into(),
            principal_kind: self.principal.kind,
            authentication_ref: self.principal.authentication_ref.into(),
            identifier_association: self.identifier.into(),
            membership_id: member.id.clone(),
            admission_ref: self.membership.admission_ref.into(),
            authority_generation: self.view.generation,
            work_status,
        })
    }

    pub fn review_record(
        self,
        id: &str,
        decision: ReviewDecision,
        rationale: &str,
        decision_evidence_ref: &str,
    ) -> Result<ReviewRecord, AuthoringError> {
        if self.action != AuthoringAction::ReviewDecision {
            return Err(AuthoringError::WrongAction);
        }
        let grant = self.review_grant.ok_or(AuthoringError::ReviewDenied)?;
        let member = self.membership.membership;
        let target = self.view.current_document;
        reserve_record(
            [
                id,
                rationale,
                decision_evidence_ref,
                target.project_id.as_str(),
                target.document_id.as_str(),
                target.source_id.as_str(),
                target
                    .author_subject
                    .as_deref()
                    .ok_or(AuthoringError::ReviewDenied)?,
                self.principal.subject,
                self.principal.authentication_ref,
                member.member_did.as_str(),
                member.id.as_str(),
                self.membership.admission_ref,
                grant.id,
                grant.admission_ref,
                self.identifier.value,
                self.identifier.scheme,
                self.identifier.issuer,
                self.identifier.project_id,
                self.identifier.evidence_ref,
            ],
            self.view.limits,
        )?;
        Ok(ReviewRecord {
            id: id.into(),
            target: target.clone(),
            reviewer_subject: self.principal.subject.into(),
            reviewer_identifier: member.member_did.clone(),
            principal_kind: self.principal.kind,
            authentication_ref: self.principal.authentication_ref.into(),
            identifier_association: self.identifier.into(),
            membership_id: member.id.clone(),
            admission_ref: self.membership.admission_ref.into(),
            review_grant_id: grant.id.into(),
            review_admission_ref: grant.admission_ref.into(),
            authority_generation: self.view.generation,
            occurred_at_unix: self.view.now_unix,
            decision,
            rationale: rationale.into(),
            decision_evidence_ref: decision_evidence_ref.into(),
        })
    }
}
