//! `did:qi` Sovereign Identity Method — Zero-Heap URI Grammar, Hint Parser,
//! Genesis SHA-256 Verifier, and Verification-Relationship Gate (Packet `IDF-03`).
//!
//! `did:qi` is the sole sovereign W3C DID method in Qualia (`Plane::Instrument`
//! controller document anchored by a genesis SHA-256 digest). It is strictly
//! disjoint from `did:q42:` (Q42 Resource Coordinates) and `did:webizen:`
//! (dual-signed ontology concept identifiers), and explicitly rejects legacy
//! or retired prefixes (`did:hcai:`, `did:hci:`, `did:hcinet:`, `did:qualia:`,
//! `did:q42:`) with [`DidQiError::RejectedPrefix`].

use sha2::{Digest, Sha256};

pub use crate::did_qi::{
    format_did, genesis_digest, parse_did, reject_forbidden_locators, sha256_32, DidQi,
    GitObjectStore, QiDocument, QiError, QiStore, MAX_DID_TEXT, METHOD_NAME,
};

/// Fixed-size genesis key/nonce bundle for zero-heap `did:qi` genesis digest derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisPayload {
    pub controller_ed25519: [u8; 32],
    pub controller_ml_dsa_65_hash: [u8; 32],
    pub session_ed25519: [u8; 32],
    pub q_link_x25519: [u8; 32],
    pub q_link_ml_kem_768_hash: [u8; 32],
    pub genesis_nonce: [u8; 16],
}

/// Serialize [`GenesisPayload`] into a deterministic 176-byte canonical buffer.
pub fn canonical_genesis_bytes(g: &GenesisPayload) -> [u8; 176] {
    let mut out = [0u8; 176];
    out[0..32].copy_from_slice(&g.controller_ed25519);
    out[32..64].copy_from_slice(&g.controller_ml_dsa_65_hash);
    out[64..96].copy_from_slice(&g.session_ed25519);
    out[96..128].copy_from_slice(&g.q_link_x25519);
    out[128..160].copy_from_slice(&g.q_link_ml_kem_768_hash);
    out[160..176].copy_from_slice(&g.genesis_nonce);
    out
}

/// Derive a [`DidQi`] from [`GenesisPayload`] via SHA-256 over [`canonical_genesis_bytes`].
pub fn did_qi_from_genesis_payload(g: &GenesisPayload) -> DidQi {
    let bytes = canonical_genesis_bytes(g);
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    DidQi(digest)
}

/// Errors returned by the zero-heap `did:qi` URI parser and lifecycle verifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidQiError {
    /// A retired or non-sovereign prefix (`did:hcai:`, `did:hci:`, `did:hcinet:`,
    /// `did:qualia:`, or `did:q42:`) was supplied where `did:qi:` is required.
    RejectedPrefix,
    /// Input does not begin with `did:qi:`.
    InvalidPrefix,
    /// Method-specific identifier is malformed or fails base58btc decoding.
    MalformedId,
    /// Unknown or malformed DID parameter hint (`;versionId=` or `;qi:backing=`).
    InvalidHint,
    /// Genesis SHA-256 digest does not match the canonical genesis payload.
    DigestMismatch,
    /// Update generation is not strictly `prev_generation + 1`.
    NonMonotonicGeneration,
    /// Document has been permanently deactivated via a tombstone record.
    Deactivated,
    /// Verification relationship is not authorized for the requested operation
    /// (e.g. `Authentication` key attempting a document update or route update).
    UnauthorizedRelationship,
    /// Service endpoint violates the no-direct-locator invariant (`https://`, IP, etc.).
    DirectLocatorForbidden,
}

impl From<QiError> for DidQiError {
    fn from(err: QiError) -> Self {
        match err {
            QiError::RejectedMethod => DidQiError::RejectedPrefix,
            QiError::InvalidPrefix => DidQiError::InvalidPrefix,
            QiError::MalformedId
            | QiError::Encoding
            | QiError::MalformedDocument
            | QiError::BufferTooSmall
            | QiError::CanonicalTooLarge
            | QiError::NotFound
            | QiError::StoreFull => DidQiError::MalformedId,
            QiError::MalformedChainId | QiError::UnsupportedChain | QiError::MalformedTx => {
                DidQiError::InvalidHint
            }
            QiError::CommitmentMismatch => DidQiError::DigestMismatch,
            QiError::StaleGeneration => DidQiError::NonMonotonicGeneration,
            QiError::Deactivated | QiError::TombstoneRequiresDeactivate => DidQiError::Deactivated,
            QiError::BadSignature | QiError::InvalidKey | QiError::ControllerMismatch => {
                DidQiError::UnauthorizedRelationship
            }
            QiError::DirectLocatorForbidden => DidQiError::DirectLocatorForbidden,
        }
    }
}

/// Parameterized backing-store / resolution hints on a `did:qi:` URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidQiBackingHint {
    Memory,
    Git,
    Utxo,
    Mailbox,
}

/// W3C DID verification relationships supported by `did:qi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationRelationship {
    /// `authentication` — session/challenge authentication only.
    Authentication,
    /// `assertionMethod` — signing Claims and Verifiable Credentials.
    AssertionMethod,
    /// `capabilityInvocation` — required for DID Document update, rotation,
    /// tombstone deactivation, and RAR route-update operations.
    CapabilityInvocation,
}

/// Operations gated by [`VerificationRelationship`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidQiOperation {
    AuthenticateSession,
    IssueClaimOrCredential,
    UpdateDocument,
    DeactivateTombstone,
    AuthorizeRouteUpdate,
}

/// Zero-heap parsed representation of a `did:qi:` URI with optional hints and fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedDidQiUri<'a> {
    pub did: DidQi,
    pub version_id: Option<u32>,
    pub backing_hint: Option<DidQiBackingHint>,
    pub fragment: Option<&'a [u8]>,
}

const REJECTED_PREFIXES: &[&[u8]] = &[
    b"did:hcai:",
    b"did:hci:",
    b"did:hcinet:",
    b"did:qualia:",
    b"did:q42:",
];

/// Parse a `did:qi:` URI byte slice in zero-heap fashion, extracting the 256-bit
/// genesis digest (`DidQi`), optional `;versionId=<u32>`, optional
/// `;qi:backing=git|utxo|memory|mailbox` hint, and optional `#fragment`.
///
/// Explicitly rejects `did:hcai:`, `did:hci:`, `did:hcinet:`, `did:qualia:`, and
/// `did:q42:` with [`DidQiError::RejectedPrefix`].
pub fn parse_did_qi_uri(uri: &[u8]) -> Result<ParsedDidQiUri<'_>, DidQiError> {
    for &bad in REJECTED_PREFIXES {
        if uri.len() >= bad.len() && uri[..bad.len()].eq_ignore_ascii_case(bad) {
            return Err(DidQiError::RejectedPrefix);
        }
    }
    if uri.len() < 7 || !uri[..7].eq_ignore_ascii_case(b"did:qi:") {
        return Err(DidQiError::InvalidPrefix);
    }

    // Split optional `#fragment` without heap allocation.
    let (before_frag, fragment) = match uri.iter().position(|&b| b == b'#') {
        Some(idx) => {
            let frag = &uri[idx + 1..];
            if frag.is_empty() {
                return Err(DidQiError::InvalidHint);
            }
            (&uri[..idx], Some(frag))
        }
        None => (uri, None),
    };

    // Split base DID and `;param=value` segments.
    let (base_bytes, params_bytes) = match before_frag.iter().position(|&b| b == b';') {
        Some(idx) => (&before_frag[..idx], Some(&before_frag[idx + 1..])),
        None => (before_frag, None),
    };

    let did = parse_did(base_bytes)?;

    let mut version_id = None;
    let mut backing_hint = None;

    if let Some(mut rest) = params_bytes {
        while !rest.is_empty() {
            let (seg, next) = match rest.iter().position(|&b| b == b';') {
                Some(i) => (&rest[..i], &rest[i + 1..]),
                None => (rest, &[][..]),
            };
            rest = next;
            if seg.is_empty() {
                continue;
            }
            if let Some(val) = seg.strip_prefix(b"versionId=") {
                if val.is_empty() {
                    return Err(DidQiError::InvalidHint);
                }
                let mut v: u32 = 0;
                for &b in val {
                    if !b.is_ascii_digit() {
                        return Err(DidQiError::InvalidHint);
                    }
                    v = v
                        .checked_mul(10)
                        .and_then(|x| x.checked_add((b - b'0') as u32))
                        .ok_or(DidQiError::InvalidHint)?;
                }
                version_id = Some(v);
            } else if let Some(val) = seg.strip_prefix(b"qi:backing=") {
                backing_hint = Some(match val {
                    b"memory" => DidQiBackingHint::Memory,
                    b"git" => DidQiBackingHint::Git,
                    b"utxo" => DidQiBackingHint::Utxo,
                    b"mailbox" => DidQiBackingHint::Mailbox,
                    _ => return Err(DidQiError::InvalidHint),
                });
            } else {
                return Err(DidQiError::InvalidHint);
            }
        }
    }

    Ok(ParsedDidQiUri {
        did,
        version_id,
        backing_hint,
        fragment,
    })
}

/// Verify that a `DidQi`'s 256-bit genesis digest equals `SHA-256(genesis_payload_bytes)`.
pub fn verify_genesis_sha256(did: &DidQi, genesis_payload_bytes: &[u8]) -> Result<(), DidQiError> {
    let digest: [u8; 32] = Sha256::digest(genesis_payload_bytes).into();
    if digest != did.0 {
        return Err(DidQiError::DigestMismatch);
    }
    Ok(())
}

/// Enforce strict monotonic `qi.generation` (`next_generation == current_generation + 1`)
/// and reject any mutation once `deactivated` (tombstone) is set.
pub fn validate_generation_transition(
    current_generation: u32,
    next_generation: u32,
    is_deactivated: bool,
) -> Result<(), DidQiError> {
    if is_deactivated {
        return Err(DidQiError::Deactivated);
    }
    if next_generation != current_generation.saturating_add(1) {
        return Err(DidQiError::NonMonotonicGeneration);
    }
    Ok(())
}

/// Enforce W3C DID verification-relationship separation:
/// - `Authentication` keys may ONLY authenticate sessions (`AuthenticateSession`);
///   they MUST NOT authorize document updates, tombstone deactivation, or route updates.
/// - `AssertionMethod` keys may ONLY issue claims/credentials (`IssueClaimOrCredential`).
/// - `CapabilityInvocation` keys are required for `UpdateDocument`, `DeactivateTombstone`,
///   and `AuthorizeRouteUpdate`.
pub fn check_verification_relationship(
    relationship: VerificationRelationship,
    operation: DidQiOperation,
) -> Result<(), DidQiError> {
    let allowed = matches!(
        (relationship, operation),
        (
            VerificationRelationship::Authentication,
            DidQiOperation::AuthenticateSession
        ) | (
            VerificationRelationship::AssertionMethod,
            DidQiOperation::IssueClaimOrCredential
        ) | (
            VerificationRelationship::CapabilityInvocation,
            DidQiOperation::UpdateDocument
                | DidQiOperation::DeactivateTombstone
                | DidQiOperation::AuthorizeRouteUpdate
        )
    );
    if allowed {
        Ok(())
    } else {
        Err(DidQiError::UnauthorizedRelationship)
    }
}
