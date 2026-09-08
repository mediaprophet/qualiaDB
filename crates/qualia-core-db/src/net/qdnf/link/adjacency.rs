//! QLink adjacency state machine. Link possession is not controller authority.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{LinkId, ObservedLocator, StrongDigest};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdjacencyState {
    Undiscovered = 0,
    BeaconSeen = 1,
    ChallengeSent = 2,
    ChallengeReceived = 3,
    EphemeralProofVerified = 4,
    LinkKeysDerived = 5,
    Adjacent = 6,
    Rekeying = 7,
    Draining = 8,
    Closed = 9,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adjacency {
    pub local: LinkId,
    pub remote: LinkId,
    pub observed_peer: ObservedLocator,
    pub state: AdjacencyState,
    pub generation: u64,
    pub mtu: u16,
}

impl Adjacency {
    pub fn activate(&mut self) -> Result<(), QdnfError> {
        if self.state != AdjacencyState::LinkKeysDerived
            && self.state != AdjacencyState::EphemeralProofVerified
        {
            return Err(QdnfError::Unauthorized);
        }
        self.state = AdjacencyState::Adjacent;
        Ok(())
    }

    pub fn is_forwarding(&self) -> bool {
        self.state == AdjacencyState::Adjacent
    }
}

pub fn link_id_from_key(public: &[u8], bearer_scope: &[u8], epoch: u64) -> LinkId {
    let mut buf = [0u8; 80];
    let label = b"qdnf:link:v1";
    buf[..label.len()].copy_from_slice(label);
    let mut off = label.len();
    let copy = public.len().min(32);
    buf[off..off + copy].copy_from_slice(&public[..copy]);
    off += copy;
    buf[off..off + 8].copy_from_slice(&epoch.to_be_bytes());
    off += 8;
    let scope_n = bearer_scope.len().min(16);
    buf[off..off + scope_n].copy_from_slice(&bearer_scope[..scope_n]);
    let digest: StrongDigest = sha384(&buf);
    let mut id = LinkId::ZERO;
    id.0.copy_from_slice(&digest.0[..16]);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfirmed_neighbor_is_not_forwarding() {
        let adj = Adjacency {
            local: LinkId::ZERO,
            remote: LinkId::ZERO,
            observed_peer: ObservedLocator::EMPTY,
            state: AdjacencyState::ChallengeSent,
            generation: 1,
            mtu: 1280,
        };
        assert!(!adj.is_forwarding());
    }
}
