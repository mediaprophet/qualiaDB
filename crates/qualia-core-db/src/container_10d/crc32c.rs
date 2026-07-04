//! Shared CRC-32C (Castagnoli, reflected) — the canonical integrity primitive
//! for the `.10d` container and the `q42/p64_weight.rs` weight container.
//!
//! **P0.3** consolidates the two previously-duplicated implementations (one in
//! `q42/p64_weight.rs`, one in `container_10d/section.rs`) into this single
//! module. Both call sites delegate here. The P0.3 acceptance gate verifies
//! that `p64_weight.rs` checksums stay byte-identical after delegation (the
//! p64 round-trip tests are the proof — they fail on any CRC change).
//!
//! Algorithm: CRC-32C (Castagnoli), reflected, init = 0xFFFF_FFFF, polynomial
//! 0x82F63B78 (the reflected form of 0x1EDC6F41), final XOR = 0xFFFF_FFFF
//! (i.e., bitwise NOT). Table-less — one byte at a time, eight rounds per
//! byte. Zero-heap, no static tables.
//!
//! The canonical check value (the ASCII string `"123456789"` → `0xE3069283`)
//! is pinned by a test below so a future refactor cannot silently change the
//! algorithm. This is the value the P0.3 acceptance gate names ("container
//! pins known CRC-32C vectors (RFC 3720 `0xE3069283`)").

/// CRC-32C (Castagnoli, reflected) over `data`. Table-less, zero-heap.
#[inline]
pub fn crc32c(data: &[u8]) -> u32 {
    !crc32c_update(0xFFFF_FFFF, data)
}

/// Incremental CRC-32C update: continue a running CRC over `data` starting
/// from `crc` (the previous state, NOT yet final-XOR'd). Returns the updated
/// state (also NOT yet final-XOR'd). To get the final checksum, bitwise-NOT
/// the result (or call [`crc32c`] for the one-shot form).
///
/// This is the form `q42/p64_weight.rs` uses internally for its two-phase
/// metadata CRC (compute over the metadata region, then continue over the
/// checksum table). Delegating both the one-shot and the incremental form
/// keeps p64's call sites byte-identical.
#[inline]
pub fn crc32c_update(mut crc: u32, data: &[u8]) -> u32 {
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32c_check_value_123456789_is_e3069283() {
        // The canonical CRC-32C check value. Pinning it here means any
        // refactor (a different polynomial, a different init/final XOR, a
        // table-based variant with a bug) is caught immediately.
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    }

    #[test]
    fn crc32c_empty_input_is_final_xor_of_init() {
        // CRC-32C of empty input = NOT(0xFFFFFFFF) = 0x00000000.
        assert_eq!(crc32c(&[]), 0x0000_0000);
    }

    #[test]
    fn crc32c_incremental_matches_one_shot() {
        // Splitting the input and using crc32c_update must match the one-shot
        // crc32c over the whole input. This is the property p64_weight.rs
        // relies on for its two-phase metadata CRC.
        let data = b"The quick brown fox jumps over the lazy dog";
        let one_shot = crc32c(data);
        let mut state = crc32c_update(0xFFFF_FFFF, &data[..10]);
        state = crc32c_update(state, &data[10..20]);
        state = crc32c_update(state, &data[20..]);
        assert_eq!(!state, one_shot);
    }

    #[test]
    fn crc32c_is_deterministic() {
        // Same input → same output, always (no state, no randomness).
        let data = b"deterministic input";
        assert_eq!(crc32c(data), crc32c(data));
    }
}
