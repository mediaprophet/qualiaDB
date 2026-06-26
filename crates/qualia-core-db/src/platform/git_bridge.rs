//! Minimal Merkle-DAG for Q42 volume history (Phase 1 — §4.5 Option B).
//!
//! `DagNode` (88 bytes, `#[repr(C)]`) is the atomic commit unit.
//! Branches are stored as BRANCHES_CONTEXT quins keyed on `q_hash(branch_name)`.
//! Contestability forks create a new node with the `FORK_DISPUTED` flag.
//!
//! The fast-export stream produced by `generate_fast_export_stream` now iterates
//! real DagNodes rather than returning a hardcoded stub.

use sha2::{Digest, Sha256};

use crate::{q_hash, NQuin};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Named-graph context for branch pointer quins.
pub const BRANCHES_CONTEXT: u64 = q_hash("urn:qualia:context:branches");
/// Predicate: "this subject is the tip of branch X"
const P_BRANCH_TIP: u64 = q_hash("urn:qualia:dag:branchTip");
/// Predicate: "this subject has parent DAG node Y" — reserved for Phase 2 graph traversal.
#[allow(dead_code)]
const P_PARENT: u64 = q_hash("urn:qualia:dag:parent");

/// Node flag: this node was created via a contestability fork.
pub const FORK_DISPUTED: u32 = 0x0001;
/// Node flag: genesis node (no parent).
pub const GENESIS: u32 = 0x0002;
/// Node flag: secondary-parent back-link in a merge commit.
///
/// A merge creates two nodes: the primary merge commit (`flags = 0`) and a
/// secondary back-link node (`flags = MERGE_SECONDARY`), whose `quins_merkle`
/// encodes the primary commit hash for bidirectional DAG traversal.
/// Conflict quins should be written to `crate::provenance::CONTEST_CONTEXT`.
pub const MERGE_SECONDARY: u32 = 0x0008;

// ── DagNode ───────────────────────────────────────────────────────────────────

/// 88-byte Merkle-DAG commit node.
///
/// Layout (32+32+8+8+4+4 = 88 bytes, all fields little-endian):
/// ```text
/// [0..32)   parent_hash     — SHA-256 of parent DagNode bytes; all-zero = genesis
/// [32..64)  quins_merkle    — SHA-256 of the NQuin slice committed here
/// [64..72)  author_did      — q_hash of the author's DID string (u64)
/// [72..80)  timestamp       — ms since Unix epoch (u64)
/// [80..84)  message_hash    — low 32 bits of q_hash of the commit message
/// [84..88)  flags           — GENESIS | FORK_DISPUTED | ...
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DagNode {
    /// SHA-256 of the parent node's bytes; `[0u8; 32]` for genesis.
    pub parent_hash:  [u8; 32],
    /// SHA-256 of the quins slice this commit adds/removes.
    pub quins_merkle: [u8; 32],
    /// `q_hash` of the author's DID string.
    pub author_did:   u64,
    /// Milliseconds since Unix epoch.
    pub timestamp:    u64,
    /// Low 32 bits of `q_hash(message)` — enough for dedup/indexing.
    pub message_hash: u32,
    /// Node flags (`GENESIS`, `FORK_DISPUTED`, …).
    pub flags:        u32,
}

const _: () = assert!(
    std::mem::size_of::<DagNode>() == 88,
    "DagNode must be exactly 88 bytes"
);

impl DagNode {
    /// Compute the SHA-256 digest of this node's canonical byte representation.
    pub fn digest(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.parent_hash);
        h.update(self.quins_merkle);
        h.update(self.author_did.to_le_bytes());
        h.update(self.timestamp.to_le_bytes());
        h.update(self.message_hash.to_le_bytes());
        h.update(self.flags.to_le_bytes());
        h.finalize().into()
    }

    /// Serialize to 88 bytes (little-endian for numeric fields).
    pub fn to_bytes(&self) -> [u8; 88] {
        let mut b = [0u8; 88];
        b[0..32].copy_from_slice(&self.parent_hash);
        b[32..64].copy_from_slice(&self.quins_merkle);
        b[64..72].copy_from_slice(&self.author_did.to_le_bytes());
        b[72..80].copy_from_slice(&self.timestamp.to_le_bytes());
        b[80..84].copy_from_slice(&self.message_hash.to_le_bytes());
        b[84..88].copy_from_slice(&self.flags.to_le_bytes());
        b
    }

    /// Deserialize from 88 bytes.
    pub fn from_bytes(b: &[u8; 88]) -> Self {
        DagNode {
            parent_hash:  b[0..32].try_into().unwrap(),
            quins_merkle: b[32..64].try_into().unwrap(),
            author_did:   u64::from_le_bytes(b[64..72].try_into().unwrap()),
            timestamp:    u64::from_le_bytes(b[72..80].try_into().unwrap()),
            message_hash: u32::from_le_bytes(b[80..84].try_into().unwrap()),
            flags:        u32::from_le_bytes(b[84..88].try_into().unwrap()),
        }
    }
}

// ── quins_merkle helper ───────────────────────────────────────────────────────

/// Compute the SHA-256 Merkle root over a sorted slice of NQuins.
pub fn quins_merkle(quins: &[NQuin]) -> [u8; 32] {
    let mut h = Sha256::new();
    for q in quins {
        h.update(q.subject.to_le_bytes());
        h.update(q.predicate.to_le_bytes());
        h.update(q.object.to_le_bytes());
        h.update(q.context.to_le_bytes());
    }
    h.finalize().into()
}

// ── DagStore ─────────────────────────────────────────────────────────────────

/// In-memory DAG node store.  Persisted to `dag_root_offset`/`dag_root_length`
/// in the Q42 v3 volume header.
pub struct DagStore {
    nodes: Vec<(DagNode, [u8; 32])>, // (node, hash)
    branches: std::collections::HashMap<u64, [u8; 32]>, // branch_name_hash → tip_hash
}

impl DagStore {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), branches: std::collections::HashMap::new() }
    }

    /// Create the genesis node (no parent).
    pub fn genesis_node(
        &mut self,
        quins: &[NQuin],
        author_did: u64,
        timestamp_ms: u64,
        message: &str,
    ) -> [u8; 32] {
        let node = DagNode {
            parent_hash: [0u8; 32],
            quins_merkle: quins_merkle(quins),
            author_did,
            timestamp: timestamp_ms,
            message_hash: q_hash(message) as u32,
            flags: GENESIS,
        };
        let hash = node.digest();
        self.nodes.push((node, hash));
        hash
    }

    /// Create a regular commit node chained from `parent_hash`.
    pub fn commit_node(
        &mut self,
        parent_hash: [u8; 32],
        quins: &[NQuin],
        author_did: u64,
        timestamp_ms: u64,
        message: &str,
    ) -> [u8; 32] {
        let node = DagNode {
            parent_hash,
            quins_merkle: quins_merkle(quins),
            author_did,
            timestamp: timestamp_ms,
            message_hash: q_hash(message) as u32,
            flags: 0,
        };
        let hash = node.digest();
        self.nodes.push((node, hash));
        hash
    }

    /// Create a contestability fork from `disputed_hash`.
    /// Marks the new node with `FORK_DISPUTED`.
    pub fn fork_node(
        &mut self,
        disputed_hash: [u8; 32],
        quins: &[NQuin],
        author_did: u64,
        timestamp_ms: u64,
        message: &str,
    ) -> [u8; 32] {
        let node = DagNode {
            parent_hash: disputed_hash,
            quins_merkle: quins_merkle(quins),
            author_did,
            timestamp: timestamp_ms,
            message_hash: q_hash(message) as u32,
            flags: FORK_DISPUTED,
        };
        let hash = node.digest();
        self.nodes.push((node, hash));
        hash
    }

    /// Point `branch_name` at `tip_hash`.  Returns an NQuin encoding the pointer
    /// for storage in BRANCHES_CONTEXT.
    pub fn write_branch_pointer(&mut self, branch_name: &str, tip_hash: [u8; 32]) -> NQuin {
        let name_hash = q_hash(branch_name);
        self.branches.insert(name_hash, tip_hash);
        // Encode the tip hash as two u64s XOR-folded into the object field.
        let tip_lo = u64::from_le_bytes(tip_hash[0..8].try_into().unwrap());
        let tip_hi = u64::from_le_bytes(tip_hash[8..16].try_into().unwrap());
        let folded = tip_lo ^ tip_hi;
        NQuin {
            subject: name_hash,
            predicate: P_BRANCH_TIP,
            object: folded,
            context: BRANCHES_CONTEXT,
            metadata: 0,
            parity: 0,
        }
    }

    /// Create a merge node spanning two parent branches.
    ///
    /// Returns `(primary_hash, secondary_hash)`:
    /// - `primary_hash`: the merge commit itself (`parent_hash = primary_parent`)
    /// - `secondary_hash`: a MERGE_SECONDARY back-link node whose `quins_merkle`
    ///   encodes `primary_hash` so the two branches stay bidirectionally linked
    ///
    /// Any conflict quins should be written to `crate::provenance::CONTEST_CONTEXT`
    /// before or after calling this function.
    pub fn merge_node(
        &mut self,
        primary_parent: [u8; 32],
        secondary_parent: [u8; 32],
        quins: &[NQuin],
        author_did: u64,
        timestamp_ms: u64,
        message: &str,
    ) -> ([u8; 32], [u8; 32]) {
        let msg_hash = q_hash(message) as u32;
        let merkle = quins_merkle(quins);

        let primary = DagNode {
            parent_hash: primary_parent,
            quins_merkle: merkle,
            author_did,
            timestamp: timestamp_ms,
            message_hash: msg_hash,
            flags: 0,
        };
        let primary_hash = primary.digest();
        self.nodes.push((primary, primary_hash));

        // Secondary back-link: parent = secondary branch tip; quins_merkle = primary_hash.
        let secondary = DagNode {
            parent_hash: secondary_parent,
            quins_merkle: primary_hash,
            author_did,
            timestamp: timestamp_ms,
            message_hash: msg_hash,
            flags: MERGE_SECONDARY,
        };
        let secondary_hash = secondary.digest();
        self.nodes.push((secondary, secondary_hash));

        (primary_hash, secondary_hash)
    }

    /// Return all node hashes with `timestamp ≤ as_of_ms` (assertion-time snapshot).
    ///
    /// Used by the SPARQL AS OF executor to reconstruct which DAG commits existed
    /// at a given point in time.
    pub fn nodes_as_of(&self, as_of_ms: u64) -> Vec<[u8; 32]> {
        self.nodes
            .iter()
            .filter(|(n, _)| n.timestamp <= as_of_ms)
            .map(|(_, h)| *h)
            .collect()
    }

    /// Return the current tip hash for `branch_name`, if set.
    pub fn branch_tip(&self, branch_name: &str) -> Option<[u8; 32]> {
        self.branches.get(&q_hash(branch_name)).copied()
    }

    /// Iterate nodes in insertion order.
    pub fn nodes(&self) -> &[(DagNode, [u8; 32])] {
        &self.nodes
    }

    /// Serialize the store to bytes for embedding in a Q42 volume.
    /// Format: `[u64 node_count] ([u8;88] node_bytes)*`
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.nodes.len() * 88);
        out.extend_from_slice(&(self.nodes.len() as u64).to_le_bytes());
        for (node, _hash) in &self.nodes {
            out.extend_from_slice(&node.to_bytes());
        }
        out
    }

    /// Deserialize from bytes previously written by `serialize`.
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 { return None; }
        let count = u64::from_le_bytes(bytes[0..8].try_into().ok()?) as usize;
        if bytes.len() < 8 + count * 88 { return None; }
        let mut nodes = Vec::with_capacity(count);
        for i in 0..count {
            let off = 8 + i * 88;
            let b: &[u8; 88] = bytes[off..off + 88].try_into().ok()?;
            let node = DagNode::from_bytes(b);
            let hash = node.digest();
            nodes.push((node, hash));
        }
        Some(Self { nodes, branches: std::collections::HashMap::new() })
    }
}

impl Default for DagStore {
    fn default() -> Self { Self::new() }
}

// ── git fast-export compatibility ─────────────────────────────────────────────

/// Generate a `git fast-export` compatible text stream from the DAG store.
///
/// Called with a `DagStore` populated from the volume's `dag_root_offset` section.
/// Falls back to a single placeholder commit when the store is empty (legacy behaviour).
pub fn generate_fast_export_stream(store: &DagStore) -> String {
    if store.nodes.is_empty() {
        return legacy_fast_export();
    }

    let mut stream = String::new();
    for (idx, (node, hash)) in store.nodes.iter().enumerate() {
        let mark = idx + 1;
        let hash_hex = hex::encode(hash);
        let ts_secs = node.timestamp / 1000;
        stream.push_str(&format!("commit refs/heads/main\nmark :{mark}\n"));
        stream.push_str(&format!(
            "committer unknown <did:key:{hash_hex}> {ts_secs} +0000\n"
        ));
        let msg = format!("quin commit {}\n", hex::encode(&node.quins_merkle[..8]));
        stream.push_str(&format!("data {}\n{msg}", msg.len()));
        if node.flags & FORK_DISPUTED != 0 {
            stream.push_str("# flags: FORK_DISPUTED\n");
        }
        let blob = format!(
            "{{\"quins_merkle\":\"{}\",\"author_did\":{},\"flags\":{}}}",
            hex::encode(node.quins_merkle),
            node.author_did,
            node.flags,
        );
        stream.push_str(&format!(
            "M 100644 inline dag_node_{mark}.json\ndata {}\n{blob}\n",
            blob.len()
        ));
    }
    stream
}

fn legacy_fast_export() -> String {
    let blob = "{\"financial\": 1200.00, \"labor_hours\": 45}";
    format!(
        "commit refs/heads/main\nmark :1\n\
         committer Alice <alice@did.key> 1717286400 +0000\n\
         data 36\nLog 4 hours of design obligation\n\
         M 100644 inline obligation_matrix.json\ndata {}\n{blob}\n",
        blob.len()
    )
}

/// Convenience wrapper: generate fast-export from a bare project ID string.
/// Creates an empty store and produces the legacy stream (backward-compatible shim).
pub fn generate_fast_export_stream_for_project(_project_id: &str) -> String {
    generate_fast_export_stream(&DagStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_commit_roundtrip() {
        let mut store = DagStore::new();
        let quins: Vec<NQuin> = Vec::new();
        let hash = store.genesis_node(&quins, 0xDEAD_BEEF, 1_717_286_400_000, "genesis");
        assert_ne!(hash, [0u8; 32]);
        assert_eq!(store.nodes().len(), 1);
        assert_eq!(store.nodes()[0].0.flags, GENESIS);
    }

    #[test]
    fn chain_commit_and_fork() {
        let mut store = DagStore::new();
        let genesis = store.genesis_node(&[], 1, 1000, "init");
        let c1 = store.commit_node(genesis, &[], 1, 2000, "add data");
        let fork = store.fork_node(c1, &[], 2, 3000, "contested");
        assert_eq!(store.nodes().len(), 3);
        assert_eq!(store.nodes()[2].0.flags, FORK_DISPUTED);
        assert_eq!(store.nodes()[2].0.parent_hash, c1);
        let _ = fork;
    }

    #[test]
    fn branch_pointer_is_retrievable() {
        let mut store = DagStore::new();
        let genesis = store.genesis_node(&[], 1, 1000, "init");
        let quin = store.write_branch_pointer("main", genesis);
        assert_eq!(quin.context, BRANCHES_CONTEXT);
        assert!(store.branch_tip("main").is_some());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut store = DagStore::new();
        store.genesis_node(&[], 42, 999, "first");
        let bytes = store.serialize();
        let restored = DagStore::deserialize(&bytes).expect("deser failed");
        assert_eq!(restored.nodes().len(), 1);
        assert_eq!(restored.nodes()[0].1, store.nodes()[0].1);
    }

    #[test]
    fn merge_node_produces_two_linked_nodes() {
        let mut store = DagStore::new();
        let branch_a = store.genesis_node(&[], 1, 1000, "branch-a init");
        let branch_b = store.genesis_node(&[], 2, 2000, "branch-b init");

        let (primary, secondary) = store.merge_node(branch_a, branch_b, &[], 1, 3000, "merge");

        assert_ne!(primary, secondary);
        assert_ne!(primary, [0u8; 32]);
        assert_ne!(secondary, [0u8; 32]);

        // Primary node must have branch_a as its parent.
        let primary_node = store.nodes().iter().find(|(_, h)| *h == primary).unwrap().0;
        assert_eq!(primary_node.parent_hash, branch_a);
        assert_eq!(primary_node.flags, 0);

        // Secondary node must have branch_b as parent and encode primary_hash in quins_merkle.
        let secondary_node = store.nodes().iter().find(|(_, h)| *h == secondary).unwrap().0;
        assert_eq!(secondary_node.parent_hash, branch_b);
        assert_eq!(secondary_node.flags, MERGE_SECONDARY);
        assert_eq!(secondary_node.quins_merkle, primary);
    }

    #[test]
    fn nodes_as_of_filters_by_timestamp() {
        let mut store = DagStore::new();
        store.genesis_node(&[], 1, 1000, "t=1000");
        store.commit_node([0u8; 32], &[], 1, 5000, "t=5000");
        store.commit_node([0u8; 32], &[], 1, 9000, "t=9000");

        let snapshot = store.nodes_as_of(5000);
        assert_eq!(snapshot.len(), 2, "should include nodes at t≤5000");

        let full = store.nodes_as_of(u64::MAX);
        assert_eq!(full.len(), 3);
    }

    #[test]
    fn dag_node_size() {
        assert_eq!(std::mem::size_of::<DagNode>(), 88);
    }

    #[test]
    fn fast_export_uses_real_nodes() {
        let mut store = DagStore::new();
        store.genesis_node(&[], 7, 1_000_000, "test commit");
        let export = generate_fast_export_stream(&store);
        assert!(export.contains("commit refs/heads/main"));
        assert!(export.contains("quins_merkle"));
    }
}
