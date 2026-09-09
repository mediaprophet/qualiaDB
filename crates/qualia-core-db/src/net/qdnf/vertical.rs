//! Example 64-byte two-peer vertical slice over `local-ipc-v1`.
//!
//! This module is **not** the application API. Applications use
//! `qualia_peer::PeerHost::{pair, exchange_protected}` (public QPR). This file
//! keeps a bounded 64-byte demo of QFrame, QLink adjacency, QRoute SPF, QSession
//! policy gate and QSync payload. libp2p is not imported.

use crate::crypto::network::kem::MlKem768Secret;
use crate::crypto::network::transcript::Transcript;
use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::ipc_pair;
use crate::net::qdnf::crypto::handshake::{
    derive_handshake_keys, initiator_complete, initiator_share, responder_complete,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{copy_payload, decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::link::{Adjacency, AdjacencyState, NeighborTable};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::route::{compute_spf, ForwardingGeneration, LinkMetric};
use crate::net::qdnf::session::packet_protection::PacketProtection;
use crate::net::qdnf::types::{Generation, LinkId, ScopeEpoch};

/// Run A↔B native exchange and return payload bytes copied at B.
///
/// Demo ceiling remains 64 bytes. The authorised application path is
/// `authorised_ipc_stream_exchange` / `PeerHost::exchange_protected` (4096).
pub fn two_peer_ipc_exchange(payload: &[u8]) -> Result<usize, QdnfError> {
    if payload.len() > 64 {
        return Err(QdnfError::Capacity);
    }
    let mut ledger = ReservationLedger::new(
        ResourceBudget {
            bytes: 4096,
            work: 64,
            io: 16,
        },
        ResourceBudget {
            bytes: 4096,
            work: 64,
            io: 16,
        },
        ResourceBudget {
            bytes: 4096,
            work: 64,
            io: 16,
        },
        ResourceBudget {
            bytes: 4096,
            work: 64,
            io: 16,
        },
        ResourceBudget {
            bytes: 4096,
            work: 64,
            io: 16,
        },
    );
    let charge = ledger.reserve(
        ResourceBudget {
            bytes: (80 + payload.len()) as u64,
            work: 1,
            io: 1,
        },
        true,
    )?;

    let (sk, pk) = MlKem768Secret::generate()?;
    let i_x = [21u8; 32];
    let r_x = [22u8; 32];
    let ishare = initiator_share(&i_x, &pk)?;
    let (rshare, r_kem, r_dh) = responder_complete(&ishare, &r_x)?;
    let (i_kem, i_dh) = initiator_complete(&sk, &i_x, &rshare)?;
    let mut t = Transcript::new();
    t.append(b"suite", b"qpr-pq-1")?;
    let keys = derive_handshake_keys(&i_kem, &i_dh, &t)?;
    use crate::net::qdnf::crypto::finished::{finished_mac, verify_finished};
    let tag = finished_mac(&keys.initiator_to_responder, &keys.transcript_digest, true)?;
    verify_finished(
        &keys.initiator_to_responder,
        &keys.transcript_digest,
        true,
        &tag,
    )?;
    assert_eq!(i_kem, r_kem);
    assert_eq!(i_dh, r_dh);

    let mut neighbors = NeighborTable::new();
    let mut remote = LinkId::ZERO;
    remote.0[0] = 2;
    neighbors.insert(Adjacency {
        local: LinkId::ZERO,
        remote,
        observed_peer: crate::net::qdnf::types::ObservedLocator::from_slice(&[0x02])?,
        state: AdjacencyState::Adjacent,
        generation: 1,
        mtu: 1280,
    })?;

    let links = [LinkMetric {
        from: 0,
        to: 1,
        cost: 1,
        bidirectional: true,
    }];
    let mut table = crate::net::qdnf::route::SpfTable::EMPTY;
    compute_spf(0, &links, &mut table)?;
    let mut gen = ForwardingGeneration::empty(1);
    gen.publish(table)?;
    let _ = gen.lookup_next(1)?;

    let mut send_p = PacketProtection::from_keys(
        keys.initiator_to_responder,
        keys.responder_to_initiator,
        Generation(1),
    )?;
    let mut recv_p = PacketProtection::from_keys(
        keys.responder_to_initiator,
        keys.initiator_to_responder,
        Generation(1),
    )?;

    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let (mut a, mut b) = ipc_pair(scope, 1280)?;
    let mut pt = [0u8; 64];
    pt[..payload.len()].copy_from_slice(payload);
    let mut sealed = [0u8; 96];
    let sn = send_p.seal(b"qsession/stream", &mut pt[..payload.len()], &mut sealed)?;
    let mut header = FrameHeader::new(FrameType::SessionStream, NextProtocol::QSession);
    header.hop_limit = 16;
    header.payload_len = sn as u16;
    header.destination_link_id = remote;
    let mut wire = [0u8; 256];
    let n = encode_frame(&header, &sealed[..sn], &mut wire)?;
    a.send(&b.locator(), &wire[..n])?;
    let mut out = [0u8; 256];
    let (got, meta) = b.recv(&mut out)?;
    if meta.observed_source.as_slice() != &[0x01] {
        return Err(QdnfError::Unauthorized);
    }
    let (decoded, off, len) = decode_frame(&out[..got])?;
    if decoded.frame_type != FrameType::SessionStream {
        return Err(QdnfError::Malformed);
    }
    let mut sealed_in = [0u8; 96];
    let copied_sealed = copy_payload(&out[..got], off, len, &mut sealed_in)?;
    let mut payload_out = [0u8; 64];
    let copied = recv_p.open(
        b"qsession/stream",
        &sealed_in[..copied_sealed],
        &mut payload_out,
    )?;
    if &payload_out[..copied] != payload {
        return Err(QdnfError::Malformed);
    }
    ledger.release(charge)?;
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::authority::PolicyOutcome;
    use crate::net::qdnf::session::{SessionBinding, SessionState};
    use crate::net::qdnf::types::{OperationId, StrongDigest};

    #[test]
    fn two_peer_native_stream_without_libp2p() {
        let n = two_peer_ipc_exchange(b"qsync-hello").unwrap();
        assert_eq!(n, 11);
    }

    #[test]
    fn demo_keeps_64_byte_ceiling() {
        assert_eq!(
            two_peer_ipc_exchange(&[0u8; 65]).unwrap_err(),
            QdnfError::Capacity
        );
        assert_eq!(two_peer_ipc_exchange(&[1u8; 64]).unwrap(), 64);
    }

    #[test]
    fn deny_policy_blocks_payload() {
        let session = SessionBinding::test_fixture(
            OperationId([1u8; 16]),
            StrongDigest([2u8; 48]),
            StrongDigest([3u8; 48]),
            StrongDigest([4u8; 48]),
            PolicyOutcome::Deny,
            SessionState::Active,
        );
        assert_eq!(session.admit_application(), Err(QdnfError::Denied));
    }
}
