//! Stable Vibe IRIs and language-version identity.

/// Canonical VibeScript schema namespace (matches `poet_host/catalog_ttl.rs`).
pub const VIBE_NS: &str = "https://qualiadb.org/schema/vibe#";

/// Metamodel schema revision for exported RDFS/SHACL artifacts.
pub const VIBE_SCHEMA_VERSION: &str = "vibe-metamodel-1";

/// IRI for the current language version string (`vibe-0.1`).
pub const LANGUAGE_VERSION_IRI: &str = "https://qualiadb.org/schema/vibe#LanguageVersion-vibe-0.1";

#[inline]
pub fn term(local: &str) -> String {
    format!("{VIBE_NS}{local}")
}
