//! `did:webizen:` Web of Data identifier parser and deterministic concept UUID engine.
//!
//! # Universal Rules Compliance
//! * **Zero heap in hot paths:** All operations take `&[u8]` and caller-owned stack buffers.
//!   No `String`, `Vec`, or `Box` are allocated.
//! * **60-bit Quin Token:** Concept tokens are masked with `0x0FFF_FFFF_FFFF_FFFF` with
//!   bit 63 cleared (`MSB = 0`), denoting a dictionary/lexicon entity rather than
//!   a physical topological coordinate (`did:q42` pointer, `MSB = 1`).
//!
//! Reference: `docs/manuals/standards/did-webizen-method.md`

use sha1::{Digest, Sha1};

/// Mandatory URI prefix for all Webizen identifiers.
pub const PREFIX: &[u8] = b"did:webizen:";

/// Canonical Webizen Ontology Namespace UUID (RFC 4122 Namespace UUID for Web of Data concepts).
/// UUID: `7f18b320-9dad-11d1-80b4-00c04fd430c8`
pub const WEBIZEN_ONTOLOGY_NAMESPACE: [u8; 16] = [
    0x7f, 0x18, 0xb3, 0x20, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
];

/// The functional realm of a `did:webizen` identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebizenRealm {
    /// Domain-decoupled semantic concept, class, or predicate.
    Concept,
    /// Versioned ontology package (`name@semver`).
    Ont,
    /// Ratified bilateral or multi-party micro-commons agreement.
    Agreement,
    /// Web of Data citizen or autonomous agent principal.
    Agent,
}

/// Zero-copy reference over a validated `did:webizen:` identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebizenDidRef<'a> {
    pub realm: WebizenRealm,
    pub payload: &'a [u8],
}

/// Errors returned by [`parse_did_webizen`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebizenDidError {
    /// URI does not start with `did:webizen:`.
    InvalidPrefix,
    /// Specifically rejected competitor or alternative method (e.g. `did:q42:`, `did:qi:`).
    RejectedMethod,
    /// The realm component is unknown or missing.
    UnknownRealm,
    /// The realm payload (after realm and colon) is empty.
    EmptyPayload,
    /// Caller-supplied buffer is too small for output.
    BufferTooSmall,
}

/// Parse a `did:webizen:` byte slice into a zero-copy [`WebizenDidRef`].
///
/// # Hot Path Contract
/// Guaranteed zero-heap. Operates entirely on the input byte slice.
pub fn parse_did_webizen(uri: &[u8]) -> Result<WebizenDidRef<'_>, WebizenDidError> {
    // 1. Explicitly reject other DID methods
    if starts_ignore_ascii(uri, b"did:q42:")
        || starts_ignore_ascii(uri, b"did:qi:")
        || starts_ignore_ascii(uri, b"did:hcai:")
        || starts_ignore_ascii(uri, b"did:hcinet:")
        || starts_ignore_ascii(uri, b"did:qualia:")
    {
        return Err(WebizenDidError::RejectedMethod);
    }

    // 2. Prefix validation
    if !starts_ignore_ascii(uri, PREFIX) {
        return Err(WebizenDidError::InvalidPrefix);
    }

    let rest = &uri[PREFIX.len()..];
    if rest.is_empty() {
        return Err(WebizenDidError::UnknownRealm);
    }

    // 3. Realm dispatch
    if let Some(payload) = strip_prefix_exact(rest, b"concept:") {
        if payload.is_empty() {
            return Err(WebizenDidError::EmptyPayload);
        }
        Ok(WebizenDidRef {
            realm: WebizenRealm::Concept,
            payload,
        })
    } else if let Some(payload) = strip_prefix_exact(rest, b"ont:") {
        if payload.is_empty() {
            return Err(WebizenDidError::EmptyPayload);
        }
        Ok(WebizenDidRef {
            realm: WebizenRealm::Ont,
            payload,
        })
    } else if let Some(payload) = strip_prefix_exact(rest, b"agreement:") {
        if payload.is_empty() {
            return Err(WebizenDidError::EmptyPayload);
        }
        Ok(WebizenDidRef {
            realm: WebizenRealm::Agreement,
            payload,
        })
    } else if let Some(payload) = strip_prefix_exact(rest, b"agent:") {
        if payload.is_empty() {
            return Err(WebizenDidError::EmptyPayload);
        }
        Ok(WebizenDidRef {
            realm: WebizenRealm::Agent,
            payload,
        })
    } else {
        Err(WebizenDidError::UnknownRealm)
    }
}

/// Compute a deterministic RFC 4122 / RFC 9562 UUIDv5 for a concept name.
///
/// Combines the canonical [`WEBIZEN_ONTOLOGY_NAMESPACE`] with the supplied
/// concept name bytes via SHA-1, setting UUID version 5 and RFC 4122 variant bits.
///
/// Guaranteed zero heap allocations. Returns the raw 16-byte UUID.
pub fn derive_concept_uuid(concept_name: &[u8]) -> [u8; 16] {
    let mut hasher = Sha1::new();
    hasher.update(WEBIZEN_ONTOLOGY_NAMESPACE);
    hasher.update(concept_name);
    let digest = hasher.finalize();

    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&digest[0..16]);

    // Set UUIDv5 version (0101 in high 4 bits of octet 6)
    uuid[6] = (uuid[6] & 0x0f) | 0x50;
    // Set RFC 4122 variant (10 in high 2 bits of octet 8)
    uuid[8] = (uuid[8] & 0x3f) | 0x80;

    uuid
}

/// Compute a post-quantum collision-resistant deterministic UUID (RFC 9562 UUIDv8)
/// for a concept name using SHA-256.
///
/// Under Grover's quantum search algorithm, SHA-256 retains 128 bits of quantum security,
/// making this concept derivation immune to quantum preimage attacks.
///
/// Guaranteed zero heap allocations.
pub fn derive_concept_uuid_pq(concept_name: &[u8]) -> [u8; 16] {
    use sha2::{Digest as Sha2Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(WEBIZEN_ONTOLOGY_NAMESPACE);
    hasher.update(concept_name);
    let digest = hasher.finalize();

    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&digest[0..16]);

    // Set UUIDv8 version (1000 in high 4 bits of octet 6: custom/vendor-defined)
    uuid[6] = (uuid[6] & 0x0f) | 0x80;
    // Set RFC 4122 / RFC 9562 variant (10 in high 2 bits of octet 8)
    uuid[8] = (uuid[8] & 0x3f) | 0x80;

    uuid
}

/// Verify an ML-DSA-65 (FIPS-204) post-quantum digital signature over a message.
///
/// Hot-path guarantee: operates on fixed-size byte references with zero heap allocation.
pub fn verify_pq_signature(
    public_key_bytes: &[u8; 1952],
    message: &[u8],
    signature_bytes: &[u8; 3309],
    context: &[u8],
) -> bool {
    crate::crypto::network::mldsa::verify(public_key_bytes, message, context, signature_bytes).is_ok()
}

/// Verify a hybrid DualProof (ML-DSA-65 + Ed25519) digital signature over a message.
///
/// Both signatures must verify over the identical message and context bytes.
/// Hot-path guarantee: operates on fixed-size byte references with zero heap allocation.
pub fn verify_dual_signature(
    mldsa_pk: &[u8; 1952],
    ed25519_pk: &[u8; 32],
    message: &[u8],
    context: &[u8],
    proof: &crate::crypto::network::dual_sign::DualProof,
) -> bool {
    crate::crypto::network::dual_sign::verify_dual(
        mldsa_pk,
        ed25519_pk,
        message,
        context,
        proof,
    )
    .is_ok()
}

/// Format a 16-byte UUID into standard 36-character hyphenated lowercase hex ASCII.
///
/// Output format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
pub fn format_uuid_hex(uuid: &[u8; 16], out: &mut [u8; 36]) {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    let mut cursor = 0;
    for (i, &byte) in uuid.iter().enumerate() {
        if i == 4 || i == 6 || i == 8 || i == 10 {
            out[cursor] = b'-';
            cursor += 1;
        }
        out[cursor] = HEX_CHARS[(byte >> 4) as usize];
        out[cursor + 1] = HEX_CHARS[(byte & 0x0f) as usize];
        cursor += 2;
    }
}

/// Format a 16-byte concept UUID into a 45-character `urn:uuid:...` ASCII slice.
pub fn format_concept_urn(uuid: &[u8; 16], out: &mut [u8; 45]) {
    out[..9].copy_from_slice(b"urn:uuid:");
    let mut hex = [0u8; 36];
    format_uuid_hex(uuid, &mut hex);
    out[9..45].copy_from_slice(&hex);
}

/// Compute the 60-bit FNV-1a Quin token for an identifier or UUID URN.
///
/// Bit 63 is unconditionally cleared (`0`) to mark this as a lexicon dictionary entity
/// rather than a `did:q42:` hardware/RAM coordinate.
#[inline(always)]
pub fn quin_concept_token(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    // Mask to 60 bits, MSB is always 0
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

fn starts_ignore_ascii(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len()
        && s[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
}

fn strip_prefix_exact<'a>(s: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if s.len() >= prefix.len() && &s[..prefix.len()] == prefix {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_realms() {
        let c = parse_did_webizen(b"did:webizen:concept:hasTrackId").unwrap();
        assert_eq!(c.realm, WebizenRealm::Concept);
        assert_eq!(c.payload, b"hasTrackId");

        let o = parse_did_webizen(b"did:webizen:ont:perception@1.2.0").unwrap();
        assert_eq!(o.realm, WebizenRealm::Ont);
        assert_eq!(o.payload, b"perception@1.2.0");

        let a = parse_did_webizen(b"did:webizen:agreement:z6MkpTHR8VNs").unwrap();
        assert_eq!(a.realm, WebizenRealm::Agreement);
        assert_eq!(a.payload, b"z6MkpTHR8VNs");

        let ag = parse_did_webizen(b"did:webizen:agent:z6MkpAgent").unwrap();
        assert_eq!(ag.realm, WebizenRealm::Agent);
        assert_eq!(ag.payload, b"z6MkpAgent");
    }

    #[test]
    fn test_reject_competing_dids() {
        assert_eq!(
            parse_did_webizen(b"did:q42:z6MkpTHR8VNs"),
            Err(WebizenDidError::RejectedMethod)
        );
        assert_eq!(
            parse_did_webizen(b"did:qi:z6MkpTHR8VNs"),
            Err(WebizenDidError::RejectedMethod)
        );
        assert_eq!(
            parse_did_webizen(b"did:qualia:0x3f8a"),
            Err(WebizenDidError::RejectedMethod)
        );
    }

    #[test]
    fn test_deterministic_uuidv5() {
        let u1 = derive_concept_uuid(b"hasTrackId");
        let u2 = derive_concept_uuid(b"hasTrackId");
        assert_eq!(u1, u2, "UUIDv5 must be deterministic");

        // Verify version 5 and variant bits
        assert_eq!(u1[6] >> 4, 5, "UUID must be version 5");
        assert_eq!(u1[8] >> 6, 2, "UUID must be RFC 4122 variant");

        let mut urn = [0u8; 45];
        format_concept_urn(&u1, &mut urn);
        assert!(urn.starts_with(b"urn:uuid:"));
        assert_eq!(urn.len(), 45);

        let token = quin_concept_token(&urn);
        assert_eq!(token >> 63, 0, "MSB must be 0 for dictionary entities");
        assert_eq!(token >> 60, 0, "Bits 60-63 must be 0 in masked token");
    }
}
