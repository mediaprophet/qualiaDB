//! Deterministic Compute Metering (Permissive Commons)
//! Tracks implicit hardware cycles entirely without heap allocation 
//! or high-latency OS-level hardware sensors.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Increments when Core 3 fetches/writes a 40KB SuperBlock
pub static SUPERBLOCK_IO_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Increments when the GPU/NPU evaluates a bitmask across 850 Quins
pub static SIEVE_OPS_COUNT: AtomicUsize = AtomicUsize::new(0);

// "Energy of Logic" tracking metrics
pub static ATOMIC_FLOPS_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static ATOMIC_INTEGRATION_STEPS: AtomicUsize = AtomicUsize::new(0);

/// Core operational telemetry aggregate dump structed inside the Webizen VM
pub static VM_CYCLES_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Resets all global deterministic metrics for a new query lifecycle.
pub fn reset_telemetry() {
    SUPERBLOCK_IO_COUNT.store(0, Ordering::SeqCst);
    SIEVE_OPS_COUNT.store(0, Ordering::SeqCst);
    ATOMIC_FLOPS_COUNT.store(0, Ordering::SeqCst);
    ATOMIC_INTEGRATION_STEPS.store(0, Ordering::SeqCst);
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

/// Zero-allocation Prometheus Exporter
/// Writes telemetry metrics directly to a byte buffer without string formatting overhead.
pub fn export_prometheus_metrics<W: std::io::Write>(mut writer: W) -> std::io::Result<()> {
    let io = SUPERBLOCK_IO_COUNT.load(Ordering::Relaxed);
    let sieve = SIEVE_OPS_COUNT.load(Ordering::Relaxed);
    let vm = VM_CYCLES_COUNT.load(Ordering::Relaxed);
    let flops = ATOMIC_FLOPS_COUNT.load(Ordering::Relaxed);
    let steps = ATOMIC_INTEGRATION_STEPS.load(Ordering::Relaxed);
    
    // Write out Prometheus metrics in text format directly (zero heap allocation)
    write!(writer, "# HELP qualia_superblock_io_total Total NVMe SuperBlock flushes\n")?;
    write!(writer, "# TYPE qualia_superblock_io_total counter\n")?;
    write!(writer, "qualia_superblock_io_total {}\n", io)?;

    write!(writer, "# HELP qualia_sieve_ops_total GPU/NPU mask operations\n")?;
    write!(writer, "# TYPE qualia_sieve_ops_total counter\n")?;
    write!(writer, "qualia_sieve_ops_total {}\n", sieve)?;

    write!(writer, "# HELP qualia_vm_cycles_total Webizen VM opcodes evaluated\n")?;
    write!(writer, "# TYPE qualia_vm_cycles_total counter\n")?;
    write!(writer, "qualia_vm_cycles_total {}\n", vm)?;

    write!(writer, "# HELP qualia_atomic_flops_total Atomic integration float ops\n")?;
    write!(writer, "# TYPE qualia_atomic_flops_total counter\n")?;
    write!(writer, "qualia_atomic_flops_total {}\n", flops)?;

    write!(writer, "# HELP qualia_atomic_steps_total Total integration steps\n")?;
    write!(writer, "# TYPE qualia_atomic_steps_total counter\n")?;
    write!(writer, "qualia_atomic_steps_total {}\n", steps)?;

    Ok(())
}

/// Logs federated system telemetry (latency, VLM compute load) as a System_Log Quin.
/// Converts the semantic metric into a 48-byte struct instead of heap strings.
pub fn log_federated_telemetry(metric_hash: u64, value: f64) -> crate::QualiaQuin {
    crate::QualiaQuin {
        subject: crate::q_hash("did:q42:local-node"),
        predicate: metric_hash,
        // Mock inline decimal representation for telemetry value
        object: (0b010u64 << 60) | (value.to_bits() & 0x0FFF_FFFF_FFFF_FFFF),
        context: 0,
        metadata: 0,
        parity: 0,
    }
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
