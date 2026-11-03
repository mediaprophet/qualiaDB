use crate::{inference::inference_agent::governed::PolicyError, NQuin};
use etalon_inference_contract::SessionIdentity;
use serde::{Deserialize, Serialize};

/// Origin labels only: Speculative/Topology do NOT attest a successful verifier.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    ModelToken = 0,
    SpeculativeToken = 1,
    TopologyToken = 2,
    GraphForcedToken = 3,
    TextReplacement = 4,
    HtmlReplacement = 5,
    SemanticArtifact = 6,
}
impl CandidateSource {
    pub const fn bit(self) -> u16 {
        1 << self as u8
    }
    pub(super) fn token(self) -> bool {
        (self as u8) < 4
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactMedia {
    Utf8Text = 1,
    Utf8Html = 2,
    CanonicalQuins = 3,
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactOrigin {
    RenderedCompletion = 1,
    GroundedReplacement = 2,
    SemanticProjection = 3,
}
impl ArtifactOrigin {
    pub const fn bit(self) -> u8 {
        1 << (self as u8 - 1)
    }
    fn accepts(self, media: ArtifactMedia) -> bool {
        matches!(
            (self, media),
            (
                Self::RenderedCompletion | Self::GroundedReplacement,
                ArtifactMedia::Utf8Text | ArtifactMedia::Utf8Html
            ) | (Self::SemanticProjection, ArtifactMedia::CanonicalQuins)
        )
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreMeaning {
    Logit = 1,
    Probability = 2,
}
#[derive(Clone, Copy, Debug)]
pub struct NumericalObservation {
    pub meaning: ScoreMeaning,
    pub value: f32,
}
impl NumericalObservation {
    pub(super) fn validate(self) -> Result<(), PolicyError> {
        if !self.value.is_finite()
            || (self.meaning == ScoreMeaning::Probability && !(0.0..=1.0).contains(&self.value))
        {
            Err(PolicyError::InvalidCandidate)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy)]
pub enum Payload<'a> {
    /// Raw tokenizer bytes may be empty (special/EOS token) or partial UTF-8.
    Token {
        token: u32,
        position: u64,
        piece: &'a [u8],
    },
    /// Empty text/HTML requires an explicitly signed allow-empty requirement.
    Artifact {
        media: ArtifactMedia,
        bytes: &'a [u8],
    },
}
#[derive(Clone, Copy)]
pub struct Proposal<'a> {
    pub identity: SessionIdentity,
    pub request_digest: [u8; 32],
    pub sequence: u64,
    pub source: CandidateSource,
    pub artifact_origin: Option<ArtifactOrigin>,
    pub payload: Payload<'a>,
    pub numerical: Option<NumericalObservation>,
    pub observation: NQuin,
    /// Digest of actual provenance supplied by the adapter, not verified by label.
    pub provenance_digest: [u8; 32],
}
/// Validated immutable Rust borrows, NOT proof that external mmap backing is frozen.
/// The runtime adapter still owns trusted origin and backing immutability.
pub struct BoundProposal<'a> {
    pub(super) proposal: Proposal<'a>,
    pub(super) payload_digest: [u8; 32],
}
impl<'a> BoundProposal<'a> {
    pub fn bind(proposal: Proposal<'a>, payload_budget: u64) -> Result<Self, PolicyError> {
        let i = proposal.identity;
        if proposal.provenance_digest == [0; 32]
            || proposal.sequence == 0
            || proposal.request_digest == [0; 32]
            || i.session_id == 0
            || i.session_generation == 0
            || i.request_id == 0
            || i.request_generation == 0
            || i.model_id == [0; 32]
            || i.model_generation == 0
            || i.device_id == 0
            || i.device_generation == 0
            || i.policy_id == [0; 32]
            || i.policy_generation == 0
        {
            return Err(PolicyError::InvalidCandidate);
        }
        let q = proposal.observation;
        if q.parity != q.subject ^ q.predicate ^ q.object ^ q.context {
            return Err(PolicyError::InvalidCandidate);
        }
        if let Some(score) = proposal.numerical {
            score.validate()?;
        }
        let bytes = match proposal.payload {
            Payload::Token { piece, .. }
                if proposal.source.token() && proposal.artifact_origin.is_none() =>
            {
                piece
            }
            Payload::Artifact { media, bytes } if !proposal.source.token() => {
                let matching = matches!(
                    (proposal.source, media),
                    (CandidateSource::TextReplacement, ArtifactMedia::Utf8Text)
                        | (CandidateSource::HtmlReplacement, ArtifactMedia::Utf8Html)
                        | (
                            CandidateSource::SemanticArtifact,
                            ArtifactMedia::CanonicalQuins
                        )
                );
                if !matching
                    || proposal.numerical.is_some()
                    || !proposal
                        .artifact_origin
                        .is_some_and(|origin| origin.accepts(media))
                {
                    return Err(PolicyError::InvalidCandidate);
                }
                bytes
            }
            _ => return Err(PolicyError::InvalidCandidate),
        };
        if u64::try_from(bytes.len()).map_err(|_| PolicyError::Capacity)? > payload_budget {
            return Err(PolicyError::Budget);
        }
        if let Payload::Artifact { media, .. } = proposal.payload {
            match media {
                ArtifactMedia::Utf8Text | ArtifactMedia::Utf8Html => {
                    std::str::from_utf8(bytes).map_err(|_| PolicyError::Encoding)?;
                }
                ArtifactMedia::CanonicalQuins => {
                    if bytes.is_empty() || bytes.len() % 48 != 0 {
                        return Err(PolicyError::Encoding);
                    }
                    for record in bytes.chunks_exact(48) {
                        let word = |n| u64::from_le_bytes(record[n..n + 8].try_into().unwrap());
                        if word(40) != word(0) ^ word(8) ^ word(16) ^ word(24) {
                            return Err(PolicyError::InvalidCandidate);
                        }
                    }
                }
            }
        }
        Ok(Self {
            payload_digest: super::encoding::payload_digest(bytes),
            proposal,
        })
    }
    pub fn bytes(&self) -> &'a [u8] {
        match self.proposal.payload {
            Payload::Token { piece, .. } => piece,
            Payload::Artifact { bytes, .. } => bytes,
        }
    }
    pub fn digest(&self) -> [u8; 32] {
        super::encoding::candidate_digest(self)
    }
    pub(super) fn recheck_payload(&self) -> Result<(), PolicyError> {
        if super::encoding::payload_digest(self.bytes()) == self.payload_digest {
            Ok(())
        } else {
            Err(PolicyError::InvalidCandidate)
        }
    }
}
