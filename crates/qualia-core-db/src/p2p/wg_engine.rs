//! Socket-free WireGuard engine. Caller supplies output buffers.

#![cfg(not(target_arch = "wasm32"))]

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::PublicKey;

use super::wireguard_userspace::{new_tunnel, WgKeypair};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineOut {
    Network { len: usize },
    InnerV6 { len: usize },
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    Invalid,
    Ipv4Inner,
}

/// Point-to-point `Tunn` with no socket and no per-packet `Vec`.
pub struct WgEngine {
    tunn: Tunn,
}

impl WgEngine {
    pub fn new(mine: &WgKeypair, peer: PublicKey, index: u32) -> Result<Self, String> {
        Ok(Self {
            tunn: new_tunnel(mine, peer, index)?,
        })
    }

    pub fn has_session(&self) -> bool {
        self.tunn.time_since_last_handshake().is_some()
    }

    pub fn encapsulate(&mut self, inner: &[u8], out: &mut [u8]) -> Result<EngineOut, EngineError> {
        match self.tunn.encapsulate(inner, out) {
            TunnResult::WriteToNetwork(p) => Ok(EngineOut::Network { len: p.len() }),
            TunnResult::Done => Ok(EngineOut::Done),
            TunnResult::Err(_) => Err(EngineError::Invalid),
            TunnResult::WriteToTunnelV4(_, _) | TunnResult::WriteToTunnelV6(_, _) => {
                Err(EngineError::Invalid)
            }
        }
    }

    pub fn decapsulate(&mut self, pkt: &[u8], out: &mut [u8]) -> Result<EngineOut, EngineError> {
        match self.tunn.decapsulate(None, pkt, out) {
            TunnResult::WriteToNetwork(p) => Ok(EngineOut::Network { len: p.len() }),
            TunnResult::WriteToTunnelV6(data, _) => Ok(EngineOut::InnerV6 { len: data.len() }),
            TunnResult::WriteToTunnelV4(_, _) => Err(EngineError::Ipv4Inner),
            TunnResult::Done => Ok(EngineOut::Done),
            TunnResult::Err(_) => Err(EngineError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::wireguard_userspace::generate_keypair;

    fn ipv6_hello() -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = 0x60;
        p[4..6].copy_from_slice(&8u16.to_be_bytes());
        p[6] = 17;
        p[7] = 64;
        p[8] = 0xfd;
        p[23] = 1;
        p[24] = 0xfd;
        p[39] = 2;
        p[40..].copy_from_slice(b"engine!!");
        p
    }

    #[test]
    fn handshake_and_inner_v6_caller_buffers() {
        let a_k = generate_keypair();
        let b_k = generate_keypair();
        let mut a = WgEngine::new(&a_k, b_k.public, 1).unwrap();
        let mut b = WgEngine::new(&b_k, a_k.public, 2).unwrap();
        let mut buf = [0u8; 2048];
        let init = match a.encapsulate(&[], &mut buf).unwrap() {
            EngineOut::Network { len } => {
                let mut p = [0u8; 2048];
                p[..len].copy_from_slice(&buf[..len]);
                (p, len)
            }
            other => panic!("{other:?}"),
        };
        let mut inflight = init;
        let mut to_b = true;
        let mut done = false;
        for _ in 0..12 {
            let (pkt, n) = inflight;
            let recv = if to_b { &mut b } else { &mut a };
            match recv.decapsulate(&pkt[..n], &mut buf).unwrap() {
                EngineOut::Network { len } => {
                    let mut p = [0u8; 2048];
                    p[..len].copy_from_slice(&buf[..len]);
                    inflight = (p, len);
                    to_b = !to_b;
                }
                EngineOut::Done | EngineOut::InnerV6 { .. } => {
                    done = true;
                    break;
                }
            }
        }
        assert!(done);
        let inner = ipv6_hello();
        let mut got = None;
        for _ in 0..4 {
            match a.encapsulate(&inner, &mut buf).unwrap() {
                EngineOut::Network { len } => {
                    let mut cipher = [0u8; 2048];
                    cipher[..len].copy_from_slice(&buf[..len]);
                    match b.decapsulate(&cipher[..len], &mut buf) {
                        Ok(EngineOut::InnerV6 { len }) => {
                            got = Some(buf[..len].to_vec());
                            break;
                        }
                        Ok(EngineOut::Network { len }) => {
                            let mut r = [0u8; 2048];
                            r[..len].copy_from_slice(&buf[..len]);
                            let _ = a.decapsulate(&r[..len], &mut buf);
                        }
                        _ => {}
                    }
                }
                EngineOut::Done => {}
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(got.as_deref(), Some(&inner[..]));
    }
}
