//! Lexicon Dictionary Manager
//! Maps multi-modal semantic concepts (Text, Audio, Visual) into the deterministic 60-bit integers
//! required by the NQuin data structure.

// Re-export the embedded triple tag from resolver for SPARQL-Star support
pub use crate::resolver::{TAG_EMBEDDED, TAG_WEBIZEN};

/// Represents the pluralistic forms that a semantic concept can take.
/// We explicitly reject the assumption that knowledge is exclusively bound to Unicode strings.
pub enum SemanticModality<'a> {
    Text(&'a str),
    AudioHash(&'a [u8]),        // For mother tongues / oral traditions
    CeremonialVisual(&'a [u8]), // For heraldry / visual concepts
    PhoneticSchema(&'a [u8]),   // For non-western phonetics
}

/// Generates a deterministic, collision-resistant 60-bit token from a raw byte stream.
/// Uses a custom FNV-1a inspired hash restricted to 60 bits.
#[inline(always)]
pub fn generate_60bit_token(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    // Truncate to 60 bits (the top 4 bits are reserved for datatype tags in the O vector)
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// Generates a Virtual ID for a SPARQL-Star embedded triple <<s p o>>.
/// 
/// This function serializes the three u64 component IDs into a 24-byte array
/// and hashes them using FNV-1a, then tags the result with TAG_EMBEDDED.
/// 
/// # Arguments
/// * subject - The subject u64 ID
/// * predicate - The predicate u64 ID  
/// * object - The object u64 ID
/// 
/// # Returns
/// A 64-bit Virtual ID with the TAG_EMBEDDED bit set, suitable for
/// storage in the Subject or Object position of a NQuin.
#[inline(always)]
pub fn generate_embedded_triple_id(subject: u64, predicate: u64, object: u64) -> u64 {
    // Serialize the three u64 IDs into a 24-byte array
    let mut bytes = [0u8; 24];
    bytes[0..8].copy_from_slice(&subject.to_le_bytes());
    bytes[8..16].copy_from_slice(&predicate.to_le_bytes());
    bytes[16..24].copy_from_slice(&object.to_le_bytes());
    
    // Hash the bytes and tag with EMBEDDED marker
    generate_60bit_token(&bytes) | TAG_EMBEDDED
}


/// In-memory Lexicon manager to handle reverse lookups in the future.
/// For now, ingestion purely maps forward (Bytes -> u64) via the hash.
pub struct LexiconManager {
    // A production database would memory-map a reverse lookup file here (u64 -> Modality)
}

impl LexiconManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Converts a multi-modal semantic concept into its 60-bit hardware representation.
    pub fn tokenize_modal(&self, modality: &SemanticModality) -> u64 {
        match modality {
            SemanticModality::Text(text) => generate_60bit_token(text.as_bytes()),
            SemanticModality::AudioHash(bytes) => generate_60bit_token(bytes),
            SemanticModality::CeremonialVisual(bytes) => generate_60bit_token(bytes),
            SemanticModality::PhoneticSchema(bytes) => generate_60bit_token(bytes),
        }
    }

    /// Legacy support for text strings
    pub fn tokenize(&self, literal: &str) -> u64 {
        self.tokenize_modal(&SemanticModality::Text(literal))
    }
}

use std::collections::HashMap;

/// Outcome of interning a `(handle, value)` pair into a collision-aware lexicon.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Intern {
    /// First time this handle is seen.
    New,
    /// Same handle, SAME value — an idempotent re-intern.
    Seen,
    /// Same handle, DIFFERENT value — a genuine handle collision (both values kept).
    Collision,
}

/// Collision-aware string interner — the lexicon collision backstop (task #22).
///
/// The plain build-time map (`HashMap<u64, String>` in `external_sort`) silently
/// OVERWRITES on a handle collision (two distinct strings hashing to the same 60-bit
/// token), losing data with no signal. This interner DETECTS the collision at intern
/// time — O(1), off the resolution hot path — and KEEPS BOTH values, so a collision
/// becomes loud + recoverable rather than silent corruption. Resolution stays
/// single-value and fast for the (overwhelmingly common) collision-free handles; only
/// a flagged handle pays a comparison over its small bucket (length-gated by `str` eq),
/// from memory. This is a host-side structure (`HashMap`/`String`), separate from the
/// 42 MB `SlgArena` — its allocations are one-time intern cost, not per-resolution.
#[derive(Default)]
pub struct LexiconInterner {
    /// handle -> first-interned value (the fast, common path).
    map: HashMap<u64, String>,
    /// handle -> all distinct values, populated ONLY when a collision occurs.
    buckets: HashMap<u64, Vec<String>>,
}

impl LexiconInterner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `(generate_60bit_token(value), value)`. Returns the collision outcome.
    pub fn intern_str(&mut self, value: &str) -> Intern {
        self.intern(generate_60bit_token(value.as_bytes()), value)
    }

    /// Intern an explicit `(handle, value)` pair — the handle need not be
    /// `generate_60bit_token(value)` (e.g. did:q42 / Webizen handles). Detects a
    /// same-handle / different-value collision and preserves both values.
    pub fn intern(&mut self, handle: u64, value: &str) -> Intern {
        match self.map.get(&handle) {
            None => {
                self.map.insert(handle, value.to_string());
                Intern::New
            }
            Some(existing) if existing == value => Intern::Seen,
            Some(existing) => {
                // Genuine collision: preserve BOTH values; never silently overwrite.
                let bucket = self
                    .buckets
                    .entry(handle)
                    .or_insert_with(|| vec![existing.clone()]);
                if !bucket.iter().any(|v| v == value) {
                    bucket.push(value.to_string());
                }
                Intern::Collision
            }
        }
    }

    /// Whether `handle` has more than one distinct interned value (a collision).
    #[inline]
    pub fn is_collision(&self, handle: u64) -> bool {
        self.buckets.contains_key(&handle)
    }

    /// Resolve `handle` -> value with collision awareness. Collision-free (common): the
    /// single value, no comparison. Collided (rare): `None` — there is no single answer,
    /// so the caller disambiguates with the query value via [`Self::resolve_value`]
    /// rather than receiving a silently-wrong one.
    pub fn resolve(&self, handle: u64) -> Option<&str> {
        if self.buckets.contains_key(&handle) {
            return None;
        }
        self.map.get(&handle).map(String::as_str)
    }

    /// Disambiguating resolve — the backstop. Returns the stored value equal to `query`
    /// iff `handle` interns it. `str` equality length-gates before the byte compare, so
    /// the rare collision path stays cheap.
    pub fn resolve_value(&self, handle: u64, query: &str) -> Option<&str> {
        if let Some(bucket) = self.buckets.get(&handle) {
            return bucket.iter().map(String::as_str).find(|&s| s == query);
        }
        self.map
            .get(&handle)
            .map(String::as_str)
            .filter(|&s| s == query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_60bit_truncation() {
        let uri = "https://mediaprophet.github.io/qualiaDB/user/123";
        let token = generate_60bit_token(uri.as_bytes());

        // Ensure the top 4 bits are strictly 0 (Lexicon ID datatype tag)
        assert_eq!(token >> 60, 0, "Token spilled over 60 bits");
    }

    /// Regression guard for the hash-space unification (task #14): the compile-time
    /// `q_hash` and the runtime `generate_60bit_token` MUST be one identity space,
    /// or compile-time-baked URIs (deontic/values/MCP) won't join runtime-parsed/
    /// ingested URIs (turtle/SPARQL/SHACL/corpus). If this ever fails, the two
    /// drifted apart again — fix the divergence, don't relax the test.
    #[test]
    fn q_hash_and_generate_60bit_token_are_one_space() {
        for s in [
            "",
            "Bob",
            "http://www.w3.org/ns/shacl#maxInclusive",
            "https://ns.webcivics.net/values/NaturalPerson",
            "q42:Principal",
            "naïve—prière—母語", // non-ASCII must agree too
        ] {
            assert_eq!(
                crate::q_hash(s),
                generate_60bit_token(s.as_bytes()),
                "q_hash and generate_60bit_token diverged for {s:?}",
            );
            // Both are pure 60-bit identifiers: top 4 bits reserved for the tag overlay.
            assert_eq!(crate::q_hash(s) >> 60, 0, "q_hash spilled past 60 bits for {s:?}");
        }
    }

    #[test]
    fn interner_detects_collision_and_keeps_both_values() {
        let mut lx = LexiconInterner::new();
        // Force a handle collision (same handle, different values) by interning explicit
        // handles — a natural 60-bit FNV collision is infeasible to find in a test.
        let h = 0x0ABC_DEF0_1234_5678;
        assert_eq!(lx.intern(h, "alpha"), Intern::New);
        assert_eq!(lx.intern(h, "alpha"), Intern::Seen); // idempotent
        assert_eq!(lx.intern(h, "beta"), Intern::Collision); // genuine collision
        assert!(lx.is_collision(h));

        // Both values survive and are recoverable via the disambiguating resolve —
        // unlike a plain HashMap, where "alpha" would have been silently overwritten.
        assert_eq!(lx.resolve_value(h, "alpha"), Some("alpha"));
        assert_eq!(lx.resolve_value(h, "beta"), Some("beta"));
        assert_eq!(lx.resolve_value(h, "gamma"), None);

        // Bare resolve refuses to guess on an ambiguous handle (no silent wrong answer).
        assert_eq!(lx.resolve(h), None);
    }

    #[test]
    fn interner_collision_free_is_the_fast_single_value_path() {
        let mut lx = LexiconInterner::new();
        let iri = "https://ns.webcivics.net/values/State";
        assert_eq!(lx.intern_str(iri), Intern::New);
        assert_eq!(lx.intern_str(iri), Intern::Seen);

        // q_hash == generate_60bit_token (post-#14), so this is the interned handle.
        let h = crate::q_hash(iri);
        assert!(!lx.is_collision(h));
        // Collision-free: bare resolve returns the single value directly, no comparison.
        assert_eq!(lx.resolve(h), Some(iri));
    }

    #[test]
    fn test_determinism() {
        let uri = "qualia:guardian";
        let t1 = generate_60bit_token(uri.as_bytes());
        let t2 = generate_60bit_token(uri.as_bytes());
        assert_eq!(t1, t2, "Tokens are not deterministic");
    }

    #[test]
    fn test_linguistic_plurality() {
        let lexicon = LexiconManager::new();

        // Simulating a written concept
        let written = SemanticModality::Text("peace_infrastructure");
        let t1 = lexicon.tokenize_modal(&written);

        // Simulating the exact same concept represented as a cryptographic audio hash of a spoken prayer
        let audio_hash = vec![0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let oral = SemanticModality::AudioHash(&audio_hash);
        let t2 = lexicon.tokenize_modal(&oral);

        // Simulating a ceremonial SVG file representation
        let svg_bytes = b"<svg>Heraldry</svg>";
        let visual = SemanticModality::CeremonialVisual(svg_bytes);
        let t3 = lexicon.tokenize_modal(&visual);

        // Prove that the database treats all modalities as valid 60-bit structural Quins
        assert!(t1 > 0 && t1 <= 0x0FFF_FFFF_FFFF_FFFF);
        assert!(t2 > 0 && t2 <= 0x0FFF_FFFF_FFFF_FFFF);
        assert!(t3 > 0 && t3 <= 0x0FFF_FFFF_FFFF_FFFF);

        // Prove that changing the audio hash creates a unique identifier
        let altered_audio = vec![0x1a, 0x2b, 0x3c, 0x4d, 0x5f];
        let altered_oral = SemanticModality::AudioHash(&altered_audio);
        let t4 = lexicon.tokenize_modal(&altered_oral);
        assert_ne!(t2, t4, "Collision in multi-modal hashing");
    }
}
