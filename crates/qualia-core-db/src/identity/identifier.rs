//! `did:q42` Q42 Resource Coordinate (`QrcCoordinate`) resolver.
//!
//! A `did:q42` URI is a **60-bit + MSB (`1 << 63`) Q42 Resource Coordinate (QRC)**
//! belonging to `Plane::Handle` / `Plane::Instrument` — a physical/topological
//! coordinate into the Qualia disk/memory layout — **never** a W3C DID Document
//! root and **never** a `NaturalAgent` (`Who`) identity.
//!
//! # Zero-allocation guarantee
//! All operations work directly on `&[u8]`.  No `String` or `Vec` is
//! constructed at any point in the hot path.
//!
//! # MSB flag convention
//! Any `u64` returned by this module has bit 63 set to `1`.  The Webizen VM
//! uses that bit to distinguish absolute hardware/disk QRC pointers (MSB = 1)
//! from local dictionary hashes produced by `q_hash` (MSB = 0).

/// The mandatory URI prefix for all `did:q42` coordinates.
const PREFIX: &[u8] = b"did:q42:";
/// Display-formatted pointer prefix emitted by `resolver::format_ntriples_to`.
const PTR_PREFIX: &[u8] = b"did:q42:ptr/";

/// Errors returned by [`parse_did_q42`] and [`parse_did_q42_or_ptr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierError {
    /// The byte slice does not begin with `did:q42:`.
    InvalidPrefix,
    /// The payload section (after the prefix) is empty or contains bytes that
    /// cannot form a valid coordinate or hex pointer.
    MalformedHash,
    /// Reserved for future checksum verification of the multicodec payload.
    InvalidChecksum,
}

/// Parse a `did:q42:` URI byte slice into a 64-bit Q42 Resource Coordinate (QRC).
///
/// # Contract
/// * Input must start with `b"did:q42:"`.
/// * The payload after the prefix must be non-empty.
/// * The returned `u64` always has **bit 63 set** — signalling to the Webizen
///   VM that this value is a QRC coordinate, not a dictionary hash.
///
/// # Example
/// ```
/// use qualia_core_db::identifier::parse_did_q42;
/// let ptr = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
/// assert_eq!(ptr >> 63, 1, "MSB must be set for QRC coordinates");
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

/// Parse either a standard `did:q42:<payload>` coordinate or a display-serialized
/// `did:q42:ptr/<hex>` / `<did:q42:ptr/<hex>>` coordinate emitted by
/// `resolver::format_ntriples_to`, returning the canonical 64-bit QRC pointer
/// (`MSB = 1`, low 60 bits = coordinate payload).
///
/// Note: `did:q42:ptr/{hex}` is strictly a display round-trip parser and never
/// grants raw memory dereference authority.
pub fn parse_did_q42_or_ptr(uri: &[u8]) -> Result<u64, IdentifierError> {
    let inner = if uri.len() >= 2 && uri[0] == b'<' && uri[uri.len() - 1] == b'>' {
        &uri[1..uri.len() - 1]
    } else {
        uri
    };

    if inner.starts_with(PTR_PREFIX) {
        let hex = &inner[PTR_PREFIX.len()..];
        if hex.is_empty() || hex.len() > 16 {
            return Err(IdentifierError::MalformedHash);
        }
        let mut val: u64 = 0;
        for &b in hex {
            let nibble = match b {
                b'0'..=b'9' => (b - b'0') as u64,
                b'a'..=b'f' => (b - b'a' + 10) as u64,
                b'A'..=b'F' => (b - b'A' + 10) as u64,
                _ => return Err(IdentifierError::MalformedHash),
            };
            val = (val << 4) | nibble;
        }
        // Bits 60..62 are reserved for inline type tags when MSB=0; for QRC
        // pointers (MSB=1), only bit 63 and the 60-bit payload may be set.
        if (val & 0x7000_0000_0000_0000) != 0 {
            return Err(IdentifierError::MalformedHash);
        }
        return Ok((val & 0x0FFF_FFFF_FFFF_FFFF) | (1u64 << 63));
    }

    parse_did_q42(inner)
}

/// FNV-1a over a raw byte slice — identical to `crate::q_hash` (60-bit identity).
/// Kept local so this module has no runtime dependency on the crate root.
/// Truncated to 60 bits so a did:q42 topological pointer shares the ONE identity
/// space (low 60 bits) with dictionary hashes, differing only by its MSB tag —
/// and so dictionary hashes (MSB always 0) never collide with topological
/// pointers (MSB set), making the MSB a reliable discriminator.
#[inline(always)]
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash & 0x0FFF_FFFF_FFFF_FFFF
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
        let plain = crate::q_hash("z6MkpTHR8VNs");
        let pointer = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        assert_eq!(pointer, plain | (1u64 << 63));
    }

    #[test]
    fn test_parse_did_q42_or_ptr_roundtrip() {
        let orig = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
        let formatted = format!("<did:q42:ptr/{orig:016x}>");
        let recovered = parse_did_q42_or_ptr(formatted.as_bytes()).unwrap();
        assert_eq!(recovered, orig);
    }
}
