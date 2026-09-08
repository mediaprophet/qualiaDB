//! Non-wrapping slot generations. Exhaustion retires the slot; it never wraps
//! into a previously valid handle.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// Advance a stored slot generation. `u64::MAX` fails closed as stale.
#[inline]
pub const fn bump_generation(current: Generation) -> Result<Generation, QdnfError> {
    current.next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::leases::{LeaseTable, LEASE_SLOTS};

    #[test]
    fn slot_at_max_next_fails_stale_generation() {
        assert_eq!(
            bump_generation(Generation(u64::MAX)),
            Err(QdnfError::StaleGeneration)
        );
        let mut table = LeaseTable::new();
        table.debug_set_generation(0, Generation(u64::MAX));
        assert_eq!(
            table.try_acquire_slot(0, 8, true),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn acquire_skips_retired_slot_and_never_wraps() {
        let mut table = LeaseTable::new();
        table.debug_set_generation(0, Generation(u64::MAX));
        let lease = table.acquire(8, true).unwrap();
        assert_eq!(lease.handle.slot, 1);
        assert_ne!(lease.handle.generation, Generation(0));
        assert_ne!(lease.handle.generation, Generation(u64::MAX));
    }

    #[test]
    fn all_slots_exhausted_is_stale_generation() {
        let mut table = LeaseTable::new();
        for i in 0..LEASE_SLOTS {
            table.debug_set_generation(i, Generation(u64::MAX));
        }
        assert_eq!(table.acquire(8, true), Err(QdnfError::StaleGeneration));
    }

    #[test]
    fn released_handle_is_not_live_at_old_generation() {
        let mut table = LeaseTable::new();
        let first = table.acquire(8, true).unwrap();
        table.release(first.handle).unwrap();
        let second = table.acquire(8, true).unwrap();
        assert_eq!(second.handle.slot, first.handle.slot);
        assert_ne!(second.handle.generation, first.handle.generation);
        assert_eq!(
            table.release(first.handle),
            Err(QdnfError::StaleGeneration)
        );
        table.release(second.handle).unwrap();
    }
}
