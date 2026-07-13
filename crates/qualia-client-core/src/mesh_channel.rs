//! Reliable message channel over the mesh datagram transport.
//!
//! `qualia_core_db::p2p::mesh_datagram` frames application payloads as IPv6/UDP datagrams over a
//! WireGuard tunnel, but — like all UDP — delivery is best-effort: datagrams can be lost, reordered,
//! or duplicated. Chat sync needs *at-least-once* delivery (a published fragment must arrive) with
//! *deduplication* (a retransmit must not double-apply). This module is that layer: a small,
//! **pure** reliable-datagram protocol — sequence numbers, acknowledgements, timed retransmission,
//! and receive-side dedup.
//!
//! It is deliberately a **pure state machine**: it never touches a socket, a thread, or the clock.
//! The caller supplies the current time (monotonic milliseconds) and moves the produced wire frames
//! in and out via the mesh (`SocialWebNet::send_datagram` / the decoded inbound payload). That keeps
//! the protocol deterministically unit-testable — including loss, reordering, and duplication — with
//! no I/O, and independent of the (separately-contended) core-db crate.
//!
//! Wire frame (carried as the datagram payload):
//! ```text
//!   byte 0    : kind  (0x00 = DATA, 0x01 = ACK)
//!   bytes 1..5: seq   (u32, big-endian)
//!   bytes 5.. : payload (DATA only; ACK carries none)
//! ```
//! Reliability is per-`ReliableEndpoint` (i.e. per peer): each endpoint numbers the datagrams it
//! sends and acknowledges the datagrams it receives. Message *ordering* is not imposed — chat
//! fragments are content-addressed and idempotent, so at-least-once + dedup is effectively
//! exactly-once for them; an app that needs ordering can sequence at its own layer.

use std::collections::{BTreeMap, BTreeSet};

const KIND_DATA: u8 = 0x00;
const KIND_ACK: u8 = 0x01;
const HEADER: usize = 5;

/// Default retransmit timeout (ms) — resend an unacked datagram after this long.
pub const DEFAULT_RTO_MS: u64 = 500;
/// Default maximum send attempts before giving up on a datagram.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 8;
/// How many of the most recent received sequence numbers to remember for dedup. Retransmits only
/// ever replay recent sequences, so a bounded window is sufficient and keeps memory flat.
const DEDUP_WINDOW: u32 = 4096;

/// A datagram awaiting acknowledgement.
struct Unacked {
    payload: Vec<u8>,
    last_sent_ms: u64,
    attempts: u32,
}

/// What processing one inbound frame produced.
#[derive(Debug, Default, PartialEq)]
pub struct Inbound {
    /// A newly-delivered application payload (absent for duplicates and for ACK frames).
    pub delivered: Option<Vec<u8>>,
    /// Frames to send back to the peer now (an ACK for a received DATA frame).
    pub to_send: Vec<Vec<u8>>,
}

/// The reliable channel to a single peer. Pure: drive it with `send`, `on_datagram`, and `on_tick`,
/// moving the returned frames over the mesh yourself.
pub struct ReliableEndpoint {
    next_seq: u32,
    unacked: BTreeMap<u32, Unacked>,
    /// Recently-received DATA sequences (for dedup), pruned to a trailing window.
    received: BTreeSet<u32>,
    highest_received: Option<u32>,
    rto_ms: u64,
    max_attempts: u32,
}

impl Default for ReliableEndpoint {
    fn default() -> Self {
        Self::new(DEFAULT_RTO_MS, DEFAULT_MAX_ATTEMPTS)
    }
}

fn data_frame(seq: u32, payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(HEADER + payload.len());
    f.push(KIND_DATA);
    f.extend_from_slice(&seq.to_be_bytes());
    f.extend_from_slice(payload);
    f
}

fn ack_frame(seq: u32) -> Vec<u8> {
    let mut f = Vec::with_capacity(HEADER);
    f.push(KIND_ACK);
    f.extend_from_slice(&seq.to_be_bytes());
    f
}

impl ReliableEndpoint {
    /// A channel with explicit retransmit timeout and attempt cap.
    pub fn new(rto_ms: u64, max_attempts: u32) -> ReliableEndpoint {
        ReliableEndpoint {
            next_seq: 0,
            unacked: BTreeMap::new(),
            received: BTreeSet::new(),
            highest_received: None,
            rto_ms,
            max_attempts,
        }
    }

    /// Queue `payload` for reliable delivery and return the DATA frame to send now. The frame is
    /// buffered for retransmission until the peer acknowledges it (see [`on_tick`](Self::on_tick)).
    pub fn send(&mut self, payload: &[u8], now_ms: u64) -> Vec<u8> {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let frame = data_frame(seq, payload);
        self.unacked.insert(
            seq,
            Unacked {
                payload: payload.to_vec(),
                last_sent_ms: now_ms,
                attempts: 1,
            },
        );
        frame
    }

    /// Number of datagrams still awaiting acknowledgement.
    pub fn pending(&self) -> usize {
        self.unacked.len()
    }

    fn remember_received(&mut self, seq: u32) {
        self.received.insert(seq);
        let hi = self.highest_received.map_or(seq, |h| h.max(seq));
        self.highest_received = Some(hi);
        // Prune sequences older than the dedup window.
        let cutoff = hi.saturating_sub(DEDUP_WINDOW);
        while let Some(&low) = self.received.iter().next() {
            if low < cutoff {
                self.received.remove(&low);
            } else {
                break;
            }
        }
    }

    /// Process one inbound wire frame. A DATA frame is deduplicated and (if new) its payload is
    /// delivered; either way an ACK is returned so the sender stops retransmitting. An ACK frame
    /// clears the matching unacked datagram. Malformed frames are ignored.
    pub fn on_datagram(&mut self, frame: &[u8], _now_ms: u64) -> Inbound {
        if frame.len() < HEADER {
            return Inbound::default();
        }
        let seq = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]);
        match frame[0] {
            KIND_DATA => {
                let is_new = !self.received.contains(&seq);
                let delivered = if is_new {
                    self.remember_received(seq);
                    Some(frame[HEADER..].to_vec())
                } else {
                    None // duplicate — still ACK it below so the sender stops resending
                };
                Inbound {
                    delivered,
                    to_send: vec![ack_frame(seq)],
                }
            }
            KIND_ACK => {
                self.unacked.remove(&seq);
                Inbound::default()
            }
            _ => Inbound::default(),
        }
    }

    /// Retransmit any datagram unacknowledged for longer than the RTO. Returns the frames to resend.
    /// Datagrams that exceed the attempt cap are given up on and reported in the second element.
    pub fn on_tick(&mut self, now_ms: u64) -> (Vec<Vec<u8>>, Vec<u32>) {
        let mut resend = Vec::new();
        let mut gave_up = Vec::new();
        let mut abandon = Vec::new();

        for (&seq, u) in self.unacked.iter_mut() {
            if now_ms.saturating_sub(u.last_sent_ms) < self.rto_ms {
                continue;
            }
            if u.attempts >= self.max_attempts {
                abandon.push(seq);
                continue;
            }
            u.attempts += 1;
            u.last_sent_ms = now_ms;
            resend.push(data_frame(seq, &u.payload));
        }
        for seq in abandon {
            self.unacked.remove(&seq);
            gave_up.push(seq);
        }
        (resend, gave_up)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq_of(frame: &[u8]) -> u32 {
        u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]])
    }

    #[test]
    fn deliver_and_ack_clears_unacked() {
        let mut a = ReliableEndpoint::default();
        let mut b = ReliableEndpoint::default();

        let data = a.send(b"hello", 0);
        assert_eq!(a.pending(), 1);

        // B receives, delivers, and returns an ACK.
        let inb = b.on_datagram(&data, 1);
        assert_eq!(inb.delivered.as_deref(), Some(&b"hello"[..]));
        assert_eq!(inb.to_send.len(), 1);

        // A receives the ACK → nothing pending.
        let ack = &inb.to_send[0];
        let back = a.on_datagram(ack, 2);
        assert!(back.delivered.is_none());
        assert_eq!(a.pending(), 0);
    }

    #[test]
    fn duplicate_data_is_delivered_once_but_acked_each_time() {
        let mut a = ReliableEndpoint::default();
        let mut b = ReliableEndpoint::default();
        let data = a.send(b"dup", 0);

        let first = b.on_datagram(&data, 1);
        assert_eq!(first.delivered.as_deref(), Some(&b"dup"[..]));
        assert_eq!(first.to_send.len(), 1, "acked");

        // A retransmit (same seq) must NOT re-deliver, but must still be ACKed.
        let second = b.on_datagram(&data, 2);
        assert!(second.delivered.is_none(), "not delivered twice");
        assert_eq!(second.to_send.len(), 1, "still acked so sender stops");
    }

    #[test]
    fn retransmits_after_rto_then_stops_once_acked() {
        let mut a = ReliableEndpoint::new(100, 8);
        let _data = a.send(b"x", 0);

        // Before the RTO: nothing to resend.
        let (resend, gave_up) = a.on_tick(50);
        assert!(resend.is_empty() && gave_up.is_empty());

        // After the RTO: one retransmit.
        let (resend, _) = a.on_tick(150);
        assert_eq!(resend.len(), 1);
        assert_eq!(seq_of(&resend[0]), 0);

        // Deliver+ack via a peer, then no more retransmits.
        let mut b = ReliableEndpoint::default();
        let inb = b.on_datagram(&resend[0], 160);
        a.on_datagram(&inb.to_send[0], 170);
        let (resend, _) = a.on_tick(1000);
        assert!(resend.is_empty(), "acked → no retransmit");
    }

    #[test]
    fn gives_up_after_max_attempts() {
        let mut a = ReliableEndpoint::new(10, 3);
        a.send(b"lost", 0); // attempt 1
        // No ACK ever. Tick past the RTO repeatedly.
        let (_r, g) = a.on_tick(20); // attempt 2
        assert!(g.is_empty());
        let (_r, g) = a.on_tick(40); // attempt 3
        assert!(g.is_empty());
        let (r, g) = a.on_tick(60); // would be attempt 4 > max 3 → give up
        assert!(r.is_empty());
        assert_eq!(g, vec![0]);
        assert_eq!(a.pending(), 0, "abandoned datagram dropped");
    }

    #[test]
    fn out_of_order_and_lossy_stream_delivers_every_payload_once() {
        // Simulate A sending 3 messages; deliver them to B out of order, with a duplicate and a drop
        // that A later retransmits. B must deliver each distinct payload exactly once.
        let mut a = ReliableEndpoint::new(100, 8);
        let f0 = a.send(b"m0", 0);
        // `m1`'s frame is deliberately dropped (never delivered to B) — we keep the `send` for its
        // side effect (buffering seq 1 for retransmit) but never use the returned frame directly.
        let _f1 = a.send(b"m1", 0);
        let f2 = a.send(b"m2", 0);

        let mut b = ReliableEndpoint::default();
        let mut delivered: Vec<Vec<u8>> = Vec::new();
        let mut push = |inb: Inbound| {
            if let Some(p) = inb.delivered {
                delivered.push(p);
            }
        };

        push(b.on_datagram(&f2, 1)); // out of order
        push(b.on_datagram(&f0, 2));
        push(b.on_datagram(&f2, 3)); // duplicate — ignored
        // f1 "lost"; A retransmits it after RTO.
        let (resend, _) = a.on_tick(200);
        assert!(resend.iter().any(|f| seq_of(f) == 1));
        for f in &resend {
            if seq_of(f) == 1 {
                push(b.on_datagram(f, 210));
            }
        }

        delivered.sort();
        assert_eq!(delivered, vec![b"m0".to_vec(), b"m1".to_vec(), b"m2".to_vec()]);
    }
}
