//! Bounded durable source-review requests, separate from HMC/package acceptance.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReviewRequest {
    pub id: String,
    pub target: DocumentRevision,
    pub requester_subject: String,
    pub principal_kind: PrincipalKind,
    pub authentication_ref: String,
    pub authority_generation: u64,
    pub occurred_at_unix: u64,
    pub evidence_ref: String,
    /// Exact canonical source decisions responding to this request.
    #[serde(default)]
    pub decision_ids: Vec<String>,
}

/// Evidence is readable only through a current authenticated native session.
/// Retained records do not restore authority or imply human acceptance.
#[derive(Clone, Debug)]
pub struct WorkflowEvidence {
    pub target: DocumentRevision,
    pub principal_subject: String,
    pub principal_kind: PrincipalKind,
    pub human_authority: HumanAuthorityEvidence,
    pub authority_generation: u64,
    pub can_contribute: bool,
    pub can_review: bool,
    pub contributions: Vec<ContributionRecord>,
    pub requests: Vec<SourceReviewRequest>,
    pub reviews: Vec<ReviewRecord>,
    pub package_writes: Vec<PackageWriteEvidence>,
}

impl NativeAuthority {
    pub fn workflow_evidence(&self, handle: &SessionHandle) -> Result<WorkflowEvidence, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.view(&state, handle, now()?, |view| {
            let principal = view.principal.ok_or("native principal unavailable")?;
            let allowed = |action| {
                authorize(
                    view,
                    &AuthoringRequest {
                        action,
                        target: view.current_document,
                        expected_authority_generation: view.generation,
                    },
                )
                .is_ok()
            };
            Ok(WorkflowEvidence {
                target: view.current_document.clone(),
                principal_subject: principal.subject.into(),
                principal_kind: principal.kind,
                human_authority: HumanAuthorityEvidence::legacy(principal.subject, principal.kind),
                authority_generation: view.generation,
                can_contribute: allowed(AuthoringAction::ContributionRecord)
                    && view.current_document.author_subject.as_deref() == Some(principal.subject),
                can_review: allowed(AuthoringAction::ReviewDecision),
                contributions: state.durable.contributions.clone(),
                requests: state.durable.requests.clone(),
                reviews: state.durable.reviews.clone(),
                package_writes: state.durable.packages.clone(),
            })
        })
    }

    pub fn request_source_review(
        &self,
        handle: &SessionHandle,
        expected: &DocumentRevision,
        id: &str,
        evidence_ref: &str,
    ) -> Result<SourceReviewRequest, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if state.durable.requests.len() >= 256
            || state
                .durable
                .requests
                .iter()
                .any(|request| request.id == id)
        {
            return Err("duplicate review request or source request budget exhausted".into());
        }
        let record = self.view(&state, handle, now()?, |view| {
            let permit = authorize(
                view,
                &AuthoringRequest {
                    action: AuthoringAction::ContributionRecord,
                    target: expected,
                    expected_authority_generation: view.generation,
                },
            )
            .map_err(|e| e.to_string())?;
            if expected.author_subject.as_deref() != Some(permit.principal_subject()) {
                return Err(
                    "only this exact source's admitted author may request its review".into(),
                );
            }
            for text in [id, evidence_ref] {
                if text.is_empty() || text.len() > view.limits.max_text_bytes {
                    return Err("source review request text exceeds bounds".into());
                }
            }
            let principal = view.principal.ok_or("native principal unavailable")?;
            Ok(SourceReviewRequest {
                id: id.into(),
                target: expected.clone(),
                requester_subject: principal.subject.into(),
                principal_kind: principal.kind,
                authentication_ref: principal.authentication_ref.into(),
                authority_generation: view.generation,
                occurred_at_unix: view.now_unix,
                evidence_ref: evidence_ref.into(),
                decision_ids: Vec::new(),
            })
        })?;
        let mut next = state.durable.clone();
        next.requests.push(record.clone());
        if let Err(error) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(error);
        }
        state.durable = next;
        Ok(record)
    }
}
