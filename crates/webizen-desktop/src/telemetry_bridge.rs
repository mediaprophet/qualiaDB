//! Telemetry bridge for ambient visualization
//!
//! This module provides a thread-safe bridge between system telemetry collection
//! and the GPU renderer. It maintains SystemTelemetry state and provides Tauri
//! commands for frontend communication.
//!
//! Zero-heap consideration: The bridge uses stack-allocated SystemTelemetry
//! (Copy type) and atomic operations where possible. Only the Arc<Mutex<>> wrapper
//! uses heap allocation, which is unavoidable for thread-safe shared state.

use std::sync::{Arc, Mutex};
use tauri::State;
use webizen_render::scene_contract::SystemTelemetry;

/// Thread-safe telemetry bridge state
///
/// Zero-heap consideration: Uses Arc<Mutex<SystemTelemetry>> where SystemTelemetry
/// is a Copy type (48 bytes). The mutex protects the 48-byte struct, making
/// lock operations extremely fast (stack copy only).
#[derive(Clone)]
pub struct TelemetryBridge {
    /// Inner telemetry state (48-byte Copy type)
    inner: Arc<Mutex<SystemTelemetry>>,
    /// Ambient visualization enabled flag
    enabled: Arc<Mutex<bool>>,
}

impl TelemetryBridge {
    /// Create a new telemetry bridge with default values
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SystemTelemetry::default())),
            enabled: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if ambient visualization is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Toggle ambient visualization state
    /// Returns the new state (true = enabled, false = disabled)
    pub fn toggle(&self) -> bool {
        let mut enabled = self.enabled.lock().unwrap();
        *enabled = !*enabled;
        *enabled
    }

    /// Get a copy of the current telemetry state
    ///
    /// Zero-heap consideration: Returns a stack-allocated copy (48 bytes)
    /// No heap allocation occurs during this operation.
    pub fn get_telemetry(&self) -> SystemTelemetry {
        *self.inner.lock().unwrap()
    }

    /// Update the entire telemetry state
    ///
    /// Zero-heap consideration: Takes a stack-allocated SystemTelemetry by value
    /// and performs a simple stack copy into the mutex.
    pub fn set_telemetry(&self, telemetry: SystemTelemetry) {
        *self.inner.lock().unwrap() = telemetry;
    }

    /// Update memory pressure metric
    pub fn set_memory_pressure(&self, value: f32) {
        self.inner.lock().unwrap().set_memory_pressure(value);
    }

    /// Update network ripple metric
    pub fn set_network_ripple(&self, value: f32) {
        self.inner.lock().unwrap().set_network_ripple(value);
    }

    /// Update baking crystallization metric
    pub fn set_baking_crystallization(&self, value: f32) {
        self.inner.lock().unwrap().set_baking_crystallization(value);
    }

    /// Update logic flashes metric
    pub fn set_logic_flashes(&self, value: f32) {
        self.inner.lock().unwrap().set_logic_flashes(value);
    }

    /// Update LLM heat metric
    pub fn set_llm_heat(&self, value: f32) {
        self.inner.lock().unwrap().set_llm_heat(value);
    }

    /// Update quantum activity metric
    pub fn set_quantum_activity(&self, value: f32) {
        self.inner.lock().unwrap().set_quantum_activity(value);
    }

    /// Update spectral shift metric
    pub fn set_spectral_shift(&self, value: f32) {
        self.inner.lock().unwrap().set_spectral_shift(value);
    }

    /// Update temporal pulse metric
    pub fn set_temporal_pulse(&self, value: f32) {
        self.inner.lock().unwrap().set_temporal_pulse(value);
    }

    /// Update epistemic density metric
    pub fn set_epistemic_density(&self, value: f32) {
        self.inner.lock().unwrap().set_epistemic_density(value);
    }

    /// Update manifold pressure metric
    pub fn set_manifold_pressure(&self, value: f32) {
        self.inner.lock().unwrap().set_manifold_pressure(value);
    }

    /// Reset all metrics to default (zero)
    pub fn reset(&self) {
        self.inner.lock().unwrap().reset();
    }
}

impl Default for TelemetryBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect system telemetry from various sources
///
/// This function samples system metrics and returns a SystemTelemetry struct.
/// It uses stack-allocated operations and avoids heap allocations.
///
/// Zero-heap consideration: All operations are stack-allocated. The function
/// returns a 48-byte Copy type by value.
pub fn collect_system_telemetry() -> SystemTelemetry {
    crate::telemetry_hooks::collect_system_telemetry()
}

// ── Tauri Commands ────────────────────────────────────────────────────────

/// Get current system telemetry
///
/// Returns a copy of the current telemetry state to the frontend.
///
/// Zero-heap consideration: Returns a 32-byte Copy type, no heap allocation.
#[tauri::command]
pub fn get_system_telemetry(bridge: State<TelemetryBridge>) -> SystemTelemetry {
    bridge.get_telemetry()
}

/// Update a specific telemetry metric
///
/// Parameters:
/// - metric_name: Name of the metric to update
/// - value: New value (0.0 to 1.0)
///
/// Zero-heap consideration: Uses stack-allocated strings and f32 values.
#[tauri::command]
pub fn update_telemetry_metric(
    bridge: State<TelemetryBridge>,
    metric_name: String,
    value: f32,
) -> Result<(), String> {
    match metric_name.as_str() {
        "memory_pressure" => bridge.set_memory_pressure(value),
        "network_ripple" => bridge.set_network_ripple(value),
        "baking_crystallization" => bridge.set_baking_crystallization(value),
        "logic_flashes" => bridge.set_logic_flashes(value),
        "llm_heat" => bridge.set_llm_heat(value),
        "quantum_activity" => bridge.set_quantum_activity(value),
        "spectral_shift" => bridge.set_spectral_shift(value),
        "temporal_pulse" => bridge.set_temporal_pulse(value),
        "epistemic_density" => bridge.set_epistemic_density(value),
        "manifold_pressure" => bridge.set_manifold_pressure(value),
        _ => return Err(format!("Unknown metric: {}", metric_name)),
    }
    Ok(())
}

/// Reset all telemetry metrics to default
///
/// Zero-heap consideration: Simple mutex lock and stack copy.
#[tauri::command]
pub fn reset_telemetry(bridge: State<TelemetryBridge>) {
    bridge.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_bridge_creation() {
        let bridge = TelemetryBridge::new();
        let telemetry = bridge.get_telemetry();
        assert_eq!(telemetry.memory_pressure, 0.0);
        assert_eq!(telemetry.network_ripple, 0.0);
    }

    #[test]
    fn test_telemetry_bridge_update() {
        let bridge = TelemetryBridge::new();
        bridge.set_memory_pressure(0.5);
        bridge.set_llm_heat(0.8);

        let telemetry = bridge.get_telemetry();
        assert_eq!(telemetry.memory_pressure, 0.5);
        assert_eq!(telemetry.llm_heat, 0.8);
    }

    #[test]
    fn test_telemetry_bridge_reset() {
        let bridge = TelemetryBridge::new();
        bridge.set_memory_pressure(0.9);
        bridge.set_llm_heat(0.7);
        bridge.reset();

        let telemetry = bridge.get_telemetry();
        assert_eq!(telemetry.memory_pressure, 0.0);
        assert_eq!(telemetry.llm_heat, 0.0);
    }

    #[test]
    fn test_telemetry_clamping() {
        let bridge = TelemetryBridge::new();
        bridge.set_memory_pressure(1.5); // Should clamp to 1.0
        bridge.set_llm_heat(-0.5); // Should clamp to 0.0

        let telemetry = bridge.get_telemetry();
        assert_eq!(telemetry.memory_pressure, 1.0);
        assert_eq!(telemetry.llm_heat, 0.0);
    }

    #[test]
    fn test_collect_system_telemetry() {
        let telemetry = collect_system_telemetry();
        // All values should be within valid range
        assert!(telemetry.memory_pressure >= 0.0 && telemetry.memory_pressure <= 1.0);
        assert!(telemetry.network_ripple >= 0.0 && telemetry.network_ripple <= 1.0);
        assert!(telemetry.baking_crystallization >= 0.0 && telemetry.baking_crystallization <= 1.0);
        assert!(telemetry.logic_flashes >= 0.0 && telemetry.logic_flashes <= 1.0);
        assert!(telemetry.llm_heat >= 0.0 && telemetry.llm_heat <= 1.0);
    }
}
