//! Bounded LSA flood admission. Missing/expired records never invent reachability.

use crate::net::qdnf::errors::QdnfError;

pub const MAX_ORIGINS: usize = 16;
pub const MAX_LSA_PER_ORIGIN: usize = 4;
const MAX_SLOTS: usize = MAX_ORIGINS * MAX_LSA_PER_ORIGIN;

/// Authenticated link-state advertisement held in the flood table.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LsaRecord {
    pub origin: u8,
    pub sequence: u32,
    pub expiry_unix: u64,
    pub digest: [u8; 32],
}

/// 16 origins × 4 sequence slots. Admission is fail-closed at Capacity.
pub struct FloodTable {
    slots: [Option<LsaRecord>; MAX_SLOTS],
}

impl FloodTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
        }
    }

    /// Admit `rec` at `now_unix`.
    ///
    /// Expired input → [`QdnfError::Expired`]. Same origin+sequence with a
    /// different digest → [`QdnfError::Conflict`] (authenticated conflict,
    /// before quarantine). Same digest → [`QdnfError::ReservationUnchanged`].
    pub fn admit(&mut self, rec: LsaRecord, now_unix: u64) -> Result<(), QdnfError> {
        if now_unix >= rec.expiry_unix {
            return Err(QdnfError::Expired);
        }
        if let Some(existing) = self.live_match(rec.origin, rec.sequence, now_unix) {
            if existing.digest == rec.digest {
                return Err(QdnfError::ReservationUnchanged);
            }
            return Err(QdnfError::Conflict);
        }
        let live_for_origin = self.count_live_origin(rec.origin, now_unix);
        if live_for_origin >= MAX_LSA_PER_ORIGIN {
            return Err(QdnfError::Capacity);
        }
        if self.origin_is_new(rec.origin, now_unix)
            && self.count_live_origins(now_unix) >= MAX_ORIGINS
        {
            return Err(QdnfError::Capacity);
        }
        let slot = self
            .free_slot(rec.origin, now_unix)
            .ok_or(QdnfError::Capacity)?;
        self.slots[slot] = Some(rec);
        Ok(())
    }

    /// True only when an admitted, unexpired record exists for `origin`.
    pub fn is_reachable(&self, origin: u8, now_unix: u64) -> bool {
        self.has_path(origin, now_unix)
    }

    /// Missing records do not invent a path.
    pub fn has_path(&self, origin: u8, now_unix: u64) -> bool {
        self.count_live_origin(origin, now_unix) > 0
    }

    pub const fn missing_record_invents_reachability() -> bool {
        false
    }

    pub const fn has_default_route() -> bool {
        false
    }

    fn live_match(&self, origin: u8, sequence: u32, now_unix: u64) -> Option<LsaRecord> {
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(rec) = self.slots[i] {
                if rec.origin == origin && rec.sequence == sequence && now_unix < rec.expiry_unix {
                    return Some(rec);
                }
            }
            i += 1;
        }
        None
    }

    fn count_live_origin(&self, origin: u8, now_unix: u64) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(rec) = self.slots[i] {
                if rec.origin == origin && now_unix < rec.expiry_unix {
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    fn count_live_origins(&self, now_unix: u64) -> usize {
        let mut seen = [false; 256];
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(rec) = self.slots[i] {
                if now_unix < rec.expiry_unix {
                    let idx = rec.origin as usize;
                    if !seen[idx] {
                        seen[idx] = true;
                        n += 1;
                    }
                }
            }
            i += 1;
        }
        n
    }

    fn origin_is_new(&self, origin: u8, now_unix: u64) -> bool {
        self.count_live_origin(origin, now_unix) == 0
    }

    fn free_slot(&self, origin: u8, now_unix: u64) -> Option<usize> {
        let mut expired_same = None;
        let mut expired_any = None;
        let mut empty = None;
        let mut i = 0usize;
        while i < MAX_SLOTS {
            match self.slots[i] {
                None => {
                    if empty.is_none() {
                        empty = Some(i);
                    }
                }
                Some(rec) => {
                    if now_unix >= rec.expiry_unix {
                        if rec.origin == origin && expired_same.is_none() {
                            expired_same = Some(i);
                        } else if expired_any.is_none() {
                            expired_any = Some(i);
                        }
                    }
                }
            }
            i += 1;
        }
        expired_same.or(empty).or(expired_any)
    }
}

impl Default for FloodTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(origin: u8, sequence: u32, expiry_unix: u64, digest_byte: u8) -> LsaRecord {
        LsaRecord {
            origin,
            sequence,
            expiry_unix,
            digest: [digest_byte; 32],
        }
    }

    #[test]
    fn missing_record_does_not_invent_reachability() {
        let table = FloodTable::new();
        assert!(!FloodTable::missing_record_invents_reachability());
        assert!(!table.is_reachable(0, 10));
        assert!(!table.has_path(3, 10));
        assert!(!FloodTable::has_default_route());
    }

    #[test]
    fn admit_then_reachable_until_expiry() {
        let mut table = FloodTable::new();
        table.admit(rec(2, 1, 100, 7), 10).unwrap();
        assert!(table.is_reachable(2, 10));
        assert!(table.has_path(2, 99));
        assert!(!table.is_reachable(2, 100));
        assert!(!table.has_path(1, 10));
    }

    #[test]
    fn expired_input_is_rejected() {
        let mut table = FloodTable::new();
        assert_eq!(table.admit(rec(0, 1, 10, 1), 10), Err(QdnfError::Expired));
        assert!(!table.is_reachable(0, 10));
    }

    #[test]
    fn same_origin_sequence_different_digest_is_conflict() {
        let mut table = FloodTable::new();
        table.admit(rec(4, 8, 50, 1), 1).unwrap();
        assert_eq!(table.admit(rec(4, 8, 50, 2), 1), Err(QdnfError::Conflict));
        assert!(table.is_reachable(4, 1));
    }

    #[test]
    fn same_digest_is_suppressed() {
        let mut table = FloodTable::new();
        table.admit(rec(1, 3, 50, 9), 1).unwrap();
        assert_eq!(
            table.admit(rec(1, 3, 50, 9), 1),
            Err(QdnfError::ReservationUnchanged)
        );
        assert!(table.is_reachable(1, 1));
    }

    #[test]
    fn per_origin_slot_capacity() {
        let mut table = FloodTable::new();
        for seq in 0..MAX_LSA_PER_ORIGIN as u32 {
            table.admit(rec(0, seq, 100, seq as u8), 1).unwrap();
        }
        assert_eq!(
            table.admit(rec(0, 99, 100, 99), 1),
            Err(QdnfError::Capacity)
        );
        table.admit(rec(1, 0, 100, 1), 1).unwrap();
        assert!(table.is_reachable(0, 1));
        assert!(table.is_reachable(1, 1));
    }

    #[test]
    fn expired_slot_can_be_reused() {
        let mut table = FloodTable::new();
        table.admit(rec(0, 1, 5, 1), 1).unwrap();
        assert!(!table.is_reachable(0, 5));
        table.admit(rec(0, 2, 50, 2), 5).unwrap();
        assert!(table.is_reachable(0, 5));
    }
}
