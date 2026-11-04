//! One live admission, one retained slot across both candidate domains.
use super::super::{ActorPolicyScope, InferenceAdmission, PolicyError};
use super::{
    artifact_session::ArtifactPolicyVerifier,
    controller::{clock, EvidenceController},
    retained::{AcceptedEvidence, AcceptedKey, EvidenceStorage},
    token_session::{self, TokenVerifier},
    Payload, Proposal,
};
use crate::poet_host::native_authority::{
    inference::AuthenticatedInferenceAssociation, NativeAuthority,
};
use etalon_inference_contract::{self as c, artifact as a};
use std::sync::Arc;

/// Not Clone/Copy: a decision can be offered to acceptance only once. The key is
/// metadata; even a copied key cannot construct another ticket or accepted view.
pub struct PendingEvidence {
    key: AcceptedKey,
    digest: [u8; 32],
    decision: c::Decision,
}
impl PendingEvidence {
    pub fn decision(&self) -> c::Decision {
        self.decision
    }
    pub fn key(&self) -> AcceptedKey {
        self.key
    }
}
pub struct GovernedEvidenceSession<'slot, 'bytes> {
    controller: Arc<EvidenceController>,
    owner: Arc<NativeAuthority>,
    association: AuthenticatedInferenceAssociation,
    actor: ActorPolicyScope,
    request: InferenceAdmission,
    identity: c::SessionIdentity,
    storage: &'slot EvidenceStorage<'bytes>,
    token: c::GovernedSession<TokenVerifier<'slot, 'bytes>>,
    artifact: a::GovernedArtifactSession<ArtifactPolicyVerifier<'slot, 'bytes>>,
    terminal: bool,
    #[cfg(test)]
    after_bulk: Option<Box<dyn FnOnce() + Send>>,
}
impl<'slot, 'bytes> GovernedEvidenceSession<'slot, 'bytes> {
    pub(crate) fn admit(
        controller: Arc<EvidenceController>,
        request: InferenceAdmission,
        session_id: u64,
        owner: Arc<NativeAuthority>,
        association: AuthenticatedInferenceAssociation,
        storage: &'slot mut EvidenceStorage<'bytes>,
    ) -> Result<Self, PolicyError> {
        let storage: &'slot EvidenceStorage<'bytes> = storage;
        let (identity, actor, token, artifact) = owner
            .with_inference_association_guard(&association, |view| {
                let _gate = controller.authority.lock()?;
                let retained = storage.lock()?;
                if !retained.is_empty() {
                    return Err(PolicyError::Capacity);
                }
                let limits = retained.limits;
                drop(retained);
                let actor = ActorPolicyScope::from_evidence(
                    view.revalidate_current()
                        .map_err(|_| PolicyError::Revoked)?,
                );
                let now = clock()?;
                let identity = controller.identity(request, actor, session_id, now)?;
                let config = c::SessionConfig {
                    identity,
                    instrumentation: request.instrumentation,
                    max_candidate_tokens: 1,
                };
                let token = c::GovernedSession::admit(
                    TokenVerifier {
                        controller: Arc::clone(&controller),
                        storage,
                        request,
                        actor,
                        config,
                    },
                    c::AdmissionRequest {
                        config,
                        purpose: c::GovernedPurpose::Application,
                        profile: c::GovernanceProfile::ApprovalBeforePublication,
                        now_tick: now,
                    },
                    &request.request_digest,
                )
                .map_err(|_| PolicyError::Session)?;
                let config = a::ArtifactConfig {
                    identity,
                    instrumentation: request.instrumentation,
                    max_artifact_bytes: limits.max_payload_bytes as u64,
                    allow_empty: controller.policy.document.observations.allow_empty_artifact,
                    max_events: limits.max_artifact_events,
                };
                let artifact = a::GovernedArtifactSession::admit(
                    ArtifactPolicyVerifier {
                        controller: Arc::clone(&controller),
                        storage,
                        request,
                        actor,
                        config,
                    },
                    a::ArtifactAdmission {
                        config,
                        purpose: c::GovernedPurpose::Application,
                        now_tick: now,
                    },
                    &request.request_digest,
                )
                .map_err(|_| PolicyError::Session)?;
                Ok((identity, actor, token, artifact))
            })
            .map_err(|_| PolicyError::Revoked)??;
        Ok(Self {
            controller,
            owner,
            association,
            actor,
            request,
            identity,
            storage,
            token,
            artifact,
            terminal: false,
            #[cfg(test)]
            after_bulk: None,
        })
    }
    pub fn identity(&self) -> c::SessionIdentity {
        self.identity
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
    pub fn cancel(&mut self) {
        self.terminal = true;
        self.token.cancel();
        self.artifact.cancel();
        if let Ok(mut stored) = self.storage.lock() {
            stored.clear();
        }
    }
    pub fn prepare_token(
        &mut self,
        proposal: Proposal<'_>,
    ) -> Result<PendingEvidence, PolicyError> {
        if !matches!(proposal.payload, Payload::Token { .. }) {
            self.cancel();
            return Err(PolicyError::InvalidCandidate);
        }
        self.prepare(proposal, None)
    }
    /// Parent is a bound declaration, not proof of a previously accepted event.
    /// The runtime adapter must derive lineage from an actual accepted record.
    pub fn prepare_artifact(
        &mut self,
        proposal: Proposal<'_>,
        parent: Option<a::ParentEventBinding>,
    ) -> Result<PendingEvidence, PolicyError> {
        if !matches!(proposal.payload, Payload::Artifact { .. }) {
            self.cancel();
            return Err(PolicyError::InvalidCandidate);
        }
        self.prepare(proposal, parent)
    }
    fn prepare(
        &mut self,
        proposal: Proposal<'_>,
        parent: Option<a::ParentEventBinding>,
    ) -> Result<PendingEvidence, PolicyError> {
        if self.terminal {
            return Err(PolicyError::Revoked);
        }
        self.terminal = true;
        let controller = Arc::clone(&self.controller);
        let result = self
            .owner
            .with_inference_association_guard(&self.association, |view| {
                let _gate = controller.authority.lock()?;
                if proposal.identity != self.identity
                    || proposal.request_digest != self.request.request_digest
                    || proposal.observation.context != self.request.graph_scope
                {
                    return Err(PolicyError::Scope);
                }
                let (digest, decision, token_output, artifact_output) = {
                    let mut stored = self.storage.lock()?;
                    stored.stage(proposal, parent)?;
                    let bound = stored.bound()?;
                    let decision = controller.evaluate(&bound, self.actor)?;
                    let digest = stored.control_digest()?;
                    match proposal.payload {
                        Payload::Token { .. } => {
                            (digest, decision, Some(token_session::output(&bound)?), None)
                        }
                        Payload::Artifact { .. } => {
                            (digest, decision, None, Some(stored.artifact()?.output()))
                        }
                    }
                };
                let now = clock()?;
                let key = if let Some(output) = token_output {
                    let key = self
                        .token
                        .propose(output, now)
                        .map_err(|_| PolicyError::Session)?;
                    if key.sequence != proposal.sequence {
                        return Err(PolicyError::InvalidCandidate);
                    }
                    self.token
                        .decide(
                            c::DecisionEnvelope {
                                candidate: key,
                                decision,
                                expires_at_tick: controller.policy.document.base.expires_at,
                            },
                            &digest,
                            now,
                        )
                        .map_err(|_| PolicyError::Session)?;
                    AcceptedKey::Token(key)
                } else {
                    let key = self
                        .artifact
                        .propose(artifact_output.ok_or(PolicyError::InvalidCandidate)?, now)
                        .map_err(|_| PolicyError::Session)?;
                    self.storage.lock()?.artifact()?.match_session_key(&key)?;
                    match self.artifact.decide(
                        a::ArtifactDecision {
                            candidate: key,
                            decision,
                            expires_at_tick: controller.policy.document.base.expires_at,
                        },
                        &digest,
                        now,
                    ) {
                        Ok(()) => (),
                        Err(a::ArtifactError::Denied) if decision == c::Decision::Deny => (),
                        Err(a::ArtifactError::Steered)
                            if matches!(decision, c::Decision::Steer { .. }) =>
                        {
                            ()
                        }
                        Err(_) => return Err(PolicyError::Session),
                    }
                    AcceptedKey::Artifact(key)
                };
                #[cfg(test)]
                if let Some(hook) = self.after_bulk.take() {
                    hook();
                }
                // Hashing/copying and both real VM passes can take time. Revalidate
                // the complete current owner predicate AFTER all bulk work.
                let actor = ActorPolicyScope::from_evidence(
                    view.revalidate_current()
                        .map_err(|_| PolicyError::Revoked)?,
                );
                if actor != self.actor {
                    return Err(PolicyError::Scope);
                }
                controller.actor(actor, clock()?)?;
                Ok(PendingEvidence {
                    key,
                    digest,
                    decision,
                })
            })
            .map_err(|_| PolicyError::Revoked)
            .and_then(|r| r);
        match &result {
            Ok(p) if p.decision == c::Decision::Allow => self.terminal = false,
            _ => self.cancel(),
        }
        result
    }
    /// CPU profile admission checks against this actual signed immutable policy.
    /// Returning these fields alone is not authorization; final acceptance still
    /// performs complete current owner/policy revalidation under both gates.
    pub(in crate::inference::inference_agent) fn cpu_projection_binding(
        &self,
    ) -> Result<crate::inference::inference_agent::governed_cpu::CpuProjectionBinding, PolicyError>
    {
        use crate::inference::inference_agent::governed_cpu::{
            check_projection_program, CpuProjectionBinding,
        };
        if self.terminal {
            return Err(PolicyError::Revoked);
        }
        self.owner
            .with_inference_association_guard(&self.association, |view| {
                let _gate = self.controller.authority.lock()?;
                let policy = &self.controller.policy.document;
                if policy.base.rules.request
                    != super::super::DigestScope::Exact(self.request.request_digest)
                    || policy.observations.allowed_sources
                        & super::CandidateSource::ModelToken.bit()
                        == 0
                    || matches!(
                        policy.observations.score,
                        super::ScoreRequirement::Required(super::ScoreMeaning::Probability)
                    )
                {
                    return Err(PolicyError::Scope);
                }
                check_projection_program(policy.base.rules.program.as_slice())
                    .map_err(|_| PolicyError::InvalidProgram)?;
                let actor = ActorPolicyScope::from_evidence(
                    view.revalidate_current()
                        .map_err(|_| PolicyError::Revoked)?,
                );
                if actor != self.actor {
                    return Err(PolicyError::Scope);
                }
                self.controller.actor(actor, clock()?)?;
                let stored = self.storage.lock()?;
                Ok(CpuProjectionBinding {
                    identity: self.identity,
                    request_digest: self.request.request_digest,
                    actor: self.actor.subject,
                    graph: self.request.graph_scope,
                    max_payload: stored.limits.max_payload_bytes,
                    first_position: self.request.first_position,
                })
            })
            .map_err(|_| PolicyError::Revoked)
            .and_then(|r| r)
    }
    /// Commit/enqueue is the linearization point under owner -> policy -> slot.
    /// The returned view is drained AFTER all authority locks have been released.
    pub fn accept<'drain>(
        &'drain mut self,
        pending: PendingEvidence,
    ) -> Result<AcceptedEvidence<'drain, 'bytes>, PolicyError> {
        self.accept_inner(pending, None, None)
    }
    /// Native cooperative cancellation linearizes at the final atomic load
    /// immediately before commit, under the existing owner/policy/slot guards.
    pub(in crate::inference::inference_agent) fn accept_native<'drain>(
        &'drain mut self,
        pending: PendingEvidence,
        control: Option<&crate::inference::inference_agent::control::DecodeControl>,
    ) -> Result<AcceptedEvidence<'drain, 'bytes>, PolicyError> {
        self.accept_inner(pending, None, control)
    }
    /// Private typed numerical integration. No caller-selected callback can run
    /// under authority locks. The private CPU runner owns the actual transaction.
    pub(in crate::inference::inference_agent) fn accept_dense<'drain>(
        &'drain mut self,
        pending: PendingEvidence,
        dense: crate::inference::inference_agent::governed_cpu::DenseAcceptanceCommit<'_, '_, '_>,
    ) -> Result<AcceptedEvidence<'drain, 'bytes>, PolicyError> {
        self.accept_inner(pending, Some(dense), None)
    }
    fn accept_inner<'drain>(
        &'drain mut self,
        pending: PendingEvidence,
        dense: Option<
            crate::inference::inference_agent::governed_cpu::DenseAcceptanceCommit<'_, '_, '_>,
        >,
        control: Option<&crate::inference::inference_agent::control::DecodeControl>,
    ) -> Result<AcceptedEvidence<'drain, 'bytes>, PolicyError> {
        if self.terminal {
            return Err(PolicyError::Revoked);
        }
        self.terminal = true;
        let controller = Arc::clone(&self.controller);
        let result = self
            .owner
            .with_inference_association_guard(&self.association, |view| {
                let _gate = controller.authority.lock()?;
                let mut stored = self.storage.lock()?;
                if stored.accepted.is_some() {
                    return Err(PolicyError::Capacity);
                }
                let digest = stored.control_digest()?;
                if pending.decision != c::Decision::Allow || digest != pending.digest {
                    return Err(PolicyError::InvalidCandidate);
                }
                // Complete digest/domain check precedes rules and fresh time.
                // This also precomputes the exact typed key for infallible finish.
                let dense = match dense {
                    Some(commit) => match pending.key {
                        AcceptedKey::Token(key) if commit.matches(digest) => Some((commit, key)),
                        _ => return Err(PolicyError::InvalidCandidate),
                    },
                    None => None,
                };
                let bound = stored.bound()?;
                match pending.key {
                    AcceptedKey::Token(key) => {
                        if token_session::output(&bound)? != key.output
                            || key.identity != self.identity
                            || key.sequence != bound.proposal.sequence
                        {
                            return Err(PolicyError::InvalidCandidate);
                        }
                    }
                    AcceptedKey::Artifact(key) => stored.artifact()?.match_session_key(&key)?,
                }
                #[cfg(test)]
                if let Some(hook) = self.after_bulk.take() {
                    hook();
                }
                if controller.evaluate(&bound, self.actor)? != c::Decision::Allow {
                    return Err(PolicyError::Scope);
                }
                let actor = ActorPolicyScope::from_evidence(
                    view.revalidate_current()
                        .map_err(|_| PolicyError::Revoked)?,
                );
                if actor != self.actor {
                    return Err(PolicyError::Scope);
                }
                let now = clock()?;
                controller.actor(actor, now)?;
                if control.is_some_and(
                    crate::inference::inference_agent::control::DecodeControl::is_cancelled,
                ) {
                    return Err(PolicyError::Revoked);
                }
                // No hashing, parsing, callback or bulk copy follows this final
                // trusted-time/current-actor check. Contract liveness also rechecks.
                match pending.key {
                    AcceptedKey::Token(key) => {
                        self.token
                            .commit(key, now)
                            .map_err(|_| PolicyError::Session)?;
                    }
                    AcceptedKey::Artifact(key) => {
                        let mut accepted = None;
                        self.artifact
                            .accept_into(key, now, &mut accepted)
                            .map_err(|_| PolicyError::Session)?;
                        if accepted.as_ref().map(|r| *r.key()) != Some(key) {
                            return Err(PolicyError::Session);
                        }
                    }
                }
                if let Some((commit, key)) = dense {
                    commit.finish(key);
                }
                stored.accepted = Some(pending.key);
                Ok(())
            })
            .map_err(|_| PolicyError::Revoked)
            .and_then(|r| r);
        if let Err(e) = result {
            self.cancel();
            return Err(e);
        }
        AcceptedEvidence::new(self.storage.lock()?, &mut self.terminal)
    }
    #[cfg(test)]
    pub(crate) fn after_bulk_once(&mut self, hook: impl FnOnce() + Send + 'static) {
        self.after_bulk = Some(Box::new(hook));
    }
}
impl Drop for GovernedEvidenceSession<'_, '_> {
    fn drop(&mut self) {
        self.cancel();
    }
}
