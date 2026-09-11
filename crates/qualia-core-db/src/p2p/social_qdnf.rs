//! QDNF QFrames over the SocialWebNet userspace-WireGuard mesh.
//!
//! Lives in `p2p/`, not `net/qdnf/`: the QDNF tree must not import IP sockets.
//! WireGuard stays the labelled SocialWebNet transition carrier. Inner packets
//! remain IPv6 (boringtun drops non-IPv6). QFrames therefore ride the existing
//! overlay datagram on [`ports::QDNF`]. Chat on [`ports::CHAT`] is unchanged.
//!
//! This adapter is **not** Native Independent and does not use libp2p.

#![cfg(not(target_arch = "wasm32"))]

use crate::net::qdnf::bearer::contract::{check_frame_mtu, Bearer, RecvMeta};
use crate::net::qdnf::bearer::mtu::MIN_QDNF_MTU;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::mesh_datagram::{decode_datagram_ref, ports};
use super::social_webnet::SocialWebNet;

/// QFrame MTU on this overlay. Inner IPv6+UDP adds 48 bytes; 1280 keeps the
/// encapsulated datagram inside a typical 1500-byte outer path.
pub const SOCIAL_QDNF_MTU: u16 = MIN_QDNF_MTU;

/// Idle `pump_all` turns before [`SocialQdnfLink::recv`] returns WouldBlock.
const MAX_RECV_IDLE: usize = 24;

/// SocialWebNet's data plane does not import or construct a libp2p Swarm.
pub const fn uses_libp2p() -> bool {
    false
}

/// Outer carrier is userspace WireGuard (`boringtun`), as in the SocialWebNet profile.
pub const fn carrier_is_wireguard() -> bool {
    true
}

/// Labelled IP/WG transition. Must not be claimed as Native Independent.
pub const fn native_independent() -> bool {
    BearerProfile::WireGuardTransitionV1.native_independent()
}

/// One peer's QDNF overlay on an existing WireGuard tunnel.
///
/// `recv` pumps the mesh and consumes inner packets. Use it on a dedicated
/// test/control path; production chat demuxes [`ports::CHAT`] from `pump_all`
/// / `MeshService` inbound and ignores this port.
pub struct SocialQdnfLink<'a> {
    mesh: &'a mut SocialWebNet,
    peer_id: &'a str,
    local: ObservedLocator,
    remote: ObservedLocator,
    scope: ScopeEpoch,
    mtu: u16,
}

impl<'a> SocialQdnfLink<'a> {
    /// Attach QDNF to an already-peered tunnel (`peer_id` must exist).
    pub fn attach(
        mesh: &'a mut SocialWebNet,
        peer_id: &'a str,
        local: ObservedLocator,
        remote: ObservedLocator,
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<Self, QdnfError> {
        if mtu < MIN_QDNF_MTU {
            return Err(QdnfError::Range);
        }
        Ok(Self {
            mesh,
            peer_id,
            local,
            remote,
            scope,
            mtu,
        })
    }

    pub fn locator(&self) -> ObservedLocator {
        self.local
    }
}

/// Copy a QDNF overlay payload from a decrypted inner IPv6 packet into `out`.
///
/// `Ok(None)` means the packet is not a QDNF datagram (chat/presence/other).
pub fn copy_qdnf_frame(inner: &[u8], out: &mut [u8]) -> Result<Option<usize>, QdnfError> {
    let Some((_src, dst, payload)) = decode_datagram_ref(inner) else {
        return Ok(None);
    };
    if dst != ports::QDNF {
        return Ok(None);
    }
    if payload.len() > out.len() {
        return Err(QdnfError::Capacity);
    }
    out[..payload.len()].copy_from_slice(payload);
    Ok(Some(payload.len()))
}

impl Bearer for SocialQdnfLink<'_> {
    fn profile(&self) -> BearerProfile {
        BearerProfile::WireGuardTransitionV1
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu)?;
        if dest.as_slice() != self.remote.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        let sent = self
            .mesh
            .send_qdnf_frame(self.peer_id, frame)
            .map_err(|_| QdnfError::Closed)?;
        if !sent {
            return Err(QdnfError::WouldBlock);
        }
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let mut idles = 0usize;
        loop {
            let events = self.mesh.pump_all();
            let mut progressed = false;
            for evt in events {
                let pkt = match evt {
                    Ok(pkt) => pkt,
                    Err(_) => return Err(QdnfError::Closed),
                };
                progressed = true;
                if pkt.peer_id != self.peer_id {
                    continue;
                }
                if let Some(n) = copy_qdnf_frame(&pkt.inner, out)? {
                    return Ok((
                        n,
                        RecvMeta {
                            observed_source: self.remote,
                            scope: self.scope,
                            mtu: self.mtu,
                        },
                    ));
                }
            }
            if progressed {
                continue;
            }
            idles += 1;
            if idles >= MAX_RECV_IDLE {
                return Err(QdnfError::WouldBlock);
            }
        }
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::frame::{decode_frame, encode_frame, FrameHeader, MAGIC};
    use crate::net::qdnf::registries::{FrameType, NextProtocol};
    use crate::net::qdnf::session::handshake::handshake_over_fragments;
    use crate::net::qdnf::types::StrongDigest;
    use crate::p2p::mesh_datagram::{decode_datagram, encode_datagram};
    use crate::p2p::social_webnet::SocialWebNet;
    use crate::p2p::wireguard_userspace::generate_keypair;
    use std::time::Duration;

    fn loc(b: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[b]).unwrap()
    }

    fn ids() -> (StrongDigest, StrongDigest) {
        (sha384(b"did:q42:social-a"), sha384(b"did:q42:social-b"))
    }

    fn peered_meshes() -> (SocialWebNet, SocialWebNet) {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let (a_pub, b_pub) = (a_keys.public_hex(), b_keys.public_hex());
        let to = Some(Duration::from_millis(50));
        let mut a = SocialWebNet::new(a_keys, "127.0.0.1".parse().unwrap(), to);
        let mut b = SocialWebNet::new(b_keys, "127.0.0.1".parse().unwrap(), to);
        let a_local = a.add_peer("b", &b_pub, None).unwrap();
        let b_local = b.add_peer("a", &a_pub, None).unwrap();
        a.set_peer_endpoint("b", b_local).unwrap();
        b.set_peer_endpoint("a", a_local).unwrap();
        a.initiate_handshake("b").unwrap();
        for _ in 0..40 {
            if a.has_session("b") && b.has_session("a") {
                break;
            }
            let _ = b.pump_all();
            let _ = a.pump_all();
        }
        assert!(a.has_session("b"), "A WireGuard session");
        let _ = b.pump_all();
        assert!(b.has_session("a"), "B WireGuard session");
        (a, b)
    }

    #[test]
    fn honesty_flags_label_wireguard_not_libp2p_or_native() {
        assert!(!uses_libp2p());
        assert!(carrier_is_wireguard());
        assert!(!native_independent());
        assert!(!BearerProfile::WireGuardTransitionV1.native_independent());
    }

    #[test]
    #[serial_test::serial]
    fn two_meshes_exchange_a_qframe_over_wireguard() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = peered_meshes();
        let dest_b = loc(0x02);
        let dest_a = loc(0x01);
        let mut a_link =
            SocialQdnfLink::attach(&mut a, "b", dest_a, dest_b, scope, SOCIAL_QDNF_MTU).unwrap();
        let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[], &mut wire).unwrap();
        assert_eq!(a_link.send(&dest_b, &wire[..n]).unwrap(), n);

        let mut b_link =
            SocialQdnfLink::attach(&mut b, "a", dest_b, dest_a, scope, SOCIAL_QDNF_MTU).unwrap();
        assert_eq!(b_link.profile(), BearerProfile::WireGuardTransitionV1);
        let mut out = [0u8; 256];
        let (got, meta) = b_link.recv(&mut out).expect("B received a QFrame");
        assert_eq!(got, n);
        assert_eq!(&out[..4], &MAGIC);
        let (decoded, _, _) = decode_frame(&out[..got]).unwrap();
        assert_eq!(decoded.frame_type, FrameType::DiscoveryBeacon);
        assert_eq!(decoded.next_protocol, NextProtocol::QLink);
        assert_eq!(meta.observed_source, dest_a);
        assert!(!a_link.profile().native_independent());
    }

    #[test]
    #[serial_test::serial]
    fn chat_and_qdnf_demux_on_the_same_tunnel() {
        let (mut a, mut b) = peered_meshes();
        assert!(a
            .send_datagram("b", ports::CHAT, ports::CHAT, b"hello chat")
            .unwrap());
        let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[], &mut wire).unwrap();
        assert!(a.send_qdnf_frame("b", &wire[..n]).unwrap());

        let mut chat = None;
        let mut qdnf = None;
        for _ in 0..40 {
            for evt in b.pump_all() {
                let Ok(pkt) = evt else { continue };
                if let Some(d) = decode_datagram(&pkt.inner) {
                    if d.dst_port == ports::CHAT {
                        chat = Some(d.payload);
                    } else if d.dst_port == ports::QDNF {
                        qdnf = Some(d.payload);
                    }
                }
            }
            if chat.is_some() && qdnf.is_some() {
                break;
            }
        }
        assert_eq!(chat.as_deref(), Some(b"hello chat".as_slice()));
        let frame = qdnf.expect("QDNF datagram");
        assert_eq!(&frame[..4], &MAGIC);
        assert_eq!(frame.len(), n);
    }

    #[test]
    #[serial_test::serial]
    fn fragmented_handshake_over_wireguard_overlay() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = peered_meshes();
        let dest_b = loc(0x02);
        let dest_a = loc(0x01);
        let mut a_link =
            SocialQdnfLink::attach(&mut a, "b", dest_a, dest_b, scope, SOCIAL_QDNF_MTU).unwrap();
        let mut b_link =
            SocialQdnfLink::attach(&mut b, "a", dest_b, dest_a, scope, SOCIAL_QDNF_MTU).unwrap();
        let (id_a, id_b) = ids();
        let done = handshake_over_fragments(&mut a_link, &mut b_link, &dest_b, &dest_a, &id_a, &id_b)
            .expect("QSession handshake over WG overlay");
        assert!(done.client_fragments >= 2);
        assert!(done.server_fragments >= 2);
        assert_ne!(
            done.keys.initiator_to_responder,
            done.keys.responder_to_initiator
        );
    }

    #[test]
    fn unused_overlay_inner_is_not_a_qdnf_frame() {
        let pkt = encode_datagram(ports::CHAT, ports::CHAT, b"not qdnf");
        let mut out = [0u8; 64];
        assert_eq!(copy_qdnf_frame(&pkt, &mut out).unwrap(), None);
    }
}
