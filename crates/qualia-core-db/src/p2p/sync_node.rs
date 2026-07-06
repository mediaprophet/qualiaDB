//! **libp2p sync node** (T3.1) — a standalone, noise-encrypted request-response node that drives the
//! [`super::sync_ops`] op-transfer protocol between peers. This is the swarm behind the sync transport:
//! it serves a local [`SyncOpRelay`] to inbound peers and exposes async `publish`/`pull` to a peer.
//!
//! Standalone by design — it does **not** touch the daemon's swarm (whose `QualiaRequest` enum is matched
//! exhaustively). The blocking `SyncTransport` adapter (client-core) is a thin `block_on` wrapper over the
//! async methods here; kept separate so libp2p stays out of the wasm-facing client crate.
//!
//! A dumb pipe: it moves opaque, already-signed operation frames and validates nothing — all trust stays
//! in the consuming node's fail-closed inbox, exactly as the in-memory and HTTP relay transports.

#![cfg(not(target_arch = "wasm32"))]

use crate::p2p::sync_ops::{SyncOpCodec, SyncOpRelay, SyncOpRequest, SyncOpResponse, SYNC_OP_PROTOCOL};
use libp2p::futures::StreamExt;
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

#[derive(NetworkBehaviour)]
struct SyncBehaviour {
    rr: request_response::Behaviour<SyncOpCodec>,
}

enum Cmd {
    Listen(Multiaddr, oneshot::Sender<Result<Multiaddr, String>>),
    AddPeer(PeerId, Multiaddr),
    Publish(PeerId, Vec<Vec<u8>>, oneshot::Sender<Result<u64, String>>),
    Pull(PeerId, u64, oneshot::Sender<Result<(Vec<Vec<u8>>, u64), String>>),
}

enum Pending {
    Publish(oneshot::Sender<Result<u64, String>>),
    Pull(oneshot::Sender<Result<(Vec<Vec<u8>>, u64), String>>),
}

/// A running libp2p sync node. Spawn it on a tokio runtime; drive it with the async methods.
pub struct Libp2pSyncNode {
    pub peer_id: PeerId,
    cmd_tx: mpsc::UnboundedSender<Cmd>,
}

impl Libp2pSyncNode {
    /// Spawn a node on the current tokio runtime, serving `relay` to inbound peers.
    pub fn spawn(relay: SyncOpRelay) -> Self {
        let key = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(key.public());
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let swarm = libp2p::SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .expect("tcp transport")
            .with_behaviour(|_| SyncBehaviour {
                rr: request_response::Behaviour::new(
                    [(StreamProtocol::new(SYNC_OP_PROTOCOL), ProtocolSupport::Full)],
                    request_response::Config::default(),
                ),
            })
            .expect("behaviour")
            .with_swarm_config(|c| c.with_idle_connection_timeout(std::time::Duration::from_secs(60)))
            .build();

        tokio::spawn(event_loop(swarm, relay, cmd_rx));
        Self { peer_id, cmd_tx }
    }

    async fn call<R>(&self, make: impl FnOnce(oneshot::Sender<R>) -> Cmd) -> Result<R, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(make(tx)).map_err(|_| "sync node stopped".to_string())?;
        rx.await.map_err(|_| "sync node dropped the response".to_string())
    }

    /// Listen on `addr` (e.g. `/ip4/127.0.0.1/tcp/0`), returning the actual bound multiaddr.
    pub async fn listen(&self, addr: &str) -> Result<Multiaddr, String> {
        let a: Multiaddr = addr.parse().map_err(|e| format!("bad multiaddr: {e}"))?;
        self.call(|tx| Cmd::Listen(a, tx)).await.and_then(|r| r)
    }

    /// Teach this node a peer's address so `publish`/`pull` can reach (and auto-dial) it.
    pub fn add_peer(&self, peer: PeerId, addr: Multiaddr) {
        let _ = self.cmd_tx.send(Cmd::AddPeer(peer, addr));
    }

    /// Publish opaque signed-op frames to `peer`; returns how many were newly accepted.
    pub async fn publish(&self, peer: PeerId, frames: Vec<Vec<u8>>) -> Result<u64, String> {
        self.call(|tx| Cmd::Publish(peer, frames, tx)).await.and_then(|r| r)
    }

    /// Pull `peer`'s frames after `since`; returns them plus the next cursor.
    pub async fn pull(&self, peer: PeerId, since: u64) -> Result<(Vec<Vec<u8>>, u64), String> {
        self.call(|tx| Cmd::Pull(peer, since, tx)).await.and_then(|r| r)
    }
}

async fn event_loop(
    mut swarm: Swarm<SyncBehaviour>,
    relay: SyncOpRelay,
    mut cmd_rx: mpsc::UnboundedReceiver<Cmd>,
) {
    let mut pending: HashMap<OutboundRequestId, Pending> = HashMap::new();
    let mut pending_listen: Option<oneshot::Sender<Result<Multiaddr, String>>> = None;

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => match cmd {
                None => break, // all handles dropped
                Some(Cmd::Listen(addr, tx)) => match swarm.listen_on(addr) {
                    Ok(_) => pending_listen = Some(tx),
                    Err(e) => { let _ = tx.send(Err(e.to_string())); }
                },
                Some(Cmd::AddPeer(peer, addr)) => {
                    // `Swarm::add_peer_address` replaces the deprecated request-response
                    // `Behaviour::add_address` (libp2p ≥ 0.54): the address book now lives on the
                    // swarm, shared across behaviours, not per-behaviour.
                    swarm.add_peer_address(peer, addr);
                }
                Some(Cmd::Publish(peer, frames, tx)) => {
                    let id = swarm.behaviour_mut().rr.send_request(&peer, SyncOpRequest::Publish { op_frames: frames });
                    pending.insert(id, Pending::Publish(tx));
                }
                Some(Cmd::Pull(peer, since, tx)) => {
                    let id = swarm.behaviour_mut().rr.send_request(&peer, SyncOpRequest::PullSince { cursor: since });
                    pending.insert(id, Pending::Pull(tx));
                }
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    if let Some(tx) = pending_listen.take() { let _ = tx.send(Ok(address)); }
                }
                SwarmEvent::Behaviour(SyncBehaviourEvent::Rr(request_response::Event::Message { message, .. })) => match message {
                    // Inbound: serve our relay (dumb pipe — no validation here).
                    request_response::Message::Request { request, channel, .. } => {
                        let resp = relay.handle(request);
                        let _ = swarm.behaviour_mut().rr.send_response(channel, resp);
                    }
                    // Outbound response: resolve the waiting caller.
                    request_response::Message::Response { request_id, response } => {
                        match (pending.remove(&request_id), response) {
                            (Some(Pending::Publish(tx)), SyncOpResponse::Published { accepted }) => { let _ = tx.send(Ok(accepted)); }
                            (Some(Pending::Pull(tx)), SyncOpResponse::Pulled { op_frames, next_cursor }) => { let _ = tx.send(Ok((op_frames, next_cursor))); }
                            (Some(Pending::Publish(tx)), _) => { let _ = tx.send(Err("unexpected response to publish".into())); }
                            (Some(Pending::Pull(tx)), _) => { let _ = tx.send(Err("unexpected response to pull".into())); }
                            (None, _) => {}
                        }
                    }
                },
                SwarmEvent::Behaviour(SyncBehaviourEvent::Rr(request_response::Event::OutboundFailure { request_id, error, .. })) => {
                    if let Some(p) = pending.remove(&request_id) {
                        let msg = format!("libp2p outbound failure: {error}");
                        match p {
                            Pending::Publish(tx) => { let _ = tx.send(Err(msg)); }
                            Pending::Pull(tx) => { let _ = tx.send(Err(msg)); }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// A **blocking** libp2p sync client — the thin `block_on` wrapper the module docs promise. It owns a
/// tokio runtime and a [`Libp2pSyncNode`], pins a single target peer, and exposes blocking
/// `publish_frames`/`pull_frames` over opaque op frames. Keeping the runtime + libp2p types here lets
/// the client-core `SyncTransport` adapter stay a plain synchronous caller (and keeps libp2p out of the
/// wasm-facing crate). A dumb pipe like the async node beneath it: it moves already-signed frames and
/// validates nothing — all trust stays in the consuming node's fail-closed inbox.
pub struct BlockingSyncClient {
    // Field order matters for drop: the node (and its `cmd_tx`) drops before the runtime, so the event
    // loop sees its command channel close and exits cleanly before the runtime is torn down.
    node: Libp2pSyncNode,
    peer: PeerId,
    rt: tokio::runtime::Runtime,
}

impl BlockingSyncClient {
    /// Connect to a relay/peer: spawn a node on a dedicated multi-threaded runtime (whose worker drives
    /// the event loop continuously), register the peer's address — the first `publish`/`pull` auto-dials
    /// it over noise-encrypted TCP — and serve `relay` to inbound peers. `peer_id` is the base58 peer id;
    /// `peer_addr` is a libp2p multiaddr (e.g. `/ip4/127.0.0.1/tcp/4001`).
    pub fn connect(relay: SyncOpRelay, peer_id: &str, peer_addr: &str) -> Result<Self, String> {
        let peer: PeerId = peer_id.parse().map_err(|e| format!("bad peer id: {e}"))?;
        let addr: Multiaddr = peer_addr.parse().map_err(|e| format!("bad multiaddr: {e}"))?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| format!("sync runtime: {e}"))?;
        // Spawn inside the runtime context so the event-loop task is scheduled on the runtime's worker
        // (which drives it in the background, independent of `block_on`).
        let node = {
            let _guard = rt.enter();
            Libp2pSyncNode::spawn(relay)
        };
        node.add_peer(peer, addr);
        Ok(Self { node, peer, rt })
    }

    /// This node's own peer id (so a peer can be told how to dial us back).
    pub fn local_peer_id(&self) -> PeerId {
        self.node.peer_id
    }

    /// Publish opaque signed-op frames to the pinned peer's relay; returns how many were newly accepted.
    pub fn publish_frames(&self, frames: Vec<Vec<u8>>) -> Result<u64, String> {
        self.rt.block_on(self.node.publish(self.peer, frames))
    }

    /// Pull the pinned peer's frames after `since`; returns them plus the next cursor.
    pub fn pull_frames(&self, since: u64) -> Result<(Vec<Vec<u8>>, u64), String> {
        self.rt.block_on(self.node.pull(self.peer, since))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two libp2p nodes exchange operations end-to-end over a real (noise-encrypted, localhost-TCP)
    /// connection: B publishes to A, then pulls them back — the p2p sync transport, proven live.
    #[test]
    fn two_nodes_exchange_ops_over_libp2p() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // Node A — the responder, holds the shared relay we can inspect.
            let relay_a = SyncOpRelay::new();
            let a = Libp2pSyncNode::spawn(relay_a.clone());
            let a_addr = a.listen("/ip4/127.0.0.1/tcp/0").await.expect("A listen");

            // Node B — dials A and exchanges ops.
            let b = Libp2pSyncNode::spawn(SyncOpRelay::new());
            b.add_peer(a.peer_id, a_addr);

            let accepted = b
                .publish(a.peer_id, vec![b"op-1".to_vec(), b"op-2".to_vec()])
                .await
                .expect("publish");
            assert_eq!(accepted, 2, "both ops newly accepted by A");
            assert_eq!(relay_a.len(), 2, "A's relay holds the published ops");

            let (frames, cursor) = b.pull(a.peer_id, 0).await.expect("pull");
            assert_eq!(frames, vec![b"op-1".to_vec(), b"op-2".to_vec()]);
            assert_eq!(cursor, 2);
        });
    }

    /// The **blocking** client (the wrapper the client-core transport drives) round-trips frames against
    /// a listening responder node — no `.await` at the call site, its own runtime driving the event loop.
    #[test]
    fn blocking_client_round_trips_against_a_listening_node() {
        // Responder A: listens and serves its relay on its own runtime; keep `a` + `rt_a` alive so its
        // command channel (and thus its event loop) stays open for the whole test.
        let rt_a = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let relay_a = SyncOpRelay::new();
        let a = {
            let _guard = rt_a.enter();
            Libp2pSyncNode::spawn(relay_a.clone())
        };
        let a_addr = rt_a.block_on(a.listen("/ip4/127.0.0.1/tcp/0")).expect("A listen");

        // B: the blocking client, constructed from A's string peer id + multiaddr (exercises parsing).
        let b = BlockingSyncClient::connect(SyncOpRelay::new(), &a.peer_id.to_string(), &a_addr.to_string())
            .expect("connect");

        let accepted = b
            .publish_frames(vec![b"op-1".to_vec(), b"op-2".to_vec()])
            .expect("publish");
        assert_eq!(accepted, 2, "both frames newly accepted by A");
        assert_eq!(relay_a.len(), 2, "A's relay holds the published frames");

        let (frames, cursor) = b.pull_frames(0).expect("pull");
        assert_eq!(frames, vec![b"op-1".to_vec(), b"op-2".to_vec()]);
        assert_eq!(cursor, 2);
    }
}
