//! Deterministic Compute Metering (Permissive Commons)
//! Tracks implicit hardware cycles entirely without heap allocation 
//! or high-latency OS-level hardware sensors.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Increments when Core 3 fetches/writes a 40KB SuperBlock
pub static SUPERBLOCK_IO_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Increments when the GPU/NPU evaluates a bitmask across 850 Quins
pub static SIEVE_OPS_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Increments for every opcode evaluated inside the Sentinel VM
pub static VM_CYCLES_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Resets all global deterministic metrics for a new query lifecycle.
pub fn reset_telemetry() {
    SUPERBLOCK_IO_COUNT.store(0, Ordering::SeqCst);
    SIEVE_OPS_COUNT.store(0, Ordering::SeqCst);
    VM_CYCLES_COUNT.store(0, Ordering::SeqCst);
}

/// Retrieves the current metric snapshot.
pub fn get_telemetry_snapshot() -> (usize, usize, usize) {
    (
        SUPERBLOCK_IO_COUNT.load(Ordering::Relaxed),
        SIEVE_OPS_COUNT.load(Ordering::Relaxed),
        VM_CYCLES_COUNT.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_atomics() {
        reset_telemetry();
        
        SUPERBLOCK_IO_COUNT.fetch_add(1, Ordering::Relaxed);
        SIEVE_OPS_COUNT.fetch_add(5, Ordering::Relaxed);
        VM_CYCLES_COUNT.fetch_add(100, Ordering::Relaxed);
        
        let (io, sieve, vm) = get_telemetry_snapshot();
        assert_eq!(io, 1);
        assert_eq!(sieve, 5);
        assert_eq!(vm, 100);
    }
}
