use super::super::{admission::live, NativeAuthority, SessionHandle, State, SubjectAdmission};
use qualia_cooperative_core::authoring::{
    AuthoringLimits, IdentifierAssociationEvidence, PrincipalKind,
};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct IdentifierSelection<'a> {
    pub value: &'a str,
    pub scheme: &'a str,
    pub issuer: &'a str,
    pub project_id: &'a str,
    pub evidence_ref: &'a str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceAssociationError {
    Clock,
    Poisoned,
    Authentication,
    Stale,
    Scope,
    Capacity,
    UnsupportedDelegation,
}

/// Copyable evidence is NOT an authentication handle or inference
/// grant. The opaque association and its owner must be rechecked at acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InferenceAssociationEvidence {
    pub subject: u64,
    pub kind: PrincipalKind,
    pub controller: [u8; 32],
    pub project_digest: [u8; 32],
    pub selected_identifier_digest: [u8; 32],
    pub association_digest: [u8; 32],
    pub admission_digest: [u8; 32],
    pub admission_sequence: u64,
    pub authority_generation: u64,
    /// Signed association/admission deadline; excludes the possibly shorter
    /// authenticated session lifetime, which is rechecked on every acceptance.
    pub valid_until: u64,
}

/// Not constructible/deserializable from evidence or identifiers. This retains
/// one actual owner/session association; it does not define a subject's identity.
pub struct AuthenticatedInferenceAssociation {
    owner: [u8; 32],
    session: SessionHandle,
    identifier_index: usize,
    evidence: InferenceAssociationEvidence,
}
impl AuthenticatedInferenceAssociation {
    pub fn evidence(&self) -> InferenceAssociationEvidence {
        self.evidence
    }
}

impl NativeAuthority {
    pub fn bind_inference_association(
        &self,
        handle: &SessionHandle,
        selection: IdentifierSelection<'_>,
    ) -> Result<AuthenticatedInferenceAssociation, InferenceAssociationError> {
        let limits = AuthoringLimits::default();
        if [
            selection.value,
            selection.scheme,
            selection.issuer,
            selection.project_id,
            selection.evidence_ref,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > limits.max_text_bytes)
        {
            return Err(InferenceAssociationError::Capacity);
        }
        let state = self
            .state
            .lock()
            .map_err(|_| InferenceAssociationError::Poisoned)?;
        let now = clock()?;
        let (_, subject) = self
            .authenticated_lookup(&state, handle, now)
            .map_err(|_| InferenceAssociationError::Authentication)?;
        if subject.identifiers.len() > limits.max_identifiers {
            return Err(InferenceAssociationError::Capacity);
        }
        let mut matched = None;
        for (index, id) in subject.identifiers.iter().enumerate() {
            if id.value == selection.value
                && id.scheme == selection.scheme
                && id.issuer == selection.issuer
                && id.project_id == selection.project_id
                && id.evidence_ref == selection.evidence_ref
            {
                if matched.replace(index).is_some() {
                    return Err(InferenceAssociationError::Scope);
                }
            }
        }
        let identifier_index = matched.ok_or(InferenceAssociationError::Scope)?;
        let evidence = checked_evidence(&state, subject, identifier_index, now)?;
        Ok(AuthenticatedInferenceAssociation {
            owner: self.incarnation,
            session: handle.clone(),
            identifier_index,
            evidence,
        })
    }

    /// Revalidate under the SAME native owner lock as authority replacement and
    /// session revocation. The closure must be short and non-reentrant: never
    /// call this owner again, block on a callback, or await. A caller must first
    /// revalidate actual Sentinel Allow and its liveness at this same acceptance
    /// boundary before any mutation/publication; a previously cached Allow is
    /// insufficient. Do not acquire a policy-host/vault/owner lock recursively.
    /// This method itself grants authentication continuity only, not inference.
    pub fn with_current_inference_association<T>(
        &self,
        association: &AuthenticatedInferenceAssociation,
        operation: impl FnOnce(InferenceAssociationEvidence) -> T,
    ) -> Result<T, InferenceAssociationError> {
        self.with_inference_association_guard(association, |view| {
            view.revalidate_current().map(operation)
        })?
    }

    /// Lends locked state, NOT authorization. Every effectful internal caller
    /// must revalidate_current after acquiring its downstream policy gate.
    /// The view and its private state references cannot escape this callback.
    pub(crate) fn with_inference_association_guard<T>(
        &self,
        association: &AuthenticatedInferenceAssociation,
        operation: impl FnOnce(&InferenceAssociationGuard<'_>) -> T,
    ) -> Result<T, InferenceAssociationError> {
        let state = self
            .state
            .lock()
            .map_err(|_| InferenceAssociationError::Poisoned)?;
        let view = InferenceAssociationGuard {
            owner: self,
            state: &state,
            association,
        };
        Ok(operation(&view))
    }
}

/// Owner-locked borrowed validation view. Private fields prevent construction;
/// returning it or any state borrow beyond the mutex guard is rejected by Rust.
pub(crate) struct InferenceAssociationGuard<'a> {
    owner: &'a NativeAuthority,
    state: &'a State,
    association: &'a AuthenticatedInferenceAssociation,
}
impl InferenceAssociationGuard<'_> {
    pub(crate) fn revalidate_current(
        &self,
    ) -> Result<InferenceAssociationEvidence, InferenceAssociationError> {
        let association = self.association;
        let state = self.state;
        if association.owner != self.owner.incarnation
            || association.evidence.authority_generation != state.generation
            || association.evidence.admission_digest != state.admission_digest
        {
            return Err(InferenceAssociationError::Stale);
        }
        let now = clock()?;
        let (_, subject) = self
            .owner
            .authenticated_lookup(state, &association.session, now)
            .map_err(|_| InferenceAssociationError::Authentication)?;
        let current = checked_evidence(state, subject, association.identifier_index, now)?;
        if current != association.evidence {
            return Err(InferenceAssociationError::Stale);
        }
        Ok(current)
    }
}

fn clock() -> Result<u64, InferenceAssociationError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_secs())
        .map_err(|_| InferenceAssociationError::Clock)
}
fn text(h: &mut Sha256, s: &str) {
    h.update((s.len() as u64).to_le_bytes());
    h.update(s.as_bytes());
}
fn identifier_digest(id: &IdentifierAssociationEvidence) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:inference:selected-identifier:v1\0");
    for s in [
        &id.value,
        &id.scheme,
        &id.issuer,
        &id.project_id,
        &id.evidence_ref,
    ] {
        text(&mut h, s);
    }
    h.update(id.valid_from_unix.to_le_bytes());
    h.update(id.valid_until_unix.to_le_bytes());
    h.finalize().into()
}
fn checked_evidence(
    state: &State,
    subject: &SubjectAdmission,
    index: usize,
    now: u64,
) -> Result<InferenceAssociationEvidence, InferenceAssociationError> {
    let snapshot = &state.durable.admission.snapshot;
    let id = subject
        .identifiers
        .get(index)
        .ok_or(InferenceAssociationError::Scope)?;
    if subject.revoked
        || !live(now, subject.valid_from_unix, subject.valid_until_unix)
        || id.project_id != snapshot.project.id
        || subject.membership.project_id != snapshot.project.id
        || !live(now, id.valid_from_unix, id.valid_until_unix)
        || u64::try_from(subject.membership.agreed_at().to_unix_secs()).map_or(true, |v| v > now)
        || !subject.identifiers.iter().any(|i| {
            i.value == subject.membership.member_did
                && i.project_id == snapshot.project.id
                && live(now, i.valid_from_unix, i.valid_until_unix)
        })
    {
        return Err(InferenceAssociationError::Scope);
    }
    let mut project = Sha256::new();
    project.update(b"qualia:inference:project:v1\0");
    text(&mut project, &snapshot.project.id);
    let selected = identifier_digest(id);
    let mut h = Sha256::new();
    h.update(b"qualia:inference:subject-association:v1\0");
    for s in [
        &subject.subject,
        &subject.admission_ref,
        &subject.membership.id,
        &subject.membership.member_did,
    ] {
        text(&mut h, s);
    }
    h.update([match subject.kind {
        PrincipalKind::Human => 0,
        PrincipalKind::SoftwareAgent => 1,
        PrincipalKind::Organization => 2,
    }]);
    h.update(subject.controller_public_key);
    h.update(selected);
    h.update(state.admission_digest);
    let valid_until = subject
        .valid_until_unix
        .min(id.valid_until_unix)
        .min(snapshot.valid_until_unix);
    h.update(valid_until.to_le_bytes());
    Ok(InferenceAssociationEvidence {
        subject: crate::q_hash(&subject.subject),
        kind: subject.kind,
        controller: subject.controller_public_key,
        project_digest: project.finalize().into(),
        selected_identifier_digest: selected,
        association_digest: h.finalize().into(),
        admission_digest: state.admission_digest,
        admission_sequence: snapshot.sequence,
        authority_generation: state.generation,
        valid_until,
    })
}
