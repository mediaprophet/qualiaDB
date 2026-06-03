//! `did:q42` topological coordinate resolver.
//!
//! A `did:q42` URI is a **physical topological coordinate** — an absolute
//! pointer into the Qualia disk/memory layout — not a human identity.
//!
//! # Zero-allocation guarantee
//! All operations work directly on `&[u8]`.  No `String` or `Vec` is
//! constructed at any point in the hot path.
//!
//! # MSB flag convention
//! Any `u64` returned by this module has bit 63 set to `1`.  The Sentinel VM
//! uses that bit to distinguish absolute hardware/disk pointers (MSB = 1) from
//! local dictionary hashes produced by `q_hash` (MSB = 0).

/// The mandatory URI prefix for all `did:q42` coordinates.
const PREFIX: &[u8] = b"did:q42:";

/// Errors returned by [`parse_did_q42`].
#[derive(Debug, PartialEq)]
pub enum IdentifierError {
    /// The byte slice does not begin with `did:q42:`.
    InvalidPrefix,
    /// The payload section (after the prefix) is empty or contains bytes that
    /// cannot form a valid base-58 / multibase coordinate.
    MalformedHash,
    /// Reserved for future checksum verification of the multicodec payload.
    InvalidChecksum,
}

/// Parse a `did:q42:` URI byte slice into a 64-bit topological pointer.
///
/// # Contract
/// * Input must start with `b"did:q42:"`.
/// * The payload after the prefix must be non-empty.
/// * The returned `u64` always has **bit 63 set** — signalling to the Sentinel
///   VM that this value is a hardware/disk coordinate, not a dictionary hash.
///
/// # Example
/// ```
/// use qualia_core_db::identifier::parse_did_q42;
/// let ptr = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
/// assert_eq!(ptr >> 63, 1, "MSB must be set for topological coordinates");
/// ```
pub fn parse_did_q42(uri: &[u8]) -> Result<u64, IdentifierError> {
    // 1. Prefix check — `starts_with` operates entirely on `&[u8]`.
    if !uri.starts_with(PREFIX) {
        return Err(IdentifierError::InvalidPrefix);
    }

    // 2. Extract payload without any allocation.
    let payload = &uri[PREFIX.len()..];
    if payload.is_empty() {
        return Err(IdentifierError::MalformedHash);
    }

    // 3. Hash the raw payload bytes with FNV-1a.
    //    We mirror `crate::q_hash` but operate on `&[u8]` directly so this
    //    module remains self-contained and `no_std`-compatible.
    let base_hash = fnv1a(payload);

    // 4. Apply the routing bitmask: flip bit 63 to mark this as a topological
    //    pointer rather than a plain dictionary hash.
    let pointer = base_hash | (1u64 << 63);

    Ok(pointer)
}

/// FNV-1a over a raw byte slice — identical algorithm to `crate::q_hash`.
/// Kept local so this module has no runtime dependency on the crate root.
#[inline(always)]
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_did_q42_msb() {
        let result = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        assert_eq!(result >> 63, 1, "MSB must be 1 for a topological pointer");
    }

    #[test]
    fn test_invalid_prefix() {
        assert_eq!(
            parse_did_q42(b"did:key:z6MkpTHR8VNs"),
            Err(IdentifierError::InvalidPrefix)
        );
    }

    #[test]
    fn test_empty_payload() {
        assert_eq!(
            parse_did_q42(b"did:q42:"),
            Err(IdentifierError::MalformedHash)
        );
    }

    #[test]
    fn test_bare_prefix_without_colon() {
        assert_eq!(
            parse_did_q42(b"did:q42"),
            Err(IdentifierError::InvalidPrefix)
        );
    }

    #[test]
    fn test_deterministic_output() {
        let a = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        let b = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        assert_eq!(a, b, "parse_did_q42 must be deterministic");
    }

    #[test]
    fn test_distinct_payloads_produce_distinct_pointers() {
        let a = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        let b = parse_did_q42(b"did:q42:z6MkpTHR8VNt").unwrap();
        assert_ne!(a, b, "distinct payloads must yield distinct pointers");
    }

    #[test]
    fn pointer_is_base_hash_or_msb() {
        // The contract is pointer == q_hash(payload) | (1 << 63), regardless
        // of whether q_hash already has bit 63 set for this particular input.
        let plain = crate::q_hash("z6MkpTHR8VNs");
        let pointer = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        assert_eq!(pointer, plain | (1u64 << 63));
    }
}
