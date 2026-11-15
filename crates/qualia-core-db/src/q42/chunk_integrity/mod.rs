//! Full BLAKE3 chunk integrity. This is a digest, not a Bao range proof.
//!
//! A 64-byte trailer contains the complete 32-byte BLAKE3 digest of the record
//! bytes followed by 32 zero bytes. Keeping the trailer aligned leaves the
//! existing 64-byte records unchanged. Legacy truncated prefixes are not seals.

pub const BLAKE3_TRAILER_BYTES: usize = 64;

/// Construct an aligned trailer without allocating or truncating the digest.
pub fn blake3_trailer(records: &[u8]) -> [u8; BLAKE3_TRAILER_BYTES] {
    let mut trailer = [0; BLAKE3_TRAILER_BYTES];
    trailer[..32].copy_from_slice(blake3::hash(records).as_bytes());
    trailer
}

/// Verify all 256 digest bits, exact trailer size, and reserved zero bytes.
pub fn verify_blake3_trailer(records: &[u8], trailer: &[u8]) -> bool {
    trailer.len() == BLAKE3_TRAILER_BYTES
        && trailer[32..].iter().all(|&b| b == 0)
        && trailer[..32] == *blake3::hash(records).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_digest_byte_and_reserved_byte_is_checked() {
        let payload = b"full BLAKE3 digest";
        let good = blake3_trailer(payload);
        assert!(verify_blake3_trailer(payload, &good));
        for i in 0..64 {
            let mut bad = good;
            bad[i] ^= 1;
            assert!(!verify_blake3_trailer(payload, &bad), "byte {i}");
        }
        assert!(!verify_blake3_trailer(b"changed", &good));
        assert!(!verify_blake3_trailer(payload, &good[..32]));
    }
}

#[cfg(test)]
mod container_tests;
