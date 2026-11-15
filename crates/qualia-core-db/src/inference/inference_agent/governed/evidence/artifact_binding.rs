use super::super::{ActorPolicyScope, PolicyError};
use super::{self as evidence_v3, BoundProposal, Payload, Proposal, VerifiedEvidencePolicy};
use etalon_inference_contract::artifact::{
    ArtifactKey, ArtifactMedia, ArtifactOutput, ArtifactSource, ParentEventBinding,
};
use etalon_inference_contract::{Decision, ExecutionMode, SessionIdentity};

/// G3 host convention only: every identity generation starts at one. This does
/// not change the published generic token/artifact metadata APIs or V2 policy.
pub(super) fn valid_identity(i: SessionIdentity) -> bool {
    i.session_id != 0
        && i.session_generation != 0
        && i.request_id != 0
        && i.request_generation != 0
        && i.model_id != [0; 32]
        && i.model_generation != 0
        && i.device_id != 0
        && i.device_generation != 0
        && i.policy_id != [0; 32]
        && i.policy_generation != 0
}

pub struct BoundArtifact<'a> {
    bound: BoundProposal<'a>,
    identity: SessionIdentity,
    mode: ExecutionMode,
    sequence: u64,
    typed_media: evidence_v3::ArtifactMedia,
    output: ArtifactOutput,
}
impl<'a> BoundArtifact<'a> {
    pub fn bind(
        proposal: Proposal<'a>,
        mode: ExecutionMode,
        parent: Option<ParentEventBinding>,
        byte_budget: u64,
    ) -> Result<Self, PolicyError> {
        use evidence_v3::ArtifactMedia as InputMedia;
        let Payload::Artifact { media, bytes } = proposal.payload else {
            return Err(PolicyError::InvalidCandidate);
        };
        if !valid_identity(proposal.identity)
            || parent.is_some_and(|p| {
                !valid_identity(p.identity) || p.sequence == 0 || p.record_digest == [0; 32]
            })
        {
            return Err(PolicyError::InvalidCandidate);
        }
        let origin = match proposal
            .artifact_origin
            .ok_or(PolicyError::InvalidCandidate)?
        {
            evidence_v3::ArtifactOrigin::RenderedCompletion => ArtifactSource::RenderedCompletion,
            evidence_v3::ArtifactOrigin::GroundedReplacement => ArtifactSource::GroundedReplacement,
            evidence_v3::ArtifactOrigin::SemanticProjection => ArtifactSource::SemanticProjection,
        };
        let bound = BoundProposal::bind(proposal, byte_budget)?;
        let typed_media = media;
        let media = match media {
            InputMedia::Utf8Text => ArtifactMedia::Utf8Text,
            InputMedia::Utf8Html => ArtifactMedia::Utf8Html,
            InputMedia::CanonicalQuins => ArtifactMedia::Binary,
        };
        let output = ArtifactOutput {
            media,
            source: origin,
            byte_len: bytes.len() as u64,
            content_digest: super::artifact_digest::content(bytes),
            provenance_digest: proposal.provenance_digest,
            observation_digest: super::artifact_digest::complete(
                bound.digest(),
                mode,
                origin,
                parent,
            ),
            parent,
        };
        Ok(Self {
            bound,
            identity: proposal.identity,
            mode,
            sequence: proposal.sequence,
            typed_media,
            output,
        })
    }
    /// Prepared output for the real artifact session. This owns no counter.
    pub fn output(&self) -> ArtifactOutput {
        self.output
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bound.bytes()
    }
    /// Retain CanonicalQuins even though the generic metadata media is Binary.
    pub fn media(&self) -> evidence_v3::ArtifactMedia {
        self.typed_media
    }
    /// Require the real session-returned key; no second event sequence is created.
    /// Matching metadata is not proof of admission or a prior parent acceptance.
    /// A live caller must terminally discard a pending session on mismatch; this
    /// pure comparison does not silently cancel or authorize another decision.
    pub fn match_session_key(&self, key: &ArtifactKey) -> Result<(), PolicyError> {
        if key.identity != self.identity
            || key.mode != self.mode
            || key.event_sequence != self.sequence
            || key.output != self.output
        {
            return Err(PolicyError::InvalidCandidate);
        }
        Ok(())
    }
    /// Actual signed-origin V3 VM/graph evaluation only. Current authority, accepted queue
    /// admission and backing validation still belong to the live host boundary.
    pub fn evaluate(
        &self,
        policy: &VerifiedEvidencePolicy<'_>,
        actor: ActorPolicyScope,
        now: u64,
    ) -> Result<Decision, PolicyError> {
        evidence_v3::evaluate_constraints(policy, &self.bound, actor, now)
    }
}
