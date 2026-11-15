//! Explicit native steward administration; no inferred enrollment privilege.
use super::super::package_review::{validate_admission_packages, PackageReviewGrant};
use super::{super::*, EnrollmentPermission, NativeSigner};

impl NativeAuthority {
    pub fn grant_package_review(
        &self,
        permission: &EnrollmentPermission,
        grant: PackageReviewGrant,
        signer: &NativeSigner,
    ) -> Result<SignedAdmission, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        if state.durable.package_grant_decisions.len() >= 256 {
            return Err("package grant audit budget exhausted".into());
        }
        self.require_current_package(&state, &grant.target)?;
        if grant.revoked || !live(now()?, grant.valid_from_unix, grant.valid_until_unix) {
            return Err("package grant must have explicit current nonrevoked scope".into());
        }
        let reviewer = state
            .durable
            .admission
            .snapshot
            .subjects
            .iter()
            .find(|s| s.subject == grant.reviewer_subject)
            .ok_or("package reviewer not explicitly admitted")?;
        if reviewer.revoked
            || !live(now()?, reviewer.valid_from_unix, reviewer.valid_until_unix)
            || state
                .durable
                .package_contributions
                .iter()
                .any(|r| r.target == grant.target && r.actor.subject == grant.reviewer_subject)
            || state.durable.contributions.iter().any(|r| {
                r.target == grant.target.source && r.contributor_subject == grant.reviewer_subject
            })
        {
            return Err("revoked/expired/self-contributing package reviewer denied".into());
        }
        let history =
            super::super::package_review::contributor_history(&state.durable, &grant.target)?;
        if history.subjects.contains(&grant.reviewer_subject.as_str()) {
            return Err("known predecessor contributor cannot receive package review grant".into());
        }
        let mut next = state.durable.clone();
        if next.admission.snapshot.package_reviews.len()
            >= AuthoringLimits::default().max_review_grants
        {
            return Err("package grant budget exhausted".into());
        }
        let grant_id = grant.id.clone();
        let evidence_ref = grant.admission_ref.clone();
        next.admission.snapshot.package_reviews.push(grant);
        validate_admission_packages(&next.admission.snapshot)?;
        self.sign_enrollment_admission(&mut next, signer)?;
        next.package_grant_decisions
            .push(super::super::package_review::PackageGrantDecision {
                grant_id,
                actor_subject: state.session(permission.session())?.subject.clone(),
                evidence_ref,
                occurred_at_unix: now()?,
                revoked: false,
                admission_digest: admission_digest(&next.admission.snapshot, 4 * 1024 * 1024)?,
            });
        let signed = next.admission.clone();
        self.commit_enrollment(&mut state, next, true)?;
        Ok(signed)
    }

    pub fn revoke_package_review(
        &self,
        permission: &EnrollmentPermission,
        grant_id: &str,
        decision_evidence_ref: &str,
        signer: &NativeSigner,
    ) -> Result<SignedAdmission, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        if state.durable.package_grant_decisions.len() >= 256 {
            return Err("package grant audit budget exhausted".into());
        }
        if decision_evidence_ref.trim().is_empty()
            || decision_evidence_ref.len() > AuthoringLimits::default().max_text_bytes
        {
            return Err("explicit bounded revocation evidence required".into());
        }
        let mut next = state.durable.clone();
        let grant = next
            .admission
            .snapshot
            .package_reviews
            .iter_mut()
            .find(|g| g.id == grant_id && !g.revoked)
            .ok_or("current exact package grant unavailable")?;
        grant.revoked = true;
        // Keep original grant adoption reference and bind the local audit to
        // the changed signed admission; the audit itself is not a signature.
        self.sign_enrollment_admission(&mut next, signer)?;
        next.package_grant_decisions
            .push(super::super::package_review::PackageGrantDecision {
                grant_id: grant_id.into(),
                actor_subject: state.session(permission.session())?.subject.clone(),
                evidence_ref: decision_evidence_ref.into(),
                occurred_at_unix: now()?,
                revoked: true,
                admission_digest: admission_digest(&next.admission.snapshot, 4 * 1024 * 1024)?,
            });
        let signed = next.admission.clone();
        self.commit_enrollment(&mut state, next, true)?;
        Ok(signed)
    }
}
