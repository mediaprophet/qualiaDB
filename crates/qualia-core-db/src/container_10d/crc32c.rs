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
//! (i.e., bitwise NOT).
//!
//! **Implementation:** 256-entry slice table (built once via `OnceLock`) —
//! bit-identical to the historical table-less 8-shift-per-byte loop (pinned by
//! the RFC 3720 check value + `crc32c_table_matches_tableless_bit_identical`).
//! Table form is far faster over multi-hundred-MB P64 tensors (toolkit probe:
//! SmolLM2 `from_p64` spent tens of seconds in table-less CRC).
//!
//! The canonical check value (the ASCII string `"123456789"` → `0xE3069283`)
//! is pinned by a test below so a future refactor cannot silently change the
//! algorithm.

use std::sync::OnceLock;

/// CRC-32C (Castagnoli, reflected) over `data`.
#[inline]
pub fn crc32c(data: &[u8]) -> u32 {
    !crc32c_update(0xFFFF_FFFF, data)
}

/// Incremental CRC-32C update: continue a running CRC over `data` starting
/// from `crc` (the previous state, NOT yet final-XOR'd). Returns the updated
/// state (also NOT yet final-XOR'd). To get the final checksum, bitwise-NOT
/// the result (or call [`crc32c`] for the one-shot form).
#[inline]
pub fn crc32c_update(mut crc: u32, data: &[u8]) -> u32 {
    let table = crc32c_table();
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    crc
}

fn crc32c_table() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for i in 0..256 {
            let mut crc = i as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0x82F6_3B78 & 0u32.wrapping_sub(crc & 1));
            }
            t[i] = crc;
        }
        t
    })
}

/// Historical table-less loop — kept for the bit-identical parity test only.
#[cfg(test)]
fn crc32c_update_tableless(mut crc: u32, data: &[u8]) -> u32 {
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
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    }

    #[test]
    fn crc32c_empty_input_is_final_xor_of_init() {
        assert_eq!(crc32c(&[]), 0x0000_0000);
    }

    #[test]
    fn crc32c_incremental_matches_one_shot() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let one_shot = crc32c(data);
        let mut state = crc32c_update(0xFFFF_FFFF, &data[..10]);
        state = crc32c_update(state, &data[10..20]);
        state = crc32c_update(state, &data[20..]);
        assert_eq!(!state, one_shot);
    }

    #[test]
    fn crc32c_table_matches_tableless_bit_identical() {
        let samples: &[&[u8]] = &[
            b"",
            b"123456789",
            b"The quick brown fox jumps over the lazy dog",
            &[0u8; 4096],
        ];
        for s in samples {
            let table = !crc32c_update(0xFFFF_FFFF, s);
            let ref_ = !crc32c_update_tableless(0xFFFF_FFFF, s);
            assert_eq!(table, ref_, "mismatch on sample len={}", s.len());
        }
        let all: Vec<u8> = (0u8..=255).collect();
        assert_eq!(
            !crc32c_update(0xFFFF_FFFF, &all),
            !crc32c_update_tableless(0xFFFF_FFFF, &all)
        );
    }

    #[test]
    fn crc32c_is_deterministic() {
        let data = b"deterministic input";
        assert_eq!(crc32c(data), crc32c(data));
    }
}
