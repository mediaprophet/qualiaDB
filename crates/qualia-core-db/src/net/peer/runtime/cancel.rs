//! Operation identity, cancel epoch, and late-completion rules.
//!
//! A completion after cancel releases outstanding storage but cannot resurrect
//! the operation. A stale generation is rejected. Cancellation alone does not
//! make a live I/O lease reusable until `complete` (or an equivalent release)
//! runs.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, OperationId};

use super::handles::LeaseHandle;
use super::leases::LeaseTable;

const OPS: usize = 32;

/// Non-wrapping cancel epoch bound to an admitted operation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CancelEpoch(pub u64);

impl CancelEpoch {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn next(self) -> Result<Self, QdnfError> {
        match self.0.checked_add(1) {
            Some(n) => Ok(Self(n)),
            None => Err(QdnfError::StaleGeneration),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationHandle {
    pub id: OperationId,
    pub generation: Generation,
    pub cancel_epoch: CancelEpoch,
}

#[derive(Clone, Copy)]
struct OpSlot {
    id: OperationId,
    generation: Generation,
    cancel_epoch: CancelEpoch,
    lease: LeaseHandle,
    occupied: bool,
    cancelled: bool,
    storage_held: bool,
}

pub struct OperationTable {
    slots: [OpSlot; OPS],
}

impl OperationTable {
    pub const fn new() -> Self {
        Self {
            slots: [OpSlot {
                id: OperationId::ZERO,
                generation: Generation::ZERO,
                cancel_epoch: CancelEpoch::ZERO,
                lease: LeaseHandle::INVALID,
                occupied: false,
                cancelled: false,
                storage_held: false,
            }; OPS],
        }
    }

    pub fn admit(
        &mut self,
        id: OperationId,
        lease: LeaseHandle,
    ) -> Result<OperationHandle, QdnfError> {
        if id == OperationId::ZERO {
            return Err(QdnfError::Malformed);
        }
        for slot in &self.slots {
            if slot.occupied && slot.id == id {
                return Err(QdnfError::Conflict);
            }
        }
        for slot in &mut self.slots {
            if slot.occupied {
                continue;
            }
            let generation = slot.generation.next()?;
            *slot = OpSlot {
                id,
                generation,
                cancel_epoch: CancelEpoch::ZERO,
                lease,
                occupied: true,
                cancelled: false,
                storage_held: true,
            };
            return Ok(OperationHandle {
                id,
                generation,
                cancel_epoch: CancelEpoch::ZERO,
            });
        }
        Err(QdnfError::Capacity)
    }

    pub fn cancel(&mut self, id: OperationId) -> Result<CancelEpoch, QdnfError> {
        let slot = self.find_mut(id).ok_or(QdnfError::Closed)?;
        if slot.cancelled {
            return Ok(slot.cancel_epoch);
        }
        slot.cancel_epoch = slot.cancel_epoch.next()?;
        slot.cancelled = true;
        Ok(slot.cancel_epoch)
    }

    pub fn is_cancelled(&self, id: OperationId) -> bool {
        self.find(id).map(|s| s.cancelled).unwrap_or(false)
    }

    /// Finish an operation. After cancel this releases storage and returns
    /// `Cancelled` so the caller cannot treat the result as a live success.
    pub fn complete(
        &mut self,
        id: OperationId,
        generation: Generation,
        leases: &mut LeaseTable,
    ) -> Result<(), QdnfError> {
        let idx = self.find_index(id).ok_or(QdnfError::Closed)?;
        if self.slots[idx].generation != generation {
            return Err(QdnfError::StaleGeneration);
        }
        let cancelled = self.slots[idx].cancelled;
        if self.slots[idx].storage_held {
            leases.release(self.slots[idx].lease)?;
            self.slots[idx].storage_held = false;
        }
        let keep_generation = self.slots[idx].generation;
        self.slots[idx].occupied = false;
        self.slots[idx].cancelled = false;
        self.slots[idx].id = OperationId::ZERO;
        self.slots[idx].lease = LeaseHandle::INVALID;
        self.slots[idx].generation = keep_generation;
        self.slots[idx].cancel_epoch = CancelEpoch::ZERO;
        if cancelled {
            Err(QdnfError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn find(&self, id: OperationId) -> Option<&OpSlot> {
        self.slots.iter().find(|s| s.occupied && s.id == id)
    }

    fn find_mut(&mut self, id: OperationId) -> Option<&mut OpSlot> {
        self.slots.iter_mut().find(|s| s.occupied && s.id == id)
    }

    fn find_index(&self, id: OperationId) -> Option<usize> {
        self.slots.iter().position(|s| s.occupied && s.id == id)
    }
}

impl Default for OperationTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::events::{EffectKind, EffectQueue, KernelEffect};
    use crate::net::peer::runtime::leases::LeaseTable;

    fn op(n: u8) -> OperationId {
        let mut id = [0u8; 16];
        id[0] = n;
        OperationId(id)
    }

    #[test]
    fn cancel_then_late_complete_releases_storage() {
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let mut effects = EffectQueue::new();
        let lease = leases.acquire(64, true).unwrap();
        let id = op(1);
        let handle = ops.admit(id, lease.handle).unwrap();
        effects
            .push(KernelEffect {
                kind: EffectKind::SendFrame,
                operation: id,
                lease: lease.handle,
            })
            .unwrap();

        let epoch = ops.cancel(id).unwrap();
        assert_eq!(epoch, CancelEpoch(1));
        assert!(ops.is_cancelled(id));
        effects.cancel_in_flight(id).unwrap();

        assert_eq!(effects.complete(), Err(QdnfError::Cancelled));
        assert_eq!(
            ops.complete(id, handle.generation, &mut leases),
            Err(QdnfError::Cancelled)
        );
        assert_eq!(
            leases.release(lease.handle),
            Err(QdnfError::DoubleRelease)
        );
        assert_eq!(
            ops.complete(id, handle.generation, &mut leases),
            Err(QdnfError::Closed)
        );
    }

    #[test]
    fn stale_generation_is_rejected() {
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let lease = leases.acquire(8, true).unwrap();
        let handle = ops.admit(op(2), lease.handle).unwrap();
        assert_eq!(
            ops.complete(op(2), Generation(handle.generation.0 + 1), &mut leases),
            Err(QdnfError::StaleGeneration)
        );
        ops.complete(op(2), handle.generation, &mut leases).unwrap();
    }

    #[test]
    fn cancel_epoch_does_not_wrap() {
        assert_eq!(
            CancelEpoch(u64::MAX).next(),
            Err(QdnfError::StaleGeneration)
        );
    }
}
