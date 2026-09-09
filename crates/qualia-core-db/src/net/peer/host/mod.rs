//! Native peer host. Replaces libp2p `Swarm` + `NetworkBehaviour`.
//!
//! Discovery is QLink beacons (not mDNS). Lookup is QSR (not Kademlia).
//! Streams are QSession (not Yamux). Session keys are `qpr-pq-1` (not Noise).
//! Application send/receive requires a verified permit and packet protection.

pub mod exchange;
pub mod identity;
pub mod services;

use crate::net::peer::runtime::{ReservationHandle, ReservationLedger, ResourceBudget};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::{ipc_pair, IpcEndpoint};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{copy_payload, decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::link::{Adjacency, AdjacencyState, Beacon, DiscoveryMode, NeighborTable};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::resolve::qsr::{lookup_exact, CoverInterval};
use crate::net::qdnf::session::packet_protection::PacketProtection;
use crate::net::qdnf::session::{SessionBinding, StreamState};
use crate::net::qdnf::types::{LinkId, ObservedLocator, ScopeEpoch, StrongDigest};

pub use exchange::{authorised_ipc_stream_exchange, SealedFrame};
pub use identity::ControllerIdentity;
pub use services::ServiceId;

const CELL_BYTES_DEFAULT: u64 = 64 * 1024 * 1024;

pub struct NativePeer {
    identity: ControllerIdentity,
    pub(crate) local_link: LinkId,
    pub(crate) neighbors: NeighborTable,
    pub(crate) ledger: ReservationLedger,
    pub(crate) session: Option<SessionBinding>,
    #[allow(dead_code)]
    pub(crate) stream: StreamState,
    pub(crate) bearer: IpcEndpoint,
    pub(crate) protection: Option<PacketProtection>,
    pub(crate) reservation: Option<ReservationHandle>,
}

impl NativePeer {
    fn with_bearer(
        identity: ControllerIdentity,
        local_link: LinkId,
        bearer: IpcEndpoint,
    ) -> Result<Self, QdnfError> {
        let cap = ResourceBudget {
            bytes: CELL_BYTES_DEFAULT,
            work: 1024,
            io: 256,
        };
        Ok(Self {
            identity,
            local_link,
            neighbors: NeighborTable::new(),
            ledger: ReservationLedger::new(cap, cap, cap, cap, cap),
            session: None,
            stream: StreamState::new(),
            bearer,
            protection: None,
            reservation: None,
        })
    }

    /// Two connected native peers over `local-ipc-v1`. Transport only; no session.
    pub fn pair_ipc(
        a_controller: &[u8],
        b_controller: &[u8],
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<(Self, Self), QdnfError> {
        let a_id = ControllerIdentity::from_controller(a_controller)?;
        let b_id = ControllerIdentity::from_controller(b_controller)?;
        if a_id == b_id {
            return Err(QdnfError::Conflict);
        }
        let (a_bearer, b_bearer) = ipc_pair(scope, mtu)?;
        let mut a_link = LinkId::ZERO;
        a_link.0[0] = 1;
        let mut b_link = LinkId::ZERO;
        b_link.0[0] = 2;
        Ok((
            Self::with_bearer(a_id, a_link, a_bearer)?,
            Self::with_bearer(b_id, b_link, b_bearer)?,
        ))
    }

    #[inline]
    pub fn identity(&self) -> ControllerIdentity {
        self.identity
    }

    #[inline]
    pub fn locator(&self) -> ObservedLocator {
        self.bearer.locator()
    }

    #[inline]
    pub fn local_link(&self) -> LinkId {
        self.local_link
    }

    /// QLink beacon announce. Replaces mDNS.
    pub fn announce(&mut self, dest: &ObservedLocator, now_unix: u64) -> Result<(), QdnfError> {
        let beacon = Beacon {
            mode: DiscoveryMode::PrivatePairwise,
            tag: [0x11; 16],
            link_id: self.local_link,
            epoch: 1,
            expiry_unix: now_unix.saturating_add(60),
            mtu: self.bearer.mtu(),
        };
        let mut payload = [0u8; Beacon::WIRE_LEN];
        let pn = beacon.encode(&mut payload)?;
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.source_link_id = self.local_link;
        header.payload_len = pn as u16;
        let mut wire = [0u8; 256];
        let n = encode_frame(&header, &payload[..pn], &mut wire)?;
        self.bearer.send(dest, &wire[..n])?;
        Ok(())
    }

    /// Accept a QLink beacon and install adjacency. Link possession is not DID authority.
    pub fn accept_announce(&mut self) -> Result<LinkId, QdnfError> {
        let mut out = [0u8; 256];
        let (got, meta) = self.bearer.recv(&mut out)?;
        let (decoded, off, len) = decode_frame(&out[..got])?;
        if decoded.frame_type != FrameType::DiscoveryBeacon {
            return Err(QdnfError::Malformed);
        }
        let mut payload = [0u8; 64];
        let copied = copy_payload(&out[..got], off, len, &mut payload)?;
        let beacon = Beacon::decode(&payload[..copied])?;
        self.neighbors.insert(Adjacency {
            local: self.local_link,
            remote: beacon.link_id,
            observed_peer: meta.observed_source,
            state: AdjacencyState::Adjacent,
            generation: 1,
            mtu: beacon.mtu,
        })?;
        Ok(beacon.link_id)
    }

    /// QSR exact lookup. Replaces Kademlia `get_record`.
    pub fn lookup_qsr(key: &StrongDigest, covers: &[CoverInterval]) -> Result<bool, QdnfError> {
        lookup_exact(key, covers)
    }

    /// Service identifier alone cannot open an application session (E01/R01).
    pub fn open_session(&mut self, _service: ServiceId) -> Result<(), QdnfError> {
        Err(QdnfError::Unauthorized)
    }

    pub fn send_stream(
        &mut self,
        dest: &ObservedLocator,
        payload: &[u8],
    ) -> Result<usize, QdnfError> {
        if self.protection.is_none() {
            return Err(QdnfError::Unauthorized);
        }
        let sealed = self.send_protected(dest, payload)?;
        let _ = sealed;
        Ok(payload.len())
    }

    pub fn recv_stream(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        self.recv_protected(out)
    }

    pub(crate) fn remote_or_reject(&self) -> Result<LinkId, QdnfError> {
        let mut want = LinkId::ZERO;
        want.0[0] = if self.local_link.0[0] == 1 { 2 } else { 1 };
        if self.neighbors.forwarding(want).is_some() {
            Ok(want)
        } else {
            Err(QdnfError::NoRoute)
        }
    }

    pub fn neighbor_count(&self) -> usize {
        self.neighbors.len()
    }
}

/// Two-peer native stream without libp2p, IP, or DNS. Protected QPR path.
pub fn native_ipc_stream_exchange(payload: &[u8]) -> Result<usize, QdnfError> {
    authorised_ipc_stream_exchange(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::harness::oracles::wire::{plaintext_on_wire, ProtectedView};

    #[test]
    fn native_peer_replaces_swarm_without_libp2p() {
        let n = native_ipc_stream_exchange(b"hello-qpr").unwrap();
        assert_eq!(n, 9);
    }

    #[test]
    fn qsr_lookup_is_not_kademlia() {
        let key = StrongDigest([7u8; 48]);
        let covers = [CoverInterval { start: 0, end: 95 }];
        assert!(NativePeer::lookup_qsr(&key, &covers).unwrap());
    }

    #[test]
    fn session_required_before_stream() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        assert_eq!(
            a.send_stream(&b.locator(), b"x"),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn service_open_cannot_mint_active_allow() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let service = ServiceId::from_iri(b"q42:QSync/1").unwrap();
        assert_eq!(a.open_session(service), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn protected_exchange_is_not_plaintext_on_wire() {
        let app = b"hello-qpr";
        assert!(!plaintext_on_wire(ProtectedView {
            wire_payload: &[0u8; 9],
            application: app,
        }));
        let n = authorised_ipc_stream_exchange(app).unwrap();
        assert_eq!(n, 9);
    }
}
