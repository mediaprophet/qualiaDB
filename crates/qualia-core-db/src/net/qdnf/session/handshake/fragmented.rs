//! Production QSession hello exchange over admit-before-buffer fragments.
//!
//! ClientHello / ServerHello exceed a 1500-byte Ethernet MTU. The session path
//! always splits through [`split_handshake`]. A split or admission failure is
//! returned as-is; the whole hello is never sent as one datagram.

use crate::crypto::network::kem::MlKem768Secret;
use crate::crypto::network::mldsa;
use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::bearer::contract::{Bearer, check_frame_mtu};
use crate::net::qdnf::bearer::fragment::{
    FRAG_HDR_LEN, HandshakeReassembler, MAX_FRAGMENTS, MAX_FRAME, MAX_HANDSHAKE_BYTES,
    admit_and_buffer, split_handshake,
};
use crate::net::qdnf::bearer::mtu::{MAX_QDNF_MTU, MIN_QDNF_MTU, negotiate_mtu, unfragmented_fit};
use crate::net::qdnf::crypto::finished::{finished_mac, verify_finished};
use crate::net::qdnf::crypto::handshake::failures::entropy_fill;
use crate::net::qdnf::crypto::handshake::{
    FORBIDDEN_ZERO_RTT, HandshakeState, initiator_complete, initiator_share,
    qualified_handshake_gate, reject_unknown_key_share, require_bound_identities,
    responder_complete,
};
use crate::net::qdnf::crypto::schedule::{HandshakeKeys, derive_handshake_keys};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{
    BASE_HEADER_LEN, FrameHeader, copy_payload, decode_frame, encode_frame,
};
use crate::net::qdnf::registries::{FrameType, NextProtocol, flags};
use crate::net::qdnf::types::{ObservedLocator, StrongDigest};

use super::hello::{
    CLIENT_HELLO_WIRE_LEN, SERVER_HELLO_WIRE_LEN, decode_client_hello, decode_server_hello,
    encode_client_hello_with_certs, encode_server_hello_with_certs,
};

const CLIENT_HELLO_MSG: u32 = 1;
const SERVER_HELLO_MSG: u32 = 2;

const _: () = assert!(FORBIDDEN_ZERO_RTT);

/// Completed fragmented handshake. Finished tags are HMAC-SHA-384.
#[derive(Clone, Copy)]
pub struct FragmentedHandshake {
    pub keys: HandshakeKeys,
    pub finished_i2r: StrongDigest,
    pub finished_r2i: StrongDigest,
    pub client_fragments: usize,
    pub server_fragments: usize,
}

/// Bytes of hello after QFrame and fragment headers on a negotiated MTU.
pub fn hello_fragment_payload_mtu(mtu: u16) -> Result<u16, QdnfError> {
    if mtu < MIN_QDNF_MTU {
        return Err(QdnfError::Range);
    }
    let inner = (mtu as usize)
        .checked_sub(BASE_HEADER_LEN)
        .ok_or(QdnfError::Range)?;
    if inner <= FRAG_HDR_LEN {
        return Err(QdnfError::Range);
    }
    u16::try_from(inner - FRAG_HDR_LEN).map_err(|_| QdnfError::Range)
}

/// Split `hello` and send each fragment as a QFrame. Split failure is fatal.
pub fn send_hello_fragments<B: Bearer>(
    sender: &mut B,
    dest: &ObservedLocator,
    msg_id: u32,
    hello: &[u8],
    mtu: u16,
    frame_type: FrameType,
) -> Result<usize, QdnfError> {
    let payload_mtu = hello_fragment_payload_mtu(mtu)?;
    send_hello_fragments_at(sender, dest, msg_id, hello, mtu, payload_mtu, frame_type)
}

fn send_hello_fragments_at<B: Bearer>(
    sender: &mut B,
    dest: &ObservedLocator,
    msg_id: u32,
    hello: &[u8],
    mtu: u16,
    payload_mtu: u16,
    frame_type: FrameType,
) -> Result<usize, QdnfError> {
    if hello.is_empty() {
        return Err(QdnfError::Malformed);
    }
    // Whole-datagram send of an oversize hello is Capacity. Never used as fallback.
    if hello.len() > mtu as usize {
        let _ = unfragmented_fit(hello.len(), mtu);
        let _ = check_frame_mtu(hello.len(), mtu);
    }
    let mut frames = [[0u8; MAX_FRAME]; MAX_FRAGMENTS];
    let mut lens = [0usize; MAX_FRAGMENTS];
    let n = split_handshake(msg_id, hello, payload_mtu, &mut frames, &mut lens)?;
    let mut i = 0usize;
    while i < n {
        let mut header = FrameHeader::new(frame_type, NextProtocol::QSession);
        header.flags |= flags::FRAGMENT;
        header.payload_len = u16::try_from(lens[i]).map_err(|_| QdnfError::Capacity)?;
        let mut wire = [0u8; MAX_QDNF_MTU as usize];
        let wn = encode_frame(&header, &frames[i][..lens[i]], &mut wire)?;
        if wn > mtu as usize {
            return Err(QdnfError::Capacity);
        }
        let sent = sender.send(dest, &wire[..wn])?;
        if sent != wn {
            return Err(QdnfError::WouldBlock);
        }
        i += 1;
    }
    Ok(n)
}

fn recv_hello_fragments<B: Bearer>(
    recv: &mut B,
    expected: FrameType,
    reassembler: &mut HandshakeReassembler,
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let mut seen = 0usize;
    loop {
        if seen >= MAX_FRAGMENTS {
            return Err(QdnfError::Capacity);
        }
        let mut wire = [0u8; MAX_QDNF_MTU as usize];
        let (got, _meta) = recv.recv(&mut wire)?;
        let (decoded, off, len) = decode_frame(&wire[..got])?;
        if decoded.frame_type != expected || decoded.next_protocol != NextProtocol::QSession {
            return Err(QdnfError::Malformed);
        }
        if !decoded.is_fragment() {
            return Err(QdnfError::Capacity);
        }
        let mut payload = [0u8; MAX_FRAME];
        let copied = copy_payload(&wire[..got], off, len, &mut payload)?;
        match admit_and_buffer(reassembler, &payload[..copied], out)? {
            Some(n) => return Ok(n),
            None => seen += 1,
        }
    }
}

fn distinct_x25519() -> Result<([u8; 32], [u8; 32]), QdnfError> {
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    entropy_fill(&mut a)?;
    entropy_fill(&mut b)?;
    if a == b {
        b[0] ^= 0x5a;
        if b.iter().all(|x| *x == 0) {
            return Err(QdnfError::EntropyFailure);
        }
    }
    Ok((a, b))
}

fn bind_handshake_transcript(
    initiator_id: &StrongDigest,
    responder_id: &StrongDigest,
    i_x: &[u8; 32],
    r_x: &[u8; 32],
) -> Result<Transcript, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"suite", b"qpr-pq-1")?;
    t.append(b"initiator", initiator_id.as_bytes())?;
    t.append(b"responder", responder_id.as_bytes())?;
    t.append(b"i-x", i_x)?;
    t.append(b"r-x", r_x)?;
    t.append(b"0rtt", &[0])?;
    require_bound_identities(&t, initiator_id, responder_id)?;
    reject_unknown_key_share(*responder_id, *responder_id)?;
    Ok(t)
}

/// Hybrid handshake over fragmented ClientHello / ServerHello on `mtu ≤ 1500`.
///
/// Fragment split or admission failure is returned. The whole hello is not sent.
pub fn handshake_over_fragments<I, R>(
    initiator: &mut I,
    responder: &mut R,
    initiator_dest: &ObservedLocator,
    responder_dest: &ObservedLocator,
    initiator_id: &StrongDigest,
    responder_id: &StrongDigest,
) -> Result<FragmentedHandshake, QdnfError>
where
    I: Bearer,
    R: Bearer,
{
    if initiator_id.is_zero() || responder_id.is_zero() || initiator_id == responder_id {
        return Err(QdnfError::Unauthorized);
    }
    // PQ hellos exceed Ethernet MTU. Cap even when the local bearer is larger
    // (local-ipc 8192) so the public path still fragments instead of whole-datagram.
    let negotiated = negotiate_mtu(initiator.mtu(), responder.mtu())?;
    let mtu = core::cmp::min(negotiated, MAX_QDNF_MTU);
    let (sk, pk) = MlKem768Secret::generate()?;
    let (i_x, r_x) = distinct_x25519()?;
    let ishare = initiator_share(&i_x, &pk)?;

    let (i_mldsa_sk, i_mldsa_pk) = mldsa::generate_keypair()?;
    let (r_mldsa_sk, r_mldsa_pk) = mldsa::generate_keypair()?;
    let mut i_ed = [0u8; 32];
    let mut r_ed = [0u8; 32];
    entropy_fill(&mut i_ed)?;
    entropy_fill(&mut r_ed)?;

    let mut client = [0u8; CLIENT_HELLO_WIRE_LEN];
    let cn = encode_client_hello_with_certs(&ishare, &i_mldsa_sk, &i_mldsa_pk, &i_ed, &mut client)?;
    if cn != CLIENT_HELLO_WIRE_LEN || cn <= MAX_QDNF_MTU as usize {
        return Err(QdnfError::Malformed);
    }
    if unfragmented_fit(cn, mtu).is_ok() {
        return Err(QdnfError::Capacity);
    }

    let client_fragments = send_hello_fragments(
        initiator,
        initiator_dest,
        CLIENT_HELLO_MSG,
        &client[..cn],
        mtu,
        FrameType::SessionChallenge,
    )?;
    if client_fragments < 2 {
        return Err(QdnfError::Capacity);
    }

    let mut reassembled = [0u8; MAX_HANDSHAKE_BYTES];
    let mut rx = HandshakeReassembler::new();
    let got = recv_hello_fragments(
        responder,
        FrameType::SessionChallenge,
        &mut rx,
        &mut reassembled,
    )?;
    if got != cn {
        return Err(QdnfError::Malformed);
    }
    let peer_ishare = decode_client_hello(&reassembled[..got])?;

    let (rshare, r_kem, r_dh) = responder_complete(&peer_ishare, &r_x)?;
    let mut server = [0u8; SERVER_HELLO_WIRE_LEN];
    let sn = encode_server_hello_with_certs(&rshare, &r_mldsa_sk, &r_mldsa_pk, &r_ed, &mut server)?;
    if sn != SERVER_HELLO_WIRE_LEN || sn <= MAX_QDNF_MTU as usize {
        return Err(QdnfError::Malformed);
    }
    if unfragmented_fit(sn, mtu).is_ok() {
        return Err(QdnfError::Capacity);
    }

    let server_fragments = send_hello_fragments(
        responder,
        responder_dest,
        SERVER_HELLO_MSG,
        &server[..sn],
        mtu,
        FrameType::SessionProof,
    )?;
    if server_fragments < 2 {
        return Err(QdnfError::Capacity);
    }

    let mut rx2 = HandshakeReassembler::new();
    let got_s = recv_hello_fragments(
        initiator,
        FrameType::SessionProof,
        &mut rx2,
        &mut reassembled,
    )?;
    if got_s != sn {
        return Err(QdnfError::Malformed);
    }
    let peer_rshare = decode_server_hello(&reassembled[..got_s])?;
    let (i_kem, i_dh) = initiator_complete(&sk, &i_x, &peer_rshare)?;
    if i_kem != r_kem || i_dh != r_dh {
        return Err(QdnfError::CryptoFailure);
    }

    let t = bind_handshake_transcript(
        initiator_id,
        responder_id,
        &ishare.x25519_pk,
        &peer_rshare.x25519_pk,
    )?;
    let keys = derive_handshake_keys(&i_kem, &i_dh, &t)?;
    let finished_i2r = finished_mac(&keys.initiator_to_responder, &keys.transcript_digest, true)?;
    let finished_r2i = finished_mac(&keys.responder_to_initiator, &keys.transcript_digest, false)?;
    verify_finished(
        &keys.initiator_to_responder,
        &keys.transcript_digest,
        true,
        &finished_i2r,
    )?;
    verify_finished(
        &keys.responder_to_initiator,
        &keys.transcript_digest,
        false,
        &finished_r2i,
    )?;
    qualified_handshake_gate(HandshakeState::Traffic)?;
    Ok(FragmentedHandshake {
        keys,
        finished_i2r,
        finished_r2i,
        client_fragments,
        server_fragments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::bearer::ipc_pair;
    use crate::net::qdnf::harness::oracles::{independent_finished, unkeyed_finished_defect};
    use crate::net::qdnf::types::ScopeEpoch;

    fn ids() -> (StrongDigest, StrongDigest) {
        (sha384(b"did:q42:a"), sha384(b"did:q42:b"))
    }

    #[test]
    fn oversized_hello_is_fragmented_admitted_reassembled_hmac_sha384() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = ipc_pair(scope, MAX_QDNF_MTU).unwrap();
        let dest_b = b.locator();
        let dest_a = a.locator();
        let (id_a, id_b) = ids();
        let done =
            handshake_over_fragments(&mut a, &mut b, &dest_b, &dest_a, &id_a, &id_b).unwrap();
        assert!(done.client_fragments >= 2);
        assert!(done.server_fragments >= 2);
        assert_ne!(
            done.keys.initiator_to_responder,
            done.keys.responder_to_initiator
        );
        assert_eq!(
            done.finished_i2r,
            independent_finished(
                &done.keys.initiator_to_responder,
                &done.keys.transcript_digest,
                true
            )
        );
        assert_eq!(
            done.finished_r2i,
            independent_finished(
                &done.keys.responder_to_initiator,
                &done.keys.transcript_digest,
                false
            )
        );
        assert_ne!(
            done.finished_i2r,
            unkeyed_finished_defect(&done.keys.transcript_digest, b"qpr-pq-1/finished/i2r")
        );
        assert_ne!(done.finished_i2r, done.finished_r2i);
        verify_finished(
            &done.keys.initiator_to_responder,
            &done.keys.transcript_digest,
            true,
            &done.finished_i2r,
        )
        .unwrap();
    }

    #[test]
    fn oversize_hello_without_admission_is_capacity() {
        assert!(CLIENT_HELLO_WIRE_LEN > MAX_QDNF_MTU as usize);
        assert_eq!(
            unfragmented_fit(CLIENT_HELLO_WIRE_LEN, MAX_QDNF_MTU),
            Err(QdnfError::Capacity)
        );
        assert_eq!(
            check_frame_mtu(CLIENT_HELLO_WIRE_LEN, MAX_QDNF_MTU),
            Err(QdnfError::Capacity)
        );

        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, b) = ipc_pair(scope, MAX_QDNF_MTU).unwrap();
        let dest = b.locator();
        let hello = [0x7Au8; CLIENT_HELLO_WIRE_LEN];
        assert_eq!(a.send(&dest, &hello), Err(QdnfError::Capacity));

        let mut header = FrameHeader::new(FrameType::SessionChallenge, NextProtocol::QSession);
        header.payload_len = 4;
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[1, 2, 3, 4], &mut wire).unwrap();
        a.send(&dest, &wire[..n]).unwrap();
        let mut rx = HandshakeReassembler::new();
        let mut out = [0u8; 16];
        let mut peer_b = b;
        assert_eq!(
            recv_hello_fragments(&mut peer_b, FrameType::SessionChallenge, &mut rx, &mut out),
            Err(QdnfError::Capacity)
        );
        assert_eq!(rx.buffered_len(CLIENT_HELLO_MSG), 0);
    }

    #[test]
    fn fragment_failure_does_not_fall_back_to_whole_datagram() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = ipc_pair(scope, MAX_QDNF_MTU).unwrap();
        let dest = b.locator();
        let hello = [0x3Cu8; CLIENT_HELLO_WIRE_LEN];
        assert_eq!(
            send_hello_fragments_at(
                &mut a,
                &dest,
                CLIENT_HELLO_MSG,
                &hello,
                MAX_QDNF_MTU,
                400,
                FrameType::SessionChallenge,
            ),
            Err(QdnfError::Capacity)
        );
        let mut out = [0u8; MAX_QDNF_MTU as usize];
        assert_eq!(b.recv(&mut out), Err(QdnfError::WouldBlock));
        assert_eq!(a.send(&dest, &hello), Err(QdnfError::Capacity));
    }
}
