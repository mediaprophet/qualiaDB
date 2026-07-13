//! Chat over the SocialWebNet mesh — bridge the chat-graph engine to peer tunnels.
//!
//! Chat already has a serializable message ([`RelayEnvelope`](crate::chat_relay::RelayEnvelope)), a
//! signer, and a transport-agnostic apply path
//! ([`apply_incoming_envelope`](crate::chat_relay::apply_incoming_envelope)). Today those ride the
//! HTTP relay; this module routes them over the mesh instead — peer-to-peer, no relay server.
//!
//! It composes two lower layers:
//! * [`crate::mesh_channel::ReliableEndpoint`] — per-peer at-least-once delivery + dedup over the
//!   (lossy) datagram transport.
//! * CBOR-encoded [`RelayEnvelope`] as the CHAT-port payload.
//!
//! Like `mesh_channel`, [`ChatMeshBridge`] is a **pure state machine**: it produces *frames to send*
//! and *envelopes delivered*, but performs no socket I/O and touches no storage. The caller moves the
//! frames with `SocialWebNet::send_datagram(peer, ports::CHAT, ports::CHAT, frame)` and applies each
//! delivered envelope with [`crate::chat_relay::apply_incoming_envelope`]. This keeps the routing
//! logic deterministically testable (two bridges exchanging a real envelope, with loss) and decoupled
//! from the live mesh runtime, which lives in `qualia-core-db`.

use std::collections::HashMap;

use crate::chat_relay::RelayEnvelope;
use crate::mesh_channel::{Inbound, ReliableEndpoint, DEFAULT_MAX_ATTEMPTS, DEFAULT_RTO_MS};

/// A reliable-channel frame destined for a specific peer. The caller frames it as a mesh datagram on
/// [`ports::CHAT`](qualia_core_db::p2p::mesh_datagram::ports) and sends it to `peer_did`.
#[derive(Debug, Clone, PartialEq)]
pub struct OutFrame {
    /// The peer (by DID) to send this frame to.
    pub peer_did: String,
    /// The reliable-channel frame (the datagram payload).
    pub frame: Vec<u8>,
}

/// The result of feeding one inbound frame to the bridge.
#[derive(Debug, Default)]
pub struct InboundChat {
    /// Acknowledgement frames to send back to the peer.
    pub acks: Vec<OutFrame>,
    /// A newly-delivered chat envelope, if this frame completed one (absent for ACKs and duplicates).
    pub delivered: Option<RelayEnvelope>,
    /// True if a frame arrived but could not be decoded as a chat envelope (corrupt / wrong version).
    pub decode_failed: bool,
}

/// CBOR-encode a chat envelope for the CHAT-port payload.
pub fn encode_envelope(env: &RelayEnvelope) -> Vec<u8> {
    let mut buf = Vec::new();
    // ciborium only fails here if the writer errors, which a `Vec` never does.
    let _ = ciborium::into_writer(env, &mut buf);
    buf
}

/// Decode a CHAT-port payload back into a chat envelope. `None` if it is not a valid CBOR envelope.
pub fn decode_envelope(bytes: &[u8]) -> Option<RelayEnvelope> {
    ciborium::from_reader(bytes).ok()
}

/// Per-peer reliable chat routing over the mesh. One bridge per node; it holds a
/// [`ReliableEndpoint`] per peer DID.
pub struct ChatMeshBridge {
    peers: HashMap<String, ReliableEndpoint>,
    rto_ms: u64,
    max_attempts: u32,
}

impl Default for ChatMeshBridge {
    fn default() -> Self {
        ChatMeshBridge {
            peers: HashMap::new(),
            rto_ms: DEFAULT_RTO_MS,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }
}

impl ChatMeshBridge {
    /// A bridge with explicit reliability parameters (mostly for tests).
    pub fn new(rto_ms: u64, max_attempts: u32) -> ChatMeshBridge {
        ChatMeshBridge {
            peers: HashMap::new(),
            rto_ms,
            max_attempts,
        }
    }

    fn endpoint(&mut self, peer_did: &str) -> &mut ReliableEndpoint {
        let (rto, max) = (self.rto_ms, self.max_attempts);
        self.peers
            .entry(peer_did.to_string())
            .or_insert_with(|| ReliableEndpoint::new(rto, max))
    }

    /// Peers this bridge currently has a channel to.
    pub fn peers(&self) -> Vec<String> {
        self.peers.keys().cloned().collect()
    }

    /// Reliably send `env` to every peer in `peer_dids`. The envelope is encoded once; each peer's
    /// channel assigns its own sequence number. Returns the frames to send.
    pub fn broadcast(
        &mut self,
        peer_dids: &[String],
        env: &RelayEnvelope,
        now_ms: u64,
    ) -> Vec<OutFrame> {
        let payload = encode_envelope(env);
        peer_dids
            .iter()
            .map(|did| {
                let frame = self.endpoint(did).send(&payload, now_ms);
                OutFrame {
                    peer_did: did.clone(),
                    frame,
                }
            })
            .collect()
    }

    /// Reliably send `env` to a single peer.
    pub fn send_to(&mut self, peer_did: &str, env: &RelayEnvelope, now_ms: u64) -> OutFrame {
        let payload = encode_envelope(env);
        let frame = self.endpoint(peer_did).send(&payload, now_ms);
        OutFrame {
            peer_did: peer_did.to_string(),
            frame,
        }
    }

    /// Process one inbound reliable-channel frame from `peer_did`. Returns the ACK to send back and,
    /// if the frame completed a new envelope, the decoded [`RelayEnvelope`] to apply.
    pub fn on_inbound(&mut self, peer_did: &str, frame: &[u8], now_ms: u64) -> InboundChat {
        let Inbound { delivered, to_send } = self.endpoint(peer_did).on_datagram(frame, now_ms);
        let acks = to_send
            .into_iter()
            .map(|frame| OutFrame {
                peer_did: peer_did.to_string(),
                frame,
            })
            .collect();
        let mut out = InboundChat {
            acks,
            delivered: None,
            decode_failed: false,
        };
        if let Some(payload) = delivered {
            match decode_envelope(&payload) {
                Some(env) => out.delivered = Some(env),
                None => out.decode_failed = true,
            }
        }
        out
    }

    /// Retransmit any unacknowledged frames whose RTO has elapsed, across all peers.
    pub fn on_tick(&mut self, now_ms: u64) -> Vec<OutFrame> {
        let mut out = Vec::new();
        for (did, ep) in self.peers.iter_mut() {
            let (resend, _gave_up) = ep.on_tick(now_ms);
            for frame in resend {
                out.push(OutFrame {
                    peer_did: did.clone(),
                    frame,
                });
            }
        }
        out
    }

    /// Total frames still awaiting acknowledgement across all peers.
    pub fn pending(&self) -> usize {
        self.peers.values().map(|e| e.pending()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(session: &str, lamport: u64, content: &str) -> RelayEnvelope {
        RelayEnvelope {
            session_id: session.into(),
            lamport,
            role: "user".into(),
            content: content.into(),
            author_did: "did:wf:alice".into(),
            author_name: Some("Alice".into()),
            reply_to_fragment: None,
            timestamp: 1_700_000_000,
            signature_hex: "deadbeef".into(),
            sub_agent_of: None,
            agent_did: None,
            model_id: None,
            agent_backend: None,
            outcome_sharing: None,
        }
    }

    fn same(a: &RelayEnvelope, b: &RelayEnvelope) -> bool {
        a.session_id == b.session_id
            && a.lamport == b.lamport
            && a.content == b.content
            && a.author_did == b.author_did
            && a.signature_hex == b.signature_hex
    }

    #[test]
    fn envelope_cbor_round_trips() {
        let env = envelope("s1", 7, "hello mesh");
        let bytes = encode_envelope(&env);
        let back = decode_envelope(&bytes).expect("decodes");
        assert!(same(&env, &back));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode_envelope(b"\xff\xff not cbor envelope").is_none());
    }

    #[test]
    fn two_bridges_deliver_a_chat_envelope_reliably() {
        // Alice's node and Bob's node, each a bridge; Alice sends a chat message to Bob.
        let mut alice = ChatMeshBridge::default();
        let mut bob = ChatMeshBridge::default();
        let env = envelope("room-1", 42, "over the mesh we chat");

        // Alice broadcasts to Bob → one frame for peer "bob".
        let out = alice.broadcast(&["bob".to_string()], &env, 0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].peer_did, "bob");
        assert_eq!(alice.pending(), 1, "unacked until Bob acks");

        // Bob receives it (tagging the sender as "alice"): delivers the envelope + emits an ACK.
        let inb = bob.on_inbound("alice", &out[0].frame, 1);
        let got = inb.delivered.expect("Bob got the envelope");
        assert!(same(&got, &env));
        assert_eq!(inb.acks.len(), 1);
        assert_eq!(inb.acks[0].peer_did, "alice");

        // Alice processes Bob's ACK → nothing pending.
        let back = alice.on_inbound("bob", &inb.acks[0].frame, 2);
        assert!(back.delivered.is_none());
        assert_eq!(alice.pending(), 0);
    }

    #[test]
    fn lost_envelope_is_retransmitted_and_deduplicated() {
        let mut alice = ChatMeshBridge::new(100, 8);
        let mut bob = ChatMeshBridge::default();
        let env = envelope("room-1", 1, "will be lost then resent");

        // Alice sends — but the frame is "lost" (never handed to Bob).
        let _lost = alice.broadcast(&["bob".to_string()], &env, 0);
        assert_eq!(alice.pending(), 1);

        // After the RTO, Alice retransmits.
        let resend = alice.on_tick(150);
        assert_eq!(resend.len(), 1);

        // Bob receives the retransmit and delivers once.
        let inb = bob.on_inbound("alice", &resend[0].frame, 160);
        assert!(same(&inb.delivered.expect("delivered on resend"), &env));

        // A *second* copy of the same frame (e.g. the original arriving late) must NOT re-deliver.
        let dup = bob.on_inbound("alice", &resend[0].frame, 170);
        assert!(dup.delivered.is_none(), "deduplicated");
        assert_eq!(dup.acks.len(), 1, "still acked");
    }
}
