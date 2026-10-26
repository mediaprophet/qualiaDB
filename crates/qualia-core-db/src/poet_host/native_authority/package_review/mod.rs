//! Native current-session package evidence. No source grant is package authority.
mod history;
pub(super) use history::{contributor_history, validate_new_grant_reviewers};
mod validation;
mod workflow;
use super::*;
pub use qualia_cooperative_core::authoring::package_review::*;
use serde::{Deserialize, Serialize};
pub(super) use validation::{
    validate_admission_packages, validate_grant_successor, validate_ledger,
};

/// Host-journal audit bound to the resulting signed admission digest. This
/// record is local evidence, not an independently signed human decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageGrantDecision {
    pub grant_id: String,
    pub actor_subject: String,
    pub evidence_ref: String,
    pub occurred_at_unix: u64,
    pub revoked: bool,
    pub admission_digest: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct PackageWorkflowEvidence {
    pub target: PackageRevision,
    pub principal_subject: String,
    pub principal_kind: PrincipalKind,
    pub authority_generation: u64,
    pub can_contribute: bool,
    pub can_review: bool,
    pub contributions: Vec<PackageContributionRecord>,
    /// Other recorded package work in the same qualified source scope. These
    /// are historical credits; ancestry/applicability to this target is not
    /// established and no prior package decision is projected as current.
    pub prior_contributions: Vec<PackageContributionRecord>,
    pub requests: Vec<PackageReviewRequest>,
    pub reviews: Vec<PackageReviewRecord>,
    pub contribution_rights_scope: String,
    pub known_destination_lineage_complete: bool,
}

fn saved_matches(saved: &PackageWriteEvidence, target: &PackageRevision) -> bool {
    saved.filename == target.filename
        && saved.result_digest == target.package_digest
        && saved.expected_digest == target.predecessor_digest
        && saved.result_bytes as u64 == target.package_bytes
        && saved.source == target.source
        && saved.actor_subject == target.producer_subject
}

impl NativeAuthority {
    /// Nonblocking native audit/source/session freshness for UI publication.
    /// No filesystem IO: external byte changes still require the trusted bounded
    /// checkpoint verification on refresh/consume. This is not continuous file
    /// monitoring and cannot authenticate a principal from serialized evidence.
    pub fn package_publication_is_current(
        &self,
        handle: &SessionHandle,
        generation: u64,
        target: &PackageRevision,
    ) -> bool {
        let Ok(now) = now() else {
            return false;
        };
        self.state.try_lock().is_ok_and(|state| {
            state.generation == generation
                && self.require_current_package(&state, target).is_ok()
                && self.view(&state, handle, now, |_| Ok(())).is_ok()
        })
    }

    fn with_package<T>(
        &self,
        state: &State,
        handle: &SessionHandle,
        expected_generation: u64,
        target: &PackageRevision,
        action: PackageAction,
        operation: impl FnOnce(PackagePermit<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.view(state, handle, now()?, |source| {
            self.require_current_package(state, target)?;
            let history = contributor_history(&state.durable, target)?;
            let view = PackageAuthorityView {
                source,
                current_package: target,
                grants: &state.durable.admission.snapshot.package_reviews,
                contributor_subjects: &history.subjects,
                known_destination_lineage_complete: history.complete,
            };
            operation(
                authorize_package(&view, target, expected_generation, action)
                    .map_err(|e| e.to_string())?,
            )
        })
    }

    pub(super) fn require_current_package(
        &self,
        state: &State,
        target: &PackageRevision,
    ) -> Result<(), String> {
        validate_package(target, AuthoringLimits::default()).map_err(|e| e.to_string())?;
        let saved = state
            .durable
            .packages
            .iter()
            .rev()
            .find(|p| p.filename == target.filename)
            .ok_or("package has no native save provenance; save before requesting review")?;
        if state.durable.document != target.source || !saved_matches(saved, target) {
            return Err("package/source/lineage changed; refresh exact saved package".into());
        }
        Ok(())
    }

    fn commit_package_records(&self, state: &mut State, next: DurableState) -> Result<(), String> {
        // Journal validates counts and bounded encoding before creating a file.
        if let Err(e) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(e);
        }
        state.durable = next;
        Ok(())
    }

    /// Metadata-only save history, still gated by a current admitted session.
    /// IDs are declared by the operator; provenance and exact signed grants bind
    /// that project-scoped declaration, never a universal referent equivalence.
    pub fn saved_package_target(
        &self,
        handle: &SessionHandle,
        filename: &str,
        package_id: &str,
    ) -> Result<PackageRevision, String> {
        if [filename, package_id]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > AuthoringLimits::default().max_text_bytes)
        {
            return Err(
                "bounded explicit package filename and project-scoped identifier required".into(),
            );
        }
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.view(&state, handle, now()?, |_| {
            let saved = state
                .durable
                .packages
                .iter()
                .rev()
                .find(|p| p.filename == filename)
                .ok_or("no native save evidence for selected package")?;
            let target = PackageRevision {
                package_id: package_id.into(),
                filename: saved.filename.clone(),
                package_digest: saved.result_digest,
                predecessor_digest: saved.expected_digest,
                package_bytes: saved.result_bytes as u64,
                source: saved.source.clone(),
                producer_subject: saved.actor_subject.clone(),
            };
            self.require_current_package(&state, &target)?;
            Ok(target)
        })
    }

    /// Lock order: authority -> trusted raw checkpoint owner -> direct bounded
    /// file check. Callback MUST NOT call an authorized editor or this authority.
    /// A successful check attests one moment-in-time snapshot; it cannot prevent
    /// an uncooperative external writer after the check. Recheck publication/use.
    pub fn package_workflow_evidence(
        &self,
        handle: &SessionHandle,
        target: &PackageRevision,
        verify_current: impl FnOnce(&PackageRevision) -> Result<(), String>,
    ) -> Result<PackageWorkflowEvidence, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.view(&state,handle,now()?,|view| {
            self.require_current_package(&state,target)?;
            verify_current(target)?;
            let principal=view.principal.ok_or("native principal unavailable")?;
            let allowed=|action|self.with_package(&state,handle,view.generation,target,action,|_|Ok(())).is_ok();
            Ok(PackageWorkflowEvidence { target:target.clone(),known_destination_lineage_complete:contributor_history(&state.durable,target)?.complete,
                principal_subject:principal.subject.into(),
                principal_kind:principal.kind,authority_generation:view.generation,
                can_contribute:allowed(PackageAction::Contribute),can_review:allowed(PackageAction::Decide),
                contributions:state.durable.package_contributions.iter().filter(|r|&r.target==target).cloned().collect(),
                prior_contributions:state.durable.package_contributions.iter().filter(|r|&r.target!=target
                    &&r.target.source.project_id==target.source.project_id
                    &&r.target.source.document_id==target.source.document_id
                    &&r.target.source.source_id==target.source.source_id).cloned().collect(),
                requests:state.durable.package_requests.iter().filter(|r|&r.target==target).cloned().collect(),
                reviews:state.durable.package_reviews.iter().filter(|r|&r.target==target).cloned().collect(),
                contribution_rights_scope:"Known recorded contributors excluded; other authorship/rights/completeness unassessed".into() })
        })
    }
}
