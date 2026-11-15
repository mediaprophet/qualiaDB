use super::super::{ActorPolicyScope, InferenceAdmission, PolicyError};
use super::{
    controller::{clock, verification_error, EvidenceController},
    retained::EvidenceStorage,
    Payload,
};
use etalon_inference_contract as c;
use std::sync::Arc;

pub(super) struct TokenVerifier<'slot, 'bytes> {
    pub controller: Arc<EvidenceController>,
    pub storage: &'slot EvidenceStorage<'bytes>,
    pub request: InferenceAdmission,
    pub actor: ActorPolicyScope,
    pub config: c::SessionConfig,
}
pub(super) fn output(bound: &super::BoundProposal<'_>) -> Result<c::CandidateOutput, PolicyError> {
    if !matches!(bound.proposal.payload, Payload::Token { .. }) {
        return Err(PolicyError::InvalidCandidate);
    }
    Ok(c::CandidateOutput {
        output_digest: bound.payload_digest,
        observation_digest: bound.digest(),
        token_count: 1,
    })
}
impl c::HostVerifier for TokenVerifier<'_, '_> {
    fn verify_admission(
        &mut self,
        request: &c::AdmissionRequest,
        evidence: &[u8],
    ) -> Result<c::ControllerBinding, c::VerificationError> {
        if request.config != self.config
            || request.purpose != c::GovernedPurpose::Application
            || request.profile != c::GovernanceProfile::ApprovalBeforePublication
            || evidence != self.request.request_digest
        {
            return Err(c::VerificationError::Unauthenticated);
        }
        self.controller
            .actor(self.actor, clock().map_err(verification_error)?)
            .map_err(verification_error)?;
        Ok(self.controller.binding())
    }
    fn verify_decision(
        &mut self,
        binding: &c::ControllerBinding,
        envelope: &c::DecisionEnvelope,
        evidence: &[u8],
    ) -> Result<(), c::VerificationError> {
        let stored = self.storage.lock().map_err(verification_error)?;
        let bound = stored.bound().map_err(verification_error)?;
        let p = bound.proposal;
        let position = envelope
            .candidate
            .sequence
            .checked_sub(1)
            .and_then(|n| self.request.first_position.checked_add(n));
        let now = clock().map_err(verification_error)?;
        self.controller
            .check_binding(binding, now)
            .map_err(verification_error)?;
        if evidence != bound.digest()
            || p.identity != self.config.identity
            || p.sequence != envelope.candidate.sequence
            || p.request_digest != self.request.request_digest
            || p.observation.context != self.request.graph_scope
            || envelope.candidate.identity != self.config.identity
            || envelope.candidate.mode != c::ExecutionMode::GovernedApplication
            || !matches!(p.payload,Payload::Token{position:actual,..} if Some(actual)==position)
            || output(&bound).map_err(verification_error)? != envelope.candidate.output
            || envelope.expires_at_tick != self.controller.policy.document.base.expires_at
            || self
                .controller
                .evaluate(&bound, self.actor)
                .map_err(verification_error)?
                != envelope.decision
        {
            return Err(c::VerificationError::Unauthenticated);
        }
        Ok(())
    }
    fn verify_liveness(
        &mut self,
        binding: &c::ControllerBinding,
        _now: u64,
    ) -> Result<(), c::VerificationError> {
        self.controller
            .check_binding(binding, clock().map_err(verification_error)?)
            .map_err(verification_error)
    }
}
