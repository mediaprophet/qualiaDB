//! Sync **transport** (T3.1) — moves [`SyncOperation`]s between this node and a peer/relay.
//!
//! The transport is a **dumb pipe**: it neither validates nor trusts operations. All trust lives in
//! the inbox's fail-closed [`validate_operation`](super::sync_protocol::validate_operation) —
//! a hostile relay or peer can therefore only ever cause *rejections* at admission, never the
//! admission of bad data. Convergence is guaranteed by
//! [`merge_operations`](super::sync_protocol::merge_operations) (add-wins by operation id), so
//! duplicate / reordered / replayed frames converge to the same validated set.
//!
//! Two backends:
//! - [`InMemoryRelay`] — a shared, in-process op store (dedup by operation id). Clone it to get a
//!   second handle onto the same relay; two nodes sharing one relay exchange ops. This is the
//!   reference transport and the one the convergence / hostile-peer tests run over.
//! - [`HttpRelayTransport`] (native) — a `reqwest::blocking` client that POSTs to `"{base}/sync/publish"`
//!   and GETs `"{base}/sync/pull?since={cursor}"`. Its server counterpart is
//!   [`super::sync_relay_server::SyncRelayServer`].

use super::sync_protocol::SyncOperation;
use std::sync::{Arc, Mutex};

/// Moves operations to/from a peer or relay. A dumb pipe — validation is the inbox's job.
pub trait SyncTransport {
    /// Publish local operations to the relay/peer. Must be idempotent at the relay (dedup by
    /// operation id), so re-publishing a queued op is harmless.
    fn publish(&self, ops: &[SyncOperation]) -> Result<(), String>;

    /// Pull operations the relay holds after cursor `since` (`0` = from the start). Returns them in
    /// relay order. Re-pulling is safe: the inbox dedups by operation id on admission.
    fn pull(&self, since: u64) -> Result<Vec<SyncOperation>, String>;
}

/// A shared in-memory relay — a dumb op store (append-only, dedup by operation id). Cloning yields
/// another handle onto the **same** underlying store, so two nodes can rendezvous through one relay.
#[derive(Clone, Default)]
pub struct InMemoryRelay {
    inner: Arc<Mutex<Vec<SyncOperation>>>,
}

impl InMemoryRelay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct operations the relay holds.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|v| v.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SyncTransport for InMemoryRelay {
    fn publish(&self, ops: &[SyncOperation]) -> Result<(), String> {
        let mut store = self.inner.lock().map_err(|_| "relay lock poisoned".to_string())?;
        for op in ops {
            if !store.iter().any(|e| e.operation_id == op.operation_id) {
                store.push(op.clone());
            }
        }
        Ok(())
    }

    fn pull(&self, since: u64) -> Result<Vec<SyncOperation>, String> {
        let store = self.inner.lock().map_err(|_| "relay lock poisoned".to_string())?;
        let start = (since as usize).min(store.len());
        Ok(store[start..].to_vec())
    }
}

/// The wire body for `/sync/publish` and the response body for `/sync/pull`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncOpsBody {
    pub ops: Vec<SyncOperation>,
}

/// An HTTP relay transport (native). POSTs a [`SyncOpsBody`] to `"{base}/sync/publish"` and GETs
/// `"{base}/sync/pull?since={cursor}"`. Mirrors the crate's `reqwest::blocking` chat-relay pattern.
#[cfg(not(target_arch = "wasm32"))]
pub struct HttpRelayTransport {
    base_url: String,
    timeout: std::time::Duration,
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpRelayTransport {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            timeout: std::time::Duration::from_secs(8),
        }
    }

    fn client(&self) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncTransport for HttpRelayTransport {
    fn publish(&self, ops: &[SyncOperation]) -> Result<(), String> {
        let url = format!("{}/sync/publish", self.base_url);
        let body = SyncOpsBody { ops: ops.to_vec() };
        let resp = self
            .client()?
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("relay publish failed: HTTP {}", resp.status()));
        }
        Ok(())
    }

    fn pull(&self, since: u64) -> Result<Vec<SyncOperation>, String> {
        let url = format!("{}/sync/pull?since={since}", self.base_url);
        let resp = self.client()?.get(&url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("relay pull failed: HTTP {}", resp.status()));
        }
        let parsed: SyncOpsBody = resp.json().map_err(|e| e.to_string())?;
        Ok(parsed.ops)
    }
}

/// A **libp2p** `SyncTransport` (native) — a noise-encrypted request-response pipe to a single peer or
/// relay. It wraps the core-db [`BlockingSyncClient`](qualia_core_db::p2p::sync_node::BlockingSyncClient):
/// each [`SyncOperation`] is serialized to a CBOR op frame on publish and decoded back on pull, so the
/// wire carries only opaque signed-op bytes. A dumb pipe like the in-memory and HTTP backends — it never
/// validates or trusts. Undecodable inbound frames are **skipped** (they could never be admitted anyway;
/// the fail-closed inbox is the trust boundary), so a hostile peer injecting junk cannot break sync for
/// well-formed operations.
#[cfg(not(target_arch = "wasm32"))]
pub struct Libp2pSyncTransport {
    client: qualia_core_db::p2p::sync_node::BlockingSyncClient,
}

#[cfg(not(target_arch = "wasm32"))]
impl Libp2pSyncTransport {
    /// Connect to `peer_addr` (a libp2p multiaddr, e.g. `/ip4/1.2.3.4/tcp/4001`) identified by `peer_id`
    /// (base58). The connection is established lazily on the first publish/pull.
    pub fn connect(peer_id: &str, peer_addr: &str) -> Result<Self, String> {
        let client = qualia_core_db::p2p::sync_node::BlockingSyncClient::connect(
            qualia_core_db::p2p::sync_ops::SyncOpRelay::new(),
            peer_id,
            peer_addr,
        )?;
        Ok(Self { client })
    }

    /// This node's own libp2p peer id (base58) — so a peer can be told how to reach us back.
    pub fn local_peer_id(&self) -> String {
        self.client.local_peer_id().to_string()
    }
}

/// Serialize an operation to its opaque CBOR wire frame.
#[cfg(not(target_arch = "wasm32"))]
fn op_to_frame(op: &SyncOperation) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ciborium::into_writer(op, &mut buf).map_err(|e| format!("encode operation: {e}"))?;
    Ok(buf)
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncTransport for Libp2pSyncTransport {
    fn publish(&self, ops: &[SyncOperation]) -> Result<(), String> {
        if ops.is_empty() {
            return Ok(());
        }
        let frames = ops.iter().map(op_to_frame).collect::<Result<Vec<_>, _>>()?;
        self.client.publish_frames(frames)?;
        Ok(())
    }

    fn pull(&self, since: u64) -> Result<Vec<SyncOperation>, String> {
        let (frames, _next_cursor) = self.client.pull_frames(since)?;
        // Dumb pipe: decode what we can, skip junk (undecodable frames can never be valid operations,
        // and the inbox would reject them regardless).
        Ok(frames
            .iter()
            .filter_map(|f| ciborium::from_reader::<SyncOperation, _>(&f[..]).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wellfair::sync_protocol::{AdmitOutcome, SyncInbox, SyncOperation};

    fn signed(id: &str, kind: &str, summary: &str, lamport: u64) -> SyncOperation {
        SyncOperation::new(
            id,
            format!("urn:wellfair:{kind}:{id}"),
            kind,
            "did:wf:remote",
            "Restricted",
            summary,
            lamport,
            1_700_000_000,
        )
        .with_signature("deadbeef")
    }

    #[test]
    fn in_memory_relay_dedups_and_pulls_from_cursor() {
        let relay = InMemoryRelay::new();
        relay.publish(&[signed("a", "ledger_entry", "1", 1)]).unwrap();
        relay.publish(&[signed("b", "ledger_entry", "2", 2)]).unwrap();
        // Re-publishing 'a' is idempotent at the relay.
        relay.publish(&[signed("a", "ledger_entry", "1", 1)]).unwrap();
        assert_eq!(relay.len(), 2);

        // Cursor: pull everything, then only what's new.
        assert_eq!(relay.pull(0).unwrap().len(), 2);
        assert_eq!(relay.pull(2).unwrap().len(), 0);
        relay.publish(&[signed("c", "ledger_entry", "3", 3)]).unwrap();
        let fresh = relay.pull(2).unwrap();
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].operation_id, "c");
    }

    #[test]
    fn two_nodes_converge_over_one_relay() {
        // Node A and Node B each publish a disjoint op to a shared relay; after both pull and admit,
        // their validated sets are identical (convergence).
        let relay = InMemoryRelay::new();
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let inbox_a = SyncInbox::open(dir_a.path()).unwrap();
        let inbox_b = SyncInbox::open(dir_b.path()).unwrap();

        let a_op = signed("a1", "contribution", "a", 1);
        let b_op = signed("b1", "contribution", "b", 2);
        // Each node admits its own op locally + publishes it.
        inbox_a.admit(&a_op, 1).unwrap();
        relay.publish(&[a_op.clone()]).unwrap();
        inbox_b.admit(&b_op, 1).unwrap();
        relay.publish(&[b_op.clone()]).unwrap();

        // Each pulls the whole relay and admits.
        for op in relay.pull(0).unwrap() {
            inbox_a.admit(&op, 2).unwrap();
            inbox_b.admit(&op, 2).unwrap();
        }

        let set_a = inbox_a.validated_operations().unwrap();
        let set_b = inbox_b.validated_operations().unwrap();
        assert_eq!(set_a, set_b, "nodes must converge to the same validated set");
        assert_eq!(set_a.len(), 2);
    }

    #[test]
    fn hostile_relay_ops_are_rejected_not_admitted() {
        // A hostile peer publishes malformed ops through the relay. The inbox rejects every one
        // fail-closed; the validated set stays empty.
        let relay = InMemoryRelay::new();
        let dir = tempfile::tempdir().unwrap();
        let inbox = SyncInbox::open(dir.path()).unwrap();

        // Unsigned.
        let mut unsigned = signed("h1", "ledger_entry", "x", 1);
        unsigned.signature = None;
        // Tampered content hash.
        let mut tampered = signed("h2", "ledger_entry", "orig", 1);
        tampered.payload_summary = "changed".into();
        // Wrong protocol version.
        let mut bad_ver = signed("h3", "ledger_entry", "x", 1);
        bad_ver.protocol_version = 99;
        // Classified lane (must never traverse the ordinary inbox).
        let mut classified = signed("h4", "sanctuary_note", "x", 1);
        classified.sensitivity = "Classified".into();

        relay
            .publish(&[unsigned, tampered, bad_ver, classified])
            .unwrap();

        let mut rejected = 0;
        for op in relay.pull(0).unwrap() {
            if matches!(inbox.admit(&op, 5).unwrap(), AdmitOutcome::Rejected(_)) {
                rejected += 1;
            }
        }
        assert_eq!(rejected, 4, "all hostile ops must be rejected");
        assert!(inbox.validated_operations().unwrap().is_empty());
    }

    #[test]
    fn replayed_pull_is_idempotent() {
        // Pulling and admitting the same relay contents twice never double-applies.
        let relay = InMemoryRelay::new();
        let dir = tempfile::tempdir().unwrap();
        let inbox = SyncInbox::open(dir.path()).unwrap();
        relay.publish(&[signed("r1", "ledger_entry", "x", 1)]).unwrap();

        for _ in 0..3 {
            for op in relay.pull(0).unwrap() {
                inbox.admit(&op, 9).unwrap();
            }
        }
        assert_eq!(inbox.validated_operations().unwrap().len(), 1);
    }

    #[test]
    fn partition_then_rejoin_converges() {
        // Two relays (a partition). Each node works its own side, then they exchange full contents
        // (the rejoin) and both admit everything — converging.
        let left = InMemoryRelay::new();
        let right = InMemoryRelay::new();
        left.publish(&[signed("L", "contribution", "l", 1)]).unwrap();
        right.publish(&[signed("R", "contribution", "r", 2)]).unwrap();

        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let inbox_a = SyncInbox::open(dir_a.path()).unwrap();
        let inbox_b = SyncInbox::open(dir_b.path()).unwrap();

        // Rejoin: merge both partitions' contents into one exchange set.
        let mut all = left.pull(0).unwrap();
        all.extend(right.pull(0).unwrap());
        for op in &all {
            inbox_a.admit(op, 1).unwrap();
            inbox_b.admit(op, 1).unwrap();
        }
        assert_eq!(inbox_a.validated_operations().unwrap(), inbox_b.validated_operations().unwrap());
        assert_eq!(inbox_a.validated_operations().unwrap().len(), 2);
    }

    /// **End-to-end over libp2p:** real signed `SyncOperation`s travel from the transport, through the
    /// noise-encrypted request-response wire, into a responder's relay, are pulled back (surviving the
    /// CBOR round-trip byte-for-byte), and admit into a fail-closed inbox as `Validated`. This proves the
    /// last piece of T3.1 — the `SyncOperation`↔frame serialization + the blocking libp2p transport.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn libp2p_transport_round_trips_real_operations_and_inbox_admits() {
        use qualia_core_db::p2p::sync_node::Libp2pSyncNode;
        use qualia_core_db::p2p::sync_ops::SyncOpRelay;

        // Responder A: listens and serves its relay on its own runtime; keep `a` + `rt_a` alive so A's
        // event loop stays up for the whole test.
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

        // The libp2p SyncTransport (the spoke) dials A.
        let transport =
            Libp2pSyncTransport::connect(&a.peer_id.to_string(), &a_addr.to_string()).expect("connect");

        // Publish two real signed operations, then pull them back.
        let ops = vec![
            signed("t1", "contribution", "alpha", 1),
            signed("t2", "contribution", "beta", 2),
        ];
        transport.publish(&ops).expect("publish");
        assert_eq!(relay_a.len(), 2, "A's relay holds both op frames");

        let pulled = transport.pull(0).expect("pull");
        assert_eq!(pulled, ops, "operations round-trip losslessly through CBOR + libp2p");

        // The pulled operations admit into a fresh fail-closed inbox as Validated.
        let dir = tempfile::tempdir().unwrap();
        let inbox = SyncInbox::open(dir.path()).unwrap();
        for op in &pulled {
            assert_eq!(inbox.admit(op, 1).unwrap(), AdmitOutcome::Validated);
        }
        assert_eq!(inbox.validated_operations().unwrap().len(), 2);
    }
}
