//! Bounded SPSC ownership receipts (E10.2).
//!
//! Packet/index ownership moves through a fixed ring. There is no shared
//! mutable packet buffer: [`pop`] transfers exclusive ownership of a Copy
//! receipt. A second take of the same `(slot, generation)` is Unauthorized.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// Ring capacity. A 33rd live receipt is [`QdnfError::Capacity`].
pub const SPSC_CAP: usize = 32;

/// Exclusive ownership of one packet or index range. Copy is a token, not a
/// second owner — [`SpscQueue::take`] of an already-popped receipt fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnershipReceipt {
    pub generation: Generation,
    pub bytes: u32,
    pub slot: u8,
}

struct RingSlot {
    receipt: OwnershipReceipt,
    filled: bool,
}

/// Single-producer / single-consumer queue of ownership receipts.
pub struct SpscQueue {
    slots: [RingSlot; SPSC_CAP],
    head: usize,
    tail: usize,
    len: usize,
    /// Last transferred generation per packet `slot` (u8 index). Zero = none.
    transferred_gen: [u64; 256],
}

impl SpscQueue {
    pub const fn new() -> Self {
        const EMPTY: RingSlot = RingSlot {
            receipt: OwnershipReceipt {
                generation: Generation::ZERO,
                bytes: 0,
                slot: 0,
            },
            filled: false,
        };
        Self {
            slots: [EMPTY; SPSC_CAP],
            head: 0,
            tail: 0,
            len: 0,
            transferred_gen: [0; 256],
        }
    }

    #[inline]
    fn is_transferred(&self, receipt: OwnershipReceipt) -> bool {
        let g = receipt.generation.0;
        g != 0 && g <= self.transferred_gen[receipt.slot as usize]
    }

    fn mark_transferred(&mut self, receipt: OwnershipReceipt) {
        self.transferred_gen[receipt.slot as usize] = receipt.generation.0;
    }

    /// Enqueue. Full ring → [`QdnfError::Capacity`]. Re-injecting a popped
    /// receipt → [`QdnfError::Unauthorized`].
    pub fn push(&mut self, receipt: OwnershipReceipt) -> Result<(), QdnfError> {
        if receipt.generation == Generation::ZERO {
            return Err(QdnfError::Malformed);
        }
        if self.is_transferred(receipt) {
            return Err(QdnfError::Unauthorized);
        }
        if self.len == SPSC_CAP {
            return Err(QdnfError::Capacity);
        }
        let i = self.tail;
        self.slots[i] = RingSlot {
            receipt,
            filled: true,
        };
        self.tail = (self.tail + 1) % SPSC_CAP;
        self.len += 1;
        Ok(())
    }

    /// Dequeue. Empty → [`QdnfError::WouldBlock`]. Transfers ownership.
    pub fn pop(&mut self) -> Result<OwnershipReceipt, QdnfError> {
        if self.len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let i = self.head;
        if !self.slots[i].filled {
            return Err(QdnfError::Incomplete);
        }
        let receipt = self.slots[i].receipt;
        if self.is_transferred(receipt) {
            return Err(QdnfError::Unauthorized);
        }
        self.slots[i].filled = false;
        self.head = (self.head + 1) % SPSC_CAP;
        self.len -= 1;
        self.mark_transferred(receipt);
        Ok(receipt)
    }

    /// Second ownership take of a Copy receipt already returned by [`pop`].
    pub fn take(&mut self, receipt: OwnershipReceipt) -> Result<OwnershipReceipt, QdnfError> {
        if self.is_transferred(receipt) {
            return Err(QdnfError::Unauthorized);
        }
        let got = self.pop()?;
        if got != receipt {
            return Err(QdnfError::Conflict);
        }
        Ok(got)
    }
}

impl Default for SpscQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;

    fn rec(slot: u8, gen: u64) -> OwnershipReceipt {
        OwnershipReceipt {
            generation: Generation(gen),
            bytes: 8,
            slot,
        }
    }

    #[test]
    fn thirty_two_ok_thirty_third_capacity() {
        let mut q = SpscQueue::new();
        let mut i = 0u8;
        while i < SPSC_CAP as u8 {
            q.push(rec(i, 1)).expect("32 live receipts");
            i = i.saturating_add(1);
        }
        assert_eq!(q.push(rec(200, 1)), Err(QdnfError::Capacity));
        let mut n = 0usize;
        while n < SPSC_CAP {
            q.pop().expect("drain");
            n += 1;
        }
        assert_eq!(q.pop(), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn pop_empty_would_block() {
        let mut q = SpscQueue::new();
        assert_eq!(q.pop(), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn pop_transfers_ownership_double_take_unauthorized() {
        let mut q = SpscQueue::new();
        let r = rec(3, 7);
        q.push(r).unwrap();
        assert_eq!(q.pop().unwrap(), r);
        assert_eq!(q.take(r), Err(QdnfError::Unauthorized));
        assert_eq!(q.push(r), Err(QdnfError::Unauthorized));
        assert_eq!(q.pop(), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn new_generation_on_same_slot_is_ok() {
        let mut q = SpscQueue::new();
        let a = rec(1, 1);
        let b = rec(1, 2);
        q.push(a).unwrap();
        assert_eq!(q.pop().unwrap(), a);
        q.push(b).unwrap();
        assert_eq!(q.pop().unwrap(), b);
        assert_eq!(q.take(a), Err(QdnfError::Unauthorized));
        assert_eq!(q.take(b), Err(QdnfError::Unauthorized));
    }
}
