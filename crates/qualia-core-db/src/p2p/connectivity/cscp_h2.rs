//! CSCP control on local TLS HTTP/2 Extended CONNECT + RFC 9297 capsules, then QSession.
//!
//! Loopback only. Not a public relay, not UDP, not MASQUE Internet. Nested
//! recovery over the reliable capsule stream is degraded (head-of-line blocking).

#![cfg(not(target_arch = "wasm32"))]

use crate::crypto::network::digest::sha384;
use crate::net::peer::fabric::carrier::PathClass;
use crate::net::peer::fabric::connect::{connect, Fabric};
use crate::net::peer::fabric::evidence::{PathEvidence, TransportWitness};
use crate::net::peer::fabric::intent::{ConnectionIntent, ProtectionPolicy, Purpose};
use crate::net::peer::fabric::kernel::KernelEvent;
use crate::net::peer::fabric::outcome::encode_outcome;
use crate::net::peer::fabric::wire::{
    connect_from_wire, decode_accept, encode_connect_request, CscpError,
};
use crate::net::peer::runtime::ResourceBudget;
use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, InstalledSessionKeys,
};
use crate::net::qdnf::bearer::contract::{check_frame_mtu, Bearer, RecvMeta};
use crate::net::qdnf::bearer::mtu::MIN_QDNF_MTU;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::session::handshake::handshake_over_fragments;
use crate::net::qdnf::session::{SessionBinding, SessionState};
use crate::net::qdnf::types::{Generation, ObservedLocator, ScopeEpoch};
use crate::p2p::connectivity::h2_capsule::{
    loopback_tls_h2_capsule, CapsuleEndpoint,
};
use crate::p2p::connectivity::wss_tls::mint_ca;

fn loc(b: u8) -> ObservedLocator {
    ObservedLocator::from_slice(&[b]).unwrap()
}

struct H2CapsuleBearer {
    ep: CapsuleEndpoint,
    remote: ObservedLocator,
    scope: ScopeEpoch,
}

impl Bearer for H2CapsuleBearer {
    fn profile(&self) -> BearerProfile {
        BearerProfile::UdpTransitionV1
    }

    fn mtu(&self) -> u16 {
        MIN_QDNF_MTU
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu())?;
        if dest.as_slice() != self.remote.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        self.ep.send_payload(frame).map_err(|_| QdnfError::Closed)?;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let p = self.ep.recv_payload().map_err(|_| QdnfError::Closed)?;
        if p.len() > out.len() {
            return Err(QdnfError::Capacity);
        }
        out[..p.len()].copy_from_slice(&p);
        Ok((
            p.len(),
            RecvMeta {
                observed_source: self.remote,
                scope: self.scope,
                mtu: self.mtu(),
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        Ok(())
    }
}

/// Two loopback TLS HTTP/2 peers exchange CSCP ConnectRequest/Accept as DATAGRAM
/// capsules, then QSession. Not a public relay and not MASQUE Internet.
pub fn cscp_then_qsession_over_local_tls_h2() -> Result<SessionBinding, String> {
    let ca = mint_ca().map_err(|e| format!("ca: {e}"))?;
    let (client, server) = loopback_tls_h2_capsule(&ca).map_err(|e| format!("tls-h2: {e}"))?;
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
    client
        .send_payload(&req[..n])
        .map_err(|e| format!("send: {e}"))?;
    let payload = server.recv_payload().map_err(|e| format!("recv: {e}"))?;
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
    server
        .send_payload(&acc[..an])
        .map_err(|e| format!("acc: {e}"))?;
    let ap = client.recv_payload().map_err(|e| format!("accrecv: {e}"))?;
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
    let mut a = H2CapsuleBearer {
        ep: client,
        remote: dest_b,
        scope,
    };
    let mut b = H2CapsuleBearer {
        ep: server,
        remote: dest_a,
        scope,
    };
    let id_a = sha384(b"did:q42:cscp-a");
    let id_b = sha384(b"did:q42:cscp-b");
    let hs = handshake_over_fragments(&mut a, &mut b, &dest_b, &dest_a, &id_a, &id_b)
        .map_err(|e| format!("qsession: {e:?}"))?;
    let now = 1_700_000_000u64;
    let mut owner = AuthorityOwner::new();
    let binding = binding_for_controllers(b"did:q42:cscp-a", b"did:q42:cscp-b", b"q42:QSync/1", b"op-c")
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
    use crate::net::peer::fabric::capsule::encode_capsule;
    use crate::p2p::connectivity::h2_capsule::loopback_tls_h2_connect;

    #[test]
    fn cscp_control_then_qsession_on_local_tls_h2() {
        let session = cscp_then_qsession_over_local_tls_h2().unwrap();
        assert_eq!(session.state(), SessionState::Active);
        assert!(!public_relay_dialed());
        assert!(!masque_bound_udp_internet_executed());
    }

    #[test]
    fn wrong_connect_protocol_rejected() {
        let ca = mint_ca().unwrap();
        let err = match loopback_tls_h2_connect(&ca, "websocket") {
            Ok(_) => panic!("wrong :protocol must be rejected"),
            Err(e) => e,
        };
        assert!(
            err.contains("capsule") || err.contains("CONNECT") || err.contains("rejected"),
            "{err}"
        );
    }

    #[test]
    fn unknown_critical_capsule_rejected() {
        let ca = mint_ca().unwrap();
        let (c, s) = loopback_tls_h2_capsule(&ca).unwrap();
        let mut frame = [0u8; 16];
        let n = encode_capsule(0x40, b"y", &mut frame).unwrap();
        c.send_raw_capsule(&frame[..n]).unwrap();
        let err = s.recv_payload().unwrap_err();
        assert!(err.contains("critical"), "{err}");
    }

    #[test]
    fn honesty_flags_remain_false_for_local_h2() {
        assert!(!public_relay_dialed());
        assert!(!masque_bound_udp_internet_executed());
    }
}
