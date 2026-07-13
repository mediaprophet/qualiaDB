//! **libp2p op-transfer sync** (T3.1 p2p backend) — the wire protocol + reference relay that actually
//! carries synchronisation operations peer-to-peer over the existing libp2p request-response swarm.
//!
//! Distinct from [`super::protocol`]'s `QualiaRequest::Sync`, which is only an *authorisation handshake*
//! (hop-count / gatekeeper / target-shapes) and transfers no operations. This module adds the missing
//! piece: **`Publish` / `PullSince` frames that move opaque, already-signed operation bytes**, plus a
//! [`SyncOpRelay`] responder store. Operations travel as opaque `Vec<u8>` frames (the sync layer that
//! owns the `SyncOperation` type serialises/deserialises them), so this stays a **dumb pipe**: it never
//! validates or trusts — all trust remains in the consuming node's fail-closed inbox, exactly as the
//! HTTP relay transport does. Encryption + auth come from libp2p's noise handshake on the connection.
//!
//! Native-only (libp2p). The swarm-driving `SyncTransport` bridge that maps the blocking
//! publish/pull trait onto this protocol is the next step; this is the tested wire + store foundation.

#![cfg(not(target_arch = "wasm32"))]

use async_trait::async_trait;
use libp2p::futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use std::sync::{Arc, Mutex};

/// The request-response stream protocol id for op transfer (distinct from `/qualia/crdt-sync/1.0.0`).
pub const SYNC_OP_PROTOCOL: &str = "/qualia/sync-ops/1.0.0";

/// A hard cap on a single frame's encoded size, so a hostile peer cannot force an unbounded allocation.
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// A request in the op-transfer protocol. `op_frames` are opaque, already-signed operation bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOpRequest {
    /// Offer operations to the peer. Idempotent at the relay (dedup by frame content).
    Publish { op_frames: Vec<Vec<u8>> },
    /// Ask for operations the relay holds after `cursor` (`0` = from the start).
    PullSince { cursor: u64 },
}

/// A response in the op-transfer protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOpResponse {
    /// How many *new* (non-duplicate) frames the publish added.
    Published { accepted: u64 },
    /// The frames after the requested cursor, plus the cursor to use next time.
    Pulled {
        op_frames: Vec<Vec<u8>>,
        next_cursor: u64,
    },
}

/// A reference op relay/store — append-only, dedup by frame content, cursor = position. The responder
/// side of the protocol. A **dumb pipe**: it stores and serves opaque frames and trusts nothing.
/// Cloning yields another handle onto the **same** store (so a node can both serve and be queried).
#[derive(Clone, Default)]
pub struct SyncOpRelay {
    frames: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl SyncOpRelay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add frames, skipping any already present (dedup by content). Returns the number newly added.
    pub fn publish(&self, op_frames: &[Vec<u8>]) -> u64 {
        let mut store = self.frames.lock().expect("relay lock");
        let mut accepted = 0;
        for f in op_frames {
            if !store.iter().any(|e| e == f) {
                store.push(f.clone());
                accepted += 1;
            }
        }
        accepted
    }

    /// The frames after `cursor` (in store order) plus the next cursor. Re-pulling is safe.
    pub fn pull_since(&self, cursor: u64) -> (Vec<Vec<u8>>, u64) {
        let store = self.frames.lock().expect("relay lock");
        let start = (cursor as usize).min(store.len());
        (store[start..].to_vec(), store.len() as u64)
    }

    /// Number of distinct frames held.
    pub fn len(&self) -> usize {
        self.frames.lock().map(|v| v.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Serve one request against the store — the responder's request→response handler.
    pub fn handle(&self, req: SyncOpRequest) -> SyncOpResponse {
        match req {
            SyncOpRequest::Publish { op_frames } => SyncOpResponse::Published {
                accepted: self.publish(&op_frames),
            },
            SyncOpRequest::PullSince { cursor } => {
                let (op_frames, next_cursor) = self.pull_since(cursor);
                SyncOpResponse::Pulled {
                    op_frames,
                    next_cursor,
                }
            }
        }
    }
}

/// The libp2p request-response wire codec for op transfer: length-prefixed plain-CBOR (ciborium). Mirrors
/// [`super::protocol::QualiaSyncCodec`]'s framing (4-byte big-endian length + body), without the Q42
/// CBOR-LD term-compaction (the payload is opaque operation bytes, so there is nothing to term-compact).
#[derive(Clone, Default)]
pub struct SyncOpCodec;

async fn read_frame<T, V>(io: &mut T) -> io::Result<V>
where
    T: AsyncRead + Unpin + Send,
    V: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "sync-op frame too large",
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    ciborium::from_reader(&buf[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

async fn write_frame<T, V>(io: &mut T, v: &V) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
    V: Serialize,
{
    let mut buf = Vec::new();
    ciborium::into_writer(v, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    io.write_all(&(buf.len() as u32).to_be_bytes()).await?;
    io.write_all(&buf).await?;
    Ok(())
}

#[async_trait]
impl Codec for SyncOpCodec {
    type Protocol = StreamProtocol;
    type Request = SyncOpRequest;
    type Response = SyncOpResponse;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<SyncOpRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn read_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<SyncOpResponse>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        req: SyncOpRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &req).await
    }

    async fn write_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        res: SyncOpResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &res).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_dedups_publish_and_pulls_from_cursor() {
        let relay = SyncOpRelay::new();
        assert_eq!(relay.publish(&[b"a".to_vec(), b"b".to_vec()]), 2);
        // Re-publishing 'a' is idempotent.
        assert_eq!(relay.publish(&[b"a".to_vec()]), 0);
        assert_eq!(relay.len(), 2);

        let (frames, cursor) = relay.pull_since(0);
        assert_eq!(frames, vec![b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(cursor, 2);
        // Only what's new after the cursor.
        assert_eq!(relay.publish(&[b"c".to_vec()]), 1);
        let (fresh, next) = relay.pull_since(2);
        assert_eq!(fresh, vec![b"c".to_vec()]);
        assert_eq!(next, 3);
    }

    #[test]
    fn handle_maps_requests_to_responses() {
        let relay = SyncOpRelay::new();
        assert_eq!(
            relay.handle(SyncOpRequest::Publish {
                op_frames: vec![b"x".to_vec()]
            }),
            SyncOpResponse::Published { accepted: 1 }
        );
        assert_eq!(
            relay.handle(SyncOpRequest::PullSince { cursor: 0 }),
            SyncOpResponse::Pulled {
                op_frames: vec![b"x".to_vec()],
                next_cursor: 1
            }
        );
    }

    /// The wire payload round-trips losslessly (the ciborium (de)serialization the codec applies before
    /// the 4-byte length prefix). The framing is identical to the proven `QualiaSyncCodec`.
    #[test]
    fn wire_payload_roundtrips_losslessly() {
        for req in [
            SyncOpRequest::Publish {
                op_frames: vec![b"op-1".to_vec(), b"op-2".to_vec()],
            },
            SyncOpRequest::PullSince { cursor: 42 },
        ] {
            let mut buf = Vec::new();
            ciborium::into_writer(&req, &mut buf).unwrap();
            let back: SyncOpRequest = ciborium::from_reader(&buf[..]).unwrap();
            assert_eq!(back, req);
        }
        for res in [
            SyncOpResponse::Published { accepted: 3 },
            SyncOpResponse::Pulled {
                op_frames: vec![b"op-1".to_vec()],
                next_cursor: 7,
            },
        ] {
            let mut buf = Vec::new();
            ciborium::into_writer(&res, &mut buf).unwrap();
            let back: SyncOpResponse = ciborium::from_reader(&buf[..]).unwrap();
            assert_eq!(back, res);
        }
    }
}
