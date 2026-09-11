//! Same-socket STUN/TURN/WireGuard demultiplexer plus overlay port dispatch.

#![cfg(not(target_arch = "wasm32"))]

use crate::p2p::mesh_datagram::{decode_datagram_ref, ports};
use crate::p2p::stun_observe::MAGIC;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpClass {
    StunTurn,
    WireGuard,
    Unknown,
}

/// RFC 7983-ish: STUN methods have the top two bits clear and magic cookie.
pub fn classify_udp(buf: &[u8]) -> UdpClass {
    if buf.len() >= 20 {
        let top = buf[0] & 0b1100_0000;
        let magic = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if top == 0 && magic == MAGIC {
            return UdpClass::StunTurn;
        }
    }
    if !buf.is_empty() {
        let t = buf[0] & 0x0f;
        if matches!(t, 1 | 2 | 4) {
            return UdpClass::WireGuard;
        }
    }
    UdpClass::Unknown
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPort {
    Chat,
    Qdnf,
    Presence,
    Share,
    Other(u16),
}

/// Consume an inner IPv6/UDP packet once. Chat and QDNF are both returned.
pub fn classify_overlay(inner: &[u8]) -> Option<(OverlayPort, u16, &[u8])> {
    let (src, dst, payload) = decode_datagram_ref(inner)?;
    let port = match dst {
        ports::CHAT => OverlayPort::Chat,
        ports::QDNF => OverlayPort::Qdnf,
        ports::PRESENCE => OverlayPort::Presence,
        ports::SHARE => OverlayPort::Share,
        other => OverlayPort::Other(other),
    };
    Some((port, src, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::mesh_datagram::encode_datagram;
    use crate::p2p::stun_observe::encode_binding_success_v4;

    #[test]
    fn stun_and_wg_and_overlay_not_lost() {
        let tid = [1u8; 12];
        let stun = encode_binding_success_v4(&tid, "1.2.3.4:5".parse().unwrap()).unwrap();
        assert_eq!(classify_udp(&stun), UdpClass::StunTurn);
        assert_eq!(classify_udp(&[1, 0, 0, 0]), UdpClass::WireGuard);
        let chat = encode_datagram(1, ports::CHAT, b"hi");
        let qdnf = encode_datagram(2, ports::QDNF, b"qf");
        assert!(matches!(
            classify_overlay(&chat),
            Some((OverlayPort::Chat, _, b"hi"))
        ));
        assert!(matches!(
            classify_overlay(&qdnf),
            Some((OverlayPort::Qdnf, _, b"qf"))
        ));
    }
}
