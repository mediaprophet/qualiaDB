//! One privately locked caller buffer, shared by the actual contract verifiers.
//! Control evidence is a digest of this retained material, never cached Allow.
use super::super::PolicyError;
use super::{
    ArtifactMedia, ArtifactOrigin, BoundArtifact, BoundProposal, CandidateSource,
    NumericalObservation, Payload, Proposal,
};
use crate::NQuin;
use etalon_inference_contract::artifact::{ArtifactKey, ParentEventBinding};
use etalon_inference_contract::{CandidateKey, ExecutionMode, SessionIdentity};
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug)]
pub struct EvidenceLimits {
    /// Charges the full supplied byte-slice capacity, not only the used prefix.
    pub storage_bytes: usize,
    pub max_payload_bytes: usize,
    pub max_artifact_events: u64,
}
pub struct EvidenceStorage<'bytes> {
    inner: Mutex<Retained<'bytes>>,
}
impl<'bytes> EvidenceStorage<'bytes> {
    pub fn new(bytes: &'bytes mut [u8], limits: EvidenceLimits) -> Result<Self, PolicyError> {
        let charged = bytes
            .len()
            .checked_add(std::mem::size_of::<Self>())
            .ok_or(PolicyError::Budget)?;
        if charged > limits.storage_bytes
            || limits.max_payload_bytes > bytes.len()
            || limits.max_payload_bytes == 0
            || limits.max_artifact_events == 0
        {
            return Err(PolicyError::Budget);
        }
        Ok(Self {
            inner: Mutex::new(Retained {
                bytes,
                limits,
                used: 0,
                header: None,
                parent: None,
                accepted: None,
            }),
        })
    }
    pub(super) fn lock(&self) -> Result<MutexGuard<'_, Retained<'bytes>>, PolicyError> {
        self.inner.lock().map_err(|_| PolicyError::Revoked)
    }
}
#[derive(Clone, Copy)]
enum PayloadHeader {
    Token { token: u32, position: u64 },
    Artifact { media: ArtifactMedia },
}
#[derive(Clone, Copy)]
struct Header {
    identity: SessionIdentity,
    request_digest: [u8; 32],
    sequence: u64,
    source: CandidateSource,
    artifact_origin: Option<ArtifactOrigin>,
    payload: PayloadHeader,
    numerical: Option<NumericalObservation>,
    observation: NQuin,
    provenance_digest: [u8; 32],
}
impl Header {
    fn from_proposal(p: Proposal<'_>) -> Self {
        Self {
            identity: p.identity,
            request_digest: p.request_digest,
            sequence: p.sequence,
            source: p.source,
            artifact_origin: p.artifact_origin,
            payload: match p.payload {
                Payload::Token {
                    token, position, ..
                } => PayloadHeader::Token { token, position },
                Payload::Artifact { media, .. } => PayloadHeader::Artifact { media },
            },
            numerical: p.numerical,
            observation: p.observation,
            provenance_digest: p.provenance_digest,
        }
    }
    fn proposal(self, bytes: &[u8]) -> Proposal<'_> {
        Proposal {
            identity: self.identity,
            request_digest: self.request_digest,
            sequence: self.sequence,
            source: self.source,
            artifact_origin: self.artifact_origin,
            payload: match self.payload {
                PayloadHeader::Token { token, position } => Payload::Token {
                    token,
                    position,
                    piece: bytes,
                },
                PayloadHeader::Artifact { media } => Payload::Artifact { media, bytes },
            },
            numerical: self.numerical,
            observation: self.observation,
            provenance_digest: self.provenance_digest,
        }
    }
}
/// Keys are inspectable metadata. Copying a key does not duplicate its accepted
/// byte record or create a live session/parent-acceptance proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptedKey {
    Token(CandidateKey),
    Artifact(ArtifactKey),
}
pub(super) struct Retained<'bytes> {
    bytes: &'bytes mut [u8],
    pub(super) limits: EvidenceLimits,
    used: usize,
    header: Option<Header>,
    parent: Option<ParentEventBinding>,
    pub(super) accepted: Option<AcceptedKey>,
}
impl Retained<'_> {
    pub(super) fn stage(
        &mut self,
        p: Proposal<'_>,
        parent: Option<ParentEventBinding>,
    ) -> Result<(), PolicyError> {
        if self.header.is_some() || self.accepted.is_some() {
            return Err(PolicyError::Capacity);
        }
        let bound = BoundProposal::bind(p, self.limits.max_payload_bytes as u64)?;
        if matches!(p.payload, Payload::Token { .. }) && parent.is_some() {
            return Err(PolicyError::InvalidCandidate);
        }
        let bytes = bound.bytes();
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.used = bytes.len();
        self.header = Some(Header::from_proposal(p));
        self.parent = parent;
        Ok(())
    }
    pub(super) fn proposal(&self) -> Result<Proposal<'_>, PolicyError> {
        Ok(self
            .header
            .ok_or(PolicyError::InvalidCandidate)?
            .proposal(&self.bytes[..self.used]))
    }
    pub(super) fn bound(&self) -> Result<BoundProposal<'_>, PolicyError> {
        BoundProposal::bind(self.proposal()?, self.limits.max_payload_bytes as u64)
    }
    pub(super) fn artifact(&self) -> Result<BoundArtifact<'_>, PolicyError> {
        BoundArtifact::bind(
            self.proposal()?,
            ExecutionMode::GovernedApplication,
            self.parent,
            self.limits.max_payload_bytes as u64,
        )
    }
    pub(super) fn control_digest(&self) -> Result<[u8; 32], PolicyError> {
        match self.proposal()?.payload {
            Payload::Token { .. } => Ok(self.bound()?.digest()),
            Payload::Artifact { .. } => Ok(self.artifact()?.output().observation_digest),
        }
    }
    pub(super) fn clear(&mut self) {
        self.header = None;
        self.accepted = None;
        self.parent = None;
        self.used = 0;
    }
    pub(super) fn is_empty(&self) -> bool {
        self.header.is_none() && self.accepted.is_none()
    }
}
/// Exclusive view of one committed record. Authority locks have been released.
/// A later revocation stops future acceptance; it does not retract this prefix.
/// The embedding callback acknowledges complete delivery explicitly. No API can
/// prove external delivery or retract bytes already emitted before an error.
pub struct AcceptedEvidence<'drain, 'bytes> {
    retained: MutexGuard<'drain, Retained<'bytes>>,
    terminal: &'drain mut bool,
    complete: bool,
}
impl<'drain, 'bytes> AcceptedEvidence<'drain, 'bytes> {
    pub(super) fn new(
        retained: MutexGuard<'drain, Retained<'bytes>>,
        terminal: &'drain mut bool,
    ) -> Result<Self, PolicyError> {
        if retained.accepted.is_none() {
            return Err(PolicyError::Session);
        }
        // Default failure disposition applies even if an external callback unwinds.
        *terminal = true;
        Ok(Self {
            retained,
            terminal,
            complete: false,
        })
    }
    pub fn key(&self) -> AcceptedKey {
        self.retained.accepted.expect("accepted record held")
    }
    pub fn bytes(&self) -> &[u8] {
        &self.retained.bytes[..self.retained.used]
    }
    pub fn artifact_media(&self) -> Option<ArtifactMedia> {
        match self.retained.header.map(|h| h.payload) {
            Some(PayloadHeader::Artifact { media }) => Some(media),
            _ => None,
        }
    }
    pub fn complete(mut self, delivered_bytes: usize) -> Result<(), PolicyError> {
        if delivered_bytes != self.retained.used {
            return Err(PolicyError::InvalidCandidate);
        }
        self.retained.clear();
        *self.terminal = false;
        self.complete = true;
        Ok(())
    }
}
impl Drop for AcceptedEvidence<'_, '_> {
    fn drop(&mut self) {
        if !self.complete {
            self.retained.clear();
            *self.terminal = true;
        }
    }
}
