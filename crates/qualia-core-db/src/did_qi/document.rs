//! Bounded Qualia Identifier document and QCDE-1 canonical encoding (spec §8–§9).
//! SHA-256 of QCDE-1 JSON is the digest for git blobs and UTXO OP_RETURN.

use sha2::{Digest, Sha256};

use super::id::{encode_b58, format_did, DidQi, MAX_DID_TEXT};
use super::service::{check_relay_only, CscpMailbox, Disclosure, MAX_HINTS};
use super::QiError;

pub const MAX_AKA: usize = 4;
pub const MAX_AKA_LEN: usize = 96;
pub const MAX_SERVICES: usize = 4;
pub const MAX_CANONICAL: usize = 2048;
pub const CREATED_UNIX_VECTOR: u32 = 1_788_998_400;
const CTX: &[u8] = b"[\"https://www.w3.org/ns/did/v1\",\"https://w3id.org/security/suites/ed25519-2020/v1\",\"https://webizen.network/ns/did-qi/v1\"]";
const PROOF_PREFIX: &[u8] = b"did:qi:document:v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AkaEntry {
    pub bytes: [u8; MAX_AKA_LEN],
    pub len: u8,
}

impl AkaEntry {
    pub fn from_slice(s: &[u8]) -> Result<Self, QiError> {
        if s.len() > MAX_AKA_LEN {
            return Err(QiError::BufferTooSmall);
        }
        let mut e = Self {
            bytes: [0u8; MAX_AKA_LEN],
            len: s.len() as u8,
        };
        e.bytes[..s.len()].copy_from_slice(s);
        Ok(e)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QiDocument {
    pub controller_pk: [u8; 32],
    pub created_unix: u32,
    pub generation: u32,
    pub deactivated: bool,
    pub has_previous: bool,
    pub previous_digest: [u8; 32],
    pub aka_count: u8,
    pub aka: [AkaEntry; MAX_AKA],
    pub service_count: u8,
    pub services: [CscpMailbox; MAX_SERVICES],
    pub has_hostname_alias: bool,
    pub hostname_did_web: AkaEntry,
}

impl QiDocument {
    pub const fn empty() -> Self {
        Self {
            controller_pk: [0u8; 32],
            created_unix: CREATED_UNIX_VECTOR,
            generation: 0,
            deactivated: false,
            has_previous: false,
            previous_digest: [0u8; 32],
            aka_count: 0,
            aka: [AkaEntry {
                bytes: [0u8; MAX_AKA_LEN],
                len: 0,
            }; MAX_AKA],
            service_count: 0,
            services: [CscpMailbox::mailbox([0u8; 32], Disclosure::ApprovedRelaysOnly); MAX_SERVICES],
            has_hostname_alias: false,
            hostname_did_web: AkaEntry {
                bytes: [0u8; MAX_AKA_LEN],
                len: 0,
            },
        }
    }

    pub fn services_slice(&self) -> &[CscpMailbox] {
        &self.services[..self.service_count as usize]
    }

    pub fn validate(&self) -> Result<(), QiError> {
        if self.aka_count as usize > MAX_AKA || self.service_count as usize > MAX_SERVICES {
            return Err(QiError::MalformedDocument);
        }
        check_relay_only(self.services_slice())
    }
}

pub fn sha256_32(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn proof_message(unsigned_digest: &[u8; 32], out: &mut [u8]) -> Result<usize, QiError> {
    let n = PROOF_PREFIX.len() + 32;
    if out.len() < n {
        return Err(QiError::BufferTooSmall);
    }
    out[..PROOF_PREFIX.len()].copy_from_slice(PROOF_PREFIX);
    out[PROOF_PREFIX.len()..n].copy_from_slice(unsigned_digest);
    Ok(n)
}

pub fn encode_pk_multibase(pk: &[u8; 32], out: &mut [u8]) -> Result<usize, QiError> {
    let mut raw = [0u8; 34];
    raw[0] = 0xed;
    raw[1] = 0x01;
    raw[2..].copy_from_slice(pk);
    let mut digits = [0u8; 64];
    let n = encode_b58(&raw, &mut digits)?;
    if out.len() < 1 + n {
        return Err(QiError::BufferTooSmall);
    }
    out[0] = b'z';
    out[1..1 + n].copy_from_slice(&digits[..n]);
    Ok(1 + n)
}

pub fn encode_mb32(raw32: &[u8; 32], out: &mut [u8]) -> Result<usize, QiError> {
    let mut digits = [0u8; 64];
    let n = encode_b58(raw32, &mut digits)?;
    if out.len() < 1 + n {
        return Err(QiError::BufferTooSmall);
    }
    out[0] = b'z';
    out[1..1 + n].copy_from_slice(&digits[..n]);
    Ok(1 + n)
}

fn push(out: &mut [u8], n: usize, bytes: &[u8]) -> Result<usize, QiError> {
    if n + bytes.len() > out.len() {
        return Err(QiError::BufferTooSmall);
    }
    out[n..n + bytes.len()].copy_from_slice(bytes);
    Ok(n + bytes.len())
}

fn push_u32(out: &mut [u8], n: usize, v: u32) -> Result<usize, QiError> {
    let mut tmp = [0u8; 10];
    let mut x = v;
    if x == 0 {
        return push(out, n, b"0");
    }
    let mut i = 10;
    while x > 0 {
        i -= 1;
        tmp[i] = b'0' + (x % 10) as u8;
        x /= 10;
    }
    push(out, n, &tmp[i..])
}

fn hex_lower(bytes: &[u8], out: &mut [u8]) -> Result<usize, QiError> {
    const H: &[u8; 16] = b"0123456789abcdef";
    if out.len() < bytes.len() * 2 {
        return Err(QiError::BufferTooSmall);
    }
    let mut i = 0;
    while i < bytes.len() {
        out[i * 2] = H[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = H[(bytes[i] & 0xf) as usize];
        i += 1;
    }
    Ok(bytes.len() * 2)
}

fn emit_did_url(out: &mut [u8], n: usize, id: Option<&DidQi>, frag: &[u8]) -> Result<usize, QiError> {
    let mut n = n;
    n = push(out, n, b"\"")?;
    if let Some(id) = id {
        let mut did = [0u8; MAX_DID_TEXT];
        let d = format_did(id, &mut did)?;
        n = push(out, n, &did[..d])?;
    }
    n = push(out, n, frag)?;
    push(out, n, b"\"")
}

fn emit_service(out: &mut [u8], mut n: usize, id: Option<&DidQi>, s: &CscpMailbox) -> Result<usize, QiError> {
    n = push(out, n, b"{\"id\":")?;
    n = emit_did_url(out, n, id, b"#mailbox")?;
    n = push(out, n, b",\"serviceEndpoint\":{\"contactKeyMultibase\":\"")?;
    let mut mb = [0u8; 64];
    let m = encode_mb32(&s.contact_key, &mut mb)?;
    n = push(out, n, &mb[..m])?;
    n = push(out, n, b"\",\"disclosure\":\"")?;
    n = push(out, n, s.disclosure.json_name())?;
    n = push(out, n, b"\",\"generation\":")?;
    n = push_u32(out, n, s.generation)?;
    n = push(out, n, b",\"locatorKind\":\"")?;
    n = push(out, n, s.kind.json_name())?;
    n = push(out, n, b"\",\"publicDht\":")?;
    n = push(out, n, if s.public_dht { b"true" } else { b"false" })?;
    n = push(out, n, b",\"relayHints\":[")?;
    let mut i = 0;
    while i < s.hint_count as usize && i < MAX_HINTS {
        if i > 0 {
            n = push(out, n, b",")?;
        }
        n = push(out, n, b"{\"hintId\":\"")?;
        n = push(out, n, &s.hints[i].hint_id[..s.hints[i].hint_id_len as usize])?;
        n = push(out, n, b"\",\"operatorHashHex\":\"")?;
        let mut hx = [0u8; 64];
        hex_lower(&s.hints[i].operator_hash, &mut hx)?;
        n = push(out, n, &hx)?;
        n = push(out, n, b"\"}")?;
        i += 1;
    }
    n = push(out, n, b"]},\"type\":\"CscpMailbox\"}")?;
    Ok(n)
}

fn emit_hostname_alias(
    out: &mut [u8],
    mut n: usize,
    id: Option<&DidQi>,
    did_web: &AkaEntry,
) -> Result<usize, QiError> {
    n = push(out, n, b"{\"id\":")?;
    n = emit_did_url(out, n, id, b"#frontdoor")?;
    n = push(out, n, b",\"serviceEndpoint\":{\"didWeb\":\"")?;
    n = push(out, n, &did_web.bytes[..did_web.len as usize])?;
    push(out, n, b"\"},\"type\":\"HostnameAlias\"}")
}

fn emit_vm(out: &mut [u8], mut n: usize, id: Option<&DidQi>, pk: &[u8; 32]) -> Result<usize, QiError> {
    n = push(out, n, b"{\"")?;
    if id.is_some() {
        n = push(out, n, b"controller\":")?;
        n = emit_did_url(out, n, id, b"")?;
        n = push(out, n, b",\"id\":")?;
    } else {
        n = push(out, n, b"id\":")?;
    }
    n = emit_did_url(out, n, id, b"#key-0")?;
    n = push(out, n, b",\"publicKeyMultibase\":\"")?;
    let mut mb = [0u8; 64];
    let m = encode_pk_multibase(pk, &mut mb)?;
    n = push(out, n, &mb[..m])?;
    push(out, n, b"\",\"type\":\"Ed25519VerificationKey2020\"}")
}

fn emit_relationships(out: &mut [u8], mut n: usize, id: Option<&DidQi>) -> Result<usize, QiError> {
    for key in [
        b"\"assertionMethod\":[" as &[u8],
        b"],\"authentication\":[",
        b"],\"capabilityInvocation\":[",
    ] {
        n = push(out, n, key)?;
        n = emit_did_url(out, n, id, b"#key-0")?;
    }
    push(out, n, b"]")
}

pub fn encode_genesis(doc: &QiDocument, out: &mut [u8]) -> Result<usize, QiError> {
    doc.validate()?;
    let mut n = push(out, 0, b"{\"@context\":")?;
    n = push(out, n, CTX)?;
    n = push(out, n, b",")?;
    n = emit_relationships(out, n, None)?;
    n = push(out, n, b",\"service\":[")?;
    n = emit_service_array(out, n, None, doc)?;
    n = push(out, n, b"],\"verificationMethod\":[")?;
    n = emit_vm(out, n, None, &doc.controller_pk)?;
    n = push(out, n, b"]}")?;
    finish_encode(out, n)
}

pub fn encode_unsigned(id: &DidQi, doc: &QiDocument, out: &mut [u8]) -> Result<usize, QiError> {
    encode_document(id, doc, None, out)
}

pub fn encode_signed(
    id: &DidQi,
    doc: &QiDocument,
    signature: &[u8; 64],
    out: &mut [u8],
) -> Result<usize, QiError> {
    encode_document(id, doc, Some(signature), out)
}

fn encode_document(
    id: &DidQi,
    doc: &QiDocument,
    signature: Option<&[u8; 64]>,
    out: &mut [u8],
) -> Result<usize, QiError> {
    doc.validate()?;
    let mut n = push(out, 0, b"{\"@context\":")?;
    n = push(out, n, CTX)?;
    if doc.aka_count > 0 {
        n = push(out, n, b",\"alsoKnownAs\":[")?;
        let mut i = 0;
        while i < doc.aka_count as usize {
            if i > 0 {
                n = push(out, n, b",")?;
            }
            n = push(out, n, b"\"")?;
            n = push(out, n, &doc.aka[i].bytes[..doc.aka[i].len as usize])?;
            n = push(out, n, b"\"")?;
            i += 1;
        }
        n = push(out, n, b"]")?;
    }
    n = push(out, n, b",")?;
    n = emit_relationships(out, n, Some(id))?;
    n = push(out, n, b",\"controller\":")?;
    n = emit_did_url(out, n, Some(id), b"")?;
    n = push(out, n, b",\"id\":")?;
    n = emit_did_url(out, n, Some(id), b"")?;
    if let Some(sig) = signature {
        n = push(
            out,
            n,
            b",\"proof\":{\"created\":\"2026-09-10T00:00:00Z\",\"proofPurpose\":\"capabilityInvocation\",\"proofValue\":\"",
        )?;
        let mut pv = [0u8; 96];
        let pn = encode_mb32_sig(sig, &mut pv)?;
        n = push(out, n, &pv[..pn])?;
        n = push(out, n, b"\",\"type\":\"QiDocumentSignature2026\",\"verificationMethod\":")?;
        n = emit_did_url(out, n, Some(id), b"#key-0")?;
        n = push(out, n, b"}")?;
    }
    n = push(out, n, b",\"qi\":{\"createdUnix\":")?;
    n = push_u32(out, n, doc.created_unix)?;
    n = push(out, n, b",\"deactivated\":")?;
    n = push(out, n, if doc.deactivated { b"true" } else { b"false" })?;
    n = push(out, n, b",\"generation\":")?;
    n = push_u32(out, n, doc.generation)?;
    if doc.has_previous {
        n = push(out, n, b",\"previousDigest\":\"")?;
        let mut pd = [0u8; 64];
        let p = encode_mb32(&doc.previous_digest, &mut pd)?;
        n = push(out, n, &pd[..p])?;
        n = push(out, n, b"\"")?;
    }
    n = push(out, n, b"},\"service\":[")?;
    n = emit_service_array(out, n, Some(id), doc)?;
    n = push(out, n, b"],\"verificationMethod\":[")?;
    n = emit_vm(out, n, Some(id), &doc.controller_pk)?;
    n = push(out, n, b"]}")?;
    finish_encode(out, n)
}

fn finish_encode(out: &[u8], n: usize) -> Result<usize, QiError> {
    super::document_decode::reject_forbidden_locators(&out[..n])?;
    Ok(n)
}

fn emit_service_array(
    out: &mut [u8],
    mut n: usize,
    id: Option<&DidQi>,
    doc: &QiDocument,
) -> Result<usize, QiError> {
    let mut wrote = false;
    let mut i = 0;
    while i < doc.service_count as usize {
        if wrote {
            n = push(out, n, b",")?;
        }
        n = emit_service(out, n, id, &doc.services[i])?;
        wrote = true;
        i += 1;
    }
    if doc.has_hostname_alias {
        if wrote {
            n = push(out, n, b",")?;
        }
        n = emit_hostname_alias(out, n, id, &doc.hostname_did_web)?;
    }
    Ok(n)
}

fn encode_mb32_sig(sig: &[u8; 64], out: &mut [u8]) -> Result<usize, QiError> {
    let mut digits = [0u8; 96];
    let n = encode_b58(sig, &mut digits)?;
    if out.len() < 1 + n {
        return Err(QiError::BufferTooSmall);
    }
    out[0] = b'z';
    out[1..1 + n].copy_from_slice(&digits[..n]);
    Ok(1 + n)
}

pub fn unsigned_digest(id: &DidQi, doc: &QiDocument) -> Result<[u8; 32], QiError> {
    let mut buf = [0u8; MAX_CANONICAL];
    let n = encode_unsigned(id, doc, &mut buf)?;
    Ok(sha256_32(&buf[..n]))
}

pub fn genesis_digest(doc: &QiDocument) -> Result<[u8; 32], QiError> {
    let mut buf = [0u8; MAX_CANONICAL];
    let n = encode_genesis(doc, &mut buf)?;
    Ok(sha256_32(&buf[..n]))
}

pub fn encode_canonical(doc: &QiDocument, out: &mut [u8]) -> Result<usize, QiError> {
    encode_genesis(doc, out)
}

pub fn canonical_digest(doc: &QiDocument) -> Result<[u8; 32], QiError> {
    genesis_digest(doc)
}

pub fn encode_jsonld_cold(id: &DidQi, doc: &QiDocument, out: &mut [u8]) -> Result<usize, QiError> {
    encode_unsigned(id, doc, out)
}

pub fn signed_git_object_id(
    id: &DidQi,
    doc: &QiDocument,
    signature: &[u8; 64],
) -> Result<[u8; 32], QiError> {
    let mut buf = [0u8; MAX_CANONICAL];
    let n = encode_signed(id, doc, signature, &mut buf)?;
    Ok(super::git_object::blob_object_id(&buf[..n]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::service::RelayHint;

    fn rfc8032_pk() -> [u8; 32] {
        let hex = b"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let mut pk = [0u8; 32];
        for i in 0..32 {
            pk[i] = ((hex_n(hex[i * 2]) << 4) | hex_n(hex[i * 2 + 1])) as u8;
        }
        pk
    }
    fn hex_n(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => 0,
        }
    }

    #[test]
    fn genesis_qcde_matches_spec_vector_did() {
        let mut doc = QiDocument::empty();
        doc.controller_pk = rfc8032_pk();
        doc.services[0] = CscpMailbox::mailbox(
            sha256_32(b"did:qi:test-vector:contact-key-0"),
            Disclosure::ApprovedRelaysOnly,
        );
        doc.services[0].hints[0] =
            RelayHint::new(b"invite-1", sha256_32(b"did:qi:test-vector:operator-0")).unwrap();
        doc.services[0].hint_count = 1;
        doc.service_count = 1;
        let d = genesis_digest(&doc).unwrap();
        let mut did = [0u8; MAX_DID_TEXT];
        let n = format_did(&DidQi(d), &mut did).unwrap();
        assert_eq!(&did[..n], b"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu");
    }

    #[test]
    fn canonical_rejects_relay_only_direct() {
        let mut doc = QiDocument::empty();
        doc.controller_pk = [3u8; 32];
        doc.services[0] = CscpMailbox::direct([3u8; 32], Disclosure::ApprovedRelaysOnly);
        doc.service_count = 1;
        let mut buf = [0u8; MAX_CANONICAL];
        assert_eq!(
            encode_genesis(&doc, &mut buf),
            Err(QiError::DirectLocatorForbidden)
        );
    }
}
