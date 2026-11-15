use super::{
    model::PreparedCpuModel, numerical::NumericalProposal, request::PreparedRequest,
    state::StagedHost, CpuAdmissionError,
};
use crate::inference::inference_agent::governed::evidence::{
    BoundProposal, Payload, Proposal, ScoreMeaning,
};
use etalon_inference_contract::CandidateKey;

/// Affine numerical transaction plus host delta. No public constructor, Clone,
/// callback, numerical buffers or raw transaction escape. Final acceptance must
/// compare this digest to the complete retained proposal before the fresh gate.
pub(in crate::inference::inference_agent) struct DenseAcceptanceCommit<'w, 'u, 'h> {
    pending: NumericalProposal<'w, 'u>,
    host: StagedHost<'h>,
    digest: [u8; 32],
}
impl<'w, 'u, 'h> DenseAcceptanceCommit<'w, 'u, 'h> {
    pub(super) fn bind(
        pending: NumericalProposal<'w, 'u>,
        host: StagedHost<'h>,
        proposal: Proposal<'_>,
        payload_limit: u64,
        model: &PreparedCpuModel<'_, '_>,
        request: &PreparedRequest<'_>,
    ) -> Result<Self, CpuAdmissionError> {
        let actual = pending.output();
        if !matches!(proposal.payload, Payload::Token { token, position, .. }
            if token == actual.token_id && host.matches(token, position)
                && u64::try_from(pending.position()) == Ok(position))
            || proposal.identity.model_id != model.model_digest
            || pending.model() != model.model_digest
            || proposal.request_digest != request.digest
            || !proposal.numerical.is_some_and(|s| {
                s.meaning == ScoreMeaning::Logit && s.value.to_bits() == actual.max_logit.to_bits()
            })
        {
            return Err(CpuAdmissionError::InvalidIdentity);
        }
        let bound = BoundProposal::bind(proposal, payload_limit)
            .map_err(|_| CpuAdmissionError::InvalidIdentity)?;
        Ok(Self {
            pending,
            host,
            digest: bound.digest(),
        })
    }
    pub(in crate::inference::inference_agent) fn matches(&self, digest: [u8; 32]) -> bool {
        self.digest == digest
    }
    /// Sealed integration point, not a general publication callback. The caller
    /// must hold live authority gates and have committed exactly the matched key.
    pub(in crate::inference::inference_agent) fn finish(self, key: CandidateKey) {
        self.pending.commit();
        self.host.finish(key);
    }
}
