//! Canonical Solid / LDP / WAC / PIM IRIs used by the bridge.
//!
//! Source ontologies (tracked under `bundled/ontologies/w3c-archives/`):
//! - `ldp.ttl` — http://www.w3.org/ns/ldp#
//! - `auth-acl.ttl` — http://www.w3.org/ns/auth/acl#
//! - `solid-terms.ttl` — http://www.w3.org/ns/solid/terms#
//! - `solid-oidc.ttl` — http://www.w3.org/ns/solid/oidc#
//! - `pim-space.ttl` — http://www.w3.org/ns/pim/space#
//! - `foaf.ttl` — http://xmlns.com/foaf/0.1/
//!
//! Copied from Timothy's W3C ns archive (`ontologies-2023/w3c archives/ns-main/w3c-ns`)
//! except Solid Terms (solid/vocab; not present in the 2023 ns-main dump).

use qualia_core_db::q_hash;

// ── Namespaces ──────────────────────────────────────────────────────────────
pub const NS_LDP: &str = "http://www.w3.org/ns/ldp#";
pub const NS_ACL: &str = "http://www.w3.org/ns/auth/acl#";
pub const NS_SOLID: &str = "http://www.w3.org/ns/solid/terms#";
pub const NS_SOLID_OIDC: &str = "http://www.w3.org/ns/solid/oidc#";
pub const NS_PIM_SPACE: &str = "http://www.w3.org/ns/pim/space#";
pub const NS_FOAF: &str = "http://xmlns.com/foaf/0.1/";
pub const NS_CERT: &str = "http://www.w3.org/ns/auth/cert#";

// ── LDP ─────────────────────────────────────────────────────────────────────
pub const LDP_RESOURCE: &str = "http://www.w3.org/ns/ldp#Resource";
pub const LDP_RDF_SOURCE: &str = "http://www.w3.org/ns/ldp#RDFSource";
pub const LDP_BASIC_CONTAINER: &str = "http://www.w3.org/ns/ldp#BasicContainer";
pub const LDP_CONTAINER: &str = "http://www.w3.org/ns/ldp#Container";
pub const LDP_CONTAINS: &str = "http://www.w3.org/ns/ldp#contains";
pub const LDP_INBOX: &str = "http://www.w3.org/ns/ldp#inbox";

// ── Solid terms ─────────────────────────────────────────────────────────────
pub const SOLID_OIDC_ISSUER: &str = "http://www.w3.org/ns/solid/terms#oidcIssuer";
pub const SOLID_STORAGE: &str = "http://www.w3.org/ns/solid/terms#storage"; // often pim:storage
pub const SOLID_OIDC_ISSUER_SERVICE: &str =
    "http://www.w3.org/ns/solid/terms#oidcIssuerRegistrationToken";

// ── PIM space ───────────────────────────────────────────────────────────────
pub const PIM_STORAGE: &str = "http://www.w3.org/ns/pim/space#storage";
pub const PIM_PREFERENCES_FILE: &str = "http://www.w3.org/ns/pim/space#preferencesFile";

// ── FOAF ────────────────────────────────────────────────────────────────────
pub const FOAF_PERSON: &str = "http://xmlns.com/foaf/0.1/Person";
pub const FOAF_AGENT: &str = "http://xmlns.com/foaf/0.1/Agent";
pub const FOAF_NAME: &str = "http://xmlns.com/foaf/0.1/name";

// ── WAC ─────────────────────────────────────────────────────────────────────
pub const ACL_AUTHORIZATION: &str = "http://www.w3.org/ns/auth/acl#Authorization";
pub const ACL_ACCESS_TO: &str = "http://www.w3.org/ns/auth/acl#accessTo";
pub const ACL_MODE: &str = "http://www.w3.org/ns/auth/acl#mode";
pub const ACL_AGENT: &str = "http://www.w3.org/ns/auth/acl#agent";
pub const ACL_AGENT_CLASS: &str = "http://www.w3.org/ns/auth/acl#agentClass";
pub const ACL_READ: &str = "http://www.w3.org/ns/auth/acl#Read";
pub const ACL_WRITE: &str = "http://www.w3.org/ns/auth/acl#Write";
pub const ACL_CONTROL: &str = "http://www.w3.org/ns/auth/acl#Control";
pub const ACL_APPEND: &str = "http://www.w3.org/ns/auth/acl#Append";

/// Stable FNV-1a 60-bit hashes for hot-path comparison (no string alloc).
pub mod hash {
    use super::*;
    use std::sync::OnceLock;

    fn h(iri: &str) -> u64 {
        q_hash(iri)
    }

    static LDP_CONTAINS: OnceLock<u64> = OnceLock::new();
    static SOLID_OIDC_ISSUER: OnceLock<u64> = OnceLock::new();
    static PIM_STORAGE: OnceLock<u64> = OnceLock::new();

    pub fn ldp_contains() -> u64 {
        *LDP_CONTAINS.get_or_init(|| h(super::LDP_CONTAINS))
    }
    pub fn solid_oidc_issuer() -> u64 {
        *SOLID_OIDC_ISSUER.get_or_init(|| h(super::SOLID_OIDC_ISSUER))
    }
    pub fn pim_storage() -> u64 {
        *PIM_STORAGE.get_or_init(|| h(super::PIM_STORAGE))
    }
}

/// Files the personal pod seeds under `/public/ontologies/` when available on disk.
pub const POD_ONTOLOGY_FILES: &[(&str, &str)] = &[
    ("ldp.ttl", "Linked Data Platform"),
    ("auth-acl.ttl", "Web Access Control"),
    ("solid-terms.ttl", "Solid Terms"),
    ("solid-oidc.ttl", "Solid-OIDC"),
    ("pim-space.ttl", "PIM Workspace / Storage"),
    ("foaf.ttl", "FOAF"),
];

/// Resolve ontology sources next to the binary, under env, or from the repo tree.
pub fn resolve_ontology_source(file_name: &str) -> Option<std::path::PathBuf> {
    if let Ok(extra) = std::env::var("QUALIA_BUNDLED_ONTOLOGIES_DIR") {
        let root = std::path::PathBuf::from(extra);
        let p = root.join(file_name);
        if p.is_file() {
            return Some(p);
        }
        let p2 = root.join("w3c-archives").join(file_name);
        if p2.is_file() {
            return Some(p2);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                format!("bundled/ontologies/w3c-archives/{file_name}"),
                format!("ontologies/w3c-archives/{file_name}"),
                format!("w3c-archives/{file_name}"),
                file_name.to_string(),
            ] {
                let mut p = dir.to_path_buf();
                for seg in rel.split('/') {
                    p.push(seg);
                }
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    // Dev tree: relative to this crate → repo root
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../bundled/ontologies/w3c-archives")
        .join(file_name);
    if repo.is_file() {
        return Some(repo);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_core_sources_resolve_in_dev_tree() {
        for (file, _) in POD_ONTOLOGY_FILES {
            let p = resolve_ontology_source(file);
            assert!(
                p.is_some(),
                "missing bundled ontology {file} — copy from ontologies-2023/w3c-ns"
            );
        }
    }

    #[test]
    fn hashes_are_stable_and_nonzero() {
        assert_ne!(hash::ldp_contains(), 0);
        assert_ne!(hash::solid_oidc_issuer(), 0);
        assert_eq!(hash::ldp_contains(), q_hash(LDP_CONTAINS));
    }
}
