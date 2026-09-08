//! Bounded kernel events and effects. Completion consumes the slot.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::OperationId;

use super::handles::LeaseHandle;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    FrameReceived = 1,
    Timer = 2,
    CryptoComplete = 3,
    Cancel = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    SendFrame = 1,
    CryptoWork = 2,
    DurableCommit = 3,
    ReleaseLease = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelEvent {
    pub kind: EventKind,
    pub operation: OperationId,
    pub lease: LeaseHandle,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelEffect {
    pub kind: EffectKind,
    pub operation: OperationId,
    pub lease: LeaseHandle,
}

#[derive(Clone, Copy)]
struct EffectSlot {
    effect: KernelEffect,
    cancelled: bool,
}

pub struct EffectQueue {
    slots: [Option<EffectSlot>; 16],
    len: usize,
}

impl EffectQueue {
    pub const fn new() -> Self {
        Self {
            slots: [None; 16],
            len: 0,
        }
    }

    pub fn push(&mut self, effect: KernelEffect) -> Result<(), QdnfError> {
        if self.len >= self.slots.len() {
            return Err(QdnfError::Capacity);
        }
        self.slots[self.len] = Some(EffectSlot {
            effect,
            cancelled: false,
        });
        self.len += 1;
        Ok(())
    }

    /// Mark in-flight effects for `operation`. Later `complete` consumes the
    /// slot (releases storage) but returns `Cancelled` so the op cannot revive.
    pub fn cancel_in_flight(&mut self, operation: OperationId) -> Result<(), QdnfError> {
        let mut found = false;
        for i in 0..self.len {
            if let Some(slot) = &mut self.slots[i] {
                if slot.effect.operation == operation {
                    slot.cancelled = true;
                    found = true;
                }
            }
        }
        if found {
            Ok(())
        } else {
            Err(QdnfError::WouldBlock)
        }
    }

    /// Consuming completion: the effect leaves the queue.
    pub fn complete(&mut self) -> Result<KernelEffect, QdnfError> {
        if self.len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let slot = self.slots[0].take().ok_or(QdnfError::Malformed)?;
        for i in 0..self.len - 1 {
            self.slots[i] = self.slots[i + 1];
        }
        self.len -= 1;
        self.slots[self.len] = None;
        if slot.cancelled {
            Err(QdnfError::Cancelled)
        } else {
            Ok(slot.effect)
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(n: u8) -> KernelEffect {
        let mut id = [0u8; 16];
        id[0] = n;
        KernelEffect {
            kind: EffectKind::SendFrame,
            operation: OperationId(id),
            lease: LeaseHandle::INVALID,
        }
    }

    #[test]
    fn complete_consumes_slot() {
        let mut q = EffectQueue::new();
        q.push(effect(1)).unwrap();
        q.push(effect(2)).unwrap();
        assert_eq!(q.complete().unwrap().operation.0[0], 1);
        assert_eq!(q.len(), 1);
        assert_eq!(q.complete().unwrap().operation.0[0], 2);
        assert_eq!(q.complete(), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn cancel_in_flight_rejects_late_complete() {
        let mut q = EffectQueue::new();
        let e = effect(7);
        q.push(e).unwrap();
        q.cancel_in_flight(e.operation).unwrap();
        assert_eq!(q.complete(), Err(QdnfError::Cancelled));
        assert_eq!(q.len(), 0);
        assert_eq!(q.complete(), Err(QdnfError::WouldBlock));
    }
}
