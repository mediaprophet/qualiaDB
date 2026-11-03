use super::{
    policy_digest, ActorPolicyScope, InferenceAdmission, PolicyError, SignedPolicy, TokenProposal,
};
use crate::modalities::{
    inference_guard::{evaluate_inference_guard, InferenceGuardAction},
    logic::WebizenVM,
};
use crate::NQuin;
use etalon_inference_contract as contract;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, MutexGuard,
};

pub(crate) struct AuthorityState {
    pub gate: Mutex<()>,
    pub generation: AtomicU64,
    pub connected: AtomicBool,
}
impl AuthorityState {
    pub fn lock(&self) -> Result<MutexGuard<'_, ()>, PolicyError> {
        self.gate.lock().map_err(|_| {
            self.connected.store(false, Ordering::Release);
            PolicyError::Revoked
        })
    }
    pub fn disconnect(&self) {
        let _guard = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        self.connected.store(false, Ordering::Release);
    }
    pub fn new(generation: u64) -> Self {
        Self {
            gate: Mutex::new(()),
            generation: AtomicU64::new(generation),
            connected: AtomicBool::new(true),
        }
    }
}

pub(crate) struct InstalledController {
    pub policy: SignedPolicy,
    pub key: [u8; 32],
    pub authority: Arc<AuthorityState>,
    pub epoch: u64,
}
impl InstalledController {
    pub(crate) fn live(&self, now: u64) -> Result<(), PolicyError> {
        if !self.authority.connected.load(Ordering::Acquire)
            || self.authority.generation.load(Ordering::Acquire) != self.epoch
        {
            return Err(PolicyError::Revoked);
        }
        self.policy.document.valid_at(now)
    }
    fn evaluate(
        &self,
        proposal: TokenProposal,
        actor: ActorPolicyScope,
        now: u64,
    ) -> Result<contract::Decision, PolicyError> {
        self.live(now)?;
        if now >= actor.valid_until
            || !self
                .policy
                .document
                .rules
                .actors
                .as_slice()
                .contains(&actor)
        {
            return Err(PolicyError::Scope);
        }
        let doc = &self.policy.document;
        let rules = &doc.rules;
        // Every proposal starts with empty registers. Preserve the actual VM's
        // sequential flag semantics; use HaltIfFalse for conjunction, not an
        // invented implicit AND across instructions.
        let mut vm = WebizenVM::new();
        for (dst, instruction) in vm.bytecode_buffer.iter_mut().zip(rules.program.as_slice()) {
            *dst = Some(instruction.lower()?);
        }
        let vm_allowed = vm.execute_constraint(&proposal.observation);
        let mut facts = [NQuin::default(); 256];
        let n = rules.facts.as_slice().len();
        facts[..n].copy_from_slice(rules.facts.as_slice());
        facts[n] = proposal.observation;
        let mut trace = [NQuin::default(); 256];
        let t = rules.trace.as_slice().len();
        trace[..t].copy_from_slice(rules.trace.as_slice());
        trace[t] = proposal.observation;
        let guard = evaluate_inference_guard(
            rules.norms.as_slice(),
            &facts[..n + 1],
            rules.epistemic.as_slice(),
            &trace[..t + 1],
            actor.subject,
            rules.required_claim,
            rules.temporal_invariant,
            now as u32,
            rules.non_derogable,
        );
        Ok(
            if !vm_allowed || guard.action == InferenceGuardAction::Deny {
                contract::Decision::Deny
            } else if guard.action == InferenceGuardAction::Steer {
                contract::Decision::Steer {
                    steering_digest: policy_digest(doc),
                }
            } else {
                contract::Decision::Allow
            },
        )
    }
}

struct SentinelVerifier {
    controller: Arc<InstalledController>,
    admission: InferenceAdmission,
    config: contract::SessionConfig,
    last_now: u64,
    actor: ActorPolicyScope,
}
impl contract::HostVerifier for SentinelVerifier {
    fn verify_admission(
        &mut self,
        request: &contract::AdmissionRequest,
        evidence: &[u8],
    ) -> Result<contract::ControllerBinding, contract::VerificationError> {
        if request.config != self.config
            || request.purpose != contract::GovernedPurpose::Application
            || evidence != self.admission.request_digest
        {
            return Err(contract::VerificationError::Unauthenticated);
        }
        self.controller
            .live(request.now_tick)
            .map_err(verification_error)?;
        let doc = &self.controller.policy.document;
        Ok(contract::ControllerBinding {
            identity: self.controller.key,
            generation: doc.controller_generation,
            purpose: request.purpose,
            expires_at_tick: doc.expires_at,
        })
    }
    fn verify_decision(
        &mut self,
        binding: &contract::ControllerBinding,
        envelope: &contract::DecisionEnvelope,
        evidence: &[u8],
    ) -> Result<(), contract::VerificationError> {
        let proposal = TokenProposal::decode(evidence).map_err(verification_error)?;
        let expected_position = envelope
            .candidate
            .sequence
            .checked_sub(1)
            .and_then(|n| self.admission.first_position.checked_add(n));
        if binding.identity != self.controller.key
            || binding.generation != self.controller.policy.document.controller_generation
            || envelope.candidate.identity != self.config.identity
            || envelope.candidate.mode != contract::ExecutionMode::GovernedApplication
            || Some(proposal.position) != expected_position
            || proposal.observation.context != self.admission.graph_scope
            || proposal.output().map_err(verification_error)? != envelope.candidate.output
            || envelope.expires_at_tick != self.controller.policy.document.expires_at
            || self
                .controller
                .evaluate(proposal, self.actor, self.last_now)
                .map_err(verification_error)?
                != envelope.decision
        {
            return Err(contract::VerificationError::Unauthenticated);
        }
        Ok(())
    }
    fn verify_liveness(
        &mut self,
        binding: &contract::ControllerBinding,
        now: u64,
    ) -> Result<(), contract::VerificationError> {
        if binding.identity != self.controller.key
            || binding.generation != self.controller.policy.document.controller_generation
        {
            return Err(contract::VerificationError::Unauthenticated);
        }
        self.controller.live(now).map_err(verification_error)?;
        self.last_now = now;
        Ok(())
    }
}
fn verification_error(error: PolicyError) -> contract::VerificationError {
    match error {
        PolicyError::Expired => contract::VerificationError::Expired,
        PolicyError::Revoked => contract::VerificationError::Disconnected,
        _ => contract::VerificationError::Unauthenticated,
    }
}

/// Private-key-bound admitted session. No public constructor from an enum or
/// purported verifier exists; the native host creates it from installed policy.
/// The caller must use a trusted clock. This API owns no numerical state.
pub(crate) struct PolicyTokenSession {
    inner: contract::GovernedSession<SentinelVerifier>,
    pub(crate) controller: Arc<InstalledController>,
    pub(crate) actor: ActorPolicyScope,
}

/// Decision for one exact proposal. Only an Allow can be committed; committing
/// advances metadata, never publishes bytes or attests a numerical restoration.
pub struct TokenDecision {
    key: contract::CandidateKey,
    proposal: TokenProposal,
    decision: contract::Decision,
}
impl TokenDecision {
    pub fn decision(&self) -> contract::Decision {
        self.decision
    }
    pub fn proposal(&self) -> TokenProposal {
        self.proposal
    }
    pub fn key(&self) -> contract::CandidateKey {
        self.key
    }
}
impl PolicyTokenSession {
    pub(crate) fn admit(
        controller: Arc<InstalledController>,
        admission: InferenceAdmission,
        actor: ActorPolicyScope,
        session_id: u64,
        now: u64,
    ) -> Result<Self, PolicyError> {
        let doc = &controller.policy.document;
        controller.live(now)?;
        actor.validate()?;
        if now >= actor.valid_until || !doc.rules.actors.as_slice().contains(&actor) {
            return Err(PolicyError::Scope);
        }
        if admission.request_digest == [0; 32]
            || admission.model_digest == [0; 32]
            || admission.device_id == 0
            || !doc.rules.model.allows(admission.model_digest)
            || !doc.rules.request.allows(admission.request_digest)
            || (!doc.rules.graph_scopes.as_slice().is_empty()
                && !doc
                    .rules
                    .graph_scopes
                    .as_slice()
                    .contains(&admission.graph_scope))
        {
            return Err(PolicyError::Scope);
        }
        let config = contract::SessionConfig {
            identity: contract::SessionIdentity {
                session_id,
                session_generation: doc.policy_generation,
                request_id: session_id,
                request_generation: doc.policy_generation,
                model_id: admission.model_digest,
                model_generation: admission.model_generation,
                device_id: admission.device_id,
                device_generation: admission.device_generation,
                policy_id: policy_digest(doc),
                policy_generation: doc.policy_generation,
            },
            instrumentation: admission.instrumentation,
            max_candidate_tokens: 1,
        };
        let verifier = SentinelVerifier {
            controller: Arc::clone(&controller),
            admission,
            config,
            last_now: now,
            actor,
        };
        let inner = contract::GovernedSession::admit(
            verifier,
            contract::AdmissionRequest {
                config,
                purpose: contract::GovernedPurpose::Application,
                profile: contract::GovernanceProfile::ApprovalBeforePublication,
                now_tick: now,
            },
            &admission.request_digest,
        )
        .map_err(|_| PolicyError::Session)?;
        Ok(Self {
            inner,
            controller,
            actor,
        })
    }
    pub fn prepare(
        &mut self,
        proposal: TokenProposal,
        now: u64,
    ) -> Result<TokenDecision, PolicyError> {
        let evidence = match proposal.encode() {
            Ok(evidence) => evidence,
            Err(error) => {
                self.inner.cancel();
                return Err(error);
            }
        };
        let key = self
            .inner
            .propose(proposal.output()?, now)
            .map_err(|_| PolicyError::Session)?;
        // Re-evaluation by the verifier binds the decision to exact evidence.
        // Both passes use the same immutable policy, actual VM and graph guard.
        let decision = match self.controller.evaluate(proposal, self.actor, now) {
            Ok(decision) => decision,
            Err(error) => {
                self.inner.disconnect();
                return Err(error);
            }
        };
        self.inner
            .decide(
                contract::DecisionEnvelope {
                    candidate: key,
                    decision,
                    expires_at_tick: self.controller.policy.document.expires_at,
                },
                &evidence,
                now,
            )
            .map_err(|_| PolicyError::Session)?;
        Ok(TokenDecision {
            key,
            proposal,
            decision,
        })
    }
    pub fn commit(&mut self, decision: &TokenDecision, now: u64) -> Result<(), PolicyError> {
        self.inner
            .commit(decision.key, now)
            .map(|_| ())
            .map_err(|_| PolicyError::Session)
    }
    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
    pub fn snapshot(&self) -> contract::SessionSnapshot {
        self.inner.snapshot()
    }
}
