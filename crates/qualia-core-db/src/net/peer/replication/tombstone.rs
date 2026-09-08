//! Tombstones retained until an authorized ack frontier (SVC-01.11 partial).
//!
//! A tombstone is not a completeness proof that every replica compacted. A
//! stale returning replica cannot contribute until reauthorize/resnapshot.
//! Transport ACK is not an authorized frontier. Packages remain open.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub const MAX_TOMBSTONES: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tombstone {
    pub op_id: StrongDigest,
    pub epoch: u64,
    pub sequence: u32,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    ts: Tombstone,
}

/// Replica contribution gate. Stale until [`ReplicaGate::reauthorize`].
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaStatus {
    Authorized = 1,
    Stale = 2,
}

pub struct TombstoneTable {
    slots: [Slot; MAX_TOMBSTONES],
    /// Exclusive: sequences `< frontier_seq` in `frontier_epoch` may compact
    /// when `has_frontier` is set. Epoch exclusion: `epoch < excluded_before`.
    has_frontier: bool,
    frontier_epoch: u64,
    frontier_seq: u32,
    excluded_before: u64,
}

pub struct ReplicaGate {
    status: ReplicaStatus,
}

impl TombstoneTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                ts: Tombstone {
                    op_id: StrongDigest::ZERO,
                    epoch: 0,
                    sequence: 0,
                },
            }; MAX_TOMBSTONES],
            has_frontier: false,
            frontier_epoch: 0,
            frontier_seq: 0,
            excluded_before: 0,
        }
    }

    pub fn retain(&mut self, ts: Tombstone) -> Result<(), QdnfError> {
        if ts.op_id == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if self.find_id(&ts.op_id).is_some() {
            return Ok(());
        }
        let idx = self.find_free().ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Slot { occupied: true, ts };
        Ok(())
    }

    pub fn occupied_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_TOMBSTONES {
            if self.slots[i].occupied {
                n += 1;
            }
            i += 1;
        }
        n
    }

    /// Transport ACK is not an authorized compaction frontier.
    pub fn apply_transport_ack(&mut self, _op_id: StrongDigest) -> Result<(), QdnfError> {
        Err(QdnfError::Unauthorized)
    }

    /// Authorized ack frontier. Compacts matching retained tombstones.
    pub fn apply_authorized_frontier(
        &mut self,
        epoch: u64,
        sequence: u32,
    ) -> Result<usize, QdnfError> {
        self.has_frontier = true;
        self.frontier_epoch = epoch;
        self.frontier_seq = sequence;
        Ok(self.compact())
    }

    /// Explicit epoch exclusion: tombstones with `epoch < excluded_before` compact.
    pub fn apply_epoch_exclusion(&mut self, excluded_before: u64) -> Result<usize, QdnfError> {
        self.excluded_before = excluded_before;
        Ok(self.compact())
    }

    pub fn contains(&self, op_id: &StrongDigest) -> bool {
        self.find_id(op_id).is_some()
    }

    fn compact(&mut self) -> usize {
        let mut released = 0usize;
        let mut i = 0usize;
        while i < MAX_TOMBSTONES {
            if self.slots[i].occupied && self.may_compact(&self.slots[i].ts) {
                self.slots[i].occupied = false;
                released += 1;
            }
            i += 1;
        }
        released
    }

    fn may_compact(&self, ts: &Tombstone) -> bool {
        if ts.epoch < self.excluded_before {
            return true;
        }
        self.has_frontier && ts.epoch == self.frontier_epoch && ts.sequence < self.frontier_seq
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_TOMBSTONES {
            if self.slots[i].occupied && self.slots[i].ts.op_id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_TOMBSTONES {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

impl Default for TombstoneTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicaGate {
    pub const fn authorized() -> Self {
        Self {
            status: ReplicaStatus::Authorized,
        }
    }

    pub const fn stale() -> Self {
        Self {
            status: ReplicaStatus::Stale,
        }
    }

    pub fn contribute(&self) -> Result<(), QdnfError> {
        match self.status {
            ReplicaStatus::Authorized => Ok(()),
            ReplicaStatus::Stale => Err(QdnfError::Unauthorized),
        }
    }

    pub fn mark_stale(&mut self) {
        self.status = ReplicaStatus::Stale;
    }

    pub fn reauthorize(&mut self) {
        self.status = ReplicaStatus::Authorized;
    }
}

pub fn tombstone_implies_global_completeness() -> bool {
    false
}

pub fn transport_ack_is_compaction_frontier() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(seq: u32) -> Tombstone {
        let mut id = StrongDigest::ZERO;
        id.0[0] = seq as u8;
        id.0[1] = 0x7a;
        Tombstone {
            op_id: id,
            epoch: 1,
            sequence: seq,
        }
    }

    #[test]
    fn retain_until_authorized_frontier() {
        let mut table = TombstoneTable::new();
        table.retain(ts(1)).unwrap();
        table.retain(ts(2)).unwrap();
        assert_eq!(table.occupied_count(), 2);
        assert_eq!(
            table.apply_transport_ack(ts(1).op_id),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(table.occupied_count(), 2);
        let released = table.apply_authorized_frontier(1, 2).unwrap();
        assert_eq!(released, 1);
        assert!(table.contains(&ts(2).op_id));
        assert!(!table.contains(&ts(1).op_id));
    }

    #[test]
    fn epoch_exclusion_compacts_older_epoch() {
        let mut table = TombstoneTable::new();
        let mut old = ts(1);
        old.epoch = 3;
        let mut keep = ts(9);
        keep.epoch = 5;
        table.retain(old).unwrap();
        table.retain(keep).unwrap();
        let n = table.apply_epoch_exclusion(4).unwrap();
        assert_eq!(n, 1);
        assert!(table.contains(&keep.op_id));
        assert!(!table.contains(&old.op_id));
    }

    #[test]
    fn seventeenth_retain_is_capacity() {
        let mut table = TombstoneTable::new();
        let mut i = 1u32;
        while i <= MAX_TOMBSTONES as u32 {
            table.retain(ts(i)).unwrap();
            i += 1;
        }
        assert_eq!(table.retain(ts(99)), Err(QdnfError::Capacity));
    }

    #[test]
    fn stale_replica_cannot_contribute_until_reauthorize() {
        let mut gate = ReplicaGate::stale();
        assert_eq!(gate.contribute(), Err(QdnfError::Unauthorized));
        gate.reauthorize();
        gate.contribute().unwrap();
        gate.mark_stale();
        assert_eq!(gate.contribute(), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn completeness_and_transport_ack_are_not_frontiers() {
        assert!(!tombstone_implies_global_completeness());
        assert!(!transport_ack_is_compaction_frontier());
    }
}
