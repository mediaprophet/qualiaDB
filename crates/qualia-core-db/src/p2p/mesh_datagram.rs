// Mesh application datagrams — IPv6/UDP framing carried inside the SocialWebNet overlay.
//
// `WgTunnel`/`SocialWebNet` carry raw *inner IPv6 packets*; on their own they move bytes but give an
// application no way to say "this is a chat message" vs "this is a QDP request". This module is that
// missing layer: it frames an application payload as a real **IPv6 + UDP datagram**, with a
// destination **port** that demultiplexes app protocols on top of one tunnel. Because the overlay is
// IPv6-only (see `wireguard_runtime`), a datagram here is a genuine, well-formed IPv6/UDP packet with
// a correct UDP checksum — so it is valid not just for our in-process decode but for a future
// kernel-TUN peer that routes it through a real stack.
//
// The inner IPv6 addresses are the overlay ULA endpoints (`fd00::/8`); since each tunnel is
// point-to-point the addresses do not themselves route (the tunnel is the addressing), but keeping
// them well-formed preserves that future-compatibility. Application ports live in [`ports`].
//
// Native-only, like the rest of `p2p`'s WireGuard path.
#![cfg(not(target_arch = "wasm32"))]

/// Well-known application ports on the SocialWebNet overlay. Apps demultiplex on the datagram's
/// destination port, exactly like UDP on the public internet.
pub mod ports {
    /// Peer-to-peer chat (chat-graph fragments / sync).
    pub const CHAT: u16 = 6420;
    /// Presence / keep-alive heartbeats.
    pub const PRESENCE: u16 = 6421;
    /// Governed record-share offers/acks.
    pub const SHARE: u16 = 6422;
    /// Self-hosted QDP profile / HTTP-like requests over the mesh (mirrors well-known HTTP).
    pub const QDP: u16 = 80;
}

/// Overlay ULA source/destination addresses used for framed datagrams (`fd00::1` → `fd00::2`).
const OVERLAY_SRC: [u8; 16] = [0xfd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01];
const OVERLAY_DST: [u8; 16] = [0xfd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02];

const IPV6_HEADER: usize = 40;
const UDP_HEADER: usize = 8;
const NEXT_HEADER_UDP: u8 = 17;

/// A decoded application datagram.
#[derive(Debug, Clone, PartialEq)]
pub struct Datagram {
    /// Source application port.
    pub src_port: u16,
    /// Destination application port (the demux key).
    pub dst_port: u16,
    /// Application payload.
    pub payload: Vec<u8>,
}

/// One's-complement Internet checksum over `bytes` seeded with `initial` (the pseudo-header sum),
/// per RFC 1071. Returns the folded 16-bit result (not yet complemented).
fn checksum_sum(bytes: &[u8], mut sum: u32) -> u32 {
    let mut i = 0;
    while i + 1 < bytes.len() {
        sum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
        i += 2;
    }
    if i < bytes.len() {
        // Odd trailing byte is treated as the high byte of a 16-bit word.
        sum += (bytes[i] as u32) << 8;
    }
    sum
}

/// Fold a 32-bit checksum accumulator to 16 bits and one's-complement it.
fn checksum_finish(mut sum: u32) -> u16 {
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Compute the IPv6 UDP checksum over the pseudo-header + UDP header + payload.
///
/// The UDP length covers the 8-byte UDP header plus the payload. `udp_segment` must be the UDP
/// header (with its checksum field zeroed) followed by the payload.
fn udp6_checksum(src: &[u8; 16], dst: &[u8; 16], udp_segment: &[u8]) -> u16 {
    // IPv6 pseudo-header: src(16) + dst(16) + upper-layer length(4, big-endian) + zeros(3) +
    // next-header(1).
    let mut sum = checksum_sum(src, 0);
    sum = checksum_sum(dst, sum);
    sum += udp_segment.len() as u32; // upper-layer packet length (fits in the low 16 bits here)
    sum += NEXT_HEADER_UDP as u32;
    sum = checksum_sum(udp_segment, sum);
    let cs = checksum_finish(sum);
    // A computed checksum of 0 is transmitted as 0xFFFF (0 means "no checksum" in IPv4 but is illegal
    // for IPv6 UDP, so it must be inverted).
    if cs == 0 {
        0xffff
    } else {
        cs
    }
}

/// Frame an application payload as an IPv6 + UDP datagram for the overlay.
///
/// The result is a complete, checksum-correct inner packet suitable for
/// [`WgTunnel::send_packet`](super::wireguard_runtime::WgTunnel::send_packet) /
/// [`SocialWebNet::send_datagram`](super::social_webnet::SocialWebNet::send_datagram).
pub fn encode_datagram(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
    let udp_len = UDP_HEADER + payload.len();
    let total = IPV6_HEADER + udp_len;
    let mut pkt = vec![0u8; total];

    // ── IPv6 header ──
    pkt[0] = 0x60; // version 6, traffic class 0
    // flow label 0 (bytes 1..4 already zero)
    pkt[4..6].copy_from_slice(&(udp_len as u16).to_be_bytes()); // payload length = UDP length
    pkt[6] = NEXT_HEADER_UDP;
    pkt[7] = 64; // hop limit
    pkt[8..24].copy_from_slice(&OVERLAY_SRC);
    pkt[24..40].copy_from_slice(&OVERLAY_DST);

    // ── UDP header (checksum zeroed for the moment) ──
    let udp = &mut pkt[IPV6_HEADER..];
    udp[0..2].copy_from_slice(&src_port.to_be_bytes());
    udp[2..4].copy_from_slice(&dst_port.to_be_bytes());
    udp[4..6].copy_from_slice(&(udp_len as u16).to_be_bytes());
    // udp[6..8] checksum = 0 for now
    udp[8..].copy_from_slice(payload);

    // ── UDP checksum over the pseudo-header + UDP segment ──
    let cs = udp6_checksum(&OVERLAY_SRC, &OVERLAY_DST, &pkt[IPV6_HEADER..]);
    pkt[IPV6_HEADER + 6..IPV6_HEADER + 8].copy_from_slice(&cs.to_be_bytes());
    pkt
}

/// Parse an inner IPv6/UDP packet back into a [`Datagram`]. Returns `None` if it is not a
/// well-formed IPv6 UDP datagram (wrong version, not UDP, truncated, or inconsistent length).
///
/// The checksum is *not* re-verified here: boringtun has already authenticated the packet end-to-end
/// (an attacker cannot forge one), so re-checking the UDP checksum would only guard against local
/// corruption, which the AEAD already rules out.
pub fn decode_datagram(packet: &[u8]) -> Option<Datagram> {
    if packet.len() < IPV6_HEADER + UDP_HEADER {
        return None;
    }
    if packet[0] >> 4 != 6 {
        return None; // not IPv6
    }
    if packet[6] != NEXT_HEADER_UDP {
        return None; // not a (bare) UDP packet — no extension-header walking on the overlay
    }
    let ip_payload_len = u16::from_be_bytes([packet[4], packet[5]]) as usize;
    if IPV6_HEADER + ip_payload_len > packet.len() {
        return None; // truncated
    }

    let udp = &packet[IPV6_HEADER..IPV6_HEADER + ip_payload_len];
    if udp.len() < UDP_HEADER {
        return None;
    }
    let src_port = u16::from_be_bytes([udp[0], udp[1]]);
    let dst_port = u16::from_be_bytes([udp[2], udp[3]]);
    let udp_len = u16::from_be_bytes([udp[4], udp[5]]) as usize;
    if udp_len < UDP_HEADER || udp_len > udp.len() {
        return None; // inconsistent UDP length
    }
    Some(Datagram {
        src_port,
        dst_port,
        payload: udp[UDP_HEADER..udp_len].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ports_and_payload() {
        let pkt = encode_datagram(ports::CHAT, ports::QDP, b"hello mesh app");
        let d = decode_datagram(&pkt).expect("valid datagram");
        assert_eq!(d.src_port, ports::CHAT);
        assert_eq!(d.dst_port, ports::QDP);
        assert_eq!(d.payload, b"hello mesh app");
    }

    #[test]
    fn empty_payload_round_trips() {
        let pkt = encode_datagram(1, 2, b"");
        let d = decode_datagram(&pkt).unwrap();
        assert_eq!((d.src_port, d.dst_port), (1, 2));
        assert!(d.payload.is_empty());
    }

    #[test]
    fn encoded_packet_is_a_valid_inner_ipv6_frame() {
        // The IPv6 header must be well-formed so boringtun's decapsulate validation accepts it: version
        // nibble 6, next-header UDP, and payload-length consistent with the buffer.
        let pkt = encode_datagram(ports::PRESENCE, ports::PRESENCE, b"beat");
        assert_eq!(pkt[0] >> 4, 6, "version 6");
        assert_eq!(pkt[6], NEXT_HEADER_UDP, "next header UDP");
        let ip_payload_len = u16::from_be_bytes([pkt[4], pkt[5]]) as usize;
        assert_eq!(IPV6_HEADER + ip_payload_len, pkt.len(), "length consistent");
    }

    #[test]
    fn udp_checksum_is_correct() {
        // Recomputing the checksum over the whole segment (including the stored checksum field) must
        // fold to zero — the standard receiver-side verification.
        let pkt = encode_datagram(6420, 80, b"checksum me");
        let udp = &pkt[IPV6_HEADER..];
        // Recompute with the stored checksum in place: a valid datagram verifies to 0x0000.
        let mut sum = checksum_sum(&OVERLAY_SRC, 0);
        sum = checksum_sum(&OVERLAY_DST, sum);
        sum += udp.len() as u32;
        sum += NEXT_HEADER_UDP as u32;
        sum = checksum_sum(udp, sum);
        assert_eq!(checksum_finish(sum), 0, "UDP checksum verifies");
    }

    #[test]
    fn rejects_non_ipv6_and_non_udp() {
        // IPv4 version nibble.
        let mut ipv4ish = encode_datagram(1, 2, b"x");
        ipv4ish[0] = 0x45;
        assert!(decode_datagram(&ipv4ish).is_none());

        // IPv6 but next-header not UDP.
        let mut not_udp = encode_datagram(1, 2, b"x");
        not_udp[6] = 6; // TCP
        assert!(decode_datagram(&not_udp).is_none());

        // Truncated.
        let short = encode_datagram(1, 2, b"payload");
        assert!(decode_datagram(&short[..IPV6_HEADER + 4]).is_none());
    }
}
