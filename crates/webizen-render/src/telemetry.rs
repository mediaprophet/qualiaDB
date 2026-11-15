//! System Telemetry Data Contract.
//!
//! Re-exports the canonical 48-byte GPU telemetry contract from
//! `etalon_render::webizen_core::telemetry`.

pub use etalon_render::webizen_core::telemetry::*;

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

    #[test]
    fn ambient_helpers_are_derived_and_stable() {
        let mut telemetry = SystemTelemetry::new();
        telemetry.set_network_ripple(0.8);
        telemetry.set_temporal_pulse(0.5);
        telemetry.set_manifold_pressure(0.2);
        assert!((telemetry.ambient_wind_strength() - 0.68).abs() < 1e-6);
        assert_eq!(SystemTelemetry::ambient_world_frame(1.0), 60);
        assert_eq!(SystemTelemetry::ambient_world_frame(f32::NAN), 0);
    }
}
