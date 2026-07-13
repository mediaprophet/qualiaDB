// SocialWebNet — the managed userspace-WireGuard mesh keyed by peer identity.
//
// `wireguard_runtime::WgTunnel` is a single point-to-point tunnel. `SocialWebNet` is the layer
// above it: the node's whole set of tunnels, one per peer, keyed by a stable peer id (the
// pairwise DID the coordination plane uses). It is the mechanism the *social* layer drives —
// the address book / connection-identifier exchange decides *who* to peer with and hands this
// mesh the peer's WireGuard public key and (once known) endpoint; this mesh owns the crypto
// state and the sockets.
//
// Separation of concerns:
//   * **This module (core-db)** owns the mesh mechanism: bind sockets, build `Tunn`s, drive
//     handshakes/timers, route inner packets to/from the right peer. It speaks raw keys and
//     endpoints, so it is testable with zero social/identity dependencies (two meshes on
//     loopback, below).
//   * **The client-core social layer** binds this to identity: it feeds `add_peer` from the
//     `social_peers` store + `node_identity`, and learns endpoints from `connection_identifier`
//     rendezvous hints. That wiring lives there, not here.
//
// Socket model: **one UDP socket per peer** (each tunnel binds its own port). This is the
// straightforward, fully-working design — each peer's coordination record advertises that peer's
// port. WireGuard's own kernel implementation instead multiplexes all peers over a single socket,
// demultiplexing by the receiver-index in each datagram; adopting that here is a pure efficiency
// refinement (fewer sockets) and is noted as future work — it does not change correctness.
//
// Native-only (`boringtun` does not build for wasm32); WASM peers use a relay.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use super::wireguard_runtime::{TunnelEvent, WgTunnel};
use super::wireguard_userspace::{public_key_from_hex, WgKeypair};

/// A decrypted inner IPv6 packet, tagged with the peer it came from.
#[derive(Debug)]
pub struct MeshPacket {
    /// The peer id (pairwise DID) the packet arrived from.
    pub peer_id: String,
    /// The decrypted inner IPv6 packet.
    pub inner: Vec<u8>,
}

/// The node's managed WireGuard mesh: a set of peer tunnels sharing this node's static keypair.
pub struct SocialWebNet {
    /// This node's WireGuard static keypair (shared by every tunnel).
    keys: WgKeypair,
    /// The IP to bind each per-peer socket on (port is always OS-chosen).
    bind_ip: IpAddr,
    /// peer id (pairwise DID) → its tunnel.
    tunnels: HashMap<String, WgTunnel>,
    /// Monotonic WireGuard session index handed to each new tunnel (must be distinct per tunnel).
    next_index: u32,
    /// Read timeout applied to each peer socket so pumping never blocks the mesh loop.
    read_timeout: Option<Duration>,
}

impl SocialWebNet {
    /// Create an empty mesh for this node. `bind_ip` is where per-peer sockets bind (e.g.
    /// `0.0.0.0` / `::` in production, `127.0.0.1` in tests); `read_timeout` bounds each
    /// [`pump`](SocialWebNet::pump) so a quiet peer does not stall the loop.
    pub fn new(keys: WgKeypair, bind_ip: IpAddr, read_timeout: Option<Duration>) -> SocialWebNet {
        SocialWebNet {
            keys,
            bind_ip,
            tunnels: HashMap::new(),
            next_index: 1,
            read_timeout,
        }
    }

    /// This node's WireGuard public key as lowercase hex — what peers need to address it.
    pub fn public_key_hex(&self) -> String {
        self.keys.public_hex()
    }

    /// The peer ids currently in the mesh.
    pub fn peers(&self) -> Vec<String> {
        self.tunnels.keys().cloned().collect()
    }

    /// Add a peer to the mesh: bind a fresh socket and build its tunnel.
    ///
    /// `peer_pubkey_hex` is the peer's WireGuard public key (64 hex chars — as stored on a
    /// `SocialPeer` / carried in a `ConnectionIdentifier`). `endpoint` is where to send to if
    /// already known (else `None`, and it is learned by roaming from the first authenticated
    /// packet). Returns the local socket address bound for this peer, so the caller can advertise
    /// it back through the coordination plane. Idempotent-ish: adding an existing peer id replaces
    /// its tunnel (a fresh socket + handshake state).
    pub fn add_peer(
        &mut self,
        peer_id: &str,
        peer_pubkey_hex: &str,
        endpoint: Option<SocketAddr>,
    ) -> Result<SocketAddr, String> {
        let peer_public = public_key_from_hex(peer_pubkey_hex)?;
        let index = self.next_index;
        self.next_index = self.next_index.wrapping_add(1);

        let bind_addr = SocketAddr::new(self.bind_ip, 0);
        let mut tunnel = WgTunnel::bind(&self.keys, peer_public, bind_addr, index)?;
        tunnel.set_read_timeout(self.read_timeout)?;
        if let Some(ep) = endpoint {
            tunnel.set_peer_endpoint(ep);
        }
        let local = tunnel.local_addr()?;
        self.tunnels.insert(peer_id.to_string(), tunnel);
        Ok(local)
    }

    /// Remove a peer and drop its tunnel/socket. Returns whether the peer was present.
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        self.tunnels.remove(peer_id).is_some()
    }

    fn tunnel_mut(&mut self, peer_id: &str) -> Result<&mut WgTunnel, String> {
        self.tunnels
            .get_mut(peer_id)
            .ok_or_else(|| format!("unknown peer '{peer_id}'"))
    }

    /// The local socket address bound for a peer, if present.
    pub fn local_addr(&self, peer_id: &str) -> Option<SocketAddr> {
        self.tunnels.get(peer_id).and_then(|t| t.local_addr().ok())
    }

    /// Point a peer's tunnel at `addr` (e.g. once its endpoint is learned from the coordination plane).
    pub fn set_peer_endpoint(&mut self, peer_id: &str, addr: SocketAddr) -> Result<(), String> {
        self.tunnel_mut(peer_id)?.set_peer_endpoint(addr);
        Ok(())
    }

    /// Whether a live WireGuard session exists with a peer.
    pub fn has_session(&self, peer_id: &str) -> bool {
        self.tunnels.get(peer_id).is_some_and(|t| t.has_session())
    }

    /// Start the handshake with a peer (initiator side). Requires the peer's endpoint to be set.
    pub fn initiate_handshake(&mut self, peer_id: &str) -> Result<(), String> {
        self.tunnel_mut(peer_id)?.initiate_handshake()
    }

    /// Encrypt and send one inner IPv6 packet to a peer. See [`WgTunnel::send_packet`] for the
    /// pre-session behaviour (a handshake init is sent and the caller retries once established).
    pub fn send_to(&mut self, peer_id: &str, inner: &[u8]) -> Result<bool, String> {
        self.tunnel_mut(peer_id)?.send_packet(inner)
    }

    /// Frame `payload` as an overlay IPv6/UDP datagram addressed to `dst_port` (from `src_port`) and
    /// send it to a peer. This is the application-message path over the mesh: the receiver recovers
    /// `(src_port, dst_port, payload)` with
    /// [`mesh_datagram::decode_datagram`](super::mesh_datagram::decode_datagram) on the
    /// [`MeshPacket::inner`] it pumps. See [`super::mesh_datagram::ports`] for well-known ports.
    pub fn send_datagram(
        &mut self,
        peer_id: &str,
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
    ) -> Result<bool, String> {
        let pkt = super::mesh_datagram::encode_datagram(src_port, dst_port, payload);
        self.send_to(peer_id, &pkt)
    }

    /// Pump one datagram for a single peer. See [`WgTunnel::pump`].
    pub fn pump(&mut self, peer_id: &str) -> Result<TunnelEvent, String> {
        self.tunnel_mut(peer_id)?.pump()
    }

    /// Pump every peer once, returning any decrypted inner packets (tagged by peer). Control-only
    /// traffic and idle sockets produce nothing. A per-peer error is surfaced against that peer id
    /// rather than aborting the whole sweep.
    pub fn pump_all(&mut self) -> Vec<Result<MeshPacket, (String, String)>> {
        let ids: Vec<String> = self.tunnels.keys().cloned().collect();
        let mut out = Vec::new();
        for id in ids {
            match self.pump(&id) {
                Ok(TunnelEvent::InnerPacket(inner)) => {
                    out.push(Ok(MeshPacket { peer_id: id, inner }))
                }
                Ok(_) => {}
                Err(e) => out.push(Err((id, e))),
            }
        }
        out
    }

    /// Drive WireGuard timers for every peer once (call ~1 Hz). Per-peer errors are collected, not
    /// fatal.
    pub fn tick_all(&mut self) -> Vec<(String, String)> {
        let ids: Vec<String> = self.tunnels.keys().cloned().collect();
        let mut errs = Vec::new();
        for id in ids {
            if let Ok(t) = self.tunnel_mut(&id) {
                if let Err(e) = t.tick() {
                    errs.push((id, e));
                }
            }
        }
        errs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::wireguard_userspace::generate_keypair;

    fn v6(payload: &[u8]) -> Vec<u8> {
        let total = 40 + payload.len();
        let mut p = vec![0u8; total];
        p[0] = 0x60;
        p[4..6].copy_from_slice(&(payload.len() as u16).to_be_bytes());
        p[6] = 17;
        p[7] = 64;
        p[8] = 0xfd;
        p[23] = 0x01;
        p[24] = 0xfd;
        p[39] = 0x02;
        p[40..].copy_from_slice(payload);
        p
    }

    /// Two full meshes, each holding the other as its single peer, over real loopback sockets:
    /// bring up the tunnel via the mesh API and carry an inner IPv6 packet A→B by peer id.
    #[test]
    fn two_meshes_peer_and_exchange_by_id() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let a_pub = a_keys.public_hex();
        let b_pub = b_keys.public_hex();

        let to = Some(Duration::from_millis(300));
        let mut a = SocialWebNet::new(a_keys, "127.0.0.1".parse().unwrap(), to);
        let mut b = SocialWebNet::new(b_keys, "127.0.0.1".parse().unwrap(), to);

        // Each adds the other by peer id; endpoints unknown until both sockets are bound.
        let a_local = a.add_peer("did:wf:bob", &b_pub, None).expect("A adds B");
        let b_local = b.add_peer("did:wf:alice", &a_pub, None).expect("B adds A");

        // Exchange the freshly-bound endpoints (the coordination plane's job in production).
        a.set_peer_endpoint("did:wf:bob", b_local).unwrap();
        b.set_peer_endpoint("did:wf:alice", a_local).unwrap();

        // A initiates; drive both meshes until A holds a session.
        a.initiate_handshake("did:wf:bob").expect("A initiates");
        for _ in 0..20 {
            if a.has_session("did:wf:bob") {
                break;
            }
            let _ = b.pump_all();
            let _ = a.pump_all();
        }
        assert!(
            a.has_session("did:wf:bob"),
            "A established a session with B"
        );
        let _ = b.pump_all(); // B consumes the keepalive to establish too
        assert!(
            b.has_session("did:wf:alice"),
            "B established a session with A"
        );

        // A sends an inner IPv6 packet addressed to peer id "did:wf:bob".
        let payload = v6(b"mesh packet by peer id");
        assert!(a.send_to("did:wf:bob", &payload).expect("A sends"));

        let mut got = None;
        for _ in 0..10 {
            for evt in b.pump_all() {
                if let Ok(pkt) = evt {
                    assert_eq!(pkt.peer_id, "did:wf:alice", "tagged with the sending peer");
                    got = Some(pkt.inner);
                }
            }
            if got.is_some() {
                break;
            }
        }
        assert_eq!(got.expect("B received the inner packet"), payload);
    }

    /// Two meshes exchange an *application datagram* (framed IPv6/UDP), and the receiver recovers the
    /// ports + payload — proving the app-message layer rides on the tunnel end-to-end.
    #[test]
    fn two_meshes_exchange_an_application_datagram() {
        use super::super::mesh_datagram::{decode_datagram, ports};

        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let (a_pub, b_pub) = (a_keys.public_hex(), b_keys.public_hex());

        let to = Some(Duration::from_millis(300));
        let mut a = SocialWebNet::new(a_keys, "127.0.0.1".parse().unwrap(), to);
        let mut b = SocialWebNet::new(b_keys, "127.0.0.1".parse().unwrap(), to);

        let a_local = a.add_peer("b", &b_pub, None).unwrap();
        let b_local = b.add_peer("a", &a_pub, None).unwrap();
        a.set_peer_endpoint("b", b_local).unwrap();
        b.set_peer_endpoint("a", a_local).unwrap();

        a.initiate_handshake("b").unwrap();
        for _ in 0..20 {
            if a.has_session("b") {
                break;
            }
            let _ = b.pump_all();
            let _ = a.pump_all();
        }
        assert!(a.has_session("b"));
        let _ = b.pump_all();

        // A sends a CHAT datagram to B.
        assert!(a
            .send_datagram("b", ports::CHAT, ports::CHAT, b"hi over the app layer")
            .unwrap());

        let mut got = None;
        for _ in 0..10 {
            for evt in b.pump_all() {
                if let Ok(pkt) = evt {
                    got = decode_datagram(&pkt.inner);
                }
            }
            if got.is_some() {
                break;
            }
        }
        let d = got.expect("B decoded the datagram");
        assert_eq!(d.dst_port, ports::CHAT, "demuxes on the chat port");
        assert_eq!(d.payload, b"hi over the app layer");
    }

    #[test]
    fn add_and_remove_peer_tracks_membership() {
        let keys = generate_keypair();
        let peer = generate_keypair();
        let mut mesh = SocialWebNet::new(keys, "127.0.0.1".parse().unwrap(), None);

        assert!(mesh.peers().is_empty());
        mesh.add_peer("did:wf:x", &peer.public_hex(), None).unwrap();
        assert_eq!(mesh.peers(), vec!["did:wf:x".to_string()]);
        assert!(mesh.local_addr("did:wf:x").is_some());
        assert!(!mesh.has_session("did:wf:x"), "no handshake yet");

        assert!(mesh.remove_peer("did:wf:x"));
        assert!(!mesh.remove_peer("did:wf:x"), "second remove is a no-op");
        assert!(mesh.peers().is_empty());
    }

    #[test]
    fn bad_peer_pubkey_is_rejected() {
        let keys = generate_keypair();
        let mut mesh = SocialWebNet::new(keys, "127.0.0.1".parse().unwrap(), None);
        let err = mesh.add_peer("did:wf:x", "not-hex", None).unwrap_err();
        assert!(err.contains("hex"), "surfaces the key-parse error: {err}");
    }
}
