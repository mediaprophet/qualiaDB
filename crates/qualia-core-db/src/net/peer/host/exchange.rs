//! Authorised protected IPC exchange. Public QPR must not copy plaintext.

use super::{ControllerIdentity, NativePeer, PeerBuilder, CELL_BYTES_DEFAULT};
use crate::net::peer::cells::host_owner::HostAdmission;
use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, ExecutionPermit, InstalledSessionKeys,
};
use crate::net::qdnf::bearer::ipc::{ipc_pair, FRAME_CAP};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::oracles::wire::{require_protected, ProtectedView};
use crate::net::qdnf::session::handshake::handshake_over_fragments;
use crate::net::qdnf::types::{Generation, LinkId, ScopeEpoch};

/// Authorised application payload ceiling (E04.5 / P2).
pub const AUTHORISED_PAYLOAD_CAP: usize = 4096;

/// Protected round-trip size. Equals [`AUTHORISED_PAYLOAD_CAP`] on local-ipc.
pub const AUTHORISED_ROUNDTRIP_CAP: usize = AUTHORISED_PAYLOAD_CAP;

/// Local-ipc MTU that fits 4096 + AEAD overhead + QFrame header.
pub const AUTHORISED_IPC_MTU: u16 = FRAME_CAP as u16;

/// Two native peers over `local-ipc-v1`, admitted from one [`HostAdmission`].
///
/// Each peer needs `cell_bytes`. If remaining host bytes cannot cover the
/// second cell, this returns [`QdnfError::Capacity`].
pub fn pair_ipc_cells(
    host: &mut HostAdmission,
    a_controller: &[u8],
    b_controller: &[u8],
    scope: ScopeEpoch,
    mtu: u16,
    cell_bytes: u64,
) -> Result<(NativePeer, NativePeer), QdnfError> {
    let a_id = ControllerIdentity::from_controller(a_controller)?;
    let b_id = ControllerIdentity::from_controller(b_controller)?;
    if a_id == b_id {
        return Err(QdnfError::Conflict);
    }
    let need = cell_bytes.checked_mul(2).ok_or(QdnfError::Capacity)?;
    if cell_bytes == 0 || host.remaining_host_bytes() < need {
        return Err(QdnfError::Capacity);
    }
    let profile = HostAdmission::profile_for_bytes(cell_bytes)?;
    let slot_a = host.admit_cell(profile, cell_bytes)?;
    let slot_b = match host.admit_cell(profile, cell_bytes) {
        Ok(s) => s,
        Err(_) => {
            let _ = host.release_cell(slot_a);
            return Err(QdnfError::Capacity);
        }
    };
    let (a_bearer, b_bearer) = match ipc_pair(scope, mtu) {
        Ok(p) => p,
        Err(e) => {
            let _ = host.release_cell(slot_b);
            let _ = host.release_cell(slot_a);
            return Err(e);
        }
    };
    let mut a_link = LinkId::ZERO;
    a_link.0[0] = 1;
    let mut b_link = LinkId::ZERO;
    b_link.0[0] = 2;
    let a = match PeerBuilder::new(a_controller, a_link, cell_bytes)
        .and_then(|b| b.with_bearer_cell(a_bearer, Some(slot_a)))
    {
        Ok(p) => p,
        Err(e) => {
            let _ = host.release_cell(slot_b);
            let _ = host.release_cell(slot_a);
            return Err(e);
        }
    };
    let b = match PeerBuilder::new(b_controller, b_link, cell_bytes)
        .and_then(|b| b.with_bearer_cell(b_bearer, Some(slot_b)))
    {
        Ok(p) => p,
        Err(e) => {
            let mut a = a;
            let _ = a.release_into_host(host);
            let _ = host.release_cell(slot_b);
            return Err(e);
        }
    };
    Ok((a, b))
}

fn handshake_on_bearer(
    a: &mut NativePeer,
    b: &mut NativePeer,
) -> Result<(InstalledSessionKeys, InstalledSessionKeys), QdnfError> {
    let dest_b = b.locator();
    let dest_a = a.locator();
    let id_a = a.identity().digest();
    let id_b = b.identity().digest();
    let hs = handshake_over_fragments(
        &mut a.bearer,
        &mut b.bearer,
        &dest_b,
        &dest_a,
        &id_a,
        &id_b,
    )?;
    if hs.keys.transcript_digest.is_zero() {
        return Err(QdnfError::CryptoFailure);
    }
    if hs.client_fragments < 2 || hs.server_fragments < 2 {
        return Err(QdnfError::Capacity);
    }
    let a_keys = InstalledSessionKeys::new(
        Generation(1),
        hs.keys.initiator_to_responder,
        hs.keys.responder_to_initiator,
        true,
    )?;
    let b_keys = InstalledSessionKeys::new(
        Generation(1),
        hs.keys.responder_to_initiator,
        hs.keys.initiator_to_responder,
        true,
    )?;
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

/// Two peers constructed through one host owner exchange authorised protected data.
pub fn authorised_ipc_stream_exchange(payload: &[u8]) -> Result<usize, QdnfError> {
    authorised_ipc_stream_exchange_in(payload, CELL_BYTES_DEFAULT, AUTHORISED_IPC_MTU)
}

/// Same protected path, with the caller’s cell budget and bearer MTU.
pub fn authorised_ipc_stream_exchange_in(
    payload: &[u8],
    cell_bytes: u64,
    mtu: u16,
) -> Result<usize, QdnfError> {
    if payload.is_empty() || payload.len() > AUTHORISED_PAYLOAD_CAP {
        return Err(QdnfError::Capacity);
    }
    let host_bytes = cell_bytes.checked_mul(2).ok_or(QdnfError::Capacity)?;
    let mut host = HostAdmission::new(host_bytes)?;
    let now = 1_700_000_000u64;
    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let a_did = b"did:q42:a";
    let b_did = b"did:q42:b";
    let purpose = b"q42:QSync/1";
    let (mut a, mut b) = pair_ipc_cells(&mut host, a_did, b_did, scope, mtu, cell_bytes)?;
    let result = authorised_exchange_peers(&mut a, &mut b, payload, purpose, now);
    let _ = a.release_into_host(&mut host);
    let _ = b.release_into_host(&mut host);
    result
}

fn authorised_exchange_peers(
    a: &mut NativePeer,
    b: &mut NativePeer,
    payload: &[u8],
    purpose: &[u8],
    now: u64,
) -> Result<usize, QdnfError> {
    let a_did = b"did:q42:a";
    let b_did = b"did:q42:b";
    a.announce(&b.locator(), now)?;
    let _ = b.accept_announce_at(now)?;
    b.announce(&a.locator(), now)?;
    let _ = a.accept_announce_at(now)?;

    let mut owner_a = AuthorityOwner::new();
    let mut owner_b = AuthorityOwner::new();
    let permit_a = issue_permit(&mut owner_a, a_did, b_did, purpose, b"op-a", now)?;
    let permit_b = issue_permit(&mut owner_b, b_did, a_did, purpose, b"op-b", now)?;
    let (keys_a, keys_b) = handshake_on_bearer(a, b)?;

    a.activate_protected(permit_a, keys_a, now)?;
    b.activate_protected(permit_b, keys_b, now)?;

    let sealed = a.send_protected(&b.locator(), payload)?;
    require_protected(ProtectedView {
        wire_payload: sealed.as_slice(),
        application: payload,
    })?;
    let mut out = [0u8; AUTHORISED_PAYLOAD_CAP];
    let n = b.recv_protected(&mut out)?;
    if &out[..n] != payload {
        return Err(QdnfError::Malformed);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::cells::host_owner::extra_identity_multiplies_host_budget;

    fn host_for_pair(cell_bytes: u64) -> HostAdmission {
        let host_bytes = cell_bytes.saturating_mul(2).max(1);
        HostAdmission::new(host_bytes).unwrap()
    }

    #[test]
    fn builder_cell_bytes_zero_is_capacity() {
        let link = LinkId::ZERO;
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", link, 0).unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn pair_ipc_cells_uses_caller_cell_bytes() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let mut host = host_for_pair(2048);
        let (a, _b) =
            pair_ipc_cells(&mut host, b"did:q42:a", b"did:q42:b", scope, 1280, 2048).unwrap();
        assert_eq!(a.remaining_host_bytes(), 2048);
        assert_eq!(host.occupied_cells(), 2);
        let mut zero = HostAdmission::new(4096).unwrap();
        assert_eq!(
            pair_ipc_cells(&mut zero, b"did:q42:a", b"did:q42:b", scope, 1280, 0).unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn native_peer_pair_shares_one_host_admission() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let cell = 2048u64;
        let mut one = HostAdmission::new(cell).unwrap();
        assert_eq!(
            pair_ipc_cells(&mut one, b"did:q42:a", b"did:q42:b", scope, 1280, cell)
                .unwrap_err(),
            QdnfError::Capacity
        );
        assert_eq!(one.occupied_cells(), 0);
        let mut two = HostAdmission::new(cell.saturating_mul(2)).unwrap();
        let (a, b) =
            pair_ipc_cells(&mut two, b"did:q42:a", b"did:q42:b", scope, 1280, cell).unwrap();
        assert_eq!(two.occupied_cells(), 2);
        assert_eq!(a.remaining_host_bytes(), cell);
        assert_eq!(b.remaining_host_bytes(), cell);
        assert!(!extra_identity_multiplies_host_budget());
    }

    #[test]
    fn authorised_path_exchanges_full_payload_cap() {
        // Packet-protection `open` body must be 4096 for this round-trip.
        let mut app = [0u8; AUTHORISED_PAYLOAD_CAP];
        let mut i = 0usize;
        while i < app.len() {
            app[i] = (i as u8).wrapping_add(0xA5);
            i = i.saturating_add(1);
        }
        let n = authorised_ipc_stream_exchange(&app).unwrap();
        assert_eq!(n, AUTHORISED_PAYLOAD_CAP);
    }

    #[test]
    fn authorised_path_rejects_over_payload_cap() {
        let app = [7u8; AUTHORISED_PAYLOAD_CAP + 1];
        assert_eq!(
            authorised_ipc_stream_exchange(&app).unwrap_err(),
            QdnfError::Capacity
        );
    }

    fn install_session(peer: &mut NativePeer, local: &[u8], remote: &[u8], tag: u8, now: u64) {
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(local, remote, b"q42:QSync/1", &[b'o', tag]).unwrap();
        let (_cred, _contact, handle) = owner
            .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, now, 4096).unwrap();
        let mut send = [tag.saturating_add(1); 32];
        let mut recv = [tag.saturating_add(80); 32];
        send[31] = 1;
        recv[31] = 2;
        let keys = InstalledSessionKeys::new(Generation(1), send, recv, true).unwrap();
        peer.activate_protected(permit, keys, now).unwrap();
    }

    #[test]
    fn send_without_announce_is_no_route() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let mut host = host_for_pair(CELL_BYTES_DEFAULT);
        let (mut a, b) = pair_ipc_cells(
            &mut host,
            b"did:q42:a",
            b"did:q42:b",
            scope,
            1280,
            CELL_BYTES_DEFAULT,
        )
        .unwrap();
        let now = 1_700_000_000u64;
        install_session(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        assert_eq!(a.send_stream(&b.locator(), b"x"), Err(QdnfError::NoRoute));
    }

    #[test]
    fn wrong_recipient_is_unauthorized() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let mut host = host_for_pair(CELL_BYTES_DEFAULT);
        let (mut a, mut b) = pair_ipc_cells(
            &mut host,
            b"did:q42:a",
            b"did:q42:b",
            scope,
            1280,
            CELL_BYTES_DEFAULT,
        )
        .unwrap();
        let now = 1_700_000_000u64;
        a.announce(&b.locator(), now).unwrap();
        let _ = b.accept_announce_at(now).unwrap();
        b.announce(&a.locator(), now).unwrap();
        let _ = a.accept_announce_at(now).unwrap();
        install_session(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        let wrong = crate::net::qdnf::types::ObservedLocator::from_slice(&[0x99]).unwrap();
        assert_eq!(
            a.send_protected(&wrong, b"x").unwrap_err(),
            QdnfError::Unauthorized
        );
    }

    #[test]
    fn stale_permit_does_not_activate() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let mut host = host_for_pair(CELL_BYTES_DEFAULT);
        let (mut a, _b) = pair_ipc_cells(
            &mut host,
            b"did:q42:a",
            b"did:q42:b",
            scope,
            1280,
            CELL_BYTES_DEFAULT,
        )
        .unwrap();
        let now = 1_700_000_000u64;
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-s").unwrap();
        let (_cred, _contact, handle) = owner
            .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, now, 4096).unwrap();
        let keys = InstalledSessionKeys::new(Generation(1), [1u8; 32], [2u8; 32], true).unwrap();
        let err = a
            .activate_protected(permit, keys, now.saturating_add(3600))
            .unwrap_err();
        assert!(err == QdnfError::Expired || err == QdnfError::Unauthorized);
    }
}
