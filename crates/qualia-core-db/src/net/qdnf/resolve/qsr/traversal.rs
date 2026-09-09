//! Closed-world exact lookup against an immutable local QSR snapshot.
//!
//! Compares the full 48-byte key. First-byte equality is not membership.
//! `key.0[0] % 96` is not used.

use super::cover::{covers_digit, validate_cover, CoverInterval};
use super::key::{digit, keys_equal};
use super::outcome::QsrOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Occupied slots in a closed-world local map.
pub const SNAPSHOT_CAP: usize = 32;

/// Immutable generation-pinned map of exact key → target digest pairs.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct QsrSnapshot {
    keys: [StrongDigest; SNAPSHOT_CAP],
    values: [StrongDigest; SNAPSHOT_CAP],
    len: u8,
    generation: Generation,
}

impl QsrSnapshot {
    pub const fn empty(generation: Generation) -> Self {
        Self {
            keys: [StrongDigest::ZERO; SNAPSHOT_CAP],
            values: [StrongDigest::ZERO; SNAPSHOT_CAP],
            len: 0,
            generation,
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Append a pair. Duplicate keys are retained so conflict detection can fire.
    pub fn insert(&mut self, key: StrongDigest, value: StrongDigest) -> Result<(), QdnfError> {
        if self.len as usize >= SNAPSHOT_CAP {
            return Err(QdnfError::Capacity);
        }
        let i = self.len as usize;
        self.keys[i] = key;
        self.values[i] = value;
        self.len = self.len + 1;
        Ok(())
    }
}

/// Exact lookup. Copies matching targets into `out` when `Found`.
pub fn lookup_into(
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    validate_cover(covers)?;
    if required_generation != snapshot.generation && required_generation != Generation(0) {
        return Ok(QsrOutcome::Stale);
    }
    let d0 = digit(key, 0)?;
    if !covers_digit(covers, d0) {
        return Ok(QsrOutcome::Incomplete);
    }

    let occupied = snapshot.len as usize;
    let mut match_i = usize::MAX;
    let mut match_n: u8 = 0;
    let mut conflict = false;
    let mut i = 0usize;
    while i < occupied {
        if keys_equal(&snapshot.keys[i], key) {
            if match_n == 0 {
                match_i = i;
                match_n = 1;
            } else if !keys_equal(&snapshot.values[i], &snapshot.values[match_i]) {
                conflict = true;
            } else if match_n < 64 {
                match_n = match_n + 1;
            }
        }
        i += 1;
    }

    if conflict {
        return Ok(QsrOutcome::Conflict);
    }
    if match_n == 0 {
        return Ok(QsrOutcome::EmptyInSnapshot);
    }
    if out.is_empty() {
        return Ok(QsrOutcome::NeedContinuation);
    }
    out[0] = snapshot.values[match_i];
    Ok(QsrOutcome::Found { count: 1 })
}

/// Exact lookup using a one-slot stack buffer for the target copy.
pub fn lookup(
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
) -> Result<QsrOutcome, QdnfError> {
    let mut slot = [StrongDigest::ZERO; 1];
    lookup_into(snapshot, key, covers, required_generation, &mut slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::resolve::qsr::key::KEY_DEPTH;

    fn key_same_first_byte(rest: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0x10;
        let mut i = 1usize;
        while i < 48 {
            k.0[i] = rest;
            i += 1;
        }
        k
    }

    fn full_cover() -> [CoverInterval; 1] {
        [CoverInterval { start: 0, end: 15 }]
    }

    #[test]
    fn shared_first_byte_is_distinguished_by_full_key() {
        let a = key_same_first_byte(0x01);
        let b = key_same_first_byte(0x02);
        assert_eq!(a.0[0], b.0[0]);
        assert_eq!(a.0[0] % KEY_DEPTH as u8, b.0[0] % KEY_DEPTH as u8);

        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(a, StrongDigest::ZERO).unwrap();
        let covers = full_cover();

        assert_eq!(
            lookup(&snap, &a, &covers, Generation(1)),
            Ok(QsrOutcome::Found { count: 1 })
        );
        assert_eq!(
            lookup(&snap, &b, &covers, Generation(1)),
            Ok(QsrOutcome::EmptyInSnapshot)
        );
    }

    #[test]
    fn first_byte_modulo_must_not_succeed() {
        let a = key_same_first_byte(0xaa);
        let b = key_same_first_byte(0xbb);
        assert_eq!(a.0[0] % 96, b.0[0] % 96);

        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(a, StrongDigest::ZERO).unwrap();
        let covers = full_cover();

        assert_eq!(
            lookup(&snap, &b, &covers, Generation(0)),
            Ok(QsrOutcome::EmptyInSnapshot)
        );
        assert_ne!(
            lookup(&snap, &b, &covers, Generation(0)),
            Ok(QsrOutcome::Found { count: 1 })
        );
    }

    #[test]
    fn stale_generation_is_stale() {
        let mut snap = QsrSnapshot::empty(Generation(2));
        snap.insert(StrongDigest::ZERO, StrongDigest::ZERO).unwrap();
        let covers = full_cover();
        assert_eq!(
            lookup(&snap, &StrongDigest::ZERO, &covers, Generation(1)),
            Ok(QsrOutcome::Stale)
        );
        assert_eq!(
            lookup(&snap, &StrongDigest::ZERO, &covers, Generation(0)),
            Ok(QsrOutcome::Found { count: 1 })
        );
    }

    #[test]
    fn empty_snapshot_full_cover_is_empty_in_snapshot() {
        let snap = QsrSnapshot::empty(Generation(1));
        let covers = full_cover();
        assert_eq!(
            lookup(&snap, &StrongDigest::ZERO, &covers, Generation(1)),
            Ok(QsrOutcome::EmptyInSnapshot)
        );
        assert_ne!(
            lookup(&snap, &StrongDigest::ZERO, &covers, Generation(1)),
            Ok(QsrOutcome::Found { count: 1 })
        );
    }

    #[test]
    fn overlapping_cover_is_rejected_before_lookup() {
        let snap = QsrSnapshot::empty(Generation(0));
        let covers = [
            CoverInterval { start: 0, end: 10 },
            CoverInterval { start: 8, end: 12 },
        ];
        assert_eq!(
            lookup(&snap, &StrongDigest::ZERO, &covers, Generation(0)),
            Err(QdnfError::Overlap)
        );
    }

    #[test]
    fn lookup_copies_target_into_caller_buffer() {
        let mut key = StrongDigest::ZERO;
        key.0[0] = 0x20;
        let mut target = StrongDigest::ZERO;
        target.0[47] = 0x77;
        let mut snap = QsrSnapshot::empty(Generation(3));
        snap.insert(key, target).unwrap();
        let covers = full_cover();
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            lookup_into(&snap, &key, &covers, Generation(3), &mut out),
            Ok(QsrOutcome::Found { count: 1 })
        );
        assert!(keys_equal(&out[0], &target));
    }

    #[test]
    fn duplicate_key_conflicting_values_are_conflict() {
        let key = StrongDigest::ZERO;
        let mut v1 = StrongDigest::ZERO;
        let mut v2 = StrongDigest::ZERO;
        v1.0[0] = 1;
        v2.0[0] = 2;
        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(key, v1).unwrap();
        snap.insert(key, v2).unwrap();
        assert_eq!(
            lookup(&snap, &key, &full_cover(), Generation(1)),
            Ok(QsrOutcome::Conflict)
        );
    }
}
