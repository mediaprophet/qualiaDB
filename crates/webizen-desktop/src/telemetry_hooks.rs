//! Telemetry extraction hooks for system metrics
//!
//! This module provides zero-heap compliant extraction functions for real-time
//! system metrics used in the ambient background visualization. All functions
//! use stack-allocated operations and return normalized f32 values (0.0 to 1.0)
//! for GPU compatibility.
//!
//! Zero-heap compliance: All extraction uses stack allocation and avoids heap
//! allocations. Missing data is handled gracefully by returning 0.0.

use std::sync::atomic::{AtomicU64, Ordering};
use webizen_render::scene_contract::SystemTelemetry;

// ── Global Atomic Counters for Real-Time Metrics ───────────────────────────────
// These are stack-allocated atomics that subsystems can increment to signal activity
// without heap allocation overhead.

/// Token generation counter (incremented by gguf_bridge during inference)
static INFERENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Network I/O byte counter (incremented by nym_adapter or BLE mesh)
static NETWORK_IO_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Ontology baking counter (incremented during topological ingestion)
static BAKING_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Query resolution counter (incremented on wavefunction collapse)
static QUERY_RESOLVE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Quantum context activity counter (incremented for q > 0 states)
static QUANTUM_ACTIVITY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Spectral payload change counter (incremented on [α, μ, σ] changes)
static SPECTRAL_SHIFT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Temporal slice activity counter (incremented on temporal navigation)
static TEMPORAL_PULSE_COUNTER: AtomicU64 = AtomicU64::new(0);

// ── Metric Extraction Functions ────────────────────────────────────────────────

/// Get memory pressure from memory-mapped volumes and tensor arrays
///
/// Source: System RAM and VRAM usage from memory-mapped volumes
/// Normalization: 0.0 = idle, 1.0 = near capacity
/// Zero-heap: Uses sysinfo with stack-allocated structs
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_memory_pressure() -> f32 {
    // Use sysinfo to get memory usage (stack-allocated operations)
    // In production, this would read from QualiaDB's internal allocation trackers
    // For now, return a placeholder that could be wired to actual metrics

    // Placeholder: read from system memory if sysinfo is available
    // This is a simplified implementation - in production, integrate with
    // QualiaDB's memory-mapped volume tracking
    0.0_f32 // Return 0.0 if metric unavailable
}

/// Get network I/O rate from nym_adapter or acoustic BLE mesh
///
/// Source: Network I/O byte rate (bytes in/out per second)
/// Normalization: 0.0 = idle, 1.0 = maximum observed rate
/// Zero-heap: Reads atomic counter and computes rate using stack arithmetic
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_network_ripple() -> f32 {
    // Read the atomic counter (stack operation)
    let current_io = NETWORK_IO_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would compute rate over time window
    // using stack-allocated time arithmetic
    if current_io == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 1MB/s as "full" scale for now
        let normalized = (current_io as f32) / (1_000_000.0);
        normalized.min(1.0)
    }
}

/// Get background topological ingestion rate (ontology baking)
///
/// Source: Background topological ingestion rate (e.g., SNOMED CT mapping)
/// Normalization: 0.0 = idle, 1.0 = maximum ingestion rate
/// Zero-heap: Reads atomic counter and computes rate
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_baking_crystallization() -> f32 {
    let job_pressure = qualia_client_core::activity_signals::ontology_baking_pressure();
    if job_pressure > 0.0 {
        return job_pressure;
    }
    let current_baking = BAKING_COUNTER.load(Ordering::Relaxed);
    if current_baking == 0 {
        0.0
    } else {
        ((current_baking as f32) / 1000.0).min(1.0)
    }
}

/// Get queue size of qpu_dispatcher and local classical exhaustion frequency
///
/// Source: qpu_dispatcher queue length + classical exhaustion events
/// Normalization: 0.0 = idle, 1.0 = maximum queue depth
/// Zero-heap: Reads atomic counter and computes combined metric
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_logic_flashes() -> f32 {
    // Read the query resolution counter (stack operation)
    let current_queries = QUERY_RESOLVE_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would combine queue depth and exhaustion frequency
    if current_queries == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 100 queries/sec as "full" scale for now
        let normalized = (current_queries as f32) / 100.0;
        normalized.min(1.0)
    }
}

/// Get token generation rate or tensor math utilization
///
/// Source: gguf_bridge token generation rate or tensor math utilization
/// Normalization: 0.0 = idle, 1.0 = maximum generation rate
/// Zero-heap: Reads atomic counter and computes rate
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_inference_heat() -> f32 {
    // Read the inference counter (stack operation)
    let current_inference = INFERENCE_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would track tokens/second or tensor ops
    if current_inference == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 100 tokens/sec as "full" scale for now
        let normalized = (current_inference as f32) / 100.0;
        normalized.min(1.0)
    }
}

/// Get quantum context activity (q > 0 states)
///
/// Source: Quantum context activity (number of pending q > 0 states)
/// Normalization: 0.0 = all collapsed (q=0), 1.0 = maximum pending states
/// Zero-heap: Reads atomic counter and normalizes
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_quantum_activity() -> f32 {
    // Read the quantum activity counter (stack operation)
    let current_activity = QUANTUM_ACTIVITY_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would track number of pending quantum contexts
    if current_activity == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 1000 pending contexts as "full" scale for now
        let normalized = (current_activity as f32) / 1000.0;
        normalized.min(1.0)
    }
}

/// Get spectral payload changes in [α, μ, σ]
///
/// Source: Spectral payload changes (amplitude, modulation, spectral signature)
/// Normalization: 0.0 = stable, 1.0 = rapid spectral flux
/// Zero-heap: Reads atomic counter and computes rate
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_spectral_shift() -> f32 {
    // Read the spectral shift counter (stack operation)
    let current_shift = SPECTRAL_SHIFT_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would track rate of [α, μ, σ] changes
    if current_shift == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 1000 changes/sec as "full" scale for now
        let normalized = (current_shift as f32) / 1000.0;
        normalized.min(1.0)
    }
}

/// Get temporal slice activity
///
/// Source: Temporal slice navigation activity
/// Normalization: 0.0 = static, 1.0 = rapid temporal navigation
/// Zero-heap: Reads atomic counter and computes rate
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_temporal_pulse() -> f32 {
    // Read the temporal pulse counter (stack operation)
    let current_pulse = TEMPORAL_PULSE_COUNTER.load(Ordering::Relaxed);

    // Normalize to 0.0-1.0 range
    // In production, this would track temporal navigation events
    if current_pulse == 0 {
        0.0
    } else {
        // Simple normalization: clamp to 0.0-1.0
        // Assume 10 navigation events/sec as "full" scale for now
        let normalized = (current_pulse as f32) / 10.0;
        normalized.min(1.0)
    }
}

/// Get epistemic state distribution
///
/// Source: Distribution of epistemic states (Collapsed, Pending, Sandbox)
/// Normalization: 0.0 = all collapsed, 1.0 = maximum pending/sandbox ratio
/// Zero-heap: Computes ratio using stack arithmetic
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_epistemic_density() -> f32 {
    // In production, this would read from QualiaDB's epistemic state tracker
    // For now, return a placeholder
    // This would compute the ratio of pending/sandbox states to total states
    0.0_f32 // Return 0.0 if metric unavailable
}

/// Get overall system load (manifold pressure)
///
/// Source: Combined system load from all subsystems
/// Normalization: 0.0 = idle, 1.0 = maximum load
/// Zero-heap: Combines other metrics using stack arithmetic
///
/// Returns normalized f32 value (0.0 to 1.0)
pub fn get_manifold_pressure() -> f32 {
    // Combine other metrics to compute overall system pressure
    // Use stack-allocated operations only
    let memory = get_memory_pressure();
    let network = get_network_ripple();
    let inference = get_inference_heat();
    let quantum = get_quantum_activity();

    // Weighted average (stack operations only)
    let total = memory * 0.3 + network * 0.2 + inference * 0.3 + quantum * 0.2;
    total.min(1.0)
}

// ── Atomic Counter Accessors for Subsystems ─────────────────────────────────────

/// Increment inference counter (called by gguf_bridge during token generation)
pub fn increment_inference_counter() {
    INFERENCE_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Increment network I/O counter (called by nym_adapter or BLE mesh)
pub fn increment_network_io_counter(bytes: u64) {
    NETWORK_IO_COUNTER.fetch_add(bytes, Ordering::Relaxed);
}

/// Increment baking counter (called during ontology ingestion)
pub fn increment_baking_counter() {
    BAKING_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Increment query resolve counter (called on wavefunction collapse)
pub fn increment_query_resolve_counter() {
    QUERY_RESOLVE_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Increment quantum activity counter (called for q > 0 states)
pub fn increment_quantum_activity_counter() {
    QUANTUM_ACTIVITY_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Increment spectral shift counter (called on [α, μ, σ] changes)
pub fn increment_spectral_shift_counter() {
    SPECTRAL_SHIFT_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Increment temporal pulse counter (called on temporal navigation)
pub fn increment_temporal_pulse_counter() {
    TEMPORAL_PULSE_COUNTER.fetch_add(1, Ordering::Relaxed);
}

// ── Counter Reset Functions (for periodic decay) ───────────────────────────────

/// Reset all counters (called periodically to implement decay)
pub fn reset_all_counters() {
    INFERENCE_COUNTER.store(0, Ordering::Relaxed);
    NETWORK_IO_COUNTER.store(0, Ordering::Relaxed);
    BAKING_COUNTER.store(0, Ordering::Relaxed);
    QUERY_RESOLVE_COUNTER.store(0, Ordering::Relaxed);
    QUANTUM_ACTIVITY_COUNTER.store(0, Ordering::Relaxed);
    SPECTRAL_SHIFT_COUNTER.store(0, Ordering::Relaxed);
    TEMPORAL_PULSE_COUNTER.store(0, Ordering::Relaxed);
}

// ── Master Collection Function ──────────────────────────────────────────────────

/// Collect all system telemetry into a single struct
///
/// This function calls all extraction functions and returns a populated
/// SystemTelemetry struct. All operations are stack-allocated and zero-heap.
///
/// Returns SystemTelemetry with all metrics populated
pub fn collect_system_telemetry() -> SystemTelemetry {
    SystemTelemetry {
        memory_pressure: get_memory_pressure(),
        network_ripple: get_network_ripple(),
        baking_crystallization: get_baking_crystallization(),
        logic_flashes: get_logic_flashes(),
        llm_heat: get_inference_heat(),
        quantum_activity: get_quantum_activity(),
        spectral_shift: get_spectral_shift(),
        temporal_pulse: get_temporal_pulse(),
        epistemic_density: get_epistemic_density(),
        manifold_pressure: get_manifold_pressure(),
        _padding: [0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_collection() {
        let telemetry = collect_system_telemetry();
        // All values should be in valid range
        assert!(telemetry.memory_pressure >= 0.0 && telemetry.memory_pressure <= 1.0);
        assert!(telemetry.network_ripple >= 0.0 && telemetry.network_ripple <= 1.0);
        assert!(telemetry.baking_crystallization >= 0.0 && telemetry.baking_crystallization <= 1.0);
        assert!(telemetry.logic_flashes >= 0.0 && telemetry.logic_flashes <= 1.0);
        assert!(telemetry.llm_heat >= 0.0 && telemetry.llm_heat <= 1.0);
        assert!(telemetry.quantum_activity >= 0.0 && telemetry.quantum_activity <= 1.0);
        assert!(telemetry.spectral_shift >= 0.0 && telemetry.spectral_shift <= 1.0);
        assert!(telemetry.temporal_pulse >= 0.0 && telemetry.temporal_pulse <= 1.0);
        assert!(telemetry.epistemic_density >= 0.0 && telemetry.epistemic_density <= 1.0);
        assert!(telemetry.manifold_pressure >= 0.0 && telemetry.manifold_pressure <= 1.0);
    }

    #[test]
    fn test_counter_increment() {
        // Test that counters can be incremented
        increment_inference_counter();
        assert!(INFERENCE_COUNTER.load(Ordering::Relaxed) > 0);

        increment_network_io_counter(100);
        assert!(NETWORK_IO_COUNTER.load(Ordering::Relaxed) >= 100);

        reset_all_counters();
        assert!(INFERENCE_COUNTER.load(Ordering::Relaxed) == 0);
    }

    #[test]
    fn test_struct_size() {
        // Ensure struct is properly aligned for GPU
        assert_eq!(std::mem::size_of::<SystemTelemetry>(), 48);
        assert_eq!(std::mem::align_of::<SystemTelemetry>(), 4);
    }
}
