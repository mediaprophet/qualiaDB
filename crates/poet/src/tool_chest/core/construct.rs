//! Construct — coherent observer/agent scope (Matrix sense: a world instantiated
//! for someone looking).
//!
//! See `docs/manuals/adr/0012-construct-is-the-distributable-composition.md`.
//! Manifolds are lenses inside the construct. Anatomy is a manifold, not a
//! construct. A workspace is this machine's live holding of an open construct.
//! QApp is not a runtime type.

use serde::{Deserialize, Serialize};

/// How a construct entered the shelf.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructSource {
    /// Shipped with POET / bundled observer-scope.
    Bundled,
    /// Authored in this principal's tree (HCF/HMC).
    Authored,
    /// Catalogue row with no manifold seed yet.
    Stub,
}

impl Default for ConstructSource {
    fn default() -> Self {
        Self::Bundled
    }
}

impl ConstructSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::Authored => "authored",
            Self::Stub => "stub",
        }
    }
}

/// Observer-scope. Shortcuts use `--construct={id}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstructSeed {
    pub id: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    /// Principal or agent DID whose embodiment this scope is. Empty = current principal.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub observer: String,
    /// `live`, `partial`, or `stub`. Never label a stub live.
    pub honesty: String,
    pub default_manifold: String,
    /// Lenses that make up this scope.
    pub manifold_ids: Vec<String>,
    /// Library Software index URI (`urn:poet:construct:…`).
    pub library_uri: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_shapes: Vec<String>,
    pub source: ConstructSource,
}

impl ConstructSeed {
    pub fn contains_manifold(&self, manifold_id: &str) -> bool {
        self.manifold_ids.iter().any(|id| id == manifold_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_honesty_is_not_live() {
        let seed = ConstructSeed {
            id: "stub-example".into(),
            label: "Example".into(),
            description: "No seed yet.".into(),
            icon: "book".into(),
            observer: String::new(),
            honesty: "stub".into(),
            default_manifold: String::new(),
            manifold_ids: vec![],
            library_uri: "urn:poet:construct:stub-example".into(),
            required_shapes: vec![],
            source: ConstructSource::Stub,
        };
        assert_eq!(seed.source, ConstructSource::Stub);
        assert_ne!(seed.honesty, "live");
        assert!(!seed.contains_manifold("health"));
    }
}
