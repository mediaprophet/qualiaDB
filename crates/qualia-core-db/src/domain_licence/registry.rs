//! Declared-domain registry (WIP §4-A): which domains this node/build declares
//! licences for, and where each licence lives.
//!
//! The effective declared set is the union of:
//!  - [`COMPILED_IN_LICENSED_DOMAINS`] (baked into the build),
//!  - signed `DomainLicence` subject lists (checked at verification, not here),
//!  - this on-disk registry (`licence_registry.json`).

use serde::{Deserialize, Serialize};

use super::verify::normalize_domain;

/// Domains compiled into this build as licensed (WIP §4-A "compiled-in
/// declaration"). Client-specific builds may patch this list; the default is
/// empty — declarations should normally travel in the signed instrument.
pub const COMPILED_IN_LICENSED_DOMAINS: &[&str] = &[];

/// Where a declared domain's licence record comes from.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrySource {
    /// Signed `.qlic.json` installed on this node.
    InstalledFile = 1,
    /// `/.well-known/qualia-licence.json` on the licensed origin.
    WellKnown = 2,
    /// `_qualia-licence` DNS TXT carrying the digest.
    DnsTxt = 3,
    /// Anchored on a nominated chain (digest discovered by lookup).
    ChainAnchor = 4,
}

/// One declared domain → licence mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Normalised domain name.
    pub domain: String,
    /// `did:q42:licence:…` this declaration resolves to.
    pub licence_id: String,
    /// Canonical digest (`sha256:<hex>`) when known — used to reject stale files.
    #[serde(default)]
    pub digest: String,
    /// How the licence was declared.
    pub source: RegistrySource,
}

/// On-disk registry of declared domains → licences.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceRegistry {
    #[serde(default)]
    pub entries: Vec<RegistryEntry>,
}

impl LicenceRegistry {
    /// Look up the declaration for a domain (normalised before compare).
    pub fn lookup(&self, domain: &str) -> Option<&RegistryEntry> {
        let d = normalize_domain(domain);
        self.entries.iter().find(|e| e.domain == d)
    }

    /// Insert or replace the declaration for `entry.domain`.
    pub fn upsert(&mut self, entry: RegistryEntry) {
        let d = normalize_domain(&entry.domain);
        self.entries.retain(|e| e.domain != d);
        self.entries.push(RegistryEntry { domain: d, ..entry });
    }

    /// Remove the declaration for a domain. Returns true if one existed.
    pub fn remove(&mut self, domain: &str) -> bool {
        let d = normalize_domain(domain);
        let before = self.entries.len();
        self.entries.retain(|e| e.domain != d);
        self.entries.len() != before
    }

    /// Is `domain` declared — in the registry or compiled into the build?
    pub fn is_declared(&self, domain: &str) -> bool {
        let d = normalize_domain(domain);
        COMPILED_IN_LICENSED_DOMAINS
            .iter()
            .any(|c| normalize_domain(c) == d)
            || self.lookup(&d).is_some()
    }

    /// All declared domains (registry ∪ compiled-in), normalised, deduped.
    pub fn declared_domains(&self) -> Vec<String> {
        let mut out: Vec<String> = self.entries.iter().map(|e| e.domain.clone()).collect();
        for c in COMPILED_IN_LICENSED_DOMAINS {
            let n = normalize_domain(c);
            if !out.contains(&n) {
                out.push(n);
            }
        }
        out
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}
