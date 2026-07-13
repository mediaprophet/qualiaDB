//! Modal-predicate identifier-KIND resolution (task #22; the hybrid-modality decision).
//!
//! The OPEN, extensible set of identifier *kinds* / namespaces lives in the graph as a
//! modal predicate `<identifier> <hasModalityKind> <kind>`, NOT in the inline object
//! tag (the top nibble is reserved for the small CLOSED set of structural datatypes —
//! see `frame_layout` "Tag policy"). Two payoffs:
//!
//! * an identifier keeps its FULL 64-bit width (the inline-tag path must spend the top
//!   nibble on a datatype tag; this path does not), so non-dictionary identifiers
//!   (content hashes, topological/did:q42 pointers, cluster-node ids) lose no entropy;
//! * the kind emerges from a relation (identifiers-not-identity), resolved via a
//!   zero-alloc `QuinIndex::object_of` point lookup at the CPU/logic layer — never in
//!   the SIMD/GPU vectorized loop.
//!
//! The lexicon is the collision backstop underneath: a handle resolves to a full value,
//! so a handle collision is detectable, not silent.

use crate::indexing::QuinIndex;
use crate::{q_hash, NQuin};

/// The modal predicate that scopes an identifier's kind.
pub const HAS_MODALITY_KIND: u64 = q_hash("https://ns.webcivics.net/cml/hasModalityKind");

// ── Open kind vocabulary ─────────────────────────────────────────────────────────
// Not exhaustive — new kinds are added as graph terms, never as new inline-tag bits.
pub const KIND_DICTIONARY: u64 = q_hash("https://ns.webcivics.net/kind/DictionaryHash");
pub const KIND_WEBIZEN: u64 = q_hash("https://ns.webcivics.net/kind/WebizenId");
pub const KIND_DID_Q42: u64 = q_hash("https://ns.webcivics.net/kind/DidQ42");
pub const KIND_DID: u64 = q_hash("https://ns.webcivics.net/kind/Did");
pub const KIND_CONTENT_HASH: u64 = q_hash("https://ns.webcivics.net/kind/ContentHash");
pub const KIND_CLUSTER_NODE: u64 = q_hash("https://ns.webcivics.net/kind/ClusterNode");

/// Build the modal-kind quin asserting `identifier` is of `kind`.
///
/// `identifier` may use its FULL 64-bit width — the kind is carried externally, so no
/// top-nibble datatype tag is reserved here (unlike an inline-tagged object value).
#[inline]
pub fn tag_kind(identifier: u64, kind: u64) -> NQuin {
    NQuin {
        subject: identifier,
        predicate: HAS_MODALITY_KIND,
        object: kind,
        context: 0,
        metadata: 0,
        parity: identifier ^ HAS_MODALITY_KIND ^ kind,
    }
}

/// Resolve an identifier's kind via a zero-alloc point lookup.
///
/// `None` = unkinded — treat as a plain dictionary reference (or defer to the lexicon).
#[inline]
pub fn resolve_kind(index: &QuinIndex, identifier: u64) -> Option<u64> {
    index.object_of(identifier, HAS_MODALITY_KIND)
}

/// Human-readable name for a known kind constant (`None` for an extension kind not in
/// the seed vocabulary — those are still valid, just not built-in).
pub fn kind_name(kind: u64) -> Option<&'static str> {
    match kind {
        KIND_DICTIONARY => Some("DictionaryHash"),
        KIND_WEBIZEN => Some("WebizenId"),
        KIND_DID_Q42 => Some("DidQ42"),
        KIND_DID => Some("Did"),
        KIND_CONTENT_HASH => Some("ContentHash"),
        KIND_CLUSTER_NODE => Some("ClusterNode"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_dictionary_identifier_kind() {
        let alice = q_hash("https://example.org/alice"); // a 60-bit dictionary identifier
        let idx = QuinIndex::from_slice(&[tag_kind(alice, KIND_DICTIONARY)]);
        assert_eq!(resolve_kind(&idx, alice), Some(KIND_DICTIONARY));
    }

    #[test]
    fn resolves_full_64bit_identifier_kind() {
        // A full-width identifier with the top nibble SET (e.g. a content hash or a
        // topological/did:q42 pointer). The inline-tag path could not carry this — it
        // needs the top nibble for the datatype tag. The modal-predicate path resolves
        // its kind regardless of width: this is the "more identifiers" payoff.
        let content_id: u64 = 0xF234_5678_9ABC_DEF0;
        assert_ne!(content_id >> 60, 0, "test id genuinely uses the top nibble");
        let idx = QuinIndex::from_slice(&[tag_kind(content_id, KIND_CONTENT_HASH)]);
        assert_eq!(resolve_kind(&idx, content_id), Some(KIND_CONTENT_HASH));
    }

    #[test]
    fn distinct_identifiers_keep_distinct_kinds() {
        let a = q_hash("did:q42:aaa");
        let b: u64 = 0x8000_0000_0000_0001; // a full-width Webizen-style identifier
        let idx = QuinIndex::from_slice(&[tag_kind(a, KIND_DID_Q42), tag_kind(b, KIND_WEBIZEN)]);
        assert_eq!(resolve_kind(&idx, a), Some(KIND_DID_Q42));
        assert_eq!(resolve_kind(&idx, b), Some(KIND_WEBIZEN));
    }

    #[test]
    fn unkinded_identifier_resolves_none() {
        let idx = QuinIndex::from_slice(&[]);
        assert_eq!(resolve_kind(&idx, q_hash("x")), None);
    }

    #[test]
    fn kinds_are_distinct_and_tag_free() {
        // Distinct kinds, and every kind is itself a pure 60-bit identifier (no tag spill).
        let kinds = [
            KIND_DICTIONARY,
            KIND_WEBIZEN,
            KIND_DID_Q42,
            KIND_DID,
            KIND_CONTENT_HASH,
            KIND_CLUSTER_NODE,
            HAS_MODALITY_KIND,
        ];
        for (i, a) in kinds.iter().enumerate() {
            assert_eq!(a >> 60, 0, "kind term must be a pure 60-bit identifier");
            for b in &kinds[i + 1..] {
                assert_ne!(a, b, "kind terms must be distinct");
            }
        }
    }
}
