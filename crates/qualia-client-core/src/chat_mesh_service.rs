//! Running chat-over-mesh service — drives [`ChatMeshBridge`] over a live [`MeshService`].
//!
//! This is the runtime that makes "chat over the mesh" flow. It owns a [`MeshService`] and a
//! [`ChatMeshBridge`], and on its own thread it
//! * publishes chat envelopes to peers (reliable, on the CHAT port),
//! * drains inbound CHAT-port datagrams, feeds them through the reliable channel, sends ACKs back,
//!   and **either** applies each newly-delivered [`RelayEnvelope`] to the session store **or**
//!   forwards it to the caller (chosen at spawn), and
//! * retransmits unacknowledged frames on a timer.
//!
//! It keeps a cloneable [`MeshControl`] so the desktop can add peers, drive handshakes and read
//! status on the *same* mesh the chat loop is running — no second set of tunnels. The lower layers
//! ([`ChatMeshBridge`], [`crate::mesh_channel`], `mesh_datagram`) are pure and independently tested;
//! this module is the thin I/O loop binding them to the live tunnels.
//!
//! Native-only (the mesh is native-only); WASM chat uses the relay path.
#![cfg(not(target_arch = "wasm32"))]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qualia_core_db::p2p::mesh_datagram::{decode_datagram, encode_datagram, ports};
use qualia_core_db::p2p::mesh_service::{MeshControl, MeshService};

use crate::chat_mesh::ChatMeshBridge;
use crate::chat_relay::RelayEnvelope;

/// How often the loop wakes to service retransmits / drain inbound when idle.
const LOOP_SLEEP: Duration = Duration::from_millis(10);

/// A chat message received from a peer over the mesh: the sending peer's id and the envelope.
pub type IncomingChat = (String, RelayEnvelope);

/// Where the loop delivers newly-received envelopes.
enum Sink {
    /// Forward to the caller's channel (generic / tests).
    Channel(Sender<IncomingChat>),
    /// Apply directly to the session store under this root (the desktop path).
    Apply(PathBuf),
}

enum Cmd {
    Publish {
        peers: Vec<String>,
        env: Box<RelayEnvelope>,
    },
    Shutdown,
}

/// A handle to the running chat-over-mesh service.
pub struct ChatMeshService {
    control: MeshControl,
    cmd_tx: Sender<Cmd>,
    /// Present only in channel mode (`spawn`); `None` in apply mode (`spawn_applying`).
    inbound_rx: Option<Receiver<IncomingChat>>,
    handle: Option<JoinHandle<()>>,
}

impl ChatMeshService {
    /// Spawn the chat loop over `mesh`, **forwarding** received envelopes to [`try_recv`] /
    /// [`recv_timeout`]. Used by tests and callers that want to handle delivery themselves.
    ///
    /// [`try_recv`]: ChatMeshService::try_recv
    /// [`recv_timeout`]: ChatMeshService::recv_timeout
    pub fn spawn(mesh: MeshService) -> ChatMeshService {
        let (inbound_tx, inbound_rx) = channel::<IncomingChat>();
        Self::spawn_with_sink(mesh, Sink::Channel(inbound_tx), Some(inbound_rx))
    }

    /// Spawn the chat loop over `mesh`, **applying** received envelopes directly to the session store
    /// under `storage_root` via [`crate::chat_relay::apply_incoming_envelope`] (dedup + agent-message
    /// validation + UI notify). This is the desktop path: publish with [`publish`], and inbound chat
    /// lands in the local sessions automatically.
    ///
    /// [`publish`]: ChatMeshService::publish
    pub fn spawn_applying(mesh: MeshService, storage_root: PathBuf) -> ChatMeshService {
        Self::spawn_with_sink(mesh, Sink::Apply(storage_root), None)
    }

    fn spawn_with_sink(
        mesh: MeshService,
        sink: Sink,
        inbound_rx: Option<Receiver<IncomingChat>>,
    ) -> ChatMeshService {
        let control = mesh.control();
        let (cmd_tx, cmd_rx) = channel::<Cmd>();
        let handle = std::thread::Builder::new()
            .name("chat-mesh".into())
            .spawn(move || run(mesh, &cmd_rx, sink))
            .expect("spawn chat-mesh thread");
        ChatMeshService {
            control,
            cmd_tx,
            inbound_rx,
            handle: Some(handle),
        }
    }

    // ── Mesh control pass-throughs (same mesh the chat loop drives) ──

    /// Add a peer to the underlying mesh; returns the local bound address.
    pub fn add_peer(
        &self,
        peer_id: &str,
        peer_pubkey_hex: &str,
        endpoint: Option<SocketAddr>,
    ) -> Result<SocketAddr, String> {
        self.control.add_peer(peer_id, peer_pubkey_hex, endpoint)
    }

    /// Point a peer's tunnel at `addr`.
    pub fn set_peer_endpoint(&self, peer_id: &str, addr: SocketAddr) -> Result<(), String> {
        self.control.set_peer_endpoint(peer_id, addr)
    }

    /// Initiate the handshake with a peer.
    pub fn initiate_handshake(&self, peer_id: &str) -> Result<(), String> {
        self.control.initiate_handshake(peer_id)
    }

    /// Peers currently in the mesh.
    pub fn peers(&self) -> Result<Vec<String>, String> {
        self.control.peers()
    }

    /// Whether a live session exists with a peer.
    pub fn has_session(&self, peer_id: &str) -> Result<bool, String> {
        self.control.has_session(peer_id)
    }

    /// Poll until a session with `peer_id` is established or `timeout` elapses.
    pub fn wait_for_session(&self, peer_id: &str, timeout: Duration) -> bool {
        self.control.wait_for_session(peer_id, timeout)
    }

    // ── Chat ──

    /// Reliably send `env` to each peer in `peers` over the mesh.
    pub fn publish(&self, peers: Vec<String>, env: RelayEnvelope) -> Result<(), String> {
        self.cmd_tx
            .send(Cmd::Publish {
                peers,
                env: Box::new(env),
            })
            .map_err(|_| "chat-mesh thread is gone".to_string())
    }

    /// Non-blocking receive of the next inbound chat message (channel mode only; `None` in apply mode).
    pub fn try_recv(&self) -> Option<IncomingChat> {
        self.inbound_rx.as_ref().and_then(|rx| rx.try_recv().ok())
    }

    /// Block up to `timeout` for the next inbound chat message (channel mode only).
    pub fn recv_timeout(&self, timeout: Duration) -> Option<IncomingChat> {
        let rx = self.inbound_rx.as_ref()?;
        match rx.recv_timeout(timeout) {
            Ok(v) => Some(v),
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Stop the chat loop and join the thread (also runs on drop).
    pub fn shutdown(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ChatMeshService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Frame a reliable-channel frame as a CHAT-port datagram and send it to `peer` over the mesh.
fn send_chat_frame(mesh: &MeshService, peer: &str, frame: &[u8]) {
    let dgram = encode_datagram(ports::CHAT, ports::CHAT, frame);
    let _ = mesh.send(peer, dgram);
}

fn deliver(sink: &Sink, peer_id: &str, env: RelayEnvelope) -> bool {
    match sink {
        Sink::Channel(tx) => tx.send((peer_id.to_string(), env)).is_ok(),
        Sink::Apply(root) => {
            // Best-effort: a message for a session we don't have (or our own echo) is simply dropped.
            let _ = crate::chat_relay::apply_incoming_envelope(root, &env.session_id, &env);
            true
        }
    }
}

fn run(mesh: MeshService, cmd_rx: &Receiver<Cmd>, sink: Sink) {
    let mut bridge = ChatMeshBridge::default();
    let start = Instant::now();

    loop {
        let now = start.elapsed().as_millis() as u64;

        // 1. Outbound: publish commands.
        loop {
            match cmd_rx.try_recv() {
                Ok(Cmd::Publish { peers, env }) => {
                    for out in bridge.broadcast(&peers, &env, now) {
                        send_chat_frame(&mesh, &out.peer_did, &out.frame);
                    }
                }
                Ok(Cmd::Shutdown) | Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }

        // 2. Inbound: drain the mesh's decrypted datagrams.
        while let Some(pkt) = mesh.try_recv() {
            let Some(d) = decode_datagram(&pkt.inner) else {
                continue;
            };
            if d.dst_port != ports::CHAT {
                continue; // not ours (presence/QDP/etc.)
            }
            let inb = bridge.on_inbound(&pkt.peer_id, &d.payload, now);
            for ack in inb.acks {
                send_chat_frame(&mesh, &ack.peer_did, &ack.frame);
            }
            if let Some(env) = inb.delivered {
                if !deliver(&sink, &pkt.peer_id, env) {
                    return; // channel receiver gone
                }
            }
        }

        // 3. Retransmit anything unacknowledged whose RTO elapsed.
        for out in bridge.on_tick(now) {
            send_chat_frame(&mesh, &out.peer_did, &out.frame);
        }

        std::thread::sleep(LOOP_SLEEP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::p2p::social_webnet::SocialWebNet;
    use qualia_core_db::p2p::wireguard_userspace::generate_keypair;

    fn envelope(content: &str) -> RelayEnvelope {
        RelayEnvelope {
            session_id: "room-1".into(),
            lamport: 5,
            role: "user".into(),
            content: content.into(),
            author_did: "did:wf:alice".into(),
            author_name: Some("Alice".into()),
            reply_to_fragment: None,
            timestamp: 1_700_000_000,
            signature_hex: "sig".into(),
            sub_agent_of: None,
            agent_did: None,
            model_id: None,
            agent_backend: None,
            outcome_sharing: None,
        }
    }

    /// End-to-end over real loopback sockets: establish two mesh nodes, wrap each in a
    /// `ChatMeshService`, and prove a chat envelope published on Alice's node is delivered to Bob's
    /// inbound channel — the full "chat over the mesh" path through the running runtime.
    #[test]
    fn chat_envelope_flows_node_to_node_over_the_mesh() {
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();
        let (a_pub, b_pub) = (a_keys.public_hex(), b_keys.public_hex());

        let to = Some(Duration::from_millis(50));
        let ip = "127.0.0.1".parse().unwrap();
        let a_mesh = MeshService::spawn(SocialWebNet::new(a_keys, ip, to));
        let b_mesh = MeshService::spawn(SocialWebNet::new(b_keys, ip, to));

        let a_local = a_mesh.add_peer("did:wf:bob", &b_pub, None).unwrap();
        let b_local = b_mesh.add_peer("did:wf:alice", &a_pub, None).unwrap();
        a_mesh.set_peer_endpoint("did:wf:bob", b_local).unwrap();
        b_mesh.set_peer_endpoint("did:wf:alice", a_local).unwrap();
        a_mesh.initiate_handshake("did:wf:bob").unwrap();
        assert!(a_mesh.wait_for_session("did:wf:bob", Duration::from_secs(3)));
        assert!(b_mesh.wait_for_session("did:wf:alice", Duration::from_secs(3)));

        let alice = ChatMeshService::spawn(a_mesh);
        let bob = ChatMeshService::spawn(b_mesh);

        let env = envelope("hello Bob, over the SocialWebNet");
        alice
            .publish(vec!["did:wf:bob".to_string()], env.clone())
            .unwrap();

        let (from, got) = bob
            .recv_timeout(Duration::from_secs(3))
            .expect("Bob received a chat envelope");
        assert_eq!(from, "did:wf:alice");
        assert_eq!(got.content, "hello Bob, over the SocialWebNet");
        assert_eq!(got.session_id, "room-1");
    }
}
