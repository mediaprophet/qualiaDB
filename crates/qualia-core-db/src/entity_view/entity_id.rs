//! Entity-centric identity for mindware HID projections.
//!
//! Stable `u64` subjects in the same 60-bit FNV space as hypermedia assets
//! (`crate::hypermedia::fnv60`) and `q_hash` IRIs.

use crate::hypermedia::fnv60;
use crate::q_hash;
use serde::{Deserialize, Serialize};

/// Stable entity id for selection, scene nodes, and attribution subjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub struct EntityId(pub u64);

impl EntityId {
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn is_unset(self) -> bool {
        self.0 == 0
    }

    /// Content-addressed id from arbitrary bytes (URI, span, DID payload).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(fnv60(bytes))
    }

    /// IRI / URI string (http, model://, ontology://, asset uri).
    pub fn from_uri(uri: &str) -> Self {
        Self::from_bytes(uri.trim().as_bytes())
    }

    /// Compile-time style IRI hash via `q_hash` (full u64; not masked to 60 bits).
    /// Prefer `from_uri` for hypermedia subject alignment with `fnv60`.
    pub fn from_iri_q_hash(iri: &str) -> Self {
        Self(q_hash(iri))
    }

    /// Agent / natural-person DID string.
    pub fn from_did(did: &str) -> Self {
        Self::from_uri(did)
    }

    /// Fragment / span element: parent uri + stable local key (offsets, claim id).
    pub fn from_fragment(parent_uri: &str, fragment_key: &str) -> Self {
        let mut buf = Vec::with_capacity(parent_uri.len() + fragment_key.len() + 1);
        buf.extend_from_slice(parent_uri.as_bytes());
        buf.push(0x1f);
        buf.extend_from_slice(fragment_key.as_bytes());
        Self::from_bytes(&buf)
    }
}

/// Coarse kind for projection / layout (not a full ontology class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EntityKind {
    #[default]
    Unknown = 0,
    /// Natural person principal or peer.
    Agent = 1,
    /// Software instrument / sub-agent.
    Instrument = 2,
    /// Lived-memory / hypermedia asset.
    Asset = 3,
    /// Web locus (https URL).
    WebLocus = 4,
    /// Fragment / span / claim element.
    Fragment = 5,
    /// Commitment / deontic norm handle.
    Commitment = 6,
    /// World layer / stratum.
    Layer = 7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_ids_are_stable_and_distinct() {
        let a = EntityId::from_uri("https://example.org/a");
        let b = EntityId::from_uri("https://example.org/b");
        assert_eq!(a, EntityId::from_uri("https://example.org/a"));
        assert_ne!(a, b);
        assert!(!a.is_unset());
    }

    #[test]
    fn fragment_differs_from_parent() {
        let parent = EntityId::from_uri("urn:doc:lesson");
        let frag = EntityId::from_fragment("urn:doc:lesson", "span:10-40");
        assert_ne!(parent, frag);
    }
}
