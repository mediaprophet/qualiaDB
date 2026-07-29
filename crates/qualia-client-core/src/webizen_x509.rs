//! Bounded X.509 helpers for Webizen trust (swarm-2 Track C / D6).
//!
//! Hard caps: no full root program, no CT, no OCSP, no MDM.
//! Agents must never invent PEMs — only verify principal-enabled material.

use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

use crate::webizen_trust::TrustStore;

/// Result of a bounded path check against enabled PEM roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathVerifyResult {
    pub accepted: bool,
    pub reason_code: &'static str,
    pub detail: String,
}

impl PathVerifyResult {
    pub fn accept(reason: &'static str, detail: impl Into<String>) -> Self {
        Self {
            accepted: true,
            reason_code: reason,
            detail: detail.into(),
        }
    }
    pub fn reject(reason: &'static str, detail: impl Into<String>) -> Self {
        Self {
            accepted: false,
            reason_code: reason,
            detail: detail.into(),
        }
    }
}

/// Split PEM into certificate DER blobs (CERTIFICATE blocks only).
pub fn pem_to_ders(pem: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut out = Vec::new();
    let mut rest = pem.as_bytes();
    loop {
        match rustls_pemfile::read_one_from_slice(rest) {
            Ok(Some((item, rem))) => {
                rest = rem;
                match item {
                    rustls_pemfile::Item::X509Certificate(der) => out.push(der.to_vec()),
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => {
                if out.is_empty() {
                    return Err(format!("PEM parse: {e:?}"));
                }
                break;
            }
        }
    }
    if out.is_empty() {
        return Err("no CERTIFICATE blocks in PEM".into());
    }
    Ok(out)
}

/// SHA-256 fingerprint of SubjectPublicKeyInfo (SPKI) as lowercase hex.
pub fn spki_sha256_hex(cert_der: &[u8]) -> Result<String, String> {
    let (_, cert) = X509Certificate::from_der(cert_der).map_err(|e| format!("x509 parse: {e}"))?;
    let spki = cert.public_key().raw;
    let mut h = Sha256::new();
    h.update(spki);
    Ok(hex::encode(h.finalize()))
}

/// SPKI pin match: leaf SPKI fingerprint equals expected hex (case-insensitive).
pub fn spki_pin_matches(leaf_pem_or_der: &str, expected_hex: &str) -> Result<bool, String> {
    let ders = if leaf_pem_or_der.contains("BEGIN CERTIFICATE") {
        pem_to_ders(leaf_pem_or_der)?
    } else {
        // treat as hex DER? not supported — require PEM
        return Err("leaf must be PEM CERTIFICATE".into());
    };
    let leaf = ders.first().ok_or("empty leaf")?;
    let got = spki_sha256_hex(leaf)?;
    Ok(got.eq_ignore_ascii_case(expected_hex.trim()))
}

/// Verify leaf (and optional intermediates) against **enabled** PEM roots in the store.
///
/// Strategy (bounded):
/// 1. Parse leaf + intermediates + each enabled root.
/// 2. Accept if leaf signature verifies against any enabled root's public key, or
///    leaf verifies against an intermediate that verifies against a root.
/// 3. Otherwise reject with reason codes.
pub fn verify_chain_against_enabled_roots(
    leaf_pem: &str,
    intermediate_pems: &[&str],
    store: &TrustStore,
) -> PathVerifyResult {
    let leaf_ders = match pem_to_ders(leaf_pem) {
        Ok(d) => d,
        Err(e) => return PathVerifyResult::reject("leaf_pem_invalid", e),
    };
    let leaf_der = match leaf_ders.first() {
        Some(d) => d.as_slice(),
        None => return PathVerifyResult::reject("leaf_empty", "no leaf certificate"),
    };
    let leaf = match X509Certificate::from_der(leaf_der) {
        Ok((_, c)) => c,
        Err(e) => return PathVerifyResult::reject("leaf_parse", format!("{e}")),
    };

    let mut intermediate_ders: Vec<Vec<u8>> = Vec::new();
    for p in intermediate_pems {
        if p.trim().is_empty() {
            continue;
        }
        match pem_to_ders(p) {
            Ok(ders) => intermediate_ders.extend(ders),
            Err(e) => {
                return PathVerifyResult::reject("intermediate_pem_invalid", e);
            }
        }
    }

    let mut roots: Vec<(String, Vec<u8>)> = Vec::new();
    for a in store
        .anchors
        .iter()
        .filter(|a| a.enabled && a.kind == crate::webizen_trust::AnchorKind::PemRoot)
    {
        if let Ok(ders) = pem_to_ders(&a.material) {
            for d in ders {
                if X509Certificate::from_der(&d).is_ok() {
                    roots.push((a.id.clone(), d));
                }
            }
        }
    }
    if roots.is_empty() {
        return PathVerifyResult::reject(
            "no_enabled_roots",
            "no enabled PEM roots in trust store — chain verify fails closed",
        );
    }

    // Direct: leaf signed by root
    for (id, root_der) in &roots {
        let Ok((_, root)) = X509Certificate::from_der(root_der) else {
            continue;
        };
        if leaf.verify_signature(Some(root.public_key())).is_ok() {
            return PathVerifyResult::accept(
                "leaf_signed_by_enabled_root",
                format!("verified against root {id}"),
            );
        }
        if leaf.tbs_certificate.subject == root.tbs_certificate.subject
            && leaf.tbs_certificate.subject == leaf.tbs_certificate.issuer
            && leaf.verify_signature(None).is_ok()
        {
            return PathVerifyResult::accept(
                "self_signed_matches_enabled_root",
                format!("self-signed leaf matches enabled root {id}"),
            );
        }
    }

    // One hop: leaf → intermediate → root
    for inter_der in &intermediate_ders {
        let Ok((_, inter)) = X509Certificate::from_der(inter_der) else {
            continue;
        };
        if leaf.verify_signature(Some(inter.public_key())).is_err() {
            continue;
        }
        for (id_r, root_der) in &roots {
            let Ok((_, root)) = X509Certificate::from_der(root_der) else {
                continue;
            };
            if inter.verify_signature(Some(root.public_key())).is_ok() {
                return PathVerifyResult::accept(
                    "leaf_via_intermediate_to_enabled_root",
                    format!("chain to root {id_r}"),
                );
            }
        }
    }

    PathVerifyResult::reject(
        "chain_not_anchored",
        "leaf/intermediates did not verify to any enabled PEM root",
    )
}

/// Build a rustls `RootCertStore` from enabled PEM roots (for agent TLS).
pub fn root_cert_store_from_trust(store: &TrustStore) -> Result<rustls::RootCertStore, String> {
    let mut roots = rustls::RootCertStore::empty();
    let mut n = 0usize;
    for a in store
        .anchors
        .iter()
        .filter(|a| a.enabled && a.kind == crate::webizen_trust::AnchorKind::PemRoot)
    {
        let ders = pem_to_ders(&a.material)?;
        for der in ders {
            let cert = rustls::pki_types::CertificateDer::from(der);
            roots
                .add(cert)
                .map_err(|e| format!("add root to RootCertStore: {e}"))?;
            n += 1;
        }
    }
    if n == 0 {
        return Err("no enabled PEM roots to build RootCertStore".into());
    }
    Ok(roots)
}

/// Honesty: how agent TLS is configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTlsMode {
    /// Custom roots only (fail closed if empty).
    CustomRootsOnly { n_roots: usize },
    /// System / default reqwest rustls roots (no custom PEMs enabled).
    SystemDefault,
}

pub fn agent_tls_mode(store: &TrustStore) -> AgentTlsMode {
    let n = store
        .anchors
        .iter()
        .filter(|a| a.enabled && a.kind == crate::webizen_trust::AnchorKind::PemRoot)
        .count();
    if n > 0 {
        AgentTlsMode::CustomRootsOnly { n_roots: n }
    } else {
        AgentTlsMode::SystemDefault
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webizen_trust::{AnchorKind, TrustAnchor, TrustStore};

    /// Minimal self-signed PEM is hard without rcgen; test reject paths + empty roots.
    #[test]
    fn empty_roots_fail_closed() {
        let store = TrustStore::new();
        let r = verify_chain_against_enabled_roots(
            "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n",
            &[],
            &store,
        );
        assert!(!r.accepted);
        // either parse fail or no_enabled_roots
        assert!(
            r.reason_code == "no_enabled_roots"
                || r.reason_code == "leaf_parse"
                || r.reason_code == "leaf_pem_invalid"
        );
    }

    #[test]
    fn agent_mode_system_when_no_pem() {
        assert_eq!(
            agent_tls_mode(&TrustStore::new()),
            AgentTlsMode::SystemDefault
        );
    }

    #[test]
    fn agent_mode_custom_when_pem_enabled() {
        let mut s = TrustStore::new();
        s.anchors.push(TrustAnchor {
            id: "pem:t".into(),
            label: "t".into(),
            kind: AnchorKind::PemRoot,
            material: "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n".into(),
            enabled: true,
            notes: "".into(),
            added_unix: 1,
        });
        match agent_tls_mode(&s) {
            AgentTlsMode::CustomRootsOnly { n_roots } => assert_eq!(n_roots, 1),
            _ => panic!("expected custom"),
        }
    }
}
