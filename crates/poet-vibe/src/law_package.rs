//! Law packages as signed content (T72, W10).
//!
//! A law declaration carries more than just the condition and
//! consequence — it carries provenance: who authored it, under what
//! licence, whether it is physical or fictional, and a cryptographic
//! signature binding the author to the law's content.
//!
//! Without signed law packages, any agent can inject a "law" that
//! says `transform.dissolve(self)` without anyone knowing who wrote
//! it or whether it's physically valid. Signed law packages make
//! the provenance travel with the law.
//!
//! ## Structure
//!
//! A `LawPackage` wraps a `LawDecl` with:
//! - **Author DID**: who wrote the law.
//! - **Licence IRI**: under what terms the law may be used.
//! - **Physical validity**: is this a physical law (conservation-
//!   respecting) or a fictional/game rule?
//! - **Signature**: Ed25519 signature over the law's canonical
//!   encoding, binding the author to the content.
//! - **Asserted at**: when the law was authored (Instant).
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.14 T72,
//! excellence-first §3.8, W10.

use crate::value::{Instant, Value};
use std::collections::BTreeMap;

/// Whether a law is physically valid or a fictional/game rule (T72).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LawKind {
    /// A physical law — respects conservation of mass, energy, charge,
    /// etc. Violations are diagnostic errors, not gameplay.
    #[default]
    Physical,
    /// A fictional/game rule — may violate physics for narrative or
    /// gameplay purposes. Provenance still travels, but the kind is
    /// explicit so physics simulations can reject it.
    Fictional,
}

impl LawKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::Fictional => "fictional",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "physical" => Some(Self::Physical),
            "fictional" => Some(Self::Fictional),
            _ => None,
        }
    }
}

/// A signed law package — a law declaration with provenance and
/// cryptographic signature (T72, W10).
#[derive(Debug, Clone, PartialEq)]
pub struct LawPackage {
    /// The law's name (matches LawDecl.name).
    pub name: String,
    /// The law's condition (canonical text form).
    pub condition_text: String,
    /// The law's consequence (canonical text form).
    pub consequence_text: String,
    /// Who authored the law (DID or IRI).
    pub author: String,
    /// Under what licence the law may be used (IRI).
    pub licence: String,
    /// Whether the law is physical or fictional.
    pub kind: LawKind,
    /// When the law was authored.
    pub asserted_at: Instant,
    /// Optional Ed25519 signature over the canonical encoding.
    pub signature: Option<[u8; 64]>,
    /// Optional version tag for law evolution.
    pub version: Option<String>,
}

impl LawPackage {
    /// Create a new unsigned law package.
    pub fn new(
        name: &str,
        condition: &str,
        consequence: &str,
        author: &str,
        licence: &str,
        kind: LawKind,
        asserted_at: Instant,
    ) -> Self {
        Self {
            name: name.into(),
            condition_text: condition.into(),
            consequence_text: consequence.into(),
            author: author.into(),
            licence: licence.into(),
            kind,
            asserted_at,
            signature: None,
            version: None,
        }
    }

    /// Attach a signature to the law package.
    pub fn with_signature(mut self, sig: [u8; 64]) -> Self {
        self.signature = Some(sig);
        self
    }

    /// Attach a version tag.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Is the law package signed?
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Canonical encoding for signing — the bytes that the signature
    /// covers. This is a deterministic encoding of the law's content
    /// (name + condition + consequence + author + licence + kind +
    /// asserted_at).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(self.name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.condition_text.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.consequence_text.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.author.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.licence.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.kind.as_str().as_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.asserted_at.secs.to_le_bytes());
        buf.extend_from_slice(&self.asserted_at.nanos.to_le_bytes());
        if let Some(ref v) = self.version {
            buf.push(0);
            buf.extend_from_slice(v.as_bytes());
        }
        buf
    }

    /// Convert to a Value::Record for inspection and graph storage.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("condition".into(), Value::String(self.condition_text.clone()));
        rec.insert("consequence".into(), Value::String(self.consequence_text.clone()));
        rec.insert("author".into(), Value::String(self.author.clone()));
        rec.insert("licence".into(), Value::String(self.licence.clone()));
        rec.insert("kind".into(), Value::String(self.kind.as_str().into()));
        rec.insert("asserted_at".into(), Value::Instant(self.asserted_at.clone()));
        rec.insert("is_signed".into(), Value::Bool(self.is_signed()));
        if let Some(ref v) = self.version {
            rec.insert("version".into(), Value::String(v.clone()));
        }
        Value::Record(rec)
    }
}

/// A registry of law packages — a law store (T72, W10).
#[derive(Debug, Clone, Default)]
pub struct LawStore {
    laws: Vec<LawPackage>,
}

impl LawStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a law package.
    pub fn register(&mut self, law: LawPackage) -> &mut Self {
        self.laws.push(law);
        self
    }

    /// Get all laws.
    pub fn all(&self) -> &[LawPackage] {
        &self.laws
    }

    /// Get laws by author.
    pub fn by_author(&self, author: &str) -> Vec<&LawPackage> {
        self.laws.iter().filter(|l| l.author == author).collect()
    }

    /// Get laws by kind (physical or fictional).
    pub fn by_kind(&self, kind: LawKind) -> Vec<&LawPackage> {
        self.laws.iter().filter(|l| l.kind == kind).collect()
    }

    /// Get a law by name.
    pub fn get(&self, name: &str) -> Option<&LawPackage> {
        self.laws.iter().find(|l| l.name == name)
    }

    /// Get only signed laws.
    pub fn signed(&self) -> Vec<&LawPackage> {
        self.laws.iter().filter(|l| l.is_signed()).collect()
    }

    /// Number of laws.
    pub fn len(&self) -> usize {
        self.laws.len()
    }

    /// Is the store empty?
    pub fn is_empty(&self) -> bool {
        self.laws.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instant() -> Instant {
        Instant::unix(1_000_000_000, 0)
    }

    // ── LawKind tests ─────────────────────────────────────────────────

    #[test]
    fn t72_law_kind_round_trip() {
        for k in &[LawKind::Physical, LawKind::Fictional] {
            let s = k.as_str();
            assert_eq!(LawKind::from_str(s), Some(*k));
        }
        assert_eq!(LawKind::from_str("unknown"), None);
    }

    #[test]
    fn t72_default_is_physical() {
        assert_eq!(LawKind::default(), LawKind::Physical);
    }

    // ── LawPackage tests ──────────────────────────────────────────────

    #[test]
    fn t72_law_package_basic() {
        let law = LawPackage::new(
            "crush",
            "when sample(pressure_ambient, pose(self)) > self.material.yield",
            "transform.yield(self)",
            "did:alice",
            "https://opensource.org/licenses/MIT",
            LawKind::Physical,
            make_instant(),
        );
        assert_eq!(law.name, "crush");
        assert_eq!(law.author, "did:alice");
        assert_eq!(law.kind, LawKind::Physical);
        assert!(!law.is_signed());
        assert!(law.version.is_none());
    }

    #[test]
    fn t72_law_package_with_signature() {
        let law = LawPackage::new(
            "crush",
            "condition",
            "consequence",
            "did:alice",
            "MIT",
            LawKind::Physical,
            make_instant(),
        )
        .with_signature([0u8; 64]);
        assert!(law.is_signed());
    }

    #[test]
    fn t72_law_package_with_version() {
        let law = LawPackage::new(
            "crush",
            "condition",
            "consequence",
            "did:alice",
            "MIT",
            LawKind::Physical,
            make_instant(),
        )
        .with_version("1.0.0");
        assert_eq!(law.version, Some("1.0.0".into()));
    }

    #[test]
    fn t72_law_package_fictional() {
        let law = LawPackage::new(
            "respawn",
            "when self.hp <= 0",
            "transform.respawn(self)",
            "did:game-dev",
            "https://example.com/game-licence",
            LawKind::Fictional,
            make_instant(),
        );
        assert_eq!(law.kind, LawKind::Fictional);
    }

    #[test]
    fn t72_canonical_bytes_deterministic() {
        let law1 = LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        );
        let law2 = LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        );
        assert_eq!(law1.canonical_bytes(), law2.canonical_bytes());
    }

    #[test]
    fn t72_canonical_bytes_differ_on_content() {
        let law1 = LawPackage::new(
            "crush", "cond1", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        );
        let law2 = LawPackage::new(
            "crush", "cond2", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        );
        assert_ne!(law1.canonical_bytes(), law2.canonical_bytes());
    }

    #[test]
    fn t72_canonical_bytes_differ_on_kind() {
        let law1 = LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        );
        let law2 = LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Fictional, make_instant(),
        );
        assert_ne!(law1.canonical_bytes(), law2.canonical_bytes());
    }

    #[test]
    fn t72_to_value() {
        let law = LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        )
        .with_version("1.0");
        let v = law.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.len(), 9);
                assert!(r.contains_key("name"));
                assert!(r.contains_key("condition"));
                assert!(r.contains_key("consequence"));
                assert!(r.contains_key("author"));
                assert!(r.contains_key("licence"));
                assert!(r.contains_key("kind"));
                assert!(r.contains_key("asserted_at"));
                assert!(r.contains_key("is_signed"));
                assert!(r.contains_key("version"));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── LawStore tests ────────────────────────────────────────────────

    #[test]
    fn t72_law_store_register_and_query() {
        let mut store = LawStore::new();
        store.register(LawPackage::new(
            "crush", "cond1", "cons1", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        ));
        store.register(LawPackage::new(
            "melt", "cond2", "cons2", "did:bob", "Apache",
            LawKind::Physical, make_instant(),
        ));
        store.register(LawPackage::new(
            "respawn", "cond3", "cons3", "did:game-dev", "GameLic",
            LawKind::Fictional, make_instant(),
        ));
        assert_eq!(store.len(), 3);
        assert_eq!(store.by_author("did:alice").len(), 1);
        assert_eq!(store.by_kind(LawKind::Physical).len(), 2);
        assert_eq!(store.by_kind(LawKind::Fictional).len(), 1);
        assert!(store.get("crush").is_some());
        assert!(store.get("unknown").is_none());
    }

    #[test]
    fn t72_law_store_signed_filter() {
        let mut store = LawStore::new();
        store.register(LawPackage::new(
            "crush", "cond", "cons", "did:alice", "MIT",
            LawKind::Physical, make_instant(),
        ));
        store.register(LawPackage::new(
            "melt", "cond", "cons", "did:bob", "MIT",
            LawKind::Physical, make_instant(),
        ).with_signature([1u8; 64]));
        assert_eq!(store.signed().len(), 1);
    }

    #[test]
    fn t72_law_store_empty() {
        let store = LawStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }
}
