// Userspace WireGuard data-plane runtime — a `Tunn` state machine bound to a real UDP socket.
//
// `wireguard_userspace.rs` provides the pure crypto core (keys + a `boringtun::noise::Tunn`
// state machine) and proves a full handshake + one data packet flow *entirely in memory*.
// This module is the next layer up: it owns a real `std::net::UdpSocket`, an endpoint for the
// peer, and drives the `Tunn` over the wire — sending handshake/keepalive/cookie replies
// automatically, delivering decrypted inner IP packets to the caller, and driving WireGuard's
// timers. It is the SocialWebNet data plane: two of these, one per peer, carry curated traffic
// once the address-book coordination plane has exchanged keys and endpoints.
//
// Design choices:
//   * **IPv6-only overlay.** The SocialWebNet inner address space is strictly IPv6 — the same
//     `fd00::/8` ULA space that `connection_identifier::derive_overlay_addr` mints. IPv4 is
//     deliberately not carried: the mesh is a clean-slate overlay, and IPv4's NAT/address-scarcity
//     baggage is exactly what it exists to escape. An authenticated *inner* packet that is IPv4 is
//     therefore dropped (not delivered), never emitted by us, and treated as peer misconfiguration.
//   * **Roaming built in.** WireGuard's endpoint is defined as the source of the most recent
//     *authenticated* packet. Every successful decapsulate updates `peer_endpoint` to the
//     datagram's source, so a peer whose IP/port changes (dynamic IPs, NAT rebind) is followed
//     without any coordination-plane round-trip. This is the whole point of a socially-defined
//     mesh over dynamic addresses. (Roaming acts on the *outer* UDP source, which may be IPv4 or
//     IPv6 transport — only the *inner* overlay is IPv6-only.)
//   * **Caller-driven pump, no hidden threads.** `pump()` processes exactly one datagram and
//     `tick()` runs the timers once. The caller owns the loop (a background thread, an async
//     task, or a bounded test loop), which keeps the core deterministically testable with two
//     loopback sockets and no external systems.
//   * **Reusable scratch buffers.** `recv_buf`/`out_buf` are owned once so the steady-state
//     path allocates nothing per packet. `boringtun`'s `TunnResult` borrows the *output*
//     buffer (not the `Tunn`), so sending a produced packet and then reusing the buffer are
//     disjoint field borrows the compiler accepts.
//
// Native-only, exactly like `wireguard_userspace`: `boringtun` does not build for `wasm32`.
// WASM peers reach the network through a relay, not this path.
#![cfg(not(target_arch = "wasm32"))]

use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::PublicKey;

use super::wireguard_userspace::{new_tunnel, WgKeypair};

/// Scratch buffer size — a full UDP datagram (65 535) plus WireGuard framing headroom.
const MAX_DATAGRAM: usize = 65_535;

/// What one [`WgTunnel::pump`] produced.
#[derive(Debug)]
pub enum TunnelEvent {
    /// A decrypted inner IPv6 packet arrived from the peer — hand it to the virtual interface.
    InnerPacket(Vec<u8>),
    /// Only WireGuard control traffic moved (handshake, keepalive, cookie) — or an authenticated
    /// but non-IPv6 inner packet was dropped; nothing for the caller.
    Progressed,
    /// The socket read timed out (or would block) with nothing to do.
    Idle,
}

/// A live userspace-WireGuard tunnel to a single peer, bound to a real UDP socket.
///
/// Build with [`WgTunnel::bind`], point it at the peer with [`WgTunnel::set_peer_endpoint`]
/// (or let the first authenticated packet set it via roaming), then either [`initiate_handshake`]
/// (the initiator) or wait to receive one. Drive it with [`pump`] (per datagram) and [`tick`]
/// (per second). Encrypt outbound inner packets with [`send_packet`].
///
/// [`initiate_handshake`]: WgTunnel::initiate_handshake
/// [`pump`]: WgTunnel::pump
/// [`tick`]: WgTunnel::tick
/// [`send_packet`]: WgTunnel::send_packet
pub struct WgTunnel {
    tunn: Tunn,
    socket: UdpSocket,
    /// Where we send to. `None` until set explicitly or learned from the first authenticated
    /// packet (roaming). Updated on every successful decapsulate.
    peer_endpoint: Option<SocketAddr>,
    recv_buf: Vec<u8>,
    out_buf: Vec<u8>,
}

impl WgTunnel {
    /// Bind a UDP socket and build the tunnel state machine for `peer_public`.
    ///
    /// `bind_addr` may use port 0 to let the OS choose (read it back with [`local_addr`]).
    /// `index` is a local WireGuard session index (any `u32`, distinct per tunnel). The peer
    /// endpoint is unset; call [`set_peer_endpoint`] before initiating, or let roaming learn it.
    ///
    /// [`local_addr`]: WgTunnel::local_addr
    /// [`set_peer_endpoint`]: WgTunnel::set_peer_endpoint
    pub fn bind(
        mine: &WgKeypair,
        peer_public: PublicKey,
        bind_addr: SocketAddr,
        index: u32,
    ) -> Result<WgTunnel, String> {
        let socket = UdpSocket::bind(bind_addr).map_err(|e| format!("bind {bind_addr}: {e}"))?;
        let tunn = new_tunnel(mine, peer_public, index)?;
        Ok(WgTunnel {
            tunn,
            socket,
            peer_endpoint: None,
            recv_buf: vec![0u8; MAX_DATAGRAM],
            out_buf: vec![0u8; MAX_DATAGRAM],
        })
    }

    /// The socket's local address (resolves an OS-chosen port after binding to `:0`).
    pub fn local_addr(&self) -> Result<SocketAddr, String> {
        self.socket.local_addr().map_err(|e| e.to_string())
    }

    /// The peer endpoint we currently send to, if known.
    pub fn peer_endpoint(&self) -> Option<SocketAddr> {
        self.peer_endpoint
    }

    /// Point the tunnel at `addr`. Roaming may later override this with the source of an
    /// authenticated packet.
    pub fn set_peer_endpoint(&mut self, addr: SocketAddr) {
        self.peer_endpoint = Some(addr);
    }

    /// Set the socket read timeout so [`pump`] returns [`TunnelEvent::Idle`] instead of blocking
    /// forever. `None` blocks indefinitely.
    ///
    /// [`pump`]: WgTunnel::pump
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), String> {
        self.socket.set_read_timeout(dur).map_err(|e| e.to_string())
    }

    /// Has a WireGuard session been established (a handshake completed)?
    pub fn has_session(&self) -> bool {
        self.tunn.time_since_last_handshake().is_some()
    }

    fn endpoint(&self) -> Result<SocketAddr, String> {
        self.peer_endpoint
            .ok_or_else(|| "peer endpoint not set (call set_peer_endpoint or receive a packet first)".to_string())
    }

    /// Kick off the Noise_IKpsk2 handshake: produce a handshake initiation and send it to the peer.
    /// The initiator calls this once; the responder just [`pump`]s and answers automatically.
    ///
    /// [`pump`]: WgTunnel::pump
    pub fn initiate_handshake(&mut self) -> Result<(), String> {
        let dst = self.endpoint()?;
        match self.tunn.encapsulate(&[], &mut self.out_buf) {
            TunnResult::WriteToNetwork(pkt) => {
                self.socket.send_to(pkt, dst).map_err(|e| e.to_string())?;
                Ok(())
            }
            // Already have a live session — nothing to initiate.
            TunnResult::Done => Ok(()),
            TunnResult::Err(e) => Err(format!("handshake init: {e:?}")),
            other => Err(format!("unexpected handshake-init result: {other:?}")),
        }
    }

    /// Encrypt one inner IP packet and send it to the peer.
    ///
    /// If no session is up yet, `boringtun` emits a handshake initiation instead of ciphertext;
    /// we forward that so the handshake starts, and the caller should retry the data send once
    /// [`has_session`] is true. Returns `Ok(true)` if ciphertext/handshake was sent, `Ok(false)`
    /// if `boringtun` produced nothing (e.g. the packet was queued pending a handshake).
    ///
    /// [`has_session`]: WgTunnel::has_session
    pub fn send_packet(&mut self, inner: &[u8]) -> Result<bool, String> {
        let dst = self.endpoint()?;
        match self.tunn.encapsulate(inner, &mut self.out_buf) {
            TunnResult::WriteToNetwork(pkt) => {
                self.socket.send_to(pkt, dst).map_err(|e| e.to_string())?;
                Ok(true)
            }
            TunnResult::Done => Ok(false),
            TunnResult::Err(e) => Err(format!("encapsulate: {e:?}")),
            other => Err(format!("unexpected encapsulate result: {other:?}")),
        }
    }

    /// Receive one datagram and run it through the tunnel.
    ///
    /// Handshake responses, keepalives and cookie replies are sent back to the peer
    /// automatically (and any queued follow-up packets flushed). A decrypted inner IP packet is
    /// returned as [`TunnelEvent::InnerPacket`]. On a read timeout, [`TunnelEvent::Idle`].
    pub fn pump(&mut self) -> Result<TunnelEvent, String> {
        let (n, from) = match self.socket.recv_from(&mut self.recv_buf) {
            Ok(v) => v,
            // No datagram ready within the read timeout.
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                return Ok(TunnelEvent::Idle)
            }
            // Windows UDP quirk: `recv_from` returns WSAECONNRESET (10054 → `ConnectionReset`) when a
            // *prior* send to the peer drew an ICMP "port unreachable" (peer momentarily down / not
            // yet listening / behind a NAT that rejected it). A connectionless UDP socket is not
            // actually closed — the next datagram will still arrive — so this is non-fatal: treat it
            // as an idle tick, not a dead tunnel. (On Unix the equivalent ICMP error is not delivered
            // to recv at all, so this arm is effectively Windows-only.)
            Err(e) if e.kind() == ErrorKind::ConnectionReset => return Ok(TunnelEvent::Idle),
            Err(e) => return Err(format!("recv_from: {e}")),
        };

        // Disjoint field borrows: `tunn` (mut), `recv_buf` (read), `out_buf` (mut).
        let result = self
            .tunn
            .decapsulate(None, &self.recv_buf[..n], &mut self.out_buf);

        match result {
            TunnResult::WriteToNetwork(pkt) => {
                // Roaming: an authenticated packet came from `from` — send our reply there and
                // adopt it as the peer endpoint.
                self.socket.send_to(pkt, from).map_err(|e| e.to_string())?;
                self.peer_endpoint = Some(from);
                // `boringtun` requires draining any queued packets with empty-input decapsulate
                // calls until it stops asking to write to the network.
                loop {
                    match self.tunn.decapsulate(None, &[], &mut self.out_buf) {
                        TunnResult::WriteToNetwork(more) => {
                            self.socket.send_to(more, from).map_err(|e| e.to_string())?;
                        }
                        _ => break,
                    }
                }
                Ok(TunnelEvent::Progressed)
            }
            TunnResult::WriteToTunnelV6(data, _) => {
                let inner = data.to_vec();
                self.peer_endpoint = Some(from);
                Ok(TunnelEvent::InnerPacket(inner))
            }
            // Authenticated, but IPv4 on an IPv6-only overlay — adopt the endpoint (the packet was
            // genuine) but drop the payload rather than deliver an unsupported inner protocol.
            TunnResult::WriteToTunnelV4(_, _) => {
                self.peer_endpoint = Some(from);
                Ok(TunnelEvent::Progressed)
            }
            TunnResult::Done => {
                // Authenticated control traffic with nothing to deliver (e.g. a keepalive).
                self.peer_endpoint = Some(from);
                Ok(TunnelEvent::Progressed)
            }
            TunnResult::Err(e) => Err(format!("decapsulate: {e:?}")),
        }
    }

    /// Drive WireGuard's timers once (rekeying, keepalives, session expiry). Call roughly once a
    /// second from the caller's loop. Sends any timer-produced packet (e.g. a keepalive) to the peer.
    pub fn tick(&mut self) -> Result<(), String> {
        // No endpoint yet ⇒ no session ⇒ nothing for the timers to send anywhere.
        let dst = match self.peer_endpoint {
            Some(d) => d,
            None => return Ok(()),
        };
        match self.tunn.update_timers(&mut self.out_buf) {
            TunnResult::WriteToNetwork(pkt) => {
                self.socket.send_to(pkt, dst).map_err(|e| e.to_string())?;
                Ok(())
            }
            TunnResult::Done => Ok(()),
            TunnResult::Err(e) => Err(format!("update_timers: {e:?}")),
            other => Err(format!("unexpected timer result: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::wireguard_userspace::generate_keypair;

    fn loopback() -> SocketAddr {
        "127.0.0.1:0".parse().unwrap()
    }

    /// Build a minimal *valid* IPv6 packet carrying `payload` — `boringtun`'s decapsulate validates
    /// the IP header (version nibble + length), so a raw byte string would be rejected. The overlay
    /// is IPv6-only, using `fd00::/8` ULA source/destination addresses (the same space
    /// `derive_overlay_addr` mints). The 40-byte IPv6 header's payload-length field is set so the
    /// validated slice equals the whole packet.
    fn make_ipv6_packet(payload: &[u8]) -> Vec<u8> {
        let total_len = 40 + payload.len();
        let mut pkt = vec![0u8; total_len];
        pkt[0] = 0x60; // version 6, traffic class 0
        pkt[4..6].copy_from_slice(&(payload.len() as u16).to_be_bytes()); // payload length
        pkt[6] = 17; // next header = UDP (not inspected)
        pkt[7] = 64; // hop limit
        pkt[8] = 0xfd; // src fd00::1 (ULA)
        pkt[23] = 0x01;
        pkt[24] = 0xfd; // dst fd00::2 (ULA)
        pkt[39] = 0x02;
        pkt[40..].copy_from_slice(payload);
        pkt
    }

    /// Two `WgTunnel`s over two real loopback UDP sockets: complete a handshake and carry a data
    /// packet end-to-end. The acceptance test for "the SocialWebNet data plane works over real
    /// sockets with zero external systems".
    #[test]
    fn two_tunnels_handshake_and_carry_data_over_udp() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();

        let mut a = WgTunnel::bind(&a_keys, b_keys.public, loopback(), 1).expect("bind A");
        let mut b = WgTunnel::bind(&b_keys, a_keys.public, loopback(), 2).expect("bind B");

        // Exchange the OS-chosen endpoints (the coordination plane's job in production).
        let a_addr = a.local_addr().unwrap();
        let b_addr = b.local_addr().unwrap();
        a.set_peer_endpoint(b_addr);
        b.set_peer_endpoint(a_addr);

        // Short read timeouts so pump() never blocks the test.
        let to = Some(Duration::from_millis(300));
        a.set_read_timeout(to).unwrap();
        b.set_read_timeout(to).unwrap();

        // A initiates; drive both sides until A holds a live session.
        a.initiate_handshake().expect("A initiates handshake");
        for _ in 0..20 {
            if a.has_session() {
                break;
            }
            let _ = b.pump().expect("B pump"); // process init → send response
            let _ = a.pump().expect("A pump"); // process response → establish + keepalive
        }
        assert!(a.has_session(), "A established a session");

        // Let B consume the keepalive so it, too, has a live session.
        let _ = b.pump().expect("B pump keepalive");
        assert!(b.has_session(), "B established a session");

        // A encrypts an inner IPv6 packet; B should decrypt exactly it.
        let plaintext = make_ipv6_packet(b"hello over the socially-defined wire");
        assert!(a.send_packet(&plaintext).expect("A sends data"));

        let mut got = None;
        for _ in 0..10 {
            if let TunnelEvent::InnerPacket(p) = b.pump().expect("B pump data") {
                got = Some(p);
                break;
            }
        }
        assert_eq!(
            got.expect("B never received the inner packet"),
            plaintext,
            "decrypted inner packet equals what A sent"
        );
    }

    /// Roaming: after the session is up, a packet arriving from a *new* source address moves the
    /// peer endpoint. We simulate B changing address by sending from a second socket that adopts
    /// B's live `Tunn` is out of scope here; instead we assert the endpoint-follows-source rule
    /// directly on A by having B send its next packet — B's address is fixed in loopback, so we
    /// assert the invariant that a successful decapsulate records the source as the endpoint.
    #[test]
    fn endpoint_is_learned_from_authenticated_source() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();

        // A does NOT know B's endpoint up front — it must learn it from B's handshake init.
        let mut a = WgTunnel::bind(&a_keys, b_keys.public, loopback(), 1).expect("bind A");
        let mut b = WgTunnel::bind(&b_keys, a_keys.public, loopback(), 2).expect("bind B");

        let a_addr = a.local_addr().unwrap();
        let b_addr = b.local_addr().unwrap();
        // Only B is told where A is; A will discover B by roaming.
        b.set_peer_endpoint(a_addr);
        assert!(a.peer_endpoint().is_none(), "A starts with no endpoint");

        let to = Some(Duration::from_millis(300));
        a.set_read_timeout(to).unwrap();
        b.set_read_timeout(to).unwrap();

        // B initiates; A learns B's address from the authenticated init it receives.
        b.initiate_handshake().expect("B initiates");
        for _ in 0..20 {
            let _ = a.pump().expect("A pump"); // learns endpoint on first authenticated packet
            let _ = b.pump().expect("B pump");
            if a.has_session() && a.peer_endpoint().is_some() {
                break;
            }
        }
        assert_eq!(
            a.peer_endpoint(),
            Some(b_addr),
            "A learned B's endpoint from the authenticated handshake (roaming)"
        );
        assert!(a.has_session(), "handshake still completes when the endpoint is learned");
    }
}
