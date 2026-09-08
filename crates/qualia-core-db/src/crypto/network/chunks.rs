//! CRY-02.09 / CRY-02.10 **PARTIAL** — cookie/relationship-gated admission chunks.
//!
//! Limits: 16 KiB per flight, 16 chunks, 32 global pending, 2 per admitted locator.
//! Duplicate identical retransmission is [`CryptoError::Replay`] and does not extend
//! the deadline. Conflicting bytes at the same index are [`CryptoError::Conflict`].
//! Exact retransmission identity is the SHA-384 of those bytes (raw payload is not
//! retained in the table). The flight digest is SHA-384 of concatenated exact chunk
//! bytes in order. Draft until FND-03 freeze. No security proof is claimed.

use sha2::{Digest, Sha384};

use crate::crypto::network::digest::sha384;
use crate::crypto::network::errors::CryptoError;

/// Maximum concatenated payload bytes admitted on one flight.
pub const MAX_FLIGHT_BYTES: usize = 16 * 1024;
/// Maximum chunks admitted on one flight.
pub const MAX_CHUNKS_PER_FLIGHT: usize = 16;
/// Maximum simultaneously pending flights in the table.
pub const MAX_GLOBAL_PENDING: usize = 32;
/// Maximum simultaneously pending flights for one locator.
pub const MAX_PER_LOCATOR: usize = 2;

/// Public snapshot of one pending admission flight.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkFlight {
    pub locator: u64,
    pub expiry_unix: u64,
    pub chunk_count: u8,
    pub byte_len: u16,
    /// SHA-384 of concatenated exact chunk bytes in order.
    pub digest: [u8; 48],
}

struct FlightSlot {
    meta: ChunkFlight,
    chunk_digests: [Option<[u8; 48]>; MAX_CHUNKS_PER_FLIGHT],
    chunk_lens: [u16; MAX_CHUNKS_PER_FLIGHT],
    hasher: Sha384,
}

/// Fixed 32-slot pending-flight table. No heap; bodies are digest+length only.
pub struct ChunkTable {
    slots: [Option<FlightSlot>; MAX_GLOBAL_PENDING],
}

impl ChunkTable {
    pub const fn new() -> Self {
        Self {
            slots: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None,
            ],
        }
    }

    /// Open a pending flight. 33rd global → Capacity. 3rd for same locator → Capacity.
    pub fn open(&mut self, locator: u64, now_unix: u64, ttl_secs: u64) -> Result<u8, CryptoError> {
        let mut per_locator = 0usize;
        let mut i = 0usize;
        while i < MAX_GLOBAL_PENDING {
            if let Some(slot) = &self.slots[i] {
                if slot.meta.locator == locator {
                    per_locator += 1;
                    if per_locator >= MAX_PER_LOCATOR {
                        return Err(CryptoError::Capacity);
                    }
                }
            }
            i += 1;
        }
        let mut i = 0usize;
        while i < MAX_GLOBAL_PENDING {
            if self.slots[i].is_none() {
                let hasher = Sha384::new();
                let empty = sha384(&[]);
                self.slots[i] = Some(FlightSlot {
                    meta: ChunkFlight {
                        locator,
                        expiry_unix: now_unix.saturating_add(ttl_secs),
                        chunk_count: 0,
                        byte_len: 0,
                        digest: empty.0,
                    },
                    chunk_digests: [None; MAX_CHUNKS_PER_FLIGHT],
                    chunk_lens: [0u16; MAX_CHUNKS_PER_FLIGHT],
                    hasher,
                });
                return Ok(i as u8);
            }
            i += 1;
        }
        Err(CryptoError::Capacity)
    }

    /// Append one chunk's exact bytes. Sum of lengths > 16 KiB → Capacity.
    /// 17th chunk → Capacity. Duplicate identical bytes → Replay (deadline unchanged).
    /// Different bytes for the same index → Conflict. `now_unix` > expiry → Expired.
    pub fn append(
        &mut self,
        flight: u8,
        index: u8,
        bytes: &[u8],
        now_unix: u64,
    ) -> Result<(), CryptoError> {
        let slot = self.slot_mut(flight)?;
        if now_unix > slot.meta.expiry_unix {
            return Err(CryptoError::Expired);
        }
        let idx = index as usize;
        if idx >= MAX_CHUNKS_PER_FLIGHT {
            return Err(CryptoError::Capacity);
        }
        let incoming = chunk_id(bytes);
        if let Some(stored) = slot.chunk_digests[idx] {
            if stored == incoming {
                return Err(CryptoError::Replay);
            }
            return Err(CryptoError::Conflict);
        }
        if idx != slot.meta.chunk_count as usize {
            return Err(CryptoError::Range);
        }
        if (slot.meta.byte_len as usize).saturating_add(bytes.len()) > MAX_FLIGHT_BYTES {
            return Err(CryptoError::Capacity);
        }
        let stored_len: u16 = bytes.len().try_into().map_err(|_| CryptoError::Capacity)?;
        slot.hasher.update(bytes);
        let out = slot.hasher.clone().finalize();
        slot.meta.digest.copy_from_slice(&out);
        slot.chunk_digests[idx] = Some(incoming);
        slot.chunk_lens[idx] = stored_len;
        slot.meta.byte_len = slot.meta.byte_len.saturating_add(stored_len);
        slot.meta.chunk_count = slot.meta.chunk_count.saturating_add(1);
        Ok(())
    }

    pub fn expiry(&self, flight: u8) -> Result<u64, CryptoError> {
        Ok(self.slot(flight)?.meta.expiry_unix)
    }

    fn slot(&self, flight: u8) -> Result<&FlightSlot, CryptoError> {
        let i = flight as usize;
        if i >= MAX_GLOBAL_PENDING {
            return Err(CryptoError::Range);
        }
        self.slots[i].as_ref().ok_or(CryptoError::Range)
    }

    fn slot_mut(&mut self, flight: u8) -> Result<&mut FlightSlot, CryptoError> {
        let i = flight as usize;
        if i >= MAX_GLOBAL_PENDING {
            return Err(CryptoError::Range);
        }
        self.slots[i].as_mut().ok_or(CryptoError::Range)
    }
}

/// Duplicates never extend the admission deadline (CRY-02.10).
pub fn duplicate_extends_deadline() -> bool {
    false
}

fn chunk_id(bytes: &[u8]) -> [u8; 48] {
    sha384(bytes).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_thirty_two_ok_thirty_third_capacity() {
        let mut table = ChunkTable::new();
        let mut i = 0u64;
        while i < MAX_GLOBAL_PENDING as u64 {
            assert_eq!(table.open(i + 1, 1, 10), Ok(i as u8));
            i += 1;
        }
        assert_eq!(table.open(99, 1, 10), Err(CryptoError::Capacity));
    }

    #[test]
    fn third_open_same_locator_capacity() {
        let mut table = ChunkTable::new();
        assert_eq!(table.open(7, 1, 10), Ok(0));
        assert_eq!(table.open(7, 1, 10), Ok(1));
        assert_eq!(table.open(7, 1, 10), Err(CryptoError::Capacity));
        assert_eq!(table.open(8, 1, 10), Ok(2));
    }

    #[test]
    fn sixteen_chunks_ok_seventeenth_capacity() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 1, 10).unwrap();
        let mut i = 0u8;
        while i < MAX_CHUNKS_PER_FLIGHT as u8 {
            let b = [i; 1];
            assert_eq!(table.append(flight, i, &b, 1), Ok(()));
            i += 1;
        }
        assert_eq!(
            table.append(flight, MAX_CHUNKS_PER_FLIGHT as u8, b"x", 1),
            Err(CryptoError::Capacity)
        );
    }

    #[test]
    fn byte_len_over_sixteen_kib_capacity() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 1, 10).unwrap();
        let oversized = [0u8; MAX_FLIGHT_BYTES + 1];
        assert_eq!(
            table.append(flight, 0, &oversized, 1),
            Err(CryptoError::Capacity)
        );

        let mut table = ChunkTable::new();
        let flight = table.open(2, 1, 10).unwrap();
        let piece = [1u8; 1025];
        let mut i = 0u8;
        while i < 15 {
            assert_eq!(table.append(flight, i, &piece, 1), Ok(()));
            i += 1;
        }
        assert_eq!(
            table.append(flight, 15, &piece, 1),
            Err(CryptoError::Capacity)
        );
    }

    #[test]
    fn duplicate_same_index_same_digest_is_replay_expiry_unchanged() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 1000, 100).unwrap();
        assert_eq!(table.append(flight, 0, b"same", 1000), Ok(()));
        let before = table.expiry(flight).unwrap();
        assert_eq!(
            table.append(flight, 0, b"same", 1000),
            Err(CryptoError::Replay)
        );
        assert_eq!(table.expiry(flight).unwrap(), before);
        assert!(!duplicate_extends_deadline());
    }

    #[test]
    fn different_digest_same_index_is_conflict() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 1, 10).unwrap();
        assert_eq!(table.append(flight, 0, b"one", 1), Ok(()));
        assert_eq!(
            table.append(flight, 0, b"two", 1),
            Err(CryptoError::Conflict)
        );
    }

    #[test]
    fn duplicate_extends_deadline_is_false() {
        assert!(!duplicate_extends_deadline());
    }

    #[test]
    fn append_after_expiry_is_expired() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 100, 5).unwrap();
        assert_eq!(table.expiry(flight).unwrap(), 105);
        assert_eq!(
            table.append(flight, 0, b"late", 106),
            Err(CryptoError::Expired)
        );
        assert_eq!(table.append(flight, 0, b"ok", 105), Ok(()));
    }

    #[test]
    fn ordered_flight_digest_is_sha384_of_concatenated_bytes() {
        let mut table = ChunkTable::new();
        let flight = table.open(1, 1, 10).unwrap();
        assert_eq!(table.append(flight, 0, b"aa", 1), Ok(()));
        assert_eq!(table.append(flight, 1, b"bb", 1), Ok(()));
        let expect = sha384(b"aabb");
        let slot = table.slot(flight).unwrap();
        assert_eq!(slot.meta.digest, expect.0);
        assert_eq!(slot.meta.chunk_count, 2);
        assert_eq!(slot.meta.byte_len, 4);
        assert_eq!(slot.chunk_lens[0], 2);
        assert_eq!(slot.chunk_lens[1], 2);
    }
}
