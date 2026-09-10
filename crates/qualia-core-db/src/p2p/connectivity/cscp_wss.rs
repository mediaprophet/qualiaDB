//! CSCP control on local TLS WSS, then QSession. Not a public relay.

#![cfg(not(target_arch = "wasm32"))]

use crate::crypto::network::digest::sha384;
use crate::did_qi::{format_did, DidQi};
use crate::net::peer::fabric::carrier::PathClass;
use crate::net::peer::fabric::connect::{connect, Fabric};
use crate::net::peer::fabric::evidence::{PathEvidence, TransportWitness};
use crate::net::peer::fabric::intent::{ProtectionPolicy, Purpose};
use crate::net::peer::fabric::kernel::KernelEvent;
use crate::net::peer::fabric::outcome::encode_outcome;
use crate::net::peer::fabric::wire::{
    connect_from_wire, decode_accept, encode_connect_request, CscpError,
};
use crate::net::peer::runtime::ResourceBudget;
use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, InstalledSessionKeys,
};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::session::handshake::handshake_over_fragments;
use crate::net::qdnf::session::{SessionBinding, SessionState};
use crate::net::qdnf::types::{Generation, ObservedLocator, ScopeEpoch};
use crate::net::peer::fabric::intent::ConnectionIntent;
use crate::p2p::connectivity::wss::{recv_datagram, send_datagram};
use crate::p2p::connectivity::wss_bearer::WssBearer;
use crate::p2p::connectivity::wss_tls::{loopback_tls_wss, mint_ca};

fn loc(b: u8) -> ObservedLocator {
    ObservedLocator::from_slice(&[b]).unwrap()
}

fn qi_controller_pair() -> ([u8; 64], usize, [u8; 64], usize) {
    let a_src = b"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu";
    let mut a = [0u8; 64];
    a[..a_src.len()].copy_from_slice(a_src);
    let mut b = [0u8; 64];
    let nb = format_did(&DidQi([0x22; 32]), &mut b).expect("did:qi b");
    (a, a_src.len(), b, nb)
}

/// Two loopback TLS WSS peers exchange CSCP ConnectRequest/Accept, then QSession.
pub fn cscp_then_qsession_over_local_tls_wss() -> Result<SessionBinding, String> {
    let ca = mint_ca().map_err(|e| format!("ca: {e}"))?;
    let (mut client, mut server) = loopback_tls_wss(&ca).map_err(|e| format!("tls: {e}"))?;
    let peer = [0xC5u8; 32];
    let intent = ConnectionIntent::new(
        peer,
        Purpose::ordinary(),
        ProtectionPolicy::RELAY_ONLY,
        ResourceBudget {
            bytes: 4096,
            work: 4,
            io: 4,
        },
        10,
        9_000,
    );
    let mut req = [0u8; 512];
    let n = encode_connect_request(&intent, &mut req).map_err(|e| format!("enc: {e:?}"))?;
    send_datagram(&mut client, 1, 1, &req[..n], true).map_err(|e| format!("send: {e}"))?;
    let (_, _, payload) = recv_datagram(&mut server).map_err(|e| format!("recv: {e}"))?;
    let mut fb = Fabric::new();
    connect_from_wire(&mut fb, &payload, 20).map_err(|e| format!("admit: {e:?}"))?;
    if intent.protection.public_dht {
        return Err("relay-only must not enable public DHT".into());
    }
    fb.kernel.step(
        KernelEvent::Descriptor {
            stale: false,
            expired: false,
            direct_locator: false,
        },
        20,
    );
    fb.kernel.step(KernelEvent::LeaseLive, 20);
    fb.kernel.step(
        KernelEvent::PathValidated {
            class: PathClass::Relayed,
        },
        20,
    );
    let path = PathEvidence::from_witness(TransportWitness::from_local(
        PathClass::Relayed,
        fb.kernel.generation,
        256,
        8,
        20,
    ));
    let mut acc = [0u8; 128];
    let an = encode_outcome(&fb.kernel, path, &mut acc).map_err(|e| format!("outcome: {e:?}"))?;
    send_datagram(&mut server, 1, 1, &acc[..an], false).map_err(|e| format!("acc: {e}"))?;
    let (_, _, ap) = recv_datagram(&mut client).map_err(|e| format!("accrecv: {e}"))?;
    let (class, _) = decode_accept(&ap).map_err(|e| format!("decode accept: {e:?}"))?;
    if class != PathClass::Relayed {
        return Err("expected relayed accept".into());
    }
    if matches!(decode_accept(&payload), Ok(_)) {
        return Err("connect request is not accept".into());
    }
    let _ = CscpError::Policy;

    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let dest_b = loc(0x02);
    let dest_a = loc(0x01);
    let mut a = WssBearer::new(client, dest_a, dest_b, scope, true);
    let mut b = WssBearer::new(server, dest_b, dest_a, scope, false);
    if a.profile() != BearerProfile::TlsWssTransitionV1
        || b.profile() != BearerProfile::TlsWssTransitionV1
    {
        return Err("wss bearer must be TlsWssTransitionV1".into());
    }
    let (did_a, na, did_b, nb) = qi_controller_pair();
    let id_a = sha384(&did_a[..na]);
    let id_b = sha384(&did_b[..nb]);
    let hs = handshake_over_fragments(&mut a, &mut b, &dest_b, &dest_a, &id_a, &id_b)
        .map_err(|e| format!("qsession: {e:?}"))?;
    let now = 1_700_000_000u64;
    let mut owner = AuthorityOwner::new();
    let binding = binding_for_controllers(&did_a[..na], &did_b[..nb], b"q42:QSync/1", b"op-c")
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
    let _ = connect(
        &mut Fabric::new(),
        peer,
        Purpose::ordinary(),
        ProtectionPolicy::RELAY_ONLY,
        intent.budget,
        10,
        9_000,
    );
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::connectivity::evidence::{
        masque_bound_udp_internet_executed, public_relay_dialed,
    };

    #[test]
    fn cscp_control_then_qsession_on_local_tls_wss() {
        let session = cscp_then_qsession_over_local_tls_wss().unwrap();
        assert_eq!(session.state(), SessionState::Active);
        assert!(!public_relay_dialed());
        assert!(!masque_bound_udp_internet_executed());
    }
}
