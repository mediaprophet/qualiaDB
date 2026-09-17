//! Thread-local hot-path allocation counter. Parallel tests need no serial flag.

use core::cell::Cell;

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

thread_local! {
    static HOT_ALLOCS: Cell<u64> = const { Cell::new(0) };
}

/// Record one hot-path heap touch on the calling thread.
#[inline]
pub fn record_alloc() {
    HOT_ALLOCS.with(|c| c.set(c.get().wrapping_add(1)));
}

/// Take and reset this thread's hot-path allocation count.
#[inline]
pub fn take_alloc() -> u64 {
    HOT_ALLOCS.with(|c| c.replace(0))
}

/// Peek without reset. Tests on other threads do not observe this value.
#[inline]
pub fn peek_alloc() -> u64 {
    HOT_ALLOCS.with(|c| c.get())
}

const POOL_SLOTS: usize = 8;

#[derive(Clone, Copy, Debug)]
struct PoolSlot {
    occupied: bool,
    owner: u8,
    bytes: u32,
    generation: Generation,
}

impl PoolSlot {
    const EMPTY: Self = Self {
        occupied: false,
        owner: 0,
        bytes: 0,
        generation: Generation::ZERO,
    };
}

/// Harness lease handle. Not the product `LeaseTable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HarnessLease {
    pub slot: u8,
    pub owner: u8,
    pub bytes: u32,
    pub generation: Generation,
}

/// Bounded reservation tracker: conservation, ownership, generation, peak bytes.
pub struct ReservationTracker {
    slots: [PoolSlot; POOL_SLOTS],
    live_bytes: u64,
    peak_bytes: u64,
    live_leases: u8,
}

impl ReservationTracker {
    pub const fn new() -> Self {
        Self {
            slots: [PoolSlot::EMPTY; POOL_SLOTS],
            live_bytes: 0,
            peak_bytes: 0,
            live_leases: 0,
        }
    }

    pub const fn live_bytes(&self) -> u64 {
        self.live_bytes
    }

    pub const fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }

    pub const fn live_leases(&self) -> u8 {
        self.live_leases
    }

    pub fn live_generation(&self, slot: u8) -> Result<Generation, QdnfError> {
        let i = slot as usize;
        if i >= POOL_SLOTS {
            return Err(QdnfError::Range);
        }
        if !self.slots[i].occupied {
            return Err(QdnfError::Incomplete);
        }
        Ok(self.slots[i].generation)
    }

    pub fn reserve(&mut self, owner: u8, bytes: u32) -> Result<HarnessLease, QdnfError> {
        let mut i = 0;
        while i < POOL_SLOTS {
            if !self.slots[i].occupied {
                let generation = self.slots[i].generation.next()?;
                self.slots[i] = PoolSlot {
                    occupied: true,
                    owner,
                    bytes,
                    generation,
                };
                self.live_leases = self.live_leases.saturating_add(1);
                self.live_bytes = self
                    .live_bytes
                    .checked_add(bytes as u64)
                    .ok_or(QdnfError::Range)?;
                if self.live_bytes > self.peak_bytes {
                    self.peak_bytes = self.live_bytes;
                }
                return Ok(HarnessLease {
                    slot: i as u8,
                    owner,
                    bytes,
                    generation,
                });
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }

    pub fn release(&mut self, lease: HarnessLease) -> Result<(), QdnfError> {
        let i = lease.slot as usize;
        if i >= POOL_SLOTS {
            return Err(QdnfError::Range);
        }
        let slot = &mut self.slots[i];
        if !slot.occupied {
            return Err(QdnfError::DoubleRelease);
        }
        if slot.generation != lease.generation {
            return Err(QdnfError::StaleGeneration);
        }
        if slot.owner != lease.owner {
            return Err(QdnfError::Denied);
        }
        self.live_bytes = self.live_bytes.saturating_sub(slot.bytes as u64);
        self.live_leases = self.live_leases.saturating_sub(1);
        slot.occupied = false;
        slot.bytes = 0;
        slot.owner = 0;
        Ok(())
    }

    /// Occupied-slot sum. Must equal `live_bytes` after every mutating call.
    pub fn occupied_bytes(&self) -> u64 {
        let mut sum = 0u64;
        let mut i = 0;
        while i < POOL_SLOTS {
            if self.slots[i].occupied {
                sum = sum.saturating_add(self.slots[i].bytes as u64);
            }
            i += 1;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_alloc_is_thread_local() {
        let _ = take_alloc();
        record_alloc();
        record_alloc();
        assert_eq!(take_alloc(), 2);
        assert_eq!(take_alloc(), 0);

        let child = std::thread::spawn(|| {
            record_alloc();
            take_alloc()
        });
        assert_eq!(child.join().unwrap(), 1);
        assert_eq!(peek_alloc(), 0);
    }

    #[test]
    fn hot_alloc_covers_success_error_cancel() {
        let _ = take_alloc();
        assert_eq!(take_alloc(), 0);

        record_alloc();
        let err = QdnfError::Capacity;
        assert_eq!(err, QdnfError::Capacity);
        assert_eq!(take_alloc(), 1);

        record_alloc();
        let cancel = QdnfError::Cancelled;
        assert_eq!(cancel, QdnfError::Cancelled);
        assert_eq!(take_alloc(), 1);
    }

    #[test]
    fn reservation_conservation_and_peak() {
        let mut pool = ReservationTracker::new();
        let a = pool.reserve(1, 100).unwrap();
        let b = pool.reserve(2, 50).unwrap();
        assert_eq!(pool.live_bytes(), 150);
        assert_eq!(pool.occupied_bytes(), 150);
        assert_eq!(pool.peak_bytes(), 150);
        assert_eq!(pool.live_leases(), 2);
        assert_eq!(pool.live_generation(a.slot).unwrap(), a.generation);

        pool.release(a).unwrap();
        assert_eq!(pool.live_bytes(), 50);
        assert_eq!(pool.occupied_bytes(), 50);
        assert_eq!(pool.peak_bytes(), 150);

        assert_eq!(pool.release(a), Err(QdnfError::DoubleRelease));
        let mut stolen = b;
        stolen.owner = 9;
        assert_eq!(pool.release(stolen), Err(QdnfError::Denied));
        pool.release(b).unwrap();
        assert_eq!(pool.live_bytes(), 0);
        assert_eq!(pool.live_leases(), 0);
    }
}
