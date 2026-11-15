use super::CpuAdmissionError;
use crate::inference::inference_agent::governed::evidence::{
    CandidateSource, NumericalObservation, Payload, Proposal, ScoreMeaning,
};
use crate::{frame_layout::INLINE_TAG_INTEGER, NQuin};
use etalon_inference_contract::SessionIdentity;

pub const TOKEN_EVENT: u64 = crate::q_hash("qualia:inference:proposed-vocabulary-token:v1");

/// Only the actual runner supplies this context from its admitted session.
pub(super) struct EventContext {
    pub identity: SessionIdentity,
    pub request: [u8; 32],
    pub actor: u64,
    pub graph: u64,
    pub sequence: u64,
    pub position: u64,
    pub provenance: [u8; 32],
}
pub(super) fn token_proposal<'a>(
    context: EventContext,
    token: u32,
    raw_logit: f32,
    piece: &'a [u8],
) -> Result<Proposal<'a>, CpuAdmissionError> {
    if !raw_logit.is_finite() || context.actor == 0 || context.graph == 0 {
        return Err(CpuAdmissionError::InvalidIdentity);
    }
    let mut observation = NQuin {
        subject: context.actor,
        predicate: TOKEN_EVENT,
        object: INLINE_TAG_INTEGER | u64::from(token),
        context: context.graph,
        // No confidence asserted. Admission refuses all metadata/float-dependent
        // base instructions; metadata is not a representation of absent score.
        metadata: 0,
        parity: 0,
    };
    observation.parity =
        observation.subject ^ observation.predicate ^ observation.object ^ observation.context;
    Ok(Proposal {
        identity: context.identity,
        request_digest: context.request,
        sequence: context.sequence,
        source: CandidateSource::ModelToken,
        artifact_origin: None,
        payload: Payload::Token {
            token,
            position: context.position,
            piece,
        },
        numerical: Some(NumericalObservation {
            meaning: ScoreMeaning::Logit,
            value: raw_logit,
        }),
        observation,
        provenance_digest: context.provenance,
    })
}
