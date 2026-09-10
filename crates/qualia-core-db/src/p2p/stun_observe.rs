//! RFC 5389 STUN Binding observe for the SocialWebNet internet path.
//!
//! Lives in `p2p/` (UDP/IP). This is labelled transition, not Native Independent.
//! A STUN reflexive address is an observation, not a signed route or a listen proof.

#![cfg(not(target_arch = "wasm32"))]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

/// STUN magic cookie (RFC 5389).
pub const MAGIC: u32 = 0x2112_A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const ATTR_XOR_MAPPED: u16 = 0x0020;
const FAMILY_IPV4: u8 = 0x01;

/// RFC 4787 mapping behaviour inferred from two STUN servers on one socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingClass {
    /// Same reflexive address toward two destinations. Hole punch *may* work;
    /// inbound still needs filtering/port-forward proof.
    EndpointIndependent,
    /// Different reflexive addresses toward two destinations. A STUN-learned
    /// address is not what a third host would hit. This host must CONNECT
    /// toward a reachable listener (or use a relay).
    AddressDependent,
}

/// Recommended probe role from mapping class. Inbound listen is never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeRole {
    /// Only safe role without a proven public UDP mapping (port-forward / VPS).
    ConnectOnly,
}

/// Two STUN samples plus classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserveReport {
    pub local: SocketAddr,
    pub sample_a: SocketAddr,
    pub sample_b: SocketAddr,
    pub class: MappingClass,
}

/// A two-host internet WireGuard handshake has not been executed in-process.
pub const fn internet_two_host_handshake_executed() -> bool {
    false
}

/// SocialWebNet internet path is the labelled WireGuard transition, not Ethernet.
pub const fn labelled_wireguard_transition() -> bool {
    true
}

/// Address-dependent mapping cannot publish a STUN address as a listen locator.
pub fn recommend_role(class: MappingClass) -> ProbeRole {
    match class {
        MappingClass::EndpointIndependent | MappingClass::AddressDependent => ProbeRole::ConnectOnly,
    }
}

/// Parse XOR-MAPPED-ADDRESS (IPv4) from a STUN success response.
pub fn parse_xor_mapped_v4(msg: &[u8], expected_tid: &[u8; 12]) -> Result<SocketAddr, String> {
    if msg.len() < 20 {
        return Err("stun truncated".into());
    }
    let typ = u16::from_be_bytes([msg[0], msg[1]]);
    if typ != BINDING_SUCCESS {
        return Err(format!("stun type {typ:#06x}, want success"));
    }
    let magic = u32::from_be_bytes([msg[4], msg[5], msg[6], msg[7]]);
    if magic != MAGIC {
        return Err("stun bad magic".into());
    }
    if &msg[8..20] != expected_tid {
        return Err("stun tid mismatch".into());
    }
    let attr_len = u16::from_be_bytes([msg[2], msg[3]]) as usize;
    let end = 20usize.saturating_add(attr_len).min(msg.len());
    let mut i = 20usize;
    while i + 4 <= end {
        let atype = u16::from_be_bytes([msg[i], msg[i + 1]]);
        let alen = u16::from_be_bytes([msg[i + 2], msg[i + 3]]) as usize;
        let val_at = i + 4;
        if val_at.saturating_add(alen) > msg.len() {
            return Err("stun attr truncated".into());
        }
        if atype == ATTR_XOR_MAPPED {
            return decode_xor_mapped(&msg[val_at..val_at + alen]);
        }
        let pad = (4 - (alen % 4)) % 4;
        i = val_at + alen + pad;
    }
    Err("stun no XOR-MAPPED-ADDRESS".into())
}

fn decode_xor_mapped(val: &[u8]) -> Result<SocketAddr, String> {
    if val.len() < 8 {
        return Err("xor-mapped short".into());
    }
    let family = val[1];
    if family != FAMILY_IPV4 {
        return Err("xor-mapped not IPv4".into());
    }
    let xport = u16::from_be_bytes([val[2], val[3]]);
    let port = xport ^ ((MAGIC >> 16) as u16);
    let mut ip = [0u8; 4];
    ip.copy_from_slice(&val[4..8]);
    let cookie = MAGIC.to_be_bytes();
    for i in 0..4 {
        ip[i] ^= cookie[i];
    }
    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
        port,
    )))
}

/// Encode a Binding success with IPv4 XOR-MAPPED-ADDRESS (test fixture / oracle).
pub fn encode_binding_success_v4(tid: &[u8; 12], mapped: SocketAddr) -> Result<[u8; 32], String> {
    let SocketAddr::V4(v4) = mapped else {
        return Err("fixture IPv4 only".into());
    };
    let mut out = [0u8; 32];
    out[0..2].copy_from_slice(&BINDING_SUCCESS.to_be_bytes());
    out[2..4].copy_from_slice(&12u16.to_be_bytes());
    out[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    out[8..20].copy_from_slice(tid);
    out[20..22].copy_from_slice(&ATTR_XOR_MAPPED.to_be_bytes());
    out[22..24].copy_from_slice(&8u16.to_be_bytes());
    out[25] = FAMILY_IPV4;
    let xport = v4.port() ^ ((MAGIC >> 16) as u16);
    out[26..28].copy_from_slice(&xport.to_be_bytes());
    let oct = v4.ip().octets();
    let cookie = MAGIC.to_be_bytes();
    for i in 0..4 {
        out[28 + i] = oct[i] ^ cookie[i];
    }
    Ok(out)
}

/// Send one Binding request on `sock` toward `server` and parse XOR-MAPPED-ADDRESS.
pub fn stun_bind(sock: &UdpSocket, server: SocketAddr, timeout: Duration) -> Result<SocketAddr, String> {
    sock.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    let mut tid = [0u8; 12];
    fill_tid(&mut tid)?;
    let mut req = [0u8; 20];
    req[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    req[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    req[8..20].copy_from_slice(&tid);
    sock.send_to(&req, server).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 512];
    let (n, _) = sock.recv_from(&mut buf).map_err(|e| e.to_string())?;
    parse_xor_mapped_v4(&buf[..n], &tid)
}

fn fill_tid(tid: &mut [u8; 12]) -> Result<(), String> {
    getrandom::fill(tid).map_err(|e| e.to_string())
}

/// Query two STUN servers on the same socket and classify mapping.
pub fn observe_mapping(
    sock: &UdpSocket,
    stun_a: SocketAddr,
    stun_b: SocketAddr,
    timeout: Duration,
) -> Result<ObserveReport, String> {
    let local = sock.local_addr().map_err(|e| e.to_string())?;
    let sample_a = stun_bind(sock, stun_a, timeout)?;
    let sample_b = stun_bind(sock, stun_b, timeout)?;
    let class = if sample_a == sample_b {
        MappingClass::EndpointIndependent
    } else {
        MappingClass::AddressDependent
    };
    Ok(ObserveReport {
        local,
        sample_a,
        sample_b,
        class,
    })
}

/// Classify two already-parsed samples (no network).
pub fn classify_samples(a: SocketAddr, b: SocketAddr) -> MappingClass {
    if a == b {
        MappingClass::EndpointIndependent
    } else {
        MappingClass::AddressDependent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_mapped_v4_round_trips() {
        let tid = [7u8; 12];
        let mapped: SocketAddr = "203.0.113.5:51820".parse().unwrap();
        let pkt = encode_binding_success_v4(&tid, mapped).unwrap();
        assert_eq!(parse_xor_mapped_v4(&pkt, &tid).unwrap(), mapped);
    }

    #[test]
    fn rejects_tid_mismatch() {
        let tid = [1u8; 12];
        let mapped: SocketAddr = "192.0.2.1:9".parse().unwrap();
        let pkt = encode_binding_success_v4(&tid, mapped).unwrap();
        let other = [2u8; 12];
        assert!(parse_xor_mapped_v4(&pkt, &other).unwrap_err().contains("tid"));
    }

    #[test]
    fn address_dependent_when_samples_differ() {
        let a: SocketAddr = "198.51.100.1:1000".parse().unwrap();
        let b: SocketAddr = "198.51.100.2:2000".parse().unwrap();
        assert_eq!(
            classify_samples(a, b),
            MappingClass::AddressDependent
        );
        assert_eq!(recommend_role(MappingClass::AddressDependent), ProbeRole::ConnectOnly);
        assert_eq!(
            classify_samples(a, a),
            MappingClass::EndpointIndependent
        );
        assert_eq!(
            recommend_role(MappingClass::EndpointIndependent),
            ProbeRole::ConnectOnly
        );
    }

    #[test]
    fn internet_handshake_is_not_claimed() {
        assert!(!internet_two_host_handshake_executed());
        assert!(labelled_wireguard_transition());
    }
}
