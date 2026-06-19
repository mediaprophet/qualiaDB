//! System Telemetry Data Contract
//!
//! Zero-heap, GPU-compatible telemetry struct for ambient background visualization.
//! This 48-byte struct is designed for direct GPU transfer via WGSL uniforms.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// System telemetry metrics for ambient visualization.
///
/// This struct is #[repr(C)] and properly aligned for GPU uniform buffers.
/// Total size: 48 bytes (12 f32 values) for strict WGSL 16-byte alignment.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable, Serialize, Deserialize)]
pub struct SystemTelemetry {
    /// Memory pressure (0.0 to 1.0)
    /// Maps system RAM/VRAM usage to structural density
    pub memory_pressure: f32,

    /// Network ripple (0.0 to 1.0)
    /// Maps network I/O rate to wave displacement effects
    pub network_ripple: f32,

    /// Baking crystallization (0.0 to 1.0)
    /// Maps ontology ingestion rate to structural order
    pub baking_crystallization: f32,

    /// Logic flashes (0.0 to 1.0)
    /// Maps query queue size to arc/collapse effects
    pub logic_flashes: f32,

    /// LLM heat (0.0 to 1.0)
    /// Maps token generation rate to localized heat/vibration
    pub llm_heat: f32,

    /// Quantum activity (0.0 to 1.0)
    /// Maps quantum context activity (q > 0 states) to phase tunneling
    pub quantum_activity: f32,

    /// Spectral shift (0.0 to 1.0)
    /// Maps spectral payload changes in [α, μ, σ] to color shifts
    pub spectral_shift: f32,

    /// Temporal pulse (0.0 to 1.0)
    /// Maps temporal slice activity to radial waves
    pub temporal_pulse: f32,

    /// Epistemic density (0.0 to 1.0)
    /// Maps epistemic state distribution to clustering
    pub epistemic_density: f32,

    /// Manifold pressure (0.0 to 1.0)
    /// Maps overall system load to radial breathing
    pub manifold_pressure: f32,

    /// Padding to 48 bytes for strict WGSL alignment
    /// WGSL uniforms require 16-byte alignment
    pub _padding: [f32; 2],
}

impl SystemTelemetry {
    /// Create a new SystemTelemetry with all values at default (0.0)
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update memory pressure from raw memory usage.
    ///
    /// # Arguments
    /// * `usage` - Memory usage ratio (0.0 to 1.0 or higher)
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    #[inline]
    pub fn update_memory_pressure(&mut self, usage: f32) {
        self.memory_pressure = usage.clamp(0.0, 1.0);
    }

    /// Update network ripple from network I/O rate.
    ///
    /// # Arguments
    /// * `bytes_per_sec` - Network throughput in bytes per second
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    /// Assumes 10 MB/s as maximum for normalization
    #[inline]
    pub fn update_network_ripple(&mut self, bytes_per_sec: f32) {
        // Normalize: assume 10 MB/s as max throughput
        const MAX_THROUGHPUT: f32 = 10_000_000.0;
        self.network_ripple = (bytes_per_sec / MAX_THROUGHPUT).clamp(0.0, 1.0);
    }

    /// Update LLM heat from token generation rate.
    ///
    /// # Arguments
    /// * `token_rate` - Token generation rate in tokens per second
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    /// Assumes 100 tokens/sec as maximum for normalization
    #[inline]
    pub fn update_llm_heat(&mut self, token_rate: f32) {
        // Normalize: assume 100 tokens/sec as max rate
        const MAX_TOKEN_RATE: f32 = 100.0;
        self.llm_heat = (token_rate / MAX_TOKEN_RATE).clamp(0.0, 1.0);
    }

    /// Update quantum activity from quantum context count.
    ///
    /// # Arguments
    /// * `active_contexts` - Number of active quantum contexts (q > 0)
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    /// Assumes 100 active contexts as maximum for normalization
    #[inline]
    pub fn update_quantum_activity(&mut self, active_contexts: f32) {
        const MAX_CONTEXTS: f32 = 100.0;
        self.quantum_activity = (active_contexts / MAX_CONTEXTS).clamp(0.0, 1.0);
    }

    /// Update spectral shift from spectral payload changes.
    ///
    /// # Arguments
    /// * `change_rate` - Rate of changes in [α, μ, σ] channels
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    /// Assumes 1000 changes/sec as maximum for normalization
    #[inline]
    pub fn update_spectral_shift(&mut self, change_rate: f32) {
        const MAX_CHANGES: f32 = 1000.0;
        self.spectral_shift = (change_rate / MAX_CHANGES).clamp(0.0, 1.0);
    }

    /// Update temporal pulse from temporal slice activity.
    ///
    /// # Arguments
    /// * `navigation_rate` - Rate of temporal navigation events
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    /// Assumes 10 events/sec as maximum for normalization
    #[inline]
    pub fn update_temporal_pulse(&mut self, navigation_rate: f32) {
        const MAX_EVENTS: f32 = 10.0;
        self.temporal_pulse = (navigation_rate / MAX_EVENTS).clamp(0.0, 1.0);
    }

    /// Update epistemic density from epistemic state distribution.
    ///
    /// # Arguments
    /// * `density_ratio` - Ratio of active epistemic states
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    #[inline]
    pub fn update_epistemic_density(&mut self, density_ratio: f32) {
        self.epistemic_density = density_ratio.clamp(0.0, 1.0);
    }

    /// Update manifold pressure from overall system load.
    ///
    /// # Arguments
    /// * `load_ratio` - Overall system load ratio (0.0 to 1.0 or higher)
    ///
    /// # Returns
    /// Normalized value clamped to 0.0-1.0 range
    #[inline]
    pub fn update_manifold_pressure(&mut self, load_ratio: f32) {
        self.manifold_pressure = load_ratio.clamp(0.0, 1.0);
    }

    /// Set memory pressure directly (already normalized)
    #[inline]
    pub fn set_memory_pressure(&mut self, value: f32) {
        self.memory_pressure = value.clamp(0.0, 1.0);
    }

    /// Set network ripple directly (already normalized)
    #[inline]
    pub fn set_network_ripple(&mut self, value: f32) {
        self.network_ripple = value.clamp(0.0, 1.0);
    }

    /// Set baking crystallization directly (already normalized)
    #[inline]
    pub fn set_baking_crystallization(&mut self, value: f32) {
        self.baking_crystallization = value.clamp(0.0, 1.0);
    }

    /// Set logic flashes directly (already normalized)
    #[inline]
    pub fn set_logic_flashes(&mut self, value: f32) {
        self.logic_flashes = value.clamp(0.0, 1.0);
    }

    /// Set LLM heat directly (already normalized)
    #[inline]
    pub fn set_llm_heat(&mut self, value: f32) {
        self.llm_heat = value.clamp(0.0, 1.0);
    }

    /// Set quantum activity directly (already normalized)
    #[inline]
    pub fn set_quantum_activity(&mut self, value: f32) {
        self.quantum_activity = value.clamp(0.0, 1.0);
    }

    /// Set spectral shift directly (already normalized)
    #[inline]
    pub fn set_spectral_shift(&mut self, value: f32) {
        self.spectral_shift = value.clamp(0.0, 1.0);
    }

    /// Set temporal pulse directly (already normalized)
    #[inline]
    pub fn set_temporal_pulse(&mut self, value: f32) {
        self.temporal_pulse = value.clamp(0.0, 1.0);
    }

    /// Set epistemic density directly (already normalized)
    #[inline]
    pub fn set_epistemic_density(&mut self, value: f32) {
        self.epistemic_density = value.clamp(0.0, 1.0);
    }

    /// Set manifold pressure directly (already normalized)
    #[inline]
    pub fn set_manifold_pressure(&mut self, value: f32) {
        self.manifold_pressure = value.clamp(0.0, 1.0);
    }

    /// Reset all metrics to zero
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Convert the telemetry struct to a byte array for GPU transfer.
    ///
    /// # Returns
    /// Slice of bytes representing the struct's memory layout
    #[inline]
    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_telemetry_size() {
        // Must be exactly 48 bytes for WGSL alignment
        assert_eq!(std::mem::size_of::<SystemTelemetry>(), 48);
    }

    #[test]
    fn test_system_telemetry_alignment() {
        // Must be at least 4-byte aligned for f32
        assert_eq!(std::mem::align_of::<SystemTelemetry>(), 4);
    }

    #[test]
    fn test_new_creates_defaults() {
        let telemetry = SystemTelemetry::new();
        assert_eq!(telemetry.memory_pressure, 0.0);
        assert_eq!(telemetry.network_ripple, 0.0);
        assert_eq!(telemetry.baking_crystallization, 0.0);
        assert_eq!(telemetry.logic_flashes, 0.0);
        assert_eq!(telemetry.llm_heat, 0.0);
        assert_eq!(telemetry.quantum_activity, 0.0);
        assert_eq!(telemetry.spectral_shift, 0.0);
        assert_eq!(telemetry.temporal_pulse, 0.0);
        assert_eq!(telemetry.epistemic_density, 0.0);
        assert_eq!(telemetry.manifold_pressure, 0.0);
        assert_eq!(telemetry._padding, [0.0, 0.0]);
    }

    #[test]
    fn test_update_memory_pressure_clamps() {
        let mut telemetry = SystemTelemetry::new();
        telemetry.update_memory_pressure(0.5);
        assert_eq!(telemetry.memory_pressure, 0.5);

        telemetry.update_memory_pressure(1.5);
        assert_eq!(telemetry.memory_pressure, 1.0);

        telemetry.update_memory_pressure(-0.5);
        assert_eq!(telemetry.memory_pressure, 0.0);
    }

    #[test]
    fn test_update_network_ripple_normalizes() {
        let mut telemetry = SystemTelemetry::new();
        telemetry.update_network_ripple(5_000_000.0);
        assert_eq!(telemetry.network_ripple, 0.5);

        telemetry.update_network_ripple(20_000_000.0);
        assert_eq!(telemetry.network_ripple, 1.0);
    }

    #[test]
    fn test_update_llm_heat_normalizes() {
        let mut telemetry = SystemTelemetry::new();
        telemetry.update_llm_heat(50.0);
        assert_eq!(telemetry.llm_heat, 0.5);

        telemetry.update_llm_heat(200.0);
        assert_eq!(telemetry.llm_heat, 1.0);
    }

    #[test]
    fn test_to_bytes_length() {
        let telemetry = SystemTelemetry::new();
        let bytes = telemetry.to_bytes();
        assert_eq!(bytes.len(), 48);
    }

    #[test]
    fn test_copy_semantics() {
        let telemetry1 = SystemTelemetry::new();
        let telemetry2 = telemetry1; // Should copy, not move
        assert_eq!(telemetry1.memory_pressure, 0.0);
        assert_eq!(telemetry2.memory_pressure, 0.0);
    }
}
