//! Installed V3 authority shares the host's existing invalidation gate/epoch.
use super::super::{
    controller::AuthorityState, ActorPolicyScope, InferenceAdmission, PolicyError,
    TrustedPolicyIdentity,
};
use super::{
    policy_digest_v3, BoundProposal, SignedEvidencePolicy, VerifiedEvidencePolicy, AUDIENCE_V3,
};
use etalon_inference_contract::{
    ControllerBinding, Decision, GovernedPurpose, SessionIdentity, VerificationError,
};
use std::sync::{atomic::Ordering, Arc};

pub(crate) struct EvidenceController {
    pub(super) policy: SignedEvidencePolicy,
    pub(super) key: [u8; 32],
    pub(crate) authority: Arc<AuthorityState>,
    epoch: u64,
}
pub(super) fn clock() -> Result<u64, PolicyError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| PolicyError::Clock)
}
pub(super) fn verification_error(e: PolicyError) -> VerificationError {
    match e {
        PolicyError::Expired => VerificationError::Expired,
        PolicyError::Revoked => VerificationError::Disconnected,
        _ => VerificationError::Unauthenticated,
    }
}
impl EvidenceController {
    pub(crate) fn new(
        policy: SignedEvidencePolicy,
        trust: TrustedPolicyIdentity,
        authority: Arc<AuthorityState>,
        epoch: u64,
    ) -> Result<Self, PolicyError> {
        if policy.document.base.audience != trust.audience {
            return Err(PolicyError::WrongAuthority);
        }
        policy.verify(
            &TrustedPolicyIdentity {
                audience: AUDIENCE_V3,
                ..trust
            },
            policy.document.base.not_before,
        )?;
        Ok(Self {
            policy,
            key: trust.key,
            authority,
            epoch,
        })
    }
    pub(super) fn live(&self, now: u64) -> Result<(), PolicyError> {
        if !self.authority.connected.load(Ordering::Acquire)
            || self.authority.generation.load(Ordering::Acquire) != self.epoch
        {
            return Err(PolicyError::Revoked);
        }
        self.policy.document.base.valid_at(now)
    }
    pub(super) fn actor(&self, actor: ActorPolicyScope, now: u64) -> Result<(), PolicyError> {
        self.live(now)?;
        actor.validate()?;
        if now >= actor.valid_until
            || !self
                .policy
                .document
                .base
                .rules
                .actors
                .as_slice()
                .contains(&actor)
        {
            return Err(PolicyError::Scope);
        }
        Ok(())
    }
    pub(super) fn identity(
        &self,
        request: InferenceAdmission,
        actor: ActorPolicyScope,
        session_id: u64,
        now: u64,
    ) -> Result<SessionIdentity, PolicyError> {
        self.actor(actor, now)?;
        let doc = &self.policy.document.base;
        if session_id == 0
            || request.request_digest == [0; 32]
            || request.model_digest == [0; 32]
            || request.model_generation == 0
            || request.device_id == 0
            || request.device_generation == 0
            || !doc.rules.model.allows(request.model_digest)
            || !doc.rules.request.allows(request.request_digest)
            || (!doc.rules.graph_scopes.as_slice().is_empty()
                && !doc
                    .rules
                    .graph_scopes
                    .as_slice()
                    .contains(&request.graph_scope))
        {
            return Err(PolicyError::Scope);
        }
        Ok(SessionIdentity {
            session_id,
            session_generation: doc.policy_generation,
            request_id: session_id,
            request_generation: doc.policy_generation,
            model_id: request.model_digest,
            model_generation: request.model_generation,
            device_id: request.device_id,
            device_generation: request.device_generation,
            policy_id: policy_digest_v3(&self.policy.document),
            policy_generation: doc.policy_generation,
        })
    }
    pub(super) fn binding(&self) -> ControllerBinding {
        ControllerBinding {
            identity: self.key,
            generation: self.policy.document.base.controller_generation,
            purpose: GovernedPurpose::Application,
            expires_at_tick: self.policy.document.base.expires_at,
        }
    }
    pub(super) fn check_binding(
        &self,
        binding: &ControllerBinding,
        now: u64,
    ) -> Result<(), PolicyError> {
        if *binding != self.binding() {
            return Err(PolicyError::WrongAuthority);
        }
        self.live(now)
    }
    pub(super) fn evaluate(
        &self,
        proposal: &BoundProposal<'_>,
        actor: ActorPolicyScope,
    ) -> Result<Decision, PolicyError> {
        // Payload work completes BEFORE the time used by temporal VM/graph
        // rules, not merely before a later actor-expiry check.
        proposal.recheck_payload()?;
        let now = clock()?;
        self.actor(actor, now)?;
        // The immutable installed policy was signature-verified on construction.
        // Every decision still executes its real VM/graph rules on retained bytes.
        let policy = VerifiedEvidencePolicy {
            signed: &self.policy,
        };
        super::evaluate::validate_context(&policy, proposal, actor, now)?;
        super::evaluate::execute_programs(&policy, proposal, actor, now)
    }
}
