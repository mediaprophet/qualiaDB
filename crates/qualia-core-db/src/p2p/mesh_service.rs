// MeshService — a running SocialWebNet: a background thread that owns the mesh and drives it.
//
// `SocialWebNet` is a passive, caller-driven state machine: something has to call `pump_all`,
// `tick_all` and `send_to` in a loop. `MeshService` is that loop. It moves the mesh onto a
// dedicated thread and exposes a small, thread-safe control surface over channels:
//
//     caller ── command (add/endpoint/initiate/send/query) ──▶  mesh thread ── owns SocialWebNet
//     caller ◀── inbound MeshPacket (decrypted inner IPv6) ───  mesh thread
//
// The thread's loop each turn: drain pending commands, `pump_all` (which paces itself on the
// per-peer socket read timeout), forward any decrypted inner packets to the inbound channel, and
// `tick_all` about once a second. It exits when the handle is dropped (or `shutdown` is called).
//
// Socket readiness is polled via the sockets' own read timeouts rather than an `mio`/`epoll`
// reactor; with a handful of peers this is simple and correct. A readiness-driven reactor (one
// shared poll over all peer sockets) is the efficiency refinement for large meshes and is noted as
// future work — it does not change behaviour.
//
// Native-only (`boringtun`/sockets); WASM peers use a relay.
#![cfg(not(target_arch = "wasm32"))]

use std::net::SocketAddr;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::social_webnet::{MeshPacket, SocialWebNet};

/// How often the mesh thread runs WireGuard's timers.
const TICK_INTERVAL: Duration = Duration::from_millis(1000);
/// Idle sleep when the mesh has no peers (nothing to pump), to avoid a busy loop.
const IDLE_SLEEP: Duration = Duration::from_millis(5);

/// A control message to the mesh thread. Request/response commands carry a one-shot reply sender.
enum Command {
    AddPeer {
        peer_id: String,
        peer_pubkey_hex: String,
        endpoint: Option<SocketAddr>,
        reply: Sender<Result<SocketAddr, String>>,
    },
    SetEndpoint {
        peer_id: String,
        addr: SocketAddr,
        reply: Sender<Result<(), String>>,
    },
    InitiateHandshake {
        peer_id: String,
        reply: Sender<Result<(), String>>,
    },
    Send {
        peer_id: String,
        inner: Vec<u8>,
        reply: Sender<Result<bool, String>>,
    },
    RemovePeer {
        peer_id: String,
        reply: Sender<bool>,
    },
    Peers {
        reply: Sender<Vec<String>>,
    },
    HasSession {
        peer_id: String,
        reply: Sender<bool>,
    },
    Shutdown,
}

/// A cloneable **control handle** to a running mesh — everything that talks to the mesh thread over
/// the command channel (peers, endpoints, handshakes, send, status), but *not* the inbound packet
/// stream. Because it carries no receiver it is freely `Clone`/`Send`/`Sync`, so several owners can
/// drive one mesh: e.g. a status UI and a chat loop sharing a single set of tunnels. Obtain one from
/// [`MeshService::control`].
#[derive(Clone)]
pub struct MeshControl {
    cmd_tx: Sender<Command>,
}

impl MeshControl {
    fn request<T>(&self, make: impl FnOnce(Sender<T>) -> Command) -> Result<T, String> {
        let (tx, rx) = channel::<T>();
        self.cmd_tx
            .send(make(tx))
            .map_err(|_| "mesh thread is gone".to_string())?;
        rx.recv().map_err(|_| "mesh thread dropped the reply".to_string())
    }

    /// Add a peer; returns the local socket address the tunnel bound.
    pub fn add_peer(
        &self,
        peer_id: &str,
        peer_pubkey_hex: &str,
        endpoint: Option<SocketAddr>,
    ) -> Result<SocketAddr, String> {
        self.request(|reply| Command::AddPeer {
            peer_id: peer_id.to_string(),
            peer_pubkey_hex: peer_pubkey_hex.to_string(),
            endpoint,
            reply,
        })?
    }

    /// Point a peer's tunnel at `addr`.
    pub fn set_peer_endpoint(&self, peer_id: &str, addr: SocketAddr) -> Result<(), String> {
        self.request(|reply| Command::SetEndpoint {
            peer_id: peer_id.to_string(),
            addr,
            reply,
        })?
    }

    /// Start the handshake with a peer (initiator side; endpoint must be set).
    pub fn initiate_handshake(&self, peer_id: &str) -> Result<(), String> {
        self.request(|reply| Command::InitiateHandshake {
            peer_id: peer_id.to_string(),
            reply,
        })?
    }

    /// Encrypt and send one inner IPv6 packet to a peer.
    pub fn send(&self, peer_id: &str, inner: Vec<u8>) -> Result<bool, String> {
        self.request(|reply| Command::Send {
            peer_id: peer_id.to_string(),
            inner,
            reply,
        })?
    }

    /// Remove a peer; returns whether it was present.
    pub fn remove_peer(&self, peer_id: &str) -> Result<bool, String> {
        self.request(|reply| Command::RemovePeer {
            peer_id: peer_id.to_string(),
            reply,
        })
    }

    /// The peer ids currently in the mesh.
    pub fn peers(&self) -> Result<Vec<String>, String> {
        self.request(|reply| Command::Peers { reply })
    }

    /// Whether a live session exists with a peer.
    pub fn has_session(&self, peer_id: &str) -> Result<bool, String> {
        self.request(|reply| Command::HasSession {
            peer_id: peer_id.to_string(),
            reply,
        })
    }

    /// Poll until a session is established with `peer_id`, or `timeout` elapses.
    pub fn wait_for_session(&self, peer_id: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.has_session(peer_id).unwrap_or(false) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// A handle to a running mesh: sole owner of the inbound packet stream and the thread's join handle,
/// plus a [`MeshControl`]. Clone [`control`](MeshService::control) for additional control handles; the
/// inbound receiver stays single-owner (only the mesh's consumer — e.g. the chat loop — drains it).
pub struct MeshService {
    cmd_tx: Sender<Command>,
    inbound_rx: Receiver<MeshPacket>,
    handle: Option<JoinHandle<()>>,
}

impl MeshService {
    /// Spawn the mesh thread, taking ownership of an already-constructed [`SocialWebNet`] (peers may
    /// be pre-added or added later via [`add_peer`](MeshService::add_peer)).
    pub fn spawn(mut mesh: SocialWebNet) -> MeshService {
        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (inbound_tx, inbound_rx) = channel::<MeshPacket>();

        let handle = std::thread::Builder::new()
            .name("socialwebnet-mesh".into())
            .spawn(move || run(&mut mesh, &cmd_rx, &inbound_tx))
            .expect("spawn mesh thread");

        MeshService {
            cmd_tx,
            inbound_rx,
            handle: Some(handle),
        }
    }

    /// A cloneable control handle to this mesh — share it with a status UI, a chat loop, etc. All
    /// control handles and the `MeshService` itself drive the same mesh thread.
    pub fn control(&self) -> MeshControl {
        MeshControl {
            cmd_tx: self.cmd_tx.clone(),
        }
    }

    fn request<T>(&self, make: impl FnOnce(Sender<T>) -> Command) -> Result<T, String> {
        let (tx, rx) = channel::<T>();
        self.cmd_tx
            .send(make(tx))
            .map_err(|_| "mesh thread is gone".to_string())?;
        rx.recv().map_err(|_| "mesh thread dropped the reply".to_string())
    }

    /// Add a peer; returns the local socket address the tunnel bound (advertise it to the peer).
    pub fn add_peer(
        &self,
        peer_id: &str,
        peer_pubkey_hex: &str,
        endpoint: Option<SocketAddr>,
    ) -> Result<SocketAddr, String> {
        self.request(|reply| Command::AddPeer {
            peer_id: peer_id.to_string(),
            peer_pubkey_hex: peer_pubkey_hex.to_string(),
            endpoint,
            reply,
        })?
    }

    /// Point a peer's tunnel at `addr`.
    pub fn set_peer_endpoint(&self, peer_id: &str, addr: SocketAddr) -> Result<(), String> {
        self.request(|reply| Command::SetEndpoint {
            peer_id: peer_id.to_string(),
            addr,
            reply,
        })?
    }

    /// Start the handshake with a peer (initiator side; endpoint must be set).
    pub fn initiate_handshake(&self, peer_id: &str) -> Result<(), String> {
        self.request(|reply| Command::InitiateHandshake {
            peer_id: peer_id.to_string(),
            reply,
        })?
    }

    /// Encrypt and send one inner IPv6 packet to a peer.
    pub fn send(&self, peer_id: &str, inner: Vec<u8>) -> Result<bool, String> {
        self.request(|reply| Command::Send {
            peer_id: peer_id.to_string(),
            inner,
            reply,
        })?
    }

    /// Remove a peer; returns whether it was present.
    pub fn remove_peer(&self, peer_id: &str) -> Result<bool, String> {
        self.request(|reply| Command::RemovePeer {
            peer_id: peer_id.to_string(),
            reply,
        })
    }

    /// The peer ids currently in the mesh.
    pub fn peers(&self) -> Result<Vec<String>, String> {
        self.request(|reply| Command::Peers { reply })
    }

    /// Whether a live session exists with a peer.
    pub fn has_session(&self, peer_id: &str) -> Result<bool, String> {
        self.request(|reply| Command::HasSession {
            peer_id: peer_id.to_string(),
            reply,
        })
    }

    /// Non-blocking receive of the next inbound inner packet, if any.
    pub fn try_recv(&self) -> Option<MeshPacket> {
        self.inbound_rx.try_recv().ok()
    }

    /// Block up to `timeout` for the next inbound inner packet.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<MeshPacket> {
        match self.inbound_rx.recv_timeout(timeout) {
            Ok(pkt) => Some(pkt),
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Poll until a session is established with `peer_id`, or `timeout` elapses. Returns whether the
    /// session came up.
    pub fn wait_for_session(&self, peer_id: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.has_session(peer_id).unwrap_or(false) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Stop the mesh thread and join it. Called automatically on drop.
    pub fn shutdown(&mut self) {
        let _ = self.cmd_tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for MeshService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The mesh thread body: drain commands, pump sockets, forward inbound packets, tick timers.
fn run(mesh: &mut SocialWebNet, cmd_rx: &Receiver<Command>, inbound_tx: &Sender<MeshPacket>) {
    let mut last_tick = Instant::now();
    loop {
        // 1. Drain all pending commands.
        loop {
            match cmd_rx.try_recv() {
                Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => return,
                Ok(cmd) => handle_command(mesh, cmd),
                Err(TryRecvError::Empty) => break,
            }
        }

        // 2. Pump every peer once; forward decrypted inner packets. (pump_all paces on the per-peer
        //    socket read timeout, so this does not busy-spin when peers exist.)
        let had_peers = !mesh.peers().is_empty();
        for evt in mesh.pump_all() {
            if let Ok(pkt) = evt {
                if inbound_tx.send(pkt).is_err() {
                    return; // receiver gone
                }
            }
        }

        // 3. Timers ~1 Hz.
        if last_tick.elapsed() >= TICK_INTERVAL {
            let _ = mesh.tick_all();
            last_tick = Instant::now();
        }

        // 4. Avoid a busy loop when there is nothing to pump.
        if !had_peers {
            std::thread::sleep(IDLE_SLEEP);
        }
    }
}

fn handle_command(mesh: &mut SocialWebNet, cmd: Command) {
    match cmd {
        Command::AddPeer {
            peer_id,
            peer_pubkey_hex,
            endpoint,
            reply,
        } => {
            let _ = reply.send(mesh.add_peer(&peer_id, &peer_pubkey_hex, endpoint));
        }
        Command::SetEndpoint {
            peer_id,
            addr,
            reply,
        } => {
            let _ = reply.send(mesh.set_peer_endpoint(&peer_id, addr));
        }
        Command::InitiateHandshake { peer_id, reply } => {
            let _ = reply.send(mesh.initiate_handshake(&peer_id));
        }
        Command::Send {
            peer_id,
            inner,
            reply,
        } => {
            let _ = reply.send(mesh.send_to(&peer_id, &inner));
        }
        Command::RemovePeer { peer_id, reply } => {
            let _ = reply.send(mesh.remove_peer(&peer_id));
        }
        Command::Peers { reply } => {
            let _ = reply.send(mesh.peers());
        }
        Command::HasSession { peer_id, reply } => {
            let _ = reply.send(mesh.has_session(&peer_id));
        }
        Command::Shutdown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::wireguard_userspace::generate_keypair;

    fn v6(body: &[u8]) -> Vec<u8> {
        let mut p = vec![0u8; 40 + body.len()];
        p[0] = 0x60;
        p[4..6].copy_from_slice(&(body.len() as u16).to_be_bytes());
        p[6] = 17;
        p[7] = 64;
        p[8] = 0xfd;
        p[23] = 0x01;
        p[24] = 0xfd;
        p[39] = 0x02;
        p[40..].copy_from_slice(body);
        p
    }

    /// Two running mesh services (each on its own thread) peer with each other over loopback and
    /// carry an inner IPv6 packet A→B — driven entirely through the async command/inbound channels.
    /// The acceptance test for "the mesh runs as a service".
    #[test]
    fn two_services_peer_and_deliver_over_channels() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let a_pub = a_keys.public_hex();
        let b_pub = b_keys.public_hex();

        let to = Some(Duration::from_millis(50));
        let ip = "127.0.0.1".parse().unwrap();
        let a = MeshService::spawn(SocialWebNet::new(a_keys, ip, to));
        let b = MeshService::spawn(SocialWebNet::new(b_keys, ip, to));

        // Add each other; get the bound endpoints back through the command channel.
        let a_local = a.add_peer("did:wf:bob", &b_pub, None).expect("A adds B");
        let b_local = b.add_peer("did:wf:alice", &a_pub, None).expect("B adds A");
        assert_eq!(a.peers().unwrap(), vec!["did:wf:bob".to_string()]);

        // Exchange endpoints and initiate.
        a.set_peer_endpoint("did:wf:bob", b_local).unwrap();
        b.set_peer_endpoint("did:wf:alice", a_local).unwrap();
        a.initiate_handshake("did:wf:bob").unwrap();

        assert!(
            a.wait_for_session("did:wf:bob", Duration::from_secs(3)),
            "session established via the service threads"
        );

        // Send A→B and receive on B's inbound channel.
        let payload = v6(b"packet over the running mesh service");
        assert!(a.send("did:wf:bob", payload.clone()).unwrap());

        let pkt = b
            .recv_timeout(Duration::from_secs(3))
            .expect("B received the inner packet");
        assert_eq!(pkt.peer_id, "did:wf:alice");
        assert_eq!(pkt.inner, payload);
    }

    #[test]
    fn shutdown_joins_the_thread() {
        let keys = generate_keypair();
        let ip = "127.0.0.1".parse().unwrap();
        let mut svc = MeshService::spawn(SocialWebNet::new(keys, ip, None));
        assert!(svc.peers().unwrap().is_empty());
        svc.shutdown();
        // After shutdown, commands fail cleanly rather than hang.
        assert!(svc.peers().is_err(), "no commands after shutdown");
    }
}
