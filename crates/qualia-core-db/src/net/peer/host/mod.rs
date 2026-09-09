//! Native peer host. Replaces libp2p `Swarm` + `NetworkBehaviour`.
//!
//! Discovery is QLink beacons (not mDNS). Lookup is QSR (not Kademlia).
//! Streams are QSession (not Yamux). Session keys are `qpr-pq-1` (not Noise).
//! Application send/receive requires a verified permit and packet protection.

pub mod builder;
pub mod driver;
pub mod exchange;
pub mod identity;
mod path_cc;
pub mod services;
pub mod session_table;

use crate::net::peer::cells::CellSlot;
use crate::net::peer::runtime::{CancelEpoch, ReservationLedger};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::IpcEndpoint;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{copy_payload, decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::link::{Adjacency, AdjacencyState, Beacon, DiscoveryMode, NeighborTable};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::resolve::qsr::{
    lookup_exact, lookup_qsr_full as qsr_lookup_full, CoverInterval, HandoverStage, QsrOutcome,
    QsrSnapshot, TokenStage,
};
use crate::net::qdnf::types::{Generation, LinkId, ObservedLocator, StrongDigest};

pub use builder::{PeerBuilder, CELL_BYTES_DEFAULT};
pub use driver::SealedFrame;
pub use exchange::authorised_ipc_stream_exchange;
pub use identity::ControllerIdentity;
pub use services::ServiceId;
pub use session_table::{SessionHandle, SessionTable, MAX_SESSIONS};

pub struct NativePeer {
    pub(crate) identity: ControllerIdentity,
    pub(crate) local_link: LinkId,
    pub(crate) neighbors: NeighborTable,
    pub(crate) ledger: ReservationLedger,
    pub(crate) sessions: session_table::SessionTable,
    pub(crate) primary: Option<SessionHandle>,
    pub(crate) bearer: IpcEndpoint,
    pub(crate) cancel_epoch: CancelEpoch,
    pub(crate) cancelled: bool,
    pub(crate) cell_slot: Option<CellSlot>,
}

impl NativePeer {
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

    /// Cover-interval membership only. Not authenticated existence.
    /// Use [`Self::lookup_qsr_full`] for tokens and handover.
    pub fn lookup_qsr(
        key: &StrongDigest,
        covers: &[CoverInterval],
    ) -> Result<QsrOutcome, QdnfError> {
        lookup_exact(key, covers)
    }

    /// Production QSR lookup: exact cover/snapshot, then tokens, then handover.
    pub fn lookup_qsr_full(
        key: &StrongDigest,
        covers: &[CoverInterval],
        snapshot: &QsrSnapshot,
        required_generation: Generation,
        token: Option<TokenStage<'_>>,
        handover: Option<HandoverStage<'_>>,
        out: &mut [StrongDigest],
    ) -> Result<QsrOutcome, QdnfError> {
        qsr_lookup_full(
            key,
            covers,
            snapshot,
            required_generation,
            token,
            handover,
            out,
        )
    }

    /// Service identifier alone cannot open an application session (E01/R01).
    pub fn open_session(&mut self, _service: ServiceId) -> Result<(), QdnfError> {
        Err(QdnfError::Unauthorized)
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
    use crate::net::peer::cells::MAX_ORDINARY_CELL;
    use crate::net::qdnf::authority::{
        binding_for_controllers, AuthorityOwner, ContactState, InstalledSessionKeys,
    };
    use crate::net::qdnf::harness::oracles::wire::{plaintext_on_wire, ProtectedView};
    use crate::net::qdnf::types::{Generation, ScopeEpoch};

    fn activate(
        peer: &mut NativePeer,
        local: &[u8],
        remote: &[u8],
        tag: u8,
        now: u64,
    ) -> SessionHandle {
        let mut owner = AuthorityOwner::new();
        let binding = binding_for_controllers(local, remote, b"q42:QSync/1", &[b'o', tag]).unwrap();
        let (_cred, _contact, handle) = owner
            .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, now, 4096).unwrap();
        let mut send = [tag.saturating_add(1); 32];
        let mut recv = [tag.saturating_add(80); 32];
        send[31] = 1;
        recv[31] = 2;
        let keys = InstalledSessionKeys::new(Generation(1), send, recv, true).unwrap();
        peer.activate_protected(permit, keys, now).unwrap()
    }

    #[test]
    fn native_peer_replaces_swarm_without_libp2p() {
        let n = native_ipc_stream_exchange(b"hello-qpr").unwrap();
        assert_eq!(n, 9);
    }

    #[test]
    fn qsr_lookup_is_not_kademlia() {
        let key = StrongDigest([7u8; 48]);
        let covers = [CoverInterval { start: 0, end: 15 }];
        assert_eq!(
            NativePeer::lookup_qsr(&key, &covers).unwrap(),
            QsrOutcome::NeedContinuation
        );
        let snap = QsrSnapshot::empty(Generation(1));
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            NativePeer::lookup_qsr_full(
                &key,
                &covers,
                &snap,
                Generation(1),
                None,
                None,
                &mut out,
            )
            .unwrap(),
            QsrOutcome::EmptyInSnapshot
        );
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

    #[test]
    fn rekey_without_protection_is_unauthorized() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let mut table =
            crate::net::qdnf::session::rekey::RekeyTable::from_keys([1u8; 32], [2u8; 32]).unwrap();
        assert_eq!(a.rekey_protected(&mut table), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn builder_rejects_zero_and_oversize_cell_bytes() {
        let link = LinkId::ZERO;
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", link, 0).unwrap_err(),
            QdnfError::Capacity
        );
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", link, MAX_ORDINARY_CELL + 1).unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn two_sessions_live_closing_one_keeps_the_other() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        let h1 = activate(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        let h2 = activate(&mut a, b"did:q42:a", b"did:q42:b", 2, now);
        assert_eq!(a.live_session_count(), 2);
        a.close_session(h1).unwrap();
        assert_eq!(a.live_session_count(), 1);
        assert_eq!(a.sessions.get_mut(h1).err(), Some(QdnfError::Closed));
        assert!(a.sessions.get_mut(h2).is_ok());
        assert_eq!(a.primary_session(), Some(h2));
    }

    #[test]
    fn fifth_session_is_capacity() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        for tag in 1u8..=4 {
            let _ = activate(&mut a, b"did:q42:a", b"did:q42:b", tag, now);
        }
        assert_eq!(a.live_session_count(), 4);
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-5").unwrap();
        let (_c, _k, handle) = owner
            .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, now, 4096).unwrap();
        let keys = InstalledSessionKeys::new(Generation(1), [9u8; 32], [10u8; 32], true).unwrap();
        assert_eq!(
            a.activate_protected(permit, keys, now).unwrap_err(),
            QdnfError::Capacity
        );
        assert_eq!(a.live_session_count(), 4);
    }

    #[test]
    fn close_then_open_reconciles_reservation() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        let before = a.remaining_host_bytes();
        let h = activate(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        let used = a.remaining_host_bytes();
        assert!(used < before);
        a.close_session(h).unwrap();
        let after_close = a.remaining_host_bytes();
        assert!(after_close > used);
        assert_eq!(after_close, before);
        let _ = activate(&mut a, b"did:q42:a", b"did:q42:b", 2, now);
        assert_eq!(a.remaining_host_bytes(), used);
    }

    #[test]
    fn poll_recv_before_deadline_is_would_block() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        a.announce(&b.locator(), now).unwrap();
        let _ = b.accept_announce().unwrap();
        b.announce(&a.locator(), now).unwrap();
        let _ = a.accept_announce().unwrap();
        let _ = activate(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        let mut out = [0u8; 64];
        assert_eq!(
            a.poll_recv(now, now.saturating_add(60), &mut out),
            Err(QdnfError::WouldBlock)
        );
        assert_eq!(
            a.poll_recv(now.saturating_add(60), now.saturating_add(60), &mut out),
            Err(QdnfError::Expired)
        );
    }
}
