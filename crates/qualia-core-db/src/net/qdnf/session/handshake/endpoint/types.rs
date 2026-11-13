use super::E;
use crate::crypto::network::{authority::PinHandle, cose};
use crate::net::qdnf::{
    bearer::{
        contract::Bearer,
        fragment::{HandshakeReassembler, MAX_FRAGMENTS, MAX_FRAME, MAX_HANDSHAKE_BYTES},
    },
    registries::BearerProfile,
    types::{ObservedLocator, ScopeEpoch, StrongDigest},
};
pub const PROFILE: &[u8] = b"qpr-pq-1/endpoint-v1";
pub const INITIATOR_PURPOSE: &[u8] = b"qpr-pq-1/endpoint-v1/initiator";
pub const RESPONDER_PURPOSE: &[u8] = b"qpr-pq-1/endpoint-v1/responder";
pub(super) const MAGIC: &[u8] = b"QSESSION-ENDPOINT-V1\0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Initiator,
    Responder,
}
impl Role {
    pub(super) fn initiator(self) -> bool {
        self == Self::Initiator
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    ClientProof,
    ClientSending,
    AwaitingClient,
    ServerProof,
    ServerSending,
    AwaitingServer,
    InitiatorFinishedSending,
    AwaitingFinished,
    Ready,
    Consumed,
    Failed,
}
#[derive(Clone, Copy)]
pub struct PeerTrust {
    pub pin: PinHandle,
}
/// Route to a host-owned nonblocking Bearer. Endpoint steps bound frame count;
/// the adapter itself must return promptly and must not reenter authority locks.
#[derive(Clone, Copy)]
pub struct Route {
    pub peer: ObservedLocator,
    pub scope: ScopeEpoch,
    pub profile: BearerProfile,
    pub mtu: u16,
}
impl Route {
    pub fn observed<B: Bearer>(bearer: &B, peer: ObservedLocator) -> Result<Self, E> {
        if peer.len == 0 || peer.len > 32 || bearer.mtu() < 1280 {
            return Err(E::Range);
        }
        Ok(Self {
            peer,
            scope: bearer.scope(),
            profile: bearer.profile(),
            mtu: bearer.mtu().min(1500),
        })
    }
    pub(super) fn check<B: Bearer>(&self, b: &B) -> Result<(), E> {
        if b.scope() != self.scope || b.profile() != self.profile || b.mtu().min(1500) != self.mtu {
            return Err(E::StaleGeneration);
        }
        Ok(())
    }
}
/// Explicit caller-owned bounded scratch. Host reservation must cover
/// workspace_bytes() plus endpoint object/provider internals before construction.
pub struct EndpointWorkspace {
    pub(super) io: [u8; MAX_HANDSHAKE_BYTES],
    pub(super) frames: [[u8; MAX_FRAME]; MAX_FRAGMENTS],
    pub(super) lens: [usize; MAX_FRAGMENTS],
    pub(super) wire: [u8; 1500],
    pub(super) reassembly: HandshakeReassembler,
    pub(super) cose: cose::Workspace,
}
impl EndpointWorkspace {
    pub fn new() -> Self {
        Self {
            io: [0; MAX_HANDSHAKE_BYTES],
            frames: [[0; MAX_FRAME]; MAX_FRAGMENTS],
            lens: [0; MAX_FRAGMENTS],
            wire: [0; 1500],
            reassembly: HandshakeReassembler::new(),
            cose: cose::Workspace::new(),
        }
    }
    pub(super) fn clear(&mut self) {
        self.io.fill(0);
        self.frames.iter_mut().for_each(|f| f.fill(0));
        self.lens.fill(0);
        self.wire.fill(0);
        self.reassembly.lose_interface();
        self.cose = cose::Workspace::new();
    }
}
impl Default for EndpointWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
pub const fn workspace_bytes() -> usize {
    core::mem::size_of::<EndpointWorkspace>()
}
/// Public pending commitment, never signing authority. Host admits an exact
/// managed Binding referencing this copied digest through its central library.
#[derive(Clone, Copy)]
pub struct SigningCommitment {
    pub digest: StrongDigest,
    pub key_ids: cose::KeyIds,
    pub epoch: u64,
    pub audience: StrongDigest,
    pub role: Role,
}
impl SigningCommitment {
    pub fn binding(&self) -> cose::Binding<'_> {
        cose::Binding {
            descriptor_digest: &self.digest.0,
            key_ids: self.key_ids,
            controller_epoch: self.epoch,
            purpose: if self.role.initiator() {
                INITIATOR_PURPOSE
            } else {
                RESPONDER_PURPOSE
            },
            audience: &self.audience.0,
        }
    }
}
pub(super) fn fresh_nonce() -> Result<[u8; 32], E> {
    let mut n = [0; 32];
    crate::crypto::network::entropy::fill_os(&mut n)?;
    if n == [0; 32] {
        return Err(E::EntropyFailure);
    }
    Ok(n)
}
