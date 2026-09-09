//! Authorised protected IPC exchange. Public QPR must not copy plaintext.

use crate::crypto::network::digest::sha384;
use crate::crypto::network::kem::MlKem768Secret;
use crate::crypto::network::transcript::Transcript;
use crate::net::peer::host::{ControllerIdentity, NativePeer};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, ExecutionPermit, InstalledSessionKeys,
};
use crate::net::qdnf::crypto::finished::{finished_mac, verify_finished};
use crate::net::qdnf::crypto::handshake::{
    initiator_complete, initiator_share, responder_complete,
};
use crate::net::qdnf::crypto::schedule::derive_handshake_keys;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::oracles::wire::{require_protected, ProtectedView};
use crate::net::qdnf::session::handshake::SessionBinding;
use crate::net::qdnf::session::packet_protection::PacketProtection;
use crate::net::qdnf::types::{Generation, ScopeEpoch};

fn handshake_keys(
    a_id: ControllerIdentity,
    b_id: ControllerIdentity,
) -> Result<(InstalledSessionKeys, InstalledSessionKeys), QdnfError> {
    let (sk, pk) = MlKem768Secret::generate()?;
    let i_x = {
        let d = sha384(a_id.digest().as_bytes());
        let mut x = [0u8; 32];
        x.copy_from_slice(&d.0[..32]);
        if x.iter().all(|b| *b == 0) {
            x[0] = 1;
        }
        x
    };
    let r_x = {
        let d = sha384(b_id.digest().as_bytes());
        let mut x = [0u8; 32];
        x.copy_from_slice(&d.0[..32]);
        if x == i_x || x.iter().all(|b| *b == 0) {
            x[0] ^= 0x5a;
        }
        x
    };
    let ishare = initiator_share(&i_x, &pk)?;
    let (rshare, r_kem, r_dh) = responder_complete(&ishare, &r_x)?;
    let (i_kem, i_dh) = initiator_complete(&sk, &i_x, &rshare)?;
    debug_assert_eq!(i_kem, r_kem);
    debug_assert_eq!(i_dh, r_dh);
    let mut t = Transcript::new();
    t.append(b"suite", b"qpr-pq-1")?;
    t.append(b"initiator", a_id.digest().as_bytes())?;
    t.append(b"responder", b_id.digest().as_bytes())?;
    t.append(b"i-x", &ishare.x25519_pk)?;
    t.append(b"r-x", &rshare.x25519_pk)?;
    t.append(b"0rtt", &[0])?;
    let keys = derive_handshake_keys(&i_kem, &i_dh, &t)?;
    let fin_i = finished_mac(&keys.initiator_to_responder, &keys.transcript_digest, true)?;
    verify_finished(
        &keys.responder_to_initiator,
        &keys.transcript_digest,
        false,
        &finished_mac(&keys.responder_to_initiator, &keys.transcript_digest, false)?,
    )?;
    verify_finished(
        &keys.initiator_to_responder,
        &keys.transcript_digest,
        true,
        &fin_i,
    )?;
    let a_keys = InstalledSessionKeys::new(
        Generation(1),
        keys.initiator_to_responder,
        keys.responder_to_initiator,
        true,
    )?;
    let b_keys = InstalledSessionKeys::new(
        Generation(1),
        keys.responder_to_initiator,
        keys.initiator_to_responder,
        true,
    )?;
    if keys.transcript_digest.is_zero() {
        return Err(QdnfError::CryptoFailure);
    }
    Ok((a_keys, b_keys))
}

fn issue_permit(
    owner: &mut AuthorityOwner,
    local: &[u8],
    remote: &[u8],
    purpose: &[u8],
    op: &[u8],
    now: u64,
) -> Result<ExecutionPermit, QdnfError> {
    let binding = binding_for_controllers(local, remote, purpose, op)?;
    let (_cred, _contact, handle) =
        owner.install_grant(binding, now, now.saturating_add(3600), ContactState::Active)?;
    owner.issue_permit(handle, binding, now, 4096)
}

/// Two separately constructed peers exchange authorised protected data.
pub fn authorised_ipc_stream_exchange(payload: &[u8]) -> Result<usize, QdnfError> {
    if payload.is_empty() || payload.len() > 64 {
        return Err(QdnfError::Capacity);
    }
    let now = 1_700_000_000u64;
    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let a_did = b"did:q42:a";
    let b_did = b"did:q42:b";
    let purpose = b"q42:QSync/1";
    let (mut a, mut b) = NativePeer::pair_ipc(a_did, b_did, scope, 1280)?;
    a.announce(&b.locator(), now)?;
    let _ = b.accept_announce()?;
    b.announce(&a.locator(), now)?;
    let _ = a.accept_announce()?;

    let mut owner_a = AuthorityOwner::new();
    let mut owner_b = AuthorityOwner::new();
    let permit_a = issue_permit(&mut owner_a, a_did, b_did, purpose, b"op-a", now)?;
    let permit_b = issue_permit(&mut owner_b, b_did, a_did, purpose, b"op-b", now)?;
    let (keys_a, keys_b) = handshake_keys(a.identity(), b.identity())?;

    a.activate_protected(permit_a, keys_a, now)?;
    b.activate_protected(permit_b, keys_b, now)?;

    let sealed = a.send_protected(&b.locator(), payload)?;
    require_protected(ProtectedView {
        wire_payload: sealed.as_slice(),
        application: payload,
    })?;
    let mut out = [0u8; 64];
    let n = b.recv_protected(&mut out)?;
    if &out[..n] != payload {
        return Err(QdnfError::Malformed);
    }
    Ok(n)
}

/// Capture helper: last sealed frame bytes for the wire oracle (bounded).
#[derive(Clone, Copy, Debug)]
pub struct SealedFrame {
    bytes: [u8; 160],
    len: u8,
}

impl SealedFrame {
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl NativePeer {
    pub fn activate_protected(
        &mut self,
        permit: ExecutionPermit,
        keys: InstalledSessionKeys,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        let session = SessionBinding::from_permit(&permit, &keys, now_unix)?;
        let protection = PacketProtection::from_keys(
            *keys.send_key(),
            *keys.recv_key(),
            keys.generation(),
        )?;
        let add = crate::net::peer::runtime::ResourceBudget {
            bytes: 256,
            work: 1,
            io: 1,
        };
        let handle = self.ledger.reserve(add, true)?;
        self.reservation = Some(handle);
        self.session = Some(session);
        self.protection = Some(protection);
        Ok(())
    }

    pub fn send_protected(
        &mut self,
        dest: &crate::net::qdnf::types::ObservedLocator,
        payload: &[u8],
    ) -> Result<SealedFrame, QdnfError> {
        let session = self.session.ok_or(QdnfError::Unauthorized)?;
        session.admit_application()?;
        let remote = self.remote_or_reject()?;
        if self.neighbors.forwarding(remote).is_none() {
            return Err(QdnfError::NoRoute);
        }
        let protection = self.protection.as_mut().ok_or(QdnfError::Unauthorized)?;
        let mut pt = [0u8; 64];
        if payload.len() > pt.len() {
            return Err(QdnfError::Capacity);
        }
        pt[..payload.len()].copy_from_slice(payload);
        let mut sealed = [0u8; 96];
        let n = protection.seal(b"qsession/stream", &mut pt[..payload.len()], &mut sealed)?;
        let mut header = crate::net::qdnf::frame::FrameHeader::new(
            crate::net::qdnf::registries::FrameType::SessionStream,
            crate::net::qdnf::registries::NextProtocol::QSession,
        );
        header.source_link_id = self.local_link;
        header.payload_len = n as u16;
        let mut wire = [0u8; 256];
        let wn = crate::net::qdnf::frame::encode_frame(&header, &sealed[..n], &mut wire)?;
        self.bearer.send(dest, &wire[..wn])?;
        let mut out = SealedFrame {
            bytes: [0u8; 160],
            len: 0,
        };
        let copy = n.min(out.bytes.len());
        out.bytes[..copy].copy_from_slice(&sealed[..copy]);
        out.len = copy as u8;
        Ok(out)
    }

    pub fn recv_protected(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        let session = self.session.ok_or(QdnfError::Unauthorized)?;
        session.admit_application()?;
        let protection = self.protection.as_mut().ok_or(QdnfError::Unauthorized)?;
        let mut wire = [0u8; 256];
        let (got, _meta) = self.bearer.recv(&mut wire)?;
        let (decoded, off, len) = crate::net::qdnf::frame::decode_frame(&wire[..got])?;
        if decoded.frame_type != crate::net::qdnf::registries::FrameType::SessionStream {
            return Err(QdnfError::Malformed);
        }
        let mut sealed = [0u8; 96];
        let copied = crate::net::qdnf::frame::copy_payload(&wire[..got], off, len, &mut sealed)?;
        protection.open(b"qsession/stream", &sealed[..copied], out)
    }
}
