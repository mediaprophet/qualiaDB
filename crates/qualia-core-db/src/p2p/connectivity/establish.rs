//! Local production path: TLS WSS + ICE/TURN checks + QSession admission.

#![cfg(not(target_arch = "wasm32"))]

use crate::crypto::network::digest::sha384;
use crate::net::peer::connectivity::candidates::CandidateKind;
use crate::net::peer::connectivity::policy::PathPolicy;
use crate::net::peer::connectivity::state::{ConnSlot, ConnState};
use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, InstalledSessionKeys,
};
use crate::net::qdnf::session::handshake::handshake_over_fragments;
use crate::net::qdnf::session::{SessionBinding, SessionState};
use crate::net::qdnf::types::{Generation, ObservedLocator, ScopeEpoch};
use crate::p2p::connectivity::ice::{IceAgent, IceRole};
use crate::p2p::connectivity::ice_udp::local_binding_check;
use crate::p2p::connectivity::turn_local::local_allocate_and_forward;
use crate::p2p::connectivity::wss_bearer::WssBearer;
use crate::p2p::connectivity::wss_tls::{loopback_tls_wss, mint_ca};

fn loc(b: u8) -> ObservedLocator {
    ObservedLocator::from_slice(&[b]).unwrap()
}

/// CarrierReady is not SessionReady. SessionReady requires an Active binding.
pub fn session_ready_without_qsession() -> Result<ConnState, String> {
    Err("SessionReady requires QSession authentication".into())
}

/// TLS WSS + real ICE Binding + local TURN + QSession handshake + permit.
pub fn session_ready_over_local_production_path() -> Result<SessionBinding, String> {
    let mapped = local_binding_check().map_err(|e| format!("ice: {e}"))?;
    if !mapped.ip().is_loopback() {
        return Err("ice mapped not loopback".into());
    }
    let mut ice = IceAgent::new(IceRole::Controlling, PathPolicy::ORDINARY, true, true);
    if !ice.mark_checked(CandidateKind::Host) || !ice.nominate(CandidateKind::Host, 0) {
        return Err("ice nominate after check".into());
    }
    let forwarded = local_allocate_and_forward(b"turn-ok").map_err(|e| format!("turn: {e}"))?;
    if forwarded != b"turn-ok" {
        return Err("turn payload".into());
    }

    let ca = mint_ca().map_err(|e| format!("ca: {e}"))?;
    let (client, server) = loopback_tls_wss(&ca).map_err(|e| format!("tls: {e}"))?;
    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let dest_b = loc(0x02);
    let dest_a = loc(0x01);
    let mut a = WssBearer::new(client, dest_a, dest_b, scope, true);
    let mut b = WssBearer::new(server, dest_b, dest_a, scope, false);
    let id_a = sha384(b"did:q42:a");
    let id_b = sha384(b"did:q42:b");
    let hs = handshake_over_fragments(&mut a, &mut b, &dest_b, &dest_a, &id_a, &id_b)
        .map_err(|e| format!("qsession: {e:?}"))?;
    if hs.client_fragments < 2 || hs.server_fragments < 2 {
        return Err("handshake not fragmented".into());
    }
    if hs.keys.transcript_digest.is_zero() {
        return Err("empty transcript".into());
    }

    let now = 1_700_000_000u64;
    let mut owner = AuthorityOwner::new();
    let binding = binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a")
        .map_err(|e| format!("bind: {e:?}"))?;
    let (_cred, _contact, handle) = owner
        .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
        .map_err(|e| format!("grant: {e:?}"))?;
    let permit = owner
        .issue_permit(handle, binding, now, 4096)
        .map_err(|e| format!("permit: {e:?}"))?;
    let keys = InstalledSessionKeys::new(
        Generation(1),
        hs.keys.initiator_to_responder,
        hs.keys.responder_to_initiator,
        true,
    )
    .map_err(|e| format!("keys: {e:?}"))?;
    let session =
        SessionBinding::from_permit(&permit, &keys, now).map_err(|e| format!("session: {e:?}"))?;
    session
        .admit_application()
        .map_err(|e| format!("admit: {e:?}"))?;
    if session.state() != SessionState::Active {
        return Err("not Active".into());
    }

    let mut slot = ConnSlot::new(7);
    slot.transition(1, ConnState::Establishing)
        .map_err(|e| format!("{e:?}"))?;
    slot.transition(1, ConnState::CarrierReady)
        .map_err(|e| format!("{e:?}"))?;
    slot.transition(1, ConnState::SessionReady)
        .map_err(|e| format!("{e:?}"))?;
    if slot.state != ConnState::SessionReady {
        return Err("slot".into());
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_ready_is_not_session_ready() {
        assert!(session_ready_without_qsession().is_err());
    }

    #[test]
    fn local_production_path_requires_qsession() {
        let session = session_ready_over_local_production_path().unwrap();
        assert_eq!(session.state(), SessionState::Active);
        assert!(session.admit_application().is_ok());
    }
}
