//! Dependency requirements and deterministic lock records (SI-05).
//! Large Q42 volumes, lexicons and datasets are dependencies, not the instrument.

use crate::semantic_instruments::canonical::sha256_prefixed;
use crate::semantic_instruments::errors::InstrumentError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    Ontology,
    Lexicon,
    Dataset,
    LawPackage,
    Instrument,
    NativeModule,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DependencyReq {
    pub id: String,
    pub kind: DependencyKind,
    pub version_constraint: String,
    pub expected_digest: String,
    pub required: bool,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CatalogRecord {
    pub id: String,
    pub version: String,
    pub digest: String,
    pub locator: String,
    pub revoked: bool,
    pub available_offline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LockEntry {
    pub id: String,
    pub version: String,
    pub digest: String,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DependencyLock {
    pub release_id: String,
    pub entries: Vec<LockEntry>,
}

impl DependencyLock {
    pub fn digest(&self) -> String {
        let mut buf = self.release_id.clone().into_bytes();
        for e in &self.entries {
            buf.extend_from_slice(e.id.as_bytes());
            buf.extend_from_slice(e.version.as_bytes());
            buf.extend_from_slice(e.digest.as_bytes());
            buf.extend_from_slice(e.locator.as_bytes());
        }
        sha256_prefixed(&buf)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Exact pin: constraint empty or equal to version. Compatible: constraint is a
/// prefix of version followed by '.' or end (`1.0` matches `1.0.0`).
pub fn version_matches(constraint: &str, version: &str) -> bool {
    let c = constraint.trim();
    if c.is_empty() || c == "*" {
        return true;
    }
    if c == version {
        return true;
    }
    let c = c.trim_start_matches('^').trim_start_matches('~').trim_end_matches('*').trim_end_matches('.');
    version == c || version.starts_with(&format!("{c}."))
}

pub fn require_digest(req: &DependencyReq, rec: &CatalogRecord) -> Result<(), InstrumentError> {
    if req.expected_digest.is_empty() {
        return Ok(());
    }
    if rec.digest == req.expected_digest {
        Ok(())
    } else {
        Err(InstrumentError::DigestMismatchDep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_compatible_versions() {
        assert!(version_matches("1.0.0", "1.0.0"));
        assert!(version_matches("1.0", "1.0.0"));
        assert!(version_matches("^1.0", "1.0.2"));
        assert!(!version_matches("1.0.0", "2.0.0"));
        assert!(version_matches("*", "9.9.9"));
    }

    #[test]
    fn lock_digest_is_stable_and_order_sensitive() {
        let mut a = DependencyLock {
            release_id: "r".into(),
            entries: vec![LockEntry {
                id: "ont".into(),
                version: "1".into(),
                digest: "sha256:aa".into(),
                locator: "local:ont".into(),
            }],
        };
        let b = a.clone();
        assert_eq!(a.digest(), b.digest());
        a.entries[0].digest = "sha256:bb".into();
        assert_ne!(a.digest(), b.digest());
    }
}
