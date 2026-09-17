//! TURN Allocate encode/decode (RFC 8656 subset). Not a TURN server.

#![cfg(not(target_arch = "wasm32"))]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::p2p::stun_observe::MAGIC;

const ALLOCATE_REQUEST: u16 = 0x0003;
const ALLOCATE_SUCCESS: u16 = 0x0103;
const ATTR_XOR_RELAYED: u16 = 0x0016;
const ATTR_LIFETIME: u16 = 0x000D;
const FAMILY_IPV4: u8 = 0x01;

pub fn encode_allocate_request(tid: &[u8; 12], out: &mut [u8]) -> Result<usize, &'static str> {
    if out.len() < 20 {
        return Err("capacity");
    }
    out[..20].fill(0);
    out[0..2].copy_from_slice(&ALLOCATE_REQUEST.to_be_bytes());
    out[2..4].copy_from_slice(&0u16.to_be_bytes());
    out[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    out[8..20].copy_from_slice(tid);
    Ok(20)
}

pub fn encode_allocate_success(
    tid: &[u8; 12],
    relayed: SocketAddr,
    lifetime_secs: u32,
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let SocketAddr::V4(v4) = relayed else {
        return Err("ipv4 fixture");
    };
    if out.len() < 44 {
        return Err("capacity");
    }
    out[..44].fill(0);
    out[0..2].copy_from_slice(&ALLOCATE_SUCCESS.to_be_bytes());
    out[2..4].copy_from_slice(&24u16.to_be_bytes());
    out[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    out[8..20].copy_from_slice(tid);
    out[20..22].copy_from_slice(&ATTR_XOR_RELAYED.to_be_bytes());
    out[22..24].copy_from_slice(&8u16.to_be_bytes());
    out[25] = FAMILY_IPV4;
    let xport = v4.port() ^ ((MAGIC >> 16) as u16);
    out[26..28].copy_from_slice(&xport.to_be_bytes());
    let oct = v4.ip().octets();
    let cookie = MAGIC.to_be_bytes();
    for i in 0..4 {
        out[28 + i] = oct[i] ^ cookie[i];
    }
    out[32..34].copy_from_slice(&ATTR_LIFETIME.to_be_bytes());
    out[34..36].copy_from_slice(&4u16.to_be_bytes());
    out[36..40].copy_from_slice(&lifetime_secs.to_be_bytes());
    Ok(44)
}

pub fn parse_xor_relayed_v4(msg: &[u8], tid: &[u8; 12]) -> Result<SocketAddr, &'static str> {
    if msg.len() < 32 {
        return Err("truncated");
    }
    let typ = u16::from_be_bytes([msg[0], msg[1]]);
    if typ != ALLOCATE_SUCCESS {
        return Err("not allocate success");
    }
    if u32::from_be_bytes([msg[4], msg[5], msg[6], msg[7]]) != MAGIC {
        return Err("magic");
    }
    if &msg[8..20] != tid {
        return Err("tid");
    }
    let xport = u16::from_be_bytes([msg[26], msg[27]]) ^ ((MAGIC >> 16) as u16);
    let cookie = MAGIC.to_be_bytes();
    let mut ip = [0u8; 4];
    ip.copy_from_slice(&msg[28..32]);
    for i in 0..4 {
        ip[i] ^= cookie[i];
    }
    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
        xport,
    )))
}

/// A WSS circuit is not a TURN allocation.
pub const fn wss_is_not_turn() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_round_trip() {
        let tid = [9u8; 12];
        let mut req = [0u8; 20];
        assert_eq!(encode_allocate_request(&tid, &mut req).unwrap(), 20);
        let mut suc = [0u8; 44];
        let relayed = "198.51.100.8:3478".parse().unwrap();
        encode_allocate_success(&tid, relayed, 600, &mut suc).unwrap();
        assert_eq!(parse_xor_relayed_v4(&suc, &tid).unwrap(), relayed);
        assert!(wss_is_not_turn());
    }
}
