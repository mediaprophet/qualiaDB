use super::{
    super::{records::reserve_record, *},
    *,
};
use crate::projects::{Contribution, ContributionPrivacy};

impl PackagePermit<'_> {
    fn reserve(&self, extra: &[&str]) -> Result<(), AuthoringError> {
        let p = &self.source;
        reserve_record(
            [
                self.target.package_id.as_str(),
                &self.target.filename,
                &self.target.producer_subject,
                &self.target.source.project_id,
                &self.target.source.document_id,
                &self.target.source.source_id,
                self.target
                    .source
                    .author_subject
                    .as_deref()
                    .unwrap_or("unknown"),
                p.principal.subject,
                p.principal.authentication_ref,
                p.membership.admission_ref,
                &p.membership.membership.id,
                &p.membership.membership.member_did,
                p.identifier.value,
                p.identifier.scheme,
                p.identifier.issuer,
                p.identifier.project_id,
                p.identifier.evidence_ref,
            ]
            .into_iter()
            .chain(extra.iter().copied()),
            p.view.limits,
        )
    }
    fn actor(&self) -> PackageActorEvidence {
        let p = &self.source;
        PackageActorEvidence {
            subject: p.principal.subject.into(),
            principal_kind: p.principal.kind,
            authentication_ref: p.principal.authentication_ref.into(),
            identifier_association: p.identifier.into(),
            membership_id: p.membership.membership.id.clone(),
            admission_ref: p.membership.admission_ref.into(),
            authority_generation: p.view.generation,
            occurred_at_unix: p.view.now_unix,
        }
    }
    pub fn contribution_record(
        self,
        id: &str,
        description: &str,
        minutes: u32,
        status: WorkStatus,
    ) -> Result<PackageContributionRecord, AuthoringError> {
        if self.action != PackageAction::Contribute {
            return Err(AuthoringError::WrongAction);
        }
        self.reserve(&[
            id,
            description,
            status.evidence().unwrap_or("unknown"),
            &self.target.source.project_id,
        ])?;
        let occurred_at_unix = u32::try_from(self.source.view.now_unix)
            .map_err(|_| AuthoringError::UnsupportedTime)?;
        Ok(PackageContributionRecord {
            contribution: Contribution {
                id: id.into(),
                project_id: self.target.source.project_id.clone(),
                contributor_did: self.source.membership.membership.member_did.clone(),
                description: description.into(),
                effort_minutes: minutes,
                capital_cents: 0,
                roi_multiplier: 1.0,
                privacy_level: ContributionPrivacy::Private,
                occurred_at_unix,
                occurred_at_instant: None,
                predecessor_id: None,
            },
            target: self.target.clone(),
            actor: self.actor(),
            work_status: status,
        })
    }
    pub fn request_record(
        self,
        id: &str,
        contribution: &PackageContributionRecord,
        evidence: &str,
    ) -> Result<PackageReviewRequest, AuthoringError> {
        if self.action != PackageAction::RequestReview {
            return Err(AuthoringError::WrongAction);
        }
        if contribution.target != *self.target
            || contribution.actor.subject != self.source.principal.subject
        {
            return Err(AuthoringError::InvalidBinding);
        }
        self.reserve(&[id, &contribution.contribution.id, evidence])?;
        Ok(PackageReviewRequest {
            id: id.into(),
            target: self.target.clone(),
            contribution_id: contribution.contribution.id.clone(),
            actor: self.actor(),
            evidence_ref: evidence.into(),
            decision_ids: Vec::new(),
            known_destination_lineage_complete: self.known_destination_lineage_complete,
            contribution_rights_scope:
                "Known native records only; other authorship, rights and completeness unassessed"
                    .into(),
        })
    }
    pub fn review_record(
        self,
        id: &str,
        request_id: &str,
        decision: ReviewDecision,
        rationale: &str,
        evidence: &str,
    ) -> Result<PackageReviewRecord, AuthoringError> {
        if self.action != PackageAction::Decide {
            return Err(AuthoringError::WrongAction);
        }
        let grant = self.grant.ok_or(AuthoringError::ReviewDenied)?;
        self.reserve(&[
            id,
            request_id,
            rationale,
            evidence,
            &grant.id,
            &grant.admission_ref,
        ])?;
        Ok(PackageReviewRecord {
            id: id.into(),
            target: self.target.clone(),
            request_id: request_id.into(),
            actor: self.actor(),
            review_grant_id: grant.id.clone(),
            review_admission_ref: grant.admission_ref.clone(),
            decision,
            rationale: rationale.into(),
            decision_evidence_ref: evidence.into(),
            known_destination_lineage_complete:self.known_destination_lineage_complete,
            contribution_rights_scope:"Known destination/source contributors only; other authorship, rights and completeness unassessed".into(),
        })
    }
}
