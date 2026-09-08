//! Exclusive buffer leases with double-release rejection.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

use super::handles::{BufferLease, LeaseHandle};

const SLOTS: usize = 32;

#[derive(Clone, Copy)]
struct Slot {
    generation: Generation,
    occupied: bool,
    exclusive: bool,
    capacity: u32,
}

pub struct LeaseTable {
    slots: [Slot; SLOTS],
}

impl LeaseTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                generation: Generation::ZERO,
                occupied: false,
                exclusive: false,
                capacity: 0,
            }; SLOTS],
        }
    }

    pub fn acquire(&mut self, capacity: u32, exclusive: bool) -> Result<BufferLease, QdnfError> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !slot.occupied {
                let generation = slot.generation.next()?;
                *slot = Slot {
                    generation,
                    occupied: true,
                    exclusive,
                    capacity,
                };
                return Ok(BufferLease {
                    handle: LeaseHandle {
                        slot: i as u16,
                        generation,
                    },
                    capacity,
                    initialized: 0,
                    exclusive,
                });
            }
        }
        Err(QdnfError::Capacity)
    }

    pub fn release(&mut self, handle: LeaseHandle) -> Result<(), QdnfError> {
        let idx = handle.slot as usize;
        if idx >= SLOTS {
            return Err(QdnfError::Range);
        }
        let slot = &mut self.slots[idx];
        if !slot.occupied || slot.generation != handle.generation {
            return Err(QdnfError::DoubleRelease);
        }
        let _ = (slot.exclusive, slot.capacity);
        slot.occupied = false;
        slot.generation = handle.generation;
        Ok(())
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
}
