use super::super::{ActorPolicyScope, InferenceAdmission};
use super::{
    controller::{clock, verification_error, EvidenceController},
    retained::EvidenceStorage,
};
use etalon_inference_contract::{
    artifact::*, ControllerBinding, GovernedPurpose, VerificationError,
};
use std::sync::Arc;

pub(super) struct ArtifactPolicyVerifier<'slot, 'bytes> {
    pub controller: Arc<EvidenceController>,
    pub storage: &'slot EvidenceStorage<'bytes>,
    pub request: InferenceAdmission,
    pub actor: ActorPolicyScope,
    pub config: ArtifactConfig,
}
impl ArtifactVerifier for ArtifactPolicyVerifier<'_, '_> {
    fn admit(
        &mut self,
        request: &ArtifactAdmission,
        evidence: &[u8],
    ) -> Result<ControllerBinding, VerificationError> {
        if request.config != self.config
            || request.purpose != GovernedPurpose::Application
            || evidence != self.request.request_digest
        {
            return Err(VerificationError::Unauthenticated);
        }
        self.controller
            .actor(self.actor, clock().map_err(verification_error)?)
            .map_err(verification_error)?;
        Ok(self.controller.binding())
    }
    fn decide(
        &mut self,
        binding: &ControllerBinding,
        decision: &ArtifactDecision,
        evidence: &[u8],
    ) -> Result<(), VerificationError> {
        let stored = self.storage.lock().map_err(verification_error)?;
        let bound = stored.artifact().map_err(verification_error)?;
        let p = stored.proposal().map_err(verification_error)?;
        bound
            .match_session_key(&decision.candidate)
            .map_err(verification_error)?;
        let now = clock().map_err(verification_error)?;
        self.controller
            .check_binding(binding, now)
            .map_err(verification_error)?;
        if evidence != bound.output().observation_digest
            || p.identity != self.config.identity
            || p.request_digest != self.request.request_digest
            || p.observation.context != self.request.graph_scope
            || decision.expires_at_tick != self.controller.policy.document.base.expires_at
            || self
                .controller
                .evaluate(&stored.bound().map_err(verification_error)?, self.actor)
                .map_err(verification_error)?
                != decision.decision
        {
            return Err(VerificationError::Unauthenticated);
        }
        Ok(())
    }
    fn live(&mut self, binding: &ControllerBinding, _now: u64) -> Result<(), VerificationError> {
        self.controller
            .check_binding(binding, clock().map_err(verification_error)?)
            .map_err(verification_error)
    }
}
