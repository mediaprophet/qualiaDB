//! Multi-modal semantic binding **logic** (§29, legal_logic.md) — the content-addressed
//! carrier binding + extraction.
//!
//! SCOPE (honest): this is the tamper-evident *binding* between a media blob and its semantic
//! graph, plus extraction — the part that matters for provenance and evidence. The actual
//! binary *container codecs* (PDF/A-3, XMP, PNG, Open Badges v3 byte layout) are task #9; this
//! module does NOT write those container formats. It content-addresses the blob (real BLAKE3)
//! and verifies that the carried graph is bound to *that exact* media (any edit breaks it).

use crate::NQuin;

/// Content-address a media blob → a 64-bit media tag (the low 8 bytes of its BLAKE3 hash, into
/// the one identifier space). `Hash(Blob) → Tag_Media`. Real cryptographic hash, not a toy.
pub fn media_tag(blob: &[u8]) -> u64 {
    let h = blake3::hash(blob);
    let b = h.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Verify a carrier's binding: re-hash `blob` and confirm it matches the `bound_media_tag` the
/// carrier recorded. Tamper-evident — any change to the media breaks the binding to its graph.
#[inline]
pub fn verify_binding(blob: &[u8], bound_media_tag: u64) -> bool {
    media_tag(blob) == bound_media_tag
}

/// Extract the payload quins carried alongside a medium into `out` (`Extract(C_VC) → Σ(Quins)`).
/// Returns the count written. Zero-heap (caller-supplied `out`).
pub fn extract_payload(payload: &[NQuin], out: &mut [NQuin]) -> usize {
    let n = payload.len().min(out.len());
    out[..n].copy_from_slice(&payload[..n]);
    n
}

// ─── Merkle-DAG content addressing (IPLD-equivalent) ──────────────────────────────

/// Hash an internal Merkle-DAG node from its ordered `children` tags: BLAKE3 over the
/// little-endian concatenation. Order-sensitive (a node commits to its ordered children).
/// Zero-heap (fixed-size BLAKE3 state on the stack).
pub fn merkle_node(children: &[u64]) -> u64 {
    let mut hasher = blake3::Hasher::new();
    for &c in children {
        hasher.update(&c.to_le_bytes());
    }
    u64::from_le_bytes(hasher.finalize().as_bytes()[..8].try_into().unwrap())
}

/// Verify a Merkle-DAG node tag against its children (recompute + compare). Tamper-evident.
#[inline]
pub fn verify_merkle_node(node_tag: u64, children: &[u64]) -> bool {
    merkle_node(children) == node_tag
}

// ─── Streaming hash (massive blobs, bypassing RAM) ────────────────────────────────

/// A streaming content-hash accumulator — feed chunks of a massive blob (e.g. via DirectStorage)
/// without ever holding it whole in RAM, then finalize to the 64-bit media tag. Zero-heap (the
/// BLAKE3 state is fixed-size on the stack; chunks are borrowed, not retained).
#[derive(Default)]
pub struct StreamHasher {
    inner: blake3::Hasher,
}

impl StreamHasher {
    pub fn new() -> Self {
        Self { inner: blake3::Hasher::new() }
    }
    /// Feed the next chunk.
    pub fn update(&mut self, chunk: &[u8]) {
        self.inner.update(chunk);
    }
    /// Finalize to the media tag. Equals [`media_tag`] over the concatenated chunks.
    pub fn finalize(&self) -> u64 {
        u64::from_le_bytes(self.inner.finalize().as_bytes()[..8].try_into().unwrap())
    }
}

// ─── Cryptographic multi-signature over the payload ───────────────────────────────

/// A **k-of-n multi-signature** over the payload is satisfied iff at least `k` distinct valid
/// signer attestations are present. The individual signatures are verified by the crypto layer
/// (Ed25519 / post-quantum ML-DSA via `fiduciary_crypto`); this is the threshold gate.
/// `valid_signers` = the count of distinct verified signers.
#[inline]
pub fn multisig_satisfied(valid_signers: usize, k: usize) -> bool {
    k > 0 && valid_signers >= k
}

// ─── Verifiable redaction ─────────────────────────────────────────────────────────

/// **Verifiable redaction**: a redacted blob hides content while preserving the signature/binding.
/// Each leaf is committed by its hash in a Merkle root; redacting a leaf replaces its *content*
/// with its *hash tag* — the leaf tags (redacted or not) still recompute the original `root`.
/// Returns true iff the (possibly-redacted) `leaf_tags` still hash to `original_root`.
pub fn redaction_preserves_root(leaf_tags: &[u64], original_root: u64) -> bool {
    merkle_node(leaf_tags) == original_root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quin(s: u64, o: u64) -> NQuin {
        let mut q = NQuin { subject: s, predicate: 7, object: o, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn media_tag_is_deterministic_and_content_addressed() {
        let blob = b"a signed evidentiary photograph's bytes";
        let tag = media_tag(blob);
        assert_eq!(tag, media_tag(blob), "deterministic");
        assert_ne!(tag, media_tag(b"different bytes"), "content-addressed");
    }

    #[test]
    fn binding_is_tamper_evident() {
        let blob = b"original media";
        let tag = media_tag(blob);
        assert!(verify_binding(blob, tag), "intact media verifies");
        assert!(!verify_binding(b"tampered media", tag), "any edit breaks the binding");
    }

    #[test]
    fn payload_extracts_round_trip() {
        let payload = [quin(1, 2), quin(3, 4)];
        let mut out = [NQuin::default(); 4];
        let n = extract_payload(&payload, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0].subject, 1);
        assert_eq!(out[1].object, 4);
    }

    #[test]
    fn merkle_dag_node_is_order_sensitive_and_tamper_evident() {
        let a = media_tag(b"leaf-a");
        let b = media_tag(b"leaf-b");
        let root = merkle_node(&[a, b]);
        assert!(verify_merkle_node(root, &[a, b]));
        assert_ne!(merkle_node(&[a, b]), merkle_node(&[b, a]), "order matters in a DAG");
        assert!(!verify_merkle_node(root, &[a, media_tag(b"tampered")]), "any child change breaks it");
    }

    #[test]
    fn streaming_hash_equals_one_shot() {
        let blob = b"a very large evidentiary recording streamed in chunks";
        let mut sh = StreamHasher::new();
        sh.update(&blob[..10]);
        sh.update(&blob[10..30]);
        sh.update(&blob[30..]);
        assert_eq!(sh.finalize(), media_tag(blob), "streaming == one-shot media_tag");
    }

    #[test]
    fn multisig_threshold_and_verifiable_redaction() {
        // 2-of-3 multisig.
        assert!(multisig_satisfied(2, 2));
        assert!(multisig_satisfied(3, 2));
        assert!(!multisig_satisfied(1, 2));
        assert!(!multisig_satisfied(3, 0), "a zero threshold is invalid");

        // Redaction: replacing a leaf's content with its hash tag preserves the root.
        let l0 = media_tag(b"public clause");
        let l1 = media_tag(b"private medical detail");
        let root = merkle_node(&[l0, l1]);
        // The "redacted" view carries l1's TAG (not its content) → same tags → same root verifies.
        assert!(redaction_preserves_root(&[l0, l1], root));
        // Substituting a different tag (forging the redaction) fails.
        assert!(!redaction_preserves_root(&[l0, media_tag(b"forged")], root));
    }
}
