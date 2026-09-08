//! Exclusive buffer leases with double-release and stale-generation rejection.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

use super::handles::{BufferLease, LeaseHandle};

pub const LEASE_SLOTS: usize = 32;

#[derive(Clone, Copy)]
struct Slot {
    generation: Generation,
    occupied: bool,
    exclusive: bool,
    capacity: u32,
}

pub struct LeaseTable {
    slots: [Slot; LEASE_SLOTS],
}

impl LeaseTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                generation: Generation::ZERO,
                occupied: false,
                exclusive: false,
                capacity: 0,
            }; LEASE_SLOTS],
        }
    }

    pub fn acquire(&mut self, capacity: u32, exclusive: bool) -> Result<BufferLease, QdnfError> {
        let mut exhausted = false;
        for i in 0..LEASE_SLOTS {
            if self.slots[i].occupied {
                continue;
            }
            match self.try_acquire_slot(i, capacity, exclusive) {
                Ok(lease) => return Ok(lease),
                Err(QdnfError::StaleGeneration) => {
                    exhausted = true;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        if exhausted {
            Err(QdnfError::StaleGeneration)
        } else {
            Err(QdnfError::Capacity)
        }
    }

    /// Acquire a specific free slot. Generation `u64::MAX` fails closed; it never wraps.
    pub fn try_acquire_slot(
        &mut self,
        slot: usize,
        capacity: u32,
        exclusive: bool,
    ) -> Result<BufferLease, QdnfError> {
        if slot >= LEASE_SLOTS {
            return Err(QdnfError::Range);
        }
        let cell = &mut self.slots[slot];
        if cell.occupied {
            return Err(QdnfError::Conflict);
        }
        let generation = cell.generation.next()?;
        *cell = Slot {
            generation,
            occupied: true,
            exclusive,
            capacity,
        };
        Ok(BufferLease {
            handle: LeaseHandle {
                slot: slot as u16,
                generation,
            },
            capacity,
            initialized: 0,
            exclusive,
        })
    }

    pub fn release(&mut self, handle: LeaseHandle) -> Result<(), QdnfError> {
        let idx = handle.slot as usize;
        if idx >= LEASE_SLOTS {
            return Err(QdnfError::Range);
        }
        let slot = &mut self.slots[idx];
        if slot.generation != handle.generation {
            return Err(QdnfError::StaleGeneration);
        }
        if !slot.occupied {
            return Err(QdnfError::DoubleRelease);
        }
        let _ = (slot.exclusive, slot.capacity);
        slot.occupied = false;
        slot.capacity = 0;
        slot.exclusive = false;
        Ok(())
    }

    pub fn occupied_count(&self) -> usize {
        let mut n = 0;
        for slot in &self.slots {
            if slot.occupied {
                n += 1;
            }
        }
        n
    }

    #[cfg(test)]
    pub(crate) fn debug_set_generation(&mut self, slot: usize, generation: Generation) {
        debug_assert!(slot < LEASE_SLOTS);
        debug_assert!(!self.slots[slot].occupied);
        self.slots[slot].generation = generation;
    }
}

impl Default for LeaseTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_release_fails() {
        let mut table = LeaseTable::new();
        let lease = table.acquire(64, true).unwrap();
        table.release(lease.handle).unwrap();
        assert_eq!(
            table.release(lease.handle),
            Err(QdnfError::DoubleRelease)
        );
    }

    #[test]
    fn thirty_third_acquire_is_capacity() {
        let mut table = LeaseTable::new();
        for _ in 0..LEASE_SLOTS {
            table.acquire(8, true).unwrap();
        }
        assert_eq!(table.occupied_count(), LEASE_SLOTS);
        assert_eq!(table.acquire(8, true), Err(QdnfError::Capacity));
    }
}
