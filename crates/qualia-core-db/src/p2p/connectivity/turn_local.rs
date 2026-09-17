//! Local TURN Allocate + Send/Data (RFC 8656 subset). Not a public server.

#![cfg(not(target_arch = "wasm32"))]

use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use super::turn::{encode_allocate_request, encode_allocate_success, parse_xor_relayed_v4};
use crate::p2p::stun_observe::MAGIC;

const SEND_IND: u16 = 0x0016;
const DATA_IND: u16 = 0x0017;
const ATTR_XOR_PEER: u16 = 0x0012;
const ATTR_DATA: u16 = 0x0013;
const FAMILY_IPV4: u8 = 0x01;

fn xor_addr(addr: SocketAddr, out: &mut [u8]) -> Result<usize, &'static str> {
    let SocketAddr::V4(v4) = addr else {
        return Err("ipv4");
    };
    if out.len() < 8 {
        return Err("capacity");
    }
    out[..8].fill(0);
    out[1] = FAMILY_IPV4;
    let xport = v4.port() ^ ((MAGIC >> 16) as u16);
    out[2..4].copy_from_slice(&xport.to_be_bytes());
    let oct = v4.ip().octets();
    let cookie = MAGIC.to_be_bytes();
    for i in 0..4 {
        out[4 + i] = oct[i] ^ cookie[i];
    }
    Ok(8)
}

fn parse_xor_addr(val: &[u8]) -> Result<SocketAddr, &'static str> {
    if val.len() < 8 || val[1] != FAMILY_IPV4 {
        return Err("xor-addr");
    }
    let xport = u16::from_be_bytes([val[2], val[3]]) ^ ((MAGIC >> 16) as u16);
    let cookie = MAGIC.to_be_bytes();
    let mut ip = [0u8; 4];
    ip.copy_from_slice(&val[4..8]);
    for i in 0..4 {
        ip[i] ^= cookie[i];
    }
    format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], xport)
        .parse()
        .map_err(|_| "parse")
}

fn encode_send(
    tid: &[u8; 12],
    peer: SocketAddr,
    data: &[u8],
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let pad = (4 - (data.len() % 4)) % 4;
    let n = 20 + 12 + 4 + data.len() + pad;
    if out.len() < n {
        return Err("capacity");
    }
    out[..n].fill(0);
    out[0..2].copy_from_slice(&SEND_IND.to_be_bytes());
    out[2..4].copy_from_slice(&((n - 20) as u16).to_be_bytes());
    out[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    out[8..20].copy_from_slice(tid);
    out[20..22].copy_from_slice(&ATTR_XOR_PEER.to_be_bytes());
    out[22..24].copy_from_slice(&8u16.to_be_bytes());
    xor_addr(peer, &mut out[24..32])?;
    out[32..34].copy_from_slice(&ATTR_DATA.to_be_bytes());
    out[34..36].copy_from_slice(&(data.len() as u16).to_be_bytes());
    out[36..36 + data.len()].copy_from_slice(data);
    Ok(n)
}

fn encode_data(
    tid: &[u8; 12],
    peer: SocketAddr,
    data: &[u8],
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let pad = (4 - (data.len() % 4)) % 4;
    let n = 20 + 12 + 4 + data.len() + pad;
    if out.len() < n {
        return Err("capacity");
    }
    out[..n].fill(0);
    out[0..2].copy_from_slice(&DATA_IND.to_be_bytes());
    out[2..4].copy_from_slice(&((n - 20) as u16).to_be_bytes());
    out[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    out[8..20].copy_from_slice(tid);
    out[20..22].copy_from_slice(&ATTR_XOR_PEER.to_be_bytes());
    out[22..24].copy_from_slice(&8u16.to_be_bytes());
    xor_addr(peer, &mut out[24..32])?;
    out[32..34].copy_from_slice(&ATTR_DATA.to_be_bytes());
    out[34..36].copy_from_slice(&(data.len() as u16).to_be_bytes());
    out[36..36 + data.len()].copy_from_slice(data);
    Ok(n)
}

fn parse_send_or_data(msg: &[u8]) -> Result<(u16, [u8; 12], SocketAddr, Vec<u8>), &'static str> {
    if msg.len() < 36 {
        return Err("truncated");
    }
    let typ = u16::from_be_bytes([msg[0], msg[1]]);
    if typ != SEND_IND && typ != DATA_IND {
        return Err("not send/data");
    }
    let mut tid = [0u8; 12];
    tid.copy_from_slice(&msg[8..20]);
    let peer = parse_xor_addr(&msg[24..32])?;
    let dlen = u16::from_be_bytes([msg[34], msg[35]]) as usize;
    if msg.len() < 36 + dlen {
        return Err("data truncated");
    }
    Ok((typ, tid, peer, msg[36..36 + dlen].to_vec()))
}

/// Two clients Allocate on a local hub; A Send reaches B as Data.
pub fn local_allocate_and_forward(payload: &[u8]) -> Result<Vec<u8>, String> {
    let hub = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let hub_addr = hub.local_addr().map_err(|e| e.to_string())?;
    let a = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let b = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    a.set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    b.set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    hub.set_read_timeout(Some(Duration::from_millis(400)))
        .map_err(|e| e.to_string())?;

    let tid_a = [1u8; 12];
    let tid_b = [2u8; 12];
    let mut req = [0u8; 20];
    encode_allocate_request(&tid_a, &mut req).map_err(|e| e.to_string())?;
    a.send_to(&req, hub_addr).map_err(|e| e.to_string())?;
    encode_allocate_request(&tid_b, &mut req).map_err(|e| e.to_string())?;
    b.send_to(&req, hub_addr).map_err(|e| e.to_string())?;

    let relay_a = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let relay_b = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let ra = relay_a.local_addr().map_err(|e| e.to_string())?;
    let rb = relay_b.local_addr().map_err(|e| e.to_string())?;

    let mut a_client = None;
    let mut b_client = None;
    let mut buf = [0u8; 2048];
    for _ in 0..8 {
        let Ok((n, from)) = hub.recv_from(&mut buf) else {
            continue;
        };
        let typ = u16::from_be_bytes([buf[0], buf[1]]);
        if typ == 0x0003 {
            let tid: [u8; 12] = buf[8..20].try_into().unwrap();
            let relayed = if a_client.is_none() { ra } else { rb };
            if a_client.is_none() {
                a_client = Some(from);
            } else {
                b_client = Some(from);
            }
            let mut suc = [0u8; 44];
            encode_allocate_success(&tid, relayed, 600, &mut suc).map_err(|e| e.to_string())?;
            hub.send_to(&suc, from).map_err(|e| e.to_string())?;
        } else if typ == SEND_IND {
            let (_t, tid, peer, data) = parse_send_or_data(&buf[..n]).map_err(|e| e.to_string())?;
            let dest = if peer == rb {
                b_client.ok_or("b missing")?
            } else {
                a_client.ok_or("a missing")?
            };
            let mut data_msg = [0u8; 2048];
            let dn = encode_data(&tid, from, &data, &mut data_msg).map_err(|e| e.to_string())?;
            hub.send_to(&data_msg[..dn], dest)
                .map_err(|e| e.to_string())?;
        }
    }

    let mut rbuf = [0u8; 64];
    let (n, _) = a.recv_from(&mut rbuf).map_err(|e| e.to_string())?;
    let got_a = parse_xor_relayed_v4(&rbuf[..n], &tid_a).map_err(|e| e.to_string())?;
    assert_eq!(got_a, ra);
    let (n, _) = b.recv_from(&mut rbuf).map_err(|e| e.to_string())?;
    let got_b = parse_xor_relayed_v4(&rbuf[..n], &tid_b).map_err(|e| e.to_string())?;
    assert_eq!(got_b, rb);

    let mut send = [0u8; 2048];
    let tid_s = [3u8; 12];
    let sn = encode_send(&tid_s, rb, payload, &mut send).map_err(|e| e.to_string())?;
    a.send_to(&send[..sn], hub_addr)
        .map_err(|e| e.to_string())?;

    // Hub must see the Send. Drain remaining Allocate-phase timeouts then the Send.
    for _ in 0..6 {
        let Ok((n, from)) = hub.recv_from(&mut buf) else {
            continue;
        };
        if u16::from_be_bytes([buf[0], buf[1]]) == SEND_IND {
            let (_t, tid, peer, data) = parse_send_or_data(&buf[..n]).map_err(|e| e.to_string())?;
            let dest = if peer == rb {
                b_client.ok_or("b missing")?
            } else {
                from
            };
            let mut data_msg = [0u8; 2048];
            let dn = encode_data(&tid, from, &data, &mut data_msg).map_err(|e| e.to_string())?;
            hub.send_to(&data_msg[..dn], dest)
                .map_err(|e| e.to_string())?;
            break;
        }
    }

    let (n, _) = b.recv_from(&mut buf).map_err(|e| e.to_string())?;
    let (_t, _tid, _peer, data) = parse_send_or_data(&buf[..n]).map_err(|e| e.to_string())?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_then_send_reaches_peer() {
        let got = local_allocate_and_forward(b"turn-ok").expect("local TURN");
        assert_eq!(got, b"turn-ok");
    }
}
