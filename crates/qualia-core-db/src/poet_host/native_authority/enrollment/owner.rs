use super::{super::*, EnrollmentRecord, EnrollmentRequest, EnrollmentStatus, NativeSigner};
use qualia_cooperative_core::projects::ProjectRole;

/// Native in-memory capability: neither deserializable nor constructible by a transport.
pub struct EnrollmentPermission {
    session: SessionHandle,
    generation: u64,
}
impl EnrollmentPermission {
    /// Enrollment-owned administration resolves this already verified handle;
    /// transports cannot construct a permission or replace its session.
    pub(super) fn session(&self) -> &SessionHandle {
        &self.session
    }
}
/// Explicit operator decision. Verification references are evidence, not legal identity proof.
pub struct EnrollmentApproval {
    pub decision_evidence_ref: String,
    pub human_verification_ref: Option<String>,
}
fn evidence(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > AuthoringLimits::default().max_text_bytes {
        Err("explicit bounded operator evidence required".into())
    } else {
        Ok(())
    }
}
impl NativeAuthority {
    /// Reopens retained signed authority, never a retained live session. The native
    /// host still supplies its independently pinned anchor and actual source revision.
    pub fn open_retained(
        root: &Path,
        anchor: TrustedProject,
        document: DocumentRevision,
        max_entry_bytes: usize,
        max_entries: u64,
    ) -> Result<Self, String> {
        let owner = Self::open_retained_current(root, anchor, max_entry_bytes, max_entries)?;
        {
            let state = owner
                .state
                .lock()
                .map_err(|_| "native authority poisoned")?;
            if state.durable.document != document {
                return Err("retained source differs; reconciliation required".into());
            }
        }
        Ok(owner)
    }
    /// Login bootstrap reads protected journal metadata only, before any HMC IO.
    /// The host must compare actual package source after authenticated reopen admission.
    pub fn open_retained_current(
        root: &Path,
        anchor: TrustedProject,
        max_entry_bytes: usize,
        max_entries: u64,
    ) -> Result<Self, String> {
        if max_entry_bytes == 0
            || max_entry_bytes > 4 * 1024 * 1024
            || max_entries == 0
            || max_entries > 4096
        {
            return Err("invalid native authority journal bounds".into());
        }
        let (journal, stored) = Journal::open(root, &anchor, max_entry_bytes, max_entries)?;
        let durable = stored.ok_or("retained native authority unavailable")?;
        if durable.document.project_id != anchor.project_id
            || durable.document.source_digest == [0; 32]
        {
            return Err(
                "retained authority differs from actual project/source; reconciliation required"
                    .into(),
            );
        }
        let admission_digest = durable.admission.verify(&anchor, max_entry_bytes)?;
        let owner = Self {
            anchor,
            incarnation: random()?,
            state: Mutex::new(State {
                durable,
                journal,
                admission_digest,
                generation: 1,
                sessions: std::array::from_fn(|_| None),
                pending: std::array::from_fn(|_| None),
                reconciliation: false,
            }),
        };
        {
            let state = owner
                .state
                .lock()
                .map_err(|_| "native authority poisoned")?;
            owner.require_admission(&state, now()?)?;
        }
        Ok(owner)
    }
    pub fn enrollment_snapshot(&self) -> Result<SignedAdmission, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_admission(&state, now()?)?;
        Ok(state.durable.admission.clone())
    }
    pub fn enrollment_permission(
        &self,
        session: &SessionHandle,
    ) -> Result<EnrollmentPermission, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let permission = EnrollmentPermission {
            session: session.clone(),
            generation: state.generation,
        };
        self.require_enrollment_permission(&state, &permission)?;
        Ok(permission)
    }
    pub(super) fn require_enrollment_permission(
        &self,
        state: &State,
        permission: &EnrollmentPermission,
    ) -> Result<(), String> {
        self.view(state, &permission.session, now()?, |_| Ok(()))?;
        if state.generation != permission.generation {
            return Err("stale enrollment owner permission".into());
        }
        let subject = &state.session(&permission.session)?.subject;
        let admitted = state
            .durable
            .admission
            .snapshot
            .subjects
            .iter()
            .find(|s| &s.subject == subject)
            .ok_or("steward admission missing")?;
        if admitted.membership.role != ProjectRole::Steward {
            return Err("current explicit steward admission required".into());
        }
        Ok(())
    }
    pub fn submit_enrollment(&self, request: EnrollmentRequest) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        self.require_admission(&state, now)?;
        request.verify(now)?;
        if request.project_id != self.anchor.project_id
            || request.admission_sequence != state.durable.admission.snapshot.sequence
            || request.admission_digest != state.admission_digest
        {
            return Err("wrong project or stale enrollment admission binding".into());
        }
        if state.durable.enrollment.len() >= 256 {
            return Err("retained enrollment budget exhausted".into());
        }
        if state
            .durable
            .enrollment
            .iter()
            .any(|r| r.request.nonce == request.nonce)
            || state.durable.admission.snapshot.subjects.iter().any(|s| {
                s.subject == request.candidate.subject
                    || s.controller_public_key == request.candidate.controller_public_key
            })
            || state.durable.enrollment.iter().any(|r| {
                r.status == EnrollmentStatus::Pending
                    && (r.request.candidate.subject == request.candidate.subject
                        || r.request.candidate.controller_public_key
                            == request.candidate.controller_public_key)
            })
        {
            return Err("duplicate/replayed enrollment subject, controller or request".into());
        }
        let id = request.id();
        let mut next = state.durable.clone();
        next.enrollment.push(EnrollmentRecord {
            request,
            status: EnrollmentStatus::Pending,
            decided_by_subject: None,
            decision_evidence_ref: None,
            human_verification_ref: None,
            decided_at_unix: None,
        });
        self.commit_enrollment(&mut state, next, false)?;
        Ok(id)
    }
    pub fn list_enrollments(
        &self,
        permission: &EnrollmentPermission,
    ) -> Result<Vec<EnrollmentRecord>, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        Ok(state.durable.enrollment.clone())
    }
    pub fn approve_enrollment(
        &self,
        permission: &EnrollmentPermission,
        request_id: &str,
        approval: EnrollmentApproval,
        signer: &NativeSigner,
    ) -> Result<SignedAdmission, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        evidence(&approval.decision_evidence_ref)?;
        let now = now()?;
        let mut next = state.durable.clone();
        let record = next
            .enrollment
            .iter_mut()
            .find(|r| r.request.id() == request_id && r.status == EnrollmentStatus::Pending)
            .ok_or("pending enrollment unavailable")?;
        record.request.verify(now)?;
        if record.request.admission_sequence != next.admission.snapshot.sequence
            || record.request.admission_digest != state.admission_digest
        {
            return Err("stale enrollment requires new signed request".into());
        }
        if record.request.candidate.kind == PrincipalKind::Human {
            evidence(
                approval
                    .human_verification_ref
                    .as_deref()
                    .ok_or("explicit operator human verification evidence required")?,
            )?;
        } else if approval.human_verification_ref.is_some() {
            return Err("non-human enrollment cannot claim human verification".into());
        }
        if next.admission.snapshot.subjects.iter().any(|s| {
            s.subject == record.request.candidate.subject
                || s.controller_public_key == record.request.candidate.controller_public_key
        }) {
            return Err("duplicate admitted subject/controller".into());
        }
        // A request cannot grant review or steward authority. Review is a separate explicit act.
        next.admission
            .snapshot
            .subjects
            .push(record.request.candidate.clone());
        record.status = EnrollmentStatus::Approved;
        record.decided_by_subject = Some(state.session(&permission.session)?.subject.clone());
        record.decision_evidence_ref = Some(approval.decision_evidence_ref);
        record.human_verification_ref = approval.human_verification_ref;
        record.decided_at_unix = Some(now);
        self.sign_enrollment_admission(&mut next, signer)?;
        let signed = next.admission.clone();
        self.commit_enrollment(&mut state, next, true)?;
        Ok(signed)
    }
    pub fn decline_enrollment(
        &self,
        permission: &EnrollmentPermission,
        request_id: &str,
        decision_evidence_ref: &str,
    ) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        evidence(decision_evidence_ref)?;
        let mut next = state.durable.clone();
        let record = next
            .enrollment
            .iter_mut()
            .find(|r| r.request.id() == request_id && r.status == EnrollmentStatus::Pending)
            .ok_or("pending enrollment unavailable")?;
        record.status = EnrollmentStatus::Declined;
        record.decision_evidence_ref = Some(decision_evidence_ref.into());
        record.decided_by_subject = Some(state.session(&permission.session)?.subject.clone());
        record.decided_at_unix = Some(now()?);
        self.commit_enrollment(&mut state, next, false)
    }
    pub fn revoke_enrollment(
        &self,
        permission: &EnrollmentPermission,
        subject: &str,
        decision_evidence_ref: &str,
        signer: &NativeSigner,
    ) -> Result<SignedAdmission, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        evidence(decision_evidence_ref)?;
        let mut next = state.durable.clone();
        let admitted = next
            .admission
            .snapshot
            .subjects
            .iter_mut()
            .find(|s| s.subject == subject && !s.revoked)
            .ok_or("current enrolled subject unavailable")?;
        admitted.revoked = true;
        for r in &mut admitted.reviews {
            r.revoked = true;
        }
        if let Some(record) = next.enrollment.iter_mut().find(|r| {
            r.request.candidate.subject == subject && r.status == EnrollmentStatus::Approved
        }) {
            record.status = EnrollmentStatus::Revoked;
            record.decision_evidence_ref = Some(decision_evidence_ref.into());
            record.decided_by_subject = Some(state.session(&permission.session)?.subject.clone());
            record.decided_at_unix = Some(now()?);
        }
        self.sign_enrollment_admission(&mut next, signer)?;
        let signed = next.admission.clone();
        self.commit_enrollment(&mut state, next, true)?;
        Ok(signed)
    }
    /// Separate signed review grant; contribution membership alone grants no review.
    pub fn grant_enrollment_review(
        &self,
        permission: &EnrollmentPermission,
        subject: &str,
        review: ReviewAdmission,
        signer: &NativeSigner,
    ) -> Result<SignedAdmission, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.require_enrollment_permission(&state, permission)?;
        evidence(&review.id)?;
        evidence(&review.document_id)?;
        evidence(&review.admission_ref)?;
        if review.revoked || !live(now()?, review.valid_from_unix, review.valid_until_unix) {
            return Err("invalid review grant window".into());
        }
        let mut next = state.durable.clone();
        let admitted = next
            .admission
            .snapshot
            .subjects
            .iter_mut()
            .find(|s| s.subject == subject && !s.revoked)
            .ok_or("review subject not admitted")?;
        if review.valid_from_unix < admitted.valid_from_unix
            || review.valid_until_unix > admitted.valid_until_unix
            || admitted.reviews.iter().any(|r| r.id == review.id)
        {
            return Err("review bounds/duplicate grant rejected".into());
        }
        admitted.reviews.push(review);
        self.sign_enrollment_admission(&mut next, signer)?;
        let signed = next.admission.clone();
        self.commit_enrollment(&mut state, next, true)?;
        Ok(signed)
    }
    pub(super) fn sign_enrollment_admission(
        &self,
        next: &mut DurableState,
        signer: &NativeSigner,
    ) -> Result<(), String> {
        if signer.public_key() != self.anchor.public_key {
            return Err("project authority signer does not match pinned key".into());
        }
        next.admission.snapshot.sequence = next
            .admission
            .snapshot
            .sequence
            .checked_add(1)
            .ok_or("admission sequence exhausted")?;
        next.admission = signer.sign_admission(next.admission.snapshot.clone())?;
        next.admission.verify(&self.anchor, 4 * 1024 * 1024)?;
        Ok(())
    }
    pub(super) fn commit_enrollment(
        &self,
        state: &mut State,
        next: DurableState,
        changed: bool,
    ) -> Result<(), String> {
        // Validate all fallible admission/generation work before durable write.
        let admission_digest = if changed {
            super::super::package_review::validate_grant_successor(
                &state.durable.admission.snapshot,
                &next.admission.snapshot,
            )?;
            next.admission.verify(&self.anchor, 4 * 1024 * 1024)?
        } else {
            state.admission_digest
        };
        // Preflight generation before writing a durable successor.
        let generation = if changed {
            state
                .generation
                .checked_add(1)
                .ok_or("authority generation exhausted")?
        } else {
            state.generation
        };
        if let Err(e) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(e);
        }
        state.durable = next;
        if changed {
            state.admission_digest = admission_digest;
            state.invalidate_sessions();
            state.generation = generation;
        }
        Ok(())
    }
}
