//! Canonical byte encoding + JSON file codec for [`DomainLicence`].
//!
//! The canonical form is a fixed-order, little-endian, length-prefixed byte
//! encoding of every field **except** `signature_hex` and `chain_anchors`
//! (WIP §7-8: anchors live outside the signed envelope so they can be added
//! post-issuance). The signature and the licence digest are computed over
//! exactly these bytes.

use sha2::{Digest, Sha256};

use super::model::{DomainLicence, LicenceAmendment};
use super::verify::LicenceError;

/// Required `format` field value.
pub const LICENCE_FORMAT: &str = "qualia-domain-licence/0.1";

/// On-chain OP_RETURN marker prefixing the licence digest (WIP §4-B).
pub const ANCHOR_OP_RETURN_PREFIX: &[u8] = b"QUALIA-LIC ";

fn write_u8(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    write_u32(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

fn write_str_list(buf: &mut Vec<u8>, list: &[String]) {
    write_u32(buf, list.len() as u32);
    for s in list {
        write_str(buf, s);
    }
}

/// Canonical signed bytes for a licence (excludes `signature_hex` and
/// `chain_anchors`).
pub fn canonical_bytes(lic: &DomainLicence) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);
    write_str(&mut buf, &lic.format);
    write_str(&mut buf, &lic.licence_id);
    write_str(&mut buf, &lic.issuer_did);
    write_str(&mut buf, &lic.issuer_pubkey_hex);
    write_str_list(&mut buf, &lic.subject_domains);
    write_str(&mut buf, &lic.domain_did);
    match &lic.delegation {
        Some(d) => {
            write_u8(&mut buf, 1);
            write_u8(&mut buf, u8::from(d.covers_subdomains));
            write_str_list(&mut buf, &d.members);
            write_str(&mut buf, &d.steward_agreement);
        }
        None => write_u8(&mut buf, 0),
    }
    write_str(&mut buf, &lic.licensee_did);
    write_str(&mut buf, &lic.licensee_agent_type);
    write_str(&mut buf, &lic.licensee_label);
    write_str(&mut buf, &lic.licence_class);
    write_str_list(&mut buf, &lic.capability_scope);
    write_u64(&mut buf, lic.not_before_unix);
    write_u64(&mut buf, lic.expires_unix);
    match &lic.payment_evidence {
        Some(p) => {
            write_u8(&mut buf, 1);
            write_str(&mut buf, &p.rail);
            write_str(&mut buf, &p.txid);
            write_u64(&mut buf, p.amount_sats);
        }
        None => write_u8(&mut buf, 0),
    }
    write_str(&mut buf, &lic.revocation_uri);
    write_str(&mut buf, &lic.terms_uri);
    buf
}

/// Canonical signed bytes for an amendment.
pub fn amendment_canonical_bytes(amend: &LicenceAmendment) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    write_str(&mut buf, &amend.licence_id);
    write_u32(&mut buf, amend.seq);
    write_str_list(&mut buf, &amend.add_members);
    write_str_list(&mut buf, &amend.remove_members);
    write_u64(&mut buf, amend.issued_unix);
    buf
}

/// SHA-256 digest of the canonical licence bytes — the value anchored on-chain.
pub fn canonical_digest(lic: &DomainLicence) -> [u8; 32] {
    Sha256::digest(&canonical_bytes(lic)).into()
}

/// Digest rendered as `"sha256:<hex>"` for `chain_anchors[].digest`.
pub fn canonical_digest_tag(lic: &DomainLicence) -> String {
    format!("sha256:{}", hex::encode(canonical_digest(lic)))
}

/// The OP_RETURN payload committing a licence digest on a nominated chain.
pub fn anchor_op_return(lic: &DomainLicence) -> Vec<u8> {
    let mut out = Vec::with_capacity(ANCHOR_OP_RETURN_PREFIX.len() + 32);
    out.extend_from_slice(ANCHOR_OP_RETURN_PREFIX);
    out.extend_from_slice(&canonical_digest(lic));
    out
}

/// Serialize a licence to its `.qlic.json` file form.
pub fn to_json(lic: &DomainLicence) -> Result<String, LicenceError> {
    serde_json::to_string_pretty(lic).map_err(|_| LicenceError::Malformed("json encode"))
}

/// Parse a licence from its `.qlic.json` file form.
pub fn from_json(text: &str) -> Result<DomainLicence, LicenceError> {
    serde_json::from_str(text).map_err(|_| LicenceError::Malformed("json parse"))
}

/// Serialize an amendment to JSON.
pub fn amendment_to_json(amend: &LicenceAmendment) -> Result<String, LicenceError> {
    serde_json::to_string_pretty(amend).map_err(|_| LicenceError::Malformed("json encode"))
}

/// Parse an amendment from JSON.
pub fn amendment_from_json(text: &str) -> Result<LicenceAmendment, LicenceError> {
    serde_json::from_str(text).map_err(|_| LicenceError::Malformed("json parse"))
}
