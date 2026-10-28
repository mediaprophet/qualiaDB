//! Native project authority: signed adoption, key possession, current grants,
//! serialized document/evidence commit. No source-facing authentication entry.
mod admission;
pub mod delegation;
pub mod enrollment;
mod workflow;
pub use workflow::{SourceReviewRequest, WorkflowEvidence};
mod journal;
mod package;
pub mod package_review;
pub use package::{PackageWriteEvidence, PackageWriteIntent, PackageWriteReceipt};
mod records;
mod session;
mod session_state;
use session_state::{Pending, Session};
pub use session_state::{MAX_NATIVE_SESSIONS, MAX_PENDING_SESSION_CHALLENGES};
#[cfg(test)]
mod tests;
use crate::{
    crypto::network::{ed25519, entropy::fill_os},
    net::qdnf::authority::{
        AuthorityBinding, AuthorityOwner as GrantOwner, ContactState, MutationHandle,
    },
};
use admission::live;
pub use admission::{
    admission_digest, AdmissionSnapshot, ReviewAdmission, SignedAdmission, SubjectAdmission,
    TrustedProject, ADMISSION_CONTEXT, SESSION_CONTEXT,
};
use journal::{DurableState, Journal};
use qualia_cooperative_core::authoring::*;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

fn now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}
fn random() -> Result<[u8; 32], String> {
    let mut bytes = [0; 32];
    fill_os(&mut bytes).map_err(|e| format!("native entropy: {e:?}"))?;
    Ok(bytes)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionHandle {
    owner: [u8; 32],
    nonce: [u8; 32],
}
/// One-use, native owner-issued proof challenge. No Deserialize.
#[derive(Clone, Debug)]
pub struct SessionChallenge {
    owner: [u8; 32],
    nonce: [u8; 32],
    digest: [u8; 32],
}
impl SessionChallenge {
    pub fn message_digest(&self) -> &[u8; 32] {
        &self.digest
    }
}
struct State {
    durable: DurableState,
    journal: Journal,
    admission_digest: [u8; 32],
    generation: u64,
    sessions: [Option<Session>; MAX_NATIVE_SESSIONS],
    pending: [Option<Pending>; MAX_PENDING_SESSION_CHALLENGES],
    reconciliation: bool,
}
/// The host supplies an adopted trust anchor and retained signed admissions.
/// Neither disk records nor matching identifiers can create a live session.
pub struct NativeAuthority {
    anchor: TrustedProject,
    incarnation: [u8; 32],
    state: Mutex<State>,
}

/// Source and authority storage are separate. A post-mutation error must report
/// the actual successor and latch reconciliation; never imply rollback happened.
pub struct DocumentTransaction<T> {
    pub result: Result<T, String>,
    pub successor: Option<DocumentRevision>,
    pub reconciliation_required: bool,
}
impl NativeAuthority {
    pub fn open(
        root: &Path,
        anchor: TrustedProject,
        admission: SignedAdmission,
        document: DocumentRevision,
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
        let digest = admission.verify(&anchor, max_entry_bytes)?;
        if document.project_id != anchor.project_id || document.source_digest == [0; 32] {
            return Err("initial document is not bound to adopted project/source".into());
        }
        let (mut journal, stored) = Journal::open(root, &anchor, max_entry_bytes, max_entries)?;
        let durable = if let Some(stored) = stored {
            if stored.document != document
                || stored.admission.snapshot.sequence != admission.snapshot.sequence
                || stored.admission.verify(&anchor, max_entry_bytes)? != digest
            {
                return Err(
                    "retained authority/source differs; explicit reconciliation required".into(),
                );
            }
            stored
        } else {
            let initial = DurableState {
                admission,
                document,
                contributions: Vec::new(),
                reviews: Vec::new(),
                packages: Vec::new(),
                requests: Vec::new(),
                enrollment: Vec::new(),
                package_contributions: Vec::new(),
                package_requests: Vec::new(),
                package_reviews: Vec::new(),
                package_grant_decisions: Vec::new(),
            };
            journal.append(&initial)?;
            initial
        };
        Ok(Self {
            anchor,
            incarnation: random()?,
            state: Mutex::new(State {
                durable,
                journal,
                admission_digest: digest,
                generation: 1,
                sessions: std::array::from_fn(|_| None),
                pending: std::array::from_fn(|_| None),
                reconciliation: false,
            }),
        })
    }
    /// A replacement must be signed by the pinned project authority and advance
    /// the adoption sequence. This invalidates all pending/live native sessions.
    pub fn replace_admission(&self, replacement: SignedAdmission) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let digest = replacement.verify(&self.anchor, 4 * 1024 * 1024)?;
        if replacement.snapshot.sequence <= state.durable.admission.snapshot.sequence {
            return Err("stale project admission sequence".into());
        }
        package_review::validate_grant_successor(
            &state.durable.admission.snapshot,
            &replacement.snapshot,
        )?;
        package_review::validate_new_grant_reviewers(&state.durable, &replacement.snapshot)?;
        let generation = state
            .generation
            .checked_add(1)
            .ok_or("authority generation exhausted")?;
        let mut next = state.durable.clone();
        next.admission = replacement;
        if let Err(e) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(e);
        }
        state.durable = next;
        state.admission_digest = digest;
        state.invalidate_sessions();
        state.generation = generation;
        Ok(())
    }
    fn require_admission(&self, state: &State, now: u64) -> Result<(), String> {
        let admission = &state.durable.admission.snapshot;
        if state.reconciliation {
            return Err("native source/authority reconciliation required".into());
        }
        if admission.revoked || !live(now, admission.valid_from_unix, admission.valid_until_unix) {
            return Err("adopted project authority expired/revoked".into());
        }
        Ok(())
    }
    fn view<T>(
        &self,
        state: &State,
        handle: &SessionHandle,
        now: u64,
        operation: impl FnOnce(&HostAuthorityView<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.require_admission(state, now)?;
        if handle.owner != self.incarnation {
            return Err("native authenticated session unavailable/foreign".into());
        }
        let session = state.session(handle)?;
        if now >= session.expires {
            return Err("native authenticated session expired".into());
        }
        session
            .grants
            .issue_permit(session.grant, session.binding, now, 1)
            .map_err(|e| format!("native session grant denied: {e:?}"))?;
        let subject = state
            .durable
            .admission
            .snapshot
            .subjects
            .iter()
            .find(|s| s.subject == session.subject)
            .ok_or("signed native subject association missing")?;
        let identifiers: Vec<_> = subject
            .identifiers
            .iter()
            .map(|i| QualifiedIdentifier {
                value: &i.value,
                scheme: &i.scheme,
                issuer: &i.issuer,
                project_id: &i.project_id,
                evidence_ref: &i.evidence_ref,
                valid_from_unix: i.valid_from_unix,
                valid_until_unix: i.valid_until_unix,
                revoked: subject.revoked,
            })
            .collect();
        let principal = AuthenticatedPrincipal {
            subject: &subject.subject,
            kind: subject.kind,
            authentication_ref: &session.authentication_ref,
            authenticated_until_unix: session.expires,
            identifiers: &identifiers,
        };
        let memberships = [AdmittedMembership {
            membership: &subject.membership,
            member_subject: &subject.subject,
            admission_ref: &subject.admission_ref,
            valid_from_unix: subject.valid_from_unix,
            valid_until_unix: subject.valid_until_unix,
            revoked: subject.revoked,
        }];
        let reviews: Vec<_> = subject
            .reviews
            .iter()
            .map(|r| ReviewGrant {
                id: &r.id,
                project_id: &self.anchor.project_id,
                document_id: &r.document_id,
                reviewer_subject: &subject.subject,
                admission_ref: &r.admission_ref,
                valid_from_unix: r.valid_from_unix,
                valid_until_unix: r.valid_until_unix,
                revoked: r.revoked,
            })
            .collect();
        let view = HostAuthorityView {
            principal: Some(&principal),
            project: &state.durable.admission.snapshot.project,
            memberships: &memberships,
            review_grants: &reviews,
            current_document: &state.durable.document,
            generation: state.generation,
            now_unix: now,
            limits: AuthoringLimits::default(),
        };
        // No callback sees source before basic canonical read admission passes.
        authorize(
            &view,
            &AuthoringRequest {
                action: AuthoringAction::ReadSource,
                target: view.current_document,
                expected_authority_generation: view.generation,
            },
        )
        .map_err(|e| e.to_string())?;
        operation(&view)
    }
    pub fn publication_is_current(&self, handle: &SessionHandle, generation: u64) -> bool {
        let Ok(now) = now() else {
            return false;
        };
        self.state.try_lock().is_ok_and(|state| {
            state.generation == generation && self.view(&state, handle, now, |_| Ok(())).is_ok()
        })
    }
    /// Source-bearing consumers bind the live session and exact source revision.
    /// A successor invalidates this context while allowing fresh reads without login.
    pub fn source_publication_is_current(
        &self,
        handle: &SessionHandle,
        generation: u64,
        expected: &DocumentRevision,
    ) -> bool {
        let Ok(now) = now() else {
            return false;
        };
        self.state.try_lock().is_ok_and(|state| {
            state.generation == generation
                && &state.durable.document == expected
                && self.view(&state, handle, now, |_| Ok(())).is_ok()
        })
    }
    pub fn transact<T>(
        &self,
        handle: &SessionHandle,
        operation: impl FnOnce(&HostAuthorityView<'_>) -> DocumentTransaction<T>,
    ) -> Result<T, String> {
        self.transact_with_package(handle, None, |view| (operation(view), None))
    }

    /// Package evidence commits under the same current native lock as the
    /// actual file write. A source review never attests these package bytes.
    pub fn transact_with_package<T>(
        &self,
        handle: &SessionHandle,
        intent: Option<&PackageWriteIntent>,
        operation: impl FnOnce(
            &HostAuthorityView<'_>,
        ) -> (DocumentTransaction<T>, Option<PackageWriteReceipt>),
    ) -> Result<T, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        if intent.is_some_and(|i| {
            i.filename.is_empty() || i.filename.len() > 240 || state.durable.packages.len() >= 256
        }) {
            return Err("invalid package audit intent or evidence budget exhausted".into());
        }
        if intent.is_some() {
            state.journal.preflight_package(&state.durable)?;
        }
        let ((transaction, package), actor) = self.view(&state, handle, now, |view| {
            let principal = view
                .principal
                .ok_or("native session principal unavailable")?;
            Ok((
                operation(view),
                (
                    principal.subject.to_string(),
                    principal.kind,
                    principal.authentication_ref.to_string(),
                    view.memberships
                        .first()
                        .ok_or("native membership unavailable")?
                        .admission_ref
                        .to_string(),
                ),
            ))
        })?;
        if transaction.reconciliation_required {
            state.reconciliation = true;
            return Err(
                "native operation may have committed; source/authority reconciliation required"
                    .into(),
            );
        }
        let mut next = state.durable.clone();
        let mut changed = false;
        if let Some(successor) = transaction.successor {
            let current = &state.durable.document;
            if successor.project_id != current.project_id
                || successor.document_id != current.document_id
                || successor.source_id != current.source_id
                || successor.revision <= current.revision
                || successor.source_digest == [0; 32]
                || successor.author_subject.as_ref() != Some(&actor.0)
            {
                state.reconciliation = true;
                return Err(
                    "invalid committed native document successor; reconciliation required".into(),
                );
            }
            next.document = successor;
            changed = true;
        }
        if let Some(receipt) = package {
            let Some(intent) = intent else {
                state.reconciliation = true;
                return Err(
                    "package write lacks native audit intent; reconciliation required".into(),
                );
            };
            if receipt.digest == [0; 32] || receipt.bytes == 0 || transaction.result.is_err() {
                state.reconciliation = true;
                return Err("package write outcome requires reconciliation".into());
            }
            next.packages.push(PackageWriteEvidence {
                filename: intent.filename.clone(),
                expected_digest: intent.expected_digest,
                result_digest: receipt.digest,
                result_bytes: receipt.bytes,
                source: next.document.clone(),
                actor_subject: actor.0,
                principal_kind: actor.1,
                admission_ref: actor.3,
                authentication_ref: actor.2,
                authority_generation: state.generation,
                occurred_at_unix: now,
            });
            changed = true;
        }
        if changed {
            if let Err(e) = state.journal.append(&next) {
                state.reconciliation = true;
                return Err(format!(
                    "native source/package may have committed; authority journal failed: {e}"
                ));
            }
            state.durable = next;
        }
        transaction.result
    }
}
