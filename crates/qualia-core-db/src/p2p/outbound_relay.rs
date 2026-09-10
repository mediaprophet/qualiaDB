//! Outbound packet relay — both peers dial the hub; the hub forwards datagrams.
//!
//! This is the DERP/TURN *shape* for SocialWebNet: neither side listens on a
//! public UDP port. WireGuard `Tunn` packets are opaque payloads. An in-process
//! hub proves the contract. A public WSS/443 deployment is a separate operator
//! step (see `qdnf-imp/nat-traversal-expert-brief.md`).
//!
//! Labelled transition. Not Native Independent. Not an internet handshake.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::{Arc, Mutex};

/// Bounded datagram size (WireGuard handshake/data frames fit).
pub const MAX_DATAGRAM: usize = 2048;
const QUEUE: usize = 16;

#[derive(Clone, Copy)]
struct Slot {
    len: usize,
    bytes: [u8; MAX_DATAGRAM],
}

impl Slot {
    const EMPTY: Self = Self {
        len: 0,
        bytes: [0u8; MAX_DATAGRAM],
    };
}

struct HubState {
    a_to_b: [Slot; QUEUE],
    a_len: usize,
    b_to_a: [Slot; QUEUE],
    b_len: usize,
}

/// One dialed end of an outbound relay pair.
pub struct RelayEndpoint {
    is_a: bool,
    hub: Arc<Mutex<HubState>>,
}

/// Both endpoints have already “dialed out”. There is no listen socket.
pub fn relay_pair() -> (RelayEndpoint, RelayEndpoint) {
    let hub = Arc::new(Mutex::new(HubState {
        a_to_b: [Slot::EMPTY; QUEUE],
        a_len: 0,
        b_to_a: [Slot::EMPTY; QUEUE],
        b_len: 0,
    }));
    (
        RelayEndpoint {
            is_a: true,
            hub: hub.clone(),
        },
        RelayEndpoint {
            is_a: false,
            hub,
        },
    )
}

impl RelayEndpoint {
    /// Queue `datagram` for the peer. Fails closed if full or oversize.
    pub fn send(&mut self, datagram: &[u8]) -> Result<usize, &'static str> {
        if datagram.len() > MAX_DATAGRAM {
            return Err("capacity");
        }
        let mut st = self.hub.lock().map_err(|_| "closed")?;
        let (q, n) = if self.is_a {
            (&mut st.a_to_b, &mut st.a_len)
        } else {
            (&mut st.b_to_a, &mut st.b_len)
        };
        if *n >= QUEUE {
            return Err("would-block");
        }
        q[*n].len = datagram.len();
        q[*n].bytes[..datagram.len()].copy_from_slice(datagram);
        *n += 1;
        Ok(datagram.len())
    }

    /// Pop one datagram. `Err("would-block")` if empty.
    pub fn recv(&mut self, out: &mut [u8]) -> Result<usize, &'static str> {
        let mut st = self.hub.lock().map_err(|_| "closed")?;
        let (q, n) = if self.is_a {
            (&mut st.b_to_a, &mut st.b_len)
        } else {
            (&mut st.a_to_b, &mut st.a_len)
        };
        if *n == 0 {
            return Err("would-block");
        }
        let slot = q[0];
        if slot.len > out.len() {
            return Err("capacity");
        }
        out[..slot.len].copy_from_slice(&slot.bytes[..slot.len]);
        let mut i = 0;
        while i + 1 < *n {
            q[i] = q[i + 1];
            i += 1;
        }
        *n -= 1;
        q[*n] = Slot::EMPTY;
        Ok(slot.len)
    }
}

/// Address-dependent mapping cannot use STUN as a listen locator; relay is required.
pub const fn relay_required_for_address_dependent() -> bool {
    true
}

/// A public internet relay has not been dialed from this build.
pub const fn public_relay_dialed() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::wireguard_userspace::{generate_keypair, new_tunnel};
    use boringtun::noise::TunnResult;

    fn ipv6_hello() -> Vec<u8> {
        let payload = b"relaywg";
        let mut p = vec![0u8; 40 + payload.len()];
        p[0] = 0x60;
        p[4..6].copy_from_slice(&(payload.len() as u16).to_be_bytes());
        p[6] = 17;
        p[7] = 64;
        p[8] = 0xfd;
        p[23] = 1;
        p[24] = 0xfd;
        p[39] = 2;
        p[40..].copy_from_slice(payload);
        p
    }

    fn shuttle(
        from: &mut RelayEndpoint,
        to: &mut RelayEndpoint,
        buf: &[u8],
        out: &mut [u8],
    ) -> usize {
        from.send(buf).expect("relay send");
        to.recv(out).expect("relay recv")
    }

    #[test]
    fn two_outbound_dials_forward_a_datagram() {
        let (mut a, mut b) = relay_pair();
        assert_eq!(a.send(b"ping").unwrap(), 4);
        let mut out = [0u8; 16];
        assert_eq!(b.recv(&mut out).unwrap(), 4);
        assert_eq!(&out[..4], b"ping");
        assert_eq!(a.recv(&mut out), Err("would-block"));
    }

    #[test]
    fn oversize_datagram_fails_closed() {
        let (mut a, _b) = relay_pair();
        let too_big = [0u8; MAX_DATAGRAM + 1];
        assert_eq!(a.send(&too_big), Err("capacity"));
    }

    #[test]
    fn wireguard_handshake_and_data_over_outbound_relay() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let mut ta = new_tunnel(&a_keys, b_keys.public, 1).unwrap();
        let mut tb = new_tunnel(&b_keys, a_keys.public, 2).unwrap();
        let (mut ea, mut eb) = relay_pair();

        let mut scratch = [0u8; MAX_DATAGRAM];
        let mut in_flight = match ta.encapsulate(&[], &mut scratch) {
            TunnResult::WriteToNetwork(p) => p.to_vec(),
            other => panic!("expected handshake init, got {other:?}"),
        };

        let mut to_b = true;
        let mut handshake_done = false;
        for _ in 0..12 {
            let n = if to_b {
                shuttle(&mut ea, &mut eb, &in_flight, &mut scratch)
            } else {
                shuttle(&mut eb, &mut ea, &in_flight, &mut scratch)
            };
            let pkt = scratch[..n].to_vec();
            let recv = if to_b { &mut tb } else { &mut ta };
            match recv.decapsulate(None, &pkt, &mut scratch) {
                TunnResult::WriteToNetwork(p) => {
                    in_flight = p.to_vec();
                    to_b = !to_b;
                }
                TunnResult::Done | TunnResult::WriteToTunnelV6(_, _) => {
                    handshake_done = true;
                    break;
                }
                TunnResult::WriteToTunnelV4(_, _) => panic!("IPv4 inner not used"),
                TunnResult::Err(e) => panic!("{e:?}"),
            }
        }
        assert!(handshake_done, "handshake did not converge over the relay");

        let inner = ipv6_hello();
        let mut got: Option<Vec<u8>> = None;
        for _ in 0..4 {
            let mut enc = [0u8; MAX_DATAGRAM];
            match ta.encapsulate(&inner, &mut enc) {
                TunnResult::WriteToNetwork(p) => {
                    let cipher = p.to_vec();
                    let n = shuttle(&mut ea, &mut eb, &cipher, &mut scratch);
                    let pkt = scratch[..n].to_vec();
                    match tb.decapsulate(None, &pkt, &mut enc) {
                        TunnResult::WriteToTunnelV6(data, _) => {
                            got = Some(data.to_vec());
                            break;
                        }
                        TunnResult::WriteToNetwork(reply) => {
                            let reply = reply.to_vec();
                            let n = shuttle(&mut eb, &mut ea, &reply, &mut scratch);
                            let back = scratch[..n].to_vec();
                            let _ = ta.decapsulate(None, &back, &mut enc);
                        }
                        TunnResult::Done => {}
                        TunnResult::WriteToTunnelV4(_, _) => panic!("IPv4 inner not used"),
                        TunnResult::Err(e) => panic!("{e:?}"),
                    }
                }
                TunnResult::Done => {}
                other => panic!("unexpected A encapsulate: {other:?}"),
            }
        }
        assert_eq!(got.expect("B never received inner IPv6"), inner);
        assert!(!public_relay_dialed());
        assert!(relay_required_for_address_dependent());
    }
}
