//! Full-key and four-bit digit operations for QSR.
//!
//! `digit` is indexing only. It never certifies that a record exists or that a
//! cover is authentic. Depth is a nibble index into the 48-byte StrongDigest
//! (96 four-bit positions). It is not a first-byte bucket count.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Nibbles in a 48-byte StrongDigest (384 bits).
pub const KEY_DEPTH: usize = 96;
/// Four-bit radix at each depth.
pub const DIGIT_RADIX: u8 = 16;

/// Nibble at `depth` (0..96). Even depth is the high nibble of that byte.
pub fn digit(key: &StrongDigest, depth: usize) -> Result<u8, QdnfError> {
    if depth >= KEY_DEPTH {
        return Err(QdnfError::Range);
    }
    let byte = key.0[depth / 2];
    Ok(if depth % 2 == 0 {
        byte >> 4
    } else {
        byte & 0x0f
    })
}

/// Full 48-byte equality. Callers must not substitute a first-byte test.
pub fn keys_equal(a: &StrongDigest, b: &StrongDigest) -> bool {
    let mut eq = true;
    let mut i = 0usize;
    while i < 48 {
        if a.0[i] != b.0[i] {
            eq = false;
        }
        i += 1;
    }
    eq
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_bytes(b0: u8, b1: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = b0;
        k.0[1] = b1;
        k.0[47] = 0xab;
        k
    }

    #[test]
    fn digit_zero_is_high_nibble_of_byte_zero() {
        let key = key_bytes(0xab, 0xcd);
        assert_eq!(digit(&key, 0), Ok(0x0a));
    }

    #[test]
    fn digit_one_is_low_nibble_of_byte_zero() {
        let key = key_bytes(0xab, 0xcd);
        assert_eq!(digit(&key, 1), Ok(0x0b));
    }

    #[test]
    fn digit_two_and_three_are_byte_one() {
        let key = key_bytes(0xab, 0xcd);
        assert_eq!(digit(&key, 2), Ok(0x0c));
        assert_eq!(digit(&key, 3), Ok(0x0d));
    }

    #[test]
    fn digit_ninety_five_is_last_nibble() {
        let key = key_bytes(0xab, 0xcd);
        assert_eq!(digit(&key, 95), Ok(0x0b));
    }

    #[test]
    fn digit_depth_ninety_six_is_range() {
        let key = StrongDigest::ZERO;
        assert_eq!(digit(&key, KEY_DEPTH), Err(QdnfError::Range));
        assert_eq!(digit(&key, 97), Err(QdnfError::Range));
    }

    #[test]
    fn keys_equal_distinguishes_later_bytes() {
        let mut a = StrongDigest::ZERO;
        let mut b = StrongDigest::ZERO;
        a.0[0] = 0x10;
        b.0[0] = 0x10;
        b.0[47] = 1;
        assert!(!keys_equal(&a, &b));
        assert!(keys_equal(&a, &a));
    }
}
