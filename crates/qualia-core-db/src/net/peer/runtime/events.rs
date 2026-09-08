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

pub struct EffectQueue {
    slots: [Option<KernelEffect>; 16],
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
        self.slots[self.len] = Some(effect);
        self.len += 1;
        Ok(())
    }

    /// Consuming completion: the effect leaves the queue.
    pub fn complete(&mut self) -> Result<KernelEffect, QdnfError> {
        if self.len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let effect = self.slots[0].take().ok_or(QdnfError::Malformed)?;
        for i in 0..self.len - 1 {
            self.slots[i] = self.slots[i + 1];
        }
        self.len -= 1;
        self.slots[self.len] = None;
        Ok(effect)
    }
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self::new()
    }
}
