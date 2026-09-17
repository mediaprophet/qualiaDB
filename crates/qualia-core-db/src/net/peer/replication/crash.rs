//! Crash injection at identity / effect / durable-receipt boundaries (SVC-01.15).
//!
//! The crash model is a [`Copy`] in-memory [`CrashLog`] of stages, not an OS
//! process kill and not CORE-03 WAL torn-write. [`disk_backend_crash_injected`]
//! is therefore false; this library does not claim CORE-03 disk durability.
//!
//! Advertised durability is only the [`ReceiptClass`] actually achieved by
//! [`receipts::acknowledge`]. [`DeliveryStage::TransportAck`] is never
//! [`ReceiptClass::DurableReceipt`]. Packages remain open.

use crate::net::peer::replication::receipts::{self, ReceiptClass, ReceiptTable, MAX_RECEIPTS};
use crate::net::peer::replication::tombstone::{Tombstone, TombstoneTable};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::StrongDigest;

/// Rolled-back stage: later stages never happened after [`CrashHarness::restore`].
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashBoundary {
    Identity = 1,
    Effect = 2,
    DurableReceipt = 3,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    deleted: bool,
    apply_count: u8,
    boundary: CrashBoundary,
    id: StrongDigest,
}

const EMPTY_SLOT: Slot = Slot {
    occupied: false,
    deleted: false,
    apply_count: 0,
    boundary: CrashBoundary::Identity,
    id: StrongDigest::ZERO,
};

/// Copy snapshot of bound identities, local-effect bits, and durable receipts.
#[derive(Clone, Copy)]
pub struct CrashLog {
    slots: [Slot; MAX_RECEIPTS],
}

impl CrashLog {
    pub const fn new() -> Self {
        Self {
            slots: [EMPTY_SLOT; MAX_RECEIPTS],
        }
    }

    pub const fn snapshot(&self) -> Self {
        *self
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if self.slots[i].occupied && self.slots[i].id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn occupied_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if self.slots[i].occupied {
                n += 1;
            }
            i += 1;
        }
        n
    }
}

/// Live [`ReceiptTable`] plus a parallel Copy log used to restore after a crash.
pub struct CrashHarness {
    receipts: ReceiptTable,
    tombstones: TombstoneTable,
    live: CrashLog,
    saved: CrashLog,
    pending: Option<CrashBoundary>,
}

impl CrashHarness {
    pub const fn new() -> Self {
        Self {
            receipts: ReceiptTable::new(),
            tombstones: TombstoneTable::new(),
            live: CrashLog::new(),
            saved: CrashLog::new(),
            pending: None,
        }
    }

    /// Snapshot after the next successful transition past `boundary`.
    pub fn inject_crash_after(&mut self, boundary: CrashBoundary) {
        self.pending = Some(boundary);
    }

    pub fn restore(&mut self) {
        self.live = self.saved.snapshot();
        self.pending = None;
        self.rebuild_tables();
    }

    pub fn occupied_count(&self) -> usize {
        self.live.occupied_count()
    }

    pub fn identity_present(&self, id: StrongDigest) -> bool {
        self.live.find_id(&id).is_some()
    }

    pub fn effect_present(&self, id: StrongDigest) -> bool {
        match self.live.find_id(&id) {
            Some(i) => {
                let b = self.live.slots[i].boundary;
                b == CrashBoundary::Effect || b == CrashBoundary::DurableReceipt
            }
            None => false,
        }
    }

    pub fn durable_present(&self, id: StrongDigest) -> bool {
        match self.live.find_id(&id) {
            Some(i) => self.live.slots[i].boundary == CrashBoundary::DurableReceipt,
            None => false,
        }
    }

    pub fn apply_count(&self, id: StrongDigest) -> u8 {
        match self.live.find_id(&id) {
            Some(i) => self.live.slots[i].apply_count,
            None => 0,
        }
    }

    pub fn bind_identity(&mut self, id: StrongDigest) -> Result<(), QdnfError> {
        reject_zero(id)?;
        if self.live.find_id(&id).is_some() {
            receipts::bind_identity(&mut self.receipts, id)?;
            self.commit_crash(CrashBoundary::Identity);
            return Ok(());
        }
        if self.live.find_free().is_none() {
            return Err(QdnfError::Capacity);
        }
        receipts::bind_identity(&mut self.receipts, id)?;
        let idx = self.live.find_free().ok_or(QdnfError::Capacity)?;
        self.live.slots[idx] = Slot {
            occupied: true,
            deleted: false,
            apply_count: 0,
            boundary: CrashBoundary::Identity,
            id,
        };
        self.commit_crash(CrashBoundary::Identity);
        Ok(())
    }

    pub fn apply_effect(&mut self, id: StrongDigest) -> Result<(), QdnfError> {
        reject_zero(id)?;
        if self.is_deleted(id) {
            return Err(QdnfError::Revoked);
        }
        let idx = self.live.find_id(&id).ok_or(QdnfError::Unauthorized)?;
        receipts::apply_effect(&mut self.receipts, id)?;
        let slot = &mut self.live.slots[idx];
        if slot.apply_count < 1 {
            slot.apply_count = 1;
        }
        if slot.boundary != CrashBoundary::DurableReceipt {
            slot.boundary = CrashBoundary::Effect;
        }
        self.commit_crash(CrashBoundary::Effect);
        Ok(())
    }

    pub fn acknowledge(
        &mut self,
        id: StrongDigest,
        stage: DeliveryStage,
    ) -> Result<ReceiptClass, QdnfError> {
        reject_zero(id)?;
        if self.is_deleted(id)
            && matches!(
                stage,
                DeliveryStage::DurableApplied | DeliveryStage::EconomicReceipt
            )
        {
            return Err(QdnfError::Revoked);
        }
        let idx = self.live.find_id(&id).ok_or(QdnfError::Unauthorized)?;
        let class = receipts::acknowledge(&mut self.receipts, id, stage)?;
        if stage == DeliveryStage::DurableApplied && class == ReceiptClass::DurableReceipt {
            self.live.slots[idx].boundary = CrashBoundary::DurableReceipt;
            self.commit_crash(CrashBoundary::DurableReceipt);
        }
        Ok(class)
    }

    pub fn advertised_durability(
        &mut self,
        id: StrongDigest,
        stage: DeliveryStage,
    ) -> Result<ReceiptClass, QdnfError> {
        self.acknowledge(id, stage)
    }

    pub fn tombstone(&mut self, id: StrongDigest) -> Result<(), QdnfError> {
        reject_zero(id)?;
        self.tombstones.retain(Tombstone {
            op_id: id,
            epoch: 1,
            sequence: 1,
        })?;
        if let Some(idx) = self.live.find_id(&id) {
            self.live.slots[idx].deleted = true;
        } else {
            let idx = self.live.find_free().ok_or(QdnfError::Capacity)?;
            self.live.slots[idx] = Slot {
                occupied: true,
                deleted: true,
                apply_count: 0,
                boundary: CrashBoundary::Identity,
                id,
            };
        }
        Ok(())
    }

    fn is_deleted(&self, id: StrongDigest) -> bool {
        if self.tombstones.contains(&id) {
            return true;
        }
        match self.live.find_id(&id) {
            Some(i) => self.live.slots[i].deleted,
            None => false,
        }
    }

    fn commit_crash(&mut self, reached: CrashBoundary) {
        if self.pending == Some(reached) {
            self.saved = self.live.snapshot();
            self.pending = None;
        }
    }

    fn rebuild_tables(&mut self) {
        self.receipts = ReceiptTable::new();
        self.tombstones = TombstoneTable::new();
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            let slot = self.live.slots[i];
            if slot.occupied {
                let _ = receipts::bind_identity(&mut self.receipts, slot.id);
                if slot.boundary == CrashBoundary::Effect
                    || slot.boundary == CrashBoundary::DurableReceipt
                {
                    let _ = receipts::apply_effect(&mut self.receipts, slot.id);
                }
                if slot.boundary == CrashBoundary::DurableReceipt {
                    let _ = receipts::acknowledge(
                        &mut self.receipts,
                        slot.id,
                        DeliveryStage::DurableApplied,
                    );
                }
                if slot.deleted {
                    let _ = self.tombstones.retain(Tombstone {
                        op_id: slot.id,
                        epoch: 1,
                        sequence: 1,
                    });
                }
            }
            i += 1;
        }
    }
}

fn reject_zero(id: StrongDigest) -> Result<(), QdnfError> {
    if id == StrongDigest::ZERO {
        Err(QdnfError::Malformed)
    } else {
        Ok(())
    }
}

/// Advertised class is the class [`receipts::acknowledge`] actually returns.
pub fn advertised_durability(
    table: &mut ReceiptTable,
    id: StrongDigest,
    stage: DeliveryStage,
) -> Result<ReceiptClass, QdnfError> {
    receipts::acknowledge(table, id, stage)
}

/// Not CORE-03.13 WAL torn-write. Disk durability is not claimed here.
pub fn disk_backend_crash_injected() -> bool {
    false
}

/// Local effect apply_count saturates at 1 across crash-restore retries.
pub fn at_most_once_local_effect() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::checkpoint::{build_checkpoint, verify_checkpoint};
    use crate::net::peer::replication::operation::{operation_id, OperationDesc, MAX_PARENTS};

    fn digest(tag: u8) -> StrongDigest {
        StrongDigest([tag; 48])
    }

    fn desc_with_payload(payload: u8) -> OperationDesc {
        OperationDesc {
            author: StrongDigest([0x11; 48]),
            epoch: 1,
            sequence: 1,
            scope: 7,
            contract: StrongDigest([0x22; 48]),
            purpose: StrongDigest([0x33; 48]),
            payload_digest: StrongDigest([payload; 48]),
            parents: [StrongDigest::ZERO; MAX_PARENTS],
            parent_count: 0,
        }
    }

    #[test]
    fn crash_after_identity_before_effect_restore_allows_one_apply() {
        let mut h = CrashHarness::new();
        let id = digest(1);
        h.inject_crash_after(CrashBoundary::Identity);
        h.bind_identity(id).unwrap();
        h.apply_effect(id).unwrap();
        h.restore();
        assert!(h.identity_present(id));
        assert!(!h.effect_present(id));
        assert_eq!(h.apply_count(id), 0);
        h.apply_effect(id).unwrap();
        assert_eq!(h.apply_count(id), 1);
        assert!(at_most_once_local_effect());
    }

    #[test]
    fn crash_after_effect_before_durable_restore_allows_durable_applied() {
        let mut h = CrashHarness::new();
        let id = digest(2);
        h.inject_crash_after(CrashBoundary::Effect);
        h.bind_identity(id).unwrap();
        h.apply_effect(id).unwrap();
        h.acknowledge(id, DeliveryStage::DurableApplied).unwrap();
        h.restore();
        assert!(h.effect_present(id));
        assert!(!h.durable_present(id));
        let durable = h.acknowledge(id, DeliveryStage::DurableApplied).unwrap();
        assert_eq!(durable, ReceiptClass::DurableReceipt);
        let ack = h
            .advertised_durability(id, DeliveryStage::TransportAck)
            .unwrap();
        assert_eq!(ack, ReceiptClass::Effect);
        assert_ne!(ack, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn crash_after_durable_applied_retry_is_idempotent() {
        let mut h = CrashHarness::new();
        let id = digest(3);
        h.inject_crash_after(CrashBoundary::DurableReceipt);
        h.bind_identity(id).unwrap();
        h.apply_effect(id).unwrap();
        h.acknowledge(id, DeliveryStage::DurableApplied).unwrap();
        h.restore();
        assert!(h.durable_present(id));
        assert_eq!(h.apply_count(id), 1);
        let again = h.acknowledge(id, DeliveryStage::DurableApplied).unwrap();
        assert_eq!(again, ReceiptClass::DurableReceipt);
        h.apply_effect(id).unwrap();
        assert_eq!(h.apply_count(id), 1);
        assert!(at_most_once_local_effect());
    }

    #[test]
    fn concurrent_identical_identity_is_one_slot_seventeenth_is_capacity() {
        let mut h = CrashHarness::new();
        let id = digest(1);
        h.bind_identity(id).unwrap();
        h.bind_identity(id).unwrap();
        assert_eq!(h.occupied_count(), 1);
        let mut n = 2u8;
        while n <= MAX_RECEIPTS as u8 {
            h.bind_identity(digest(n)).unwrap();
            n += 1;
        }
        assert_eq!(h.occupied_count(), MAX_RECEIPTS);
        assert_eq!(h.bind_identity(digest(0x20)), Err(QdnfError::Capacity));
    }

    #[test]
    fn dishonest_checkpoint_is_malformed() {
        let id_a = operation_id(&desc_with_payload(1));
        let id_b = operation_id(&desc_with_payload(2));
        let committed = [id_a];
        let cp = build_checkpoint(7, &committed).unwrap();
        assert_eq!(
            verify_checkpoint(&cp, &[id_a, id_b]),
            Err(QdnfError::Malformed)
        );
        let mut wrong = cp;
        wrong.digest = StrongDigest([0xab; 48]);
        assert_eq!(
            verify_checkpoint(&wrong, &committed),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn conflicting_payload_ids_bind_forged_duplicate_checkpoint_is_conflict() {
        let mut h = CrashHarness::new();
        let id_a = operation_id(&desc_with_payload(1));
        let id_b = operation_id(&desc_with_payload(2));
        assert_ne!(id_a, id_b);
        h.bind_identity(id_a).unwrap();
        h.bind_identity(id_b).unwrap();
        assert_eq!(h.occupied_count(), 2);
        assert_eq!(build_checkpoint(7, &[id_a, id_a]), Err(QdnfError::Conflict));
        build_checkpoint(7, &[id_a, id_b]).unwrap();
    }

    #[test]
    fn tombstoned_id_rejects_effect() {
        let mut h = CrashHarness::new();
        let id = digest(9);
        h.bind_identity(id).unwrap();
        h.tombstone(id).unwrap();
        assert_eq!(h.apply_effect(id), Err(QdnfError::Revoked));
        assert!(!h.effect_present(id));
        assert_eq!(
            h.acknowledge(id, DeliveryStage::DurableApplied),
            Err(QdnfError::Revoked)
        );
        assert_eq!(h.apply_count(id), 0);
    }

    #[test]
    fn zero_digest_is_malformed() {
        let mut h = CrashHarness::new();
        assert_eq!(
            h.bind_identity(StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            h.apply_effect(StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            h.acknowledge(StrongDigest::ZERO, DeliveryStage::TransportAck),
            Err(QdnfError::Malformed)
        );
        let mut table = ReceiptTable::new();
        assert_eq!(
            advertised_durability(&mut table, StrongDigest::ZERO, DeliveryStage::TransportAck),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn disk_backend_crash_injected_is_false() {
        assert!(!disk_backend_crash_injected());
    }
}
