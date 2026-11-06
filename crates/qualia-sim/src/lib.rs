//! QualiaDB Deterministic Simulation Engine (`qualia-sim`).
//!
//! Consolidated into `etalon-sim` and `etalon-render` (see
//! `docs/plans/etalon-sim-render-consolidation-uplift-2026-11-06.md`).
//! Preserves 100% backwards-compatible module and symbol paths while exposing
//! the uplifted multi-domain simulation, physics, skeletal, camera, hydrological,
//! and botanical capabilities.

pub use etalon_sim::*;
pub use etalon_render::{skeletal, vegetation, water_surface};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualia_sim_reexport_and_uplifts() {
        let mut clock = MultiRateSimClock::new(3);
        let (substeps, ticks) = clock.advance(0.05);
        assert_eq!(substeps, 3);
        assert_eq!(ticks, 1);

        let cam = OrbitCamera::default();
        let vp = cam.view_projection(16.0 / 9.0);
        assert!(vp[0][0].is_finite());
        let motor = cam.to_pga_motor();
        assert!(motor.r[0].is_finite());

        let flora = FloraBiomechanicsEvaluator::new(FloraArchetypeConfig::culinary_herb(), 0.8);
        let bending = vegetation::derive_bending_params_from_flora(&flora, [1.0, 0.0], 0.4);
        assert!(bending.stiffness > 0.0);

        let (solar, _hale) = evaluate_solar_cycle_phase(2025.5);
        let report = evaluate_heliophysical_fire_weather(
            &solar,
            &SolarWindState::default(),
            &BarometricState::default(),
            &BoundaryLayerProfile::default(),
            &ForestFuelStratum::default(),
            &FuelMoistureState::default(),
            5.0,
            100.0,
        );
        assert!(report.air_density_kg_m3 > 1.0);
    }
}
