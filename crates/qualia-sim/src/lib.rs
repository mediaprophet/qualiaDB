//! QualiaDB Deterministic Simulation Engine (`qualia-sim`)
//!
//! Provides:
//! - 64 Hz fixed timestep simulation accumulator ($1/64 = 0.015625\text{ s} = 2^{-6}$)
//! - Zero-heap bitset input engine with single-frame edge transitions
//! - Mobile multi-touch virtual joysticks and touch gesture trackers
//! - Pixel-perfect screen-to-world raycasting and spring-damper pursuit cameras

pub mod animation;
pub mod atmosphere;
pub mod camera;
pub mod character;
pub mod differential_codec;
pub mod ecs;
pub mod fire_ecology;
pub mod flora_archetype;
pub mod geomechanics;
pub mod ground_cover;
pub mod hydro;
pub mod input;
pub mod kinematics;
pub mod leaf_biophysics;
pub mod metamer_fspm;
pub mod navmesh;
pub mod optics;
pub mod physics3d;
pub mod ripple;
pub mod space_weather;
pub mod time;

pub use space_weather::{
    evaluate_solar_cycle_phase, evaluate_space_weather_coupling, AtmosphericElectricCircuit,
    GeomagneticStormState, HaleMagneticCycle, SchwabeSolarCycle, SolarWindState,
    BASELINE_TSI_W_M2, HALE_CYCLE_YEARS, SCHWABE_CYCLE_YEARS,
};

pub use fire_ecology::{
    evaluate_dead_fine_fuel_moisture, evaluate_fireline_intensity, FirelineRegime, FirelineVerdict,
    ForestFuelStratum, FuelMoistureState, DEAD_FUEL_EXTINCTION_MOISTURE, EUCALYPT_HEAT_YIELD_KJ_KG,
};

pub use geomechanics::{
    critical_trench_depth_m, evaluate_excavation_pass, evaluate_soil_compaction,
    ExcavationVerdict, SoilMechanicsProfile, ToolEffector, ToolKind,
};

pub use differential_codec::{
    DisocclusionForecaster, EntitySpatialBounds, KinematicExtrapolator,
    ProgressiveMonteCarloGovernor, SaliencePerceptualGovernor, MAX_PREDICTED_DISOCCLUSIONS,
};

pub use animation::{
    AnimationStateMachine, Transform3D, TransitionMode, ZeroHeapSkeleton, MAX_BONES,
};
pub use atmosphere::{
    roughness, AeroelasticBranch, BarometricState, BoundaryLayerProfile, LeafFlutterState,
    GAS_CONSTANT_DRY_AIR, GAS_CONSTANT_WATER_VAPOR, STANDARD_MSLP_HPA, VON_KARMAN_KAPPA,
};
pub use camera::{
    unproject_screen_to_ray, FlyCamera, OrbitCamera, Ray3D, SpringChaseCamera,
};
pub use character::{step_character_3d, CharacterConfig, CharacterController, CharacterState};
pub use ecs::{Added, Changed, SimTable, SimTableError, Tick, INVALID_ROW};
pub use physics3d::{
    cast_capsule, cast_ray, cast_sphere, compute_narrowphase_contact,
    couple_rigid_body_with_botanical_stem, couple_rigid_body_with_hydro,
    couple_rigid_body_with_soil, evaluate_and_fracture_body, gjk_intersect, overlap_aabb,
    step_xpbd_world, Aabb3D, ArticulatedRagdoll, BallMotorJoint, BroadphaseBvh, ColliderShape3D,
    ContactManifold3D, ContactPoint3D, FixedMotorJoint, FractureOutcome3D, MotorJoint3D,
    NarrowphaseContact, RagdollMode, RayHit3D, RevoluteMotorJoint, RigidBody3D, RigidBodyType3D,
    SdfPrimitive, ShapeCastHit3D, SpringSuspensionJoint,
};
pub use flora_archetype::{
    FloraArchetypeConfig, FloraBiomechanicsEvaluator, RaunkiaerLifeForm, StemSupportRegime,
};
pub use ground_cover::{
    spatial_hash_2d, EcotopePatch, GroundCoverLod, ToroidalGroundCoverClipmap,
    TOROIDAL_GRID_DIM, TOROIDAL_TOTAL_CELLS,
};
pub use hydro::{
    ArchimedesProxy, BuoyancyResult, BuoyancySphere, GerstnerWave, HydrologicalHysteresisGate,
    SimdGerstnerSpectrum, SubmersionState, GRAVITY_M_S2, MAX_BUOYANCY_SPHERES,
    MAX_GERSTNER_HARMONICS, WATER_DENSITY_KG_M3,
};
pub use leaf_biophysics::{
    LeafEconomicsProfile, LeafVascularSkeleton, VeinEdge, MAX_VEIN_EDGES, NO_PARENT_VEIN,
};
pub use metamer_fspm::{
    Metamer, MetamerOrgan, MetamerPlantGraph, ThermalPhenology, MAX_METAMERS,
    NO_PARENT_METAMER,
};
pub use optics::{
    BioGeoOpticalConstituents, DielectricVolumeIop, PURE_WATER_ABSORPTION_RGB,
    PURE_WATER_RAYLEIGH_RGB, WAVELENGTH_BLUE_NM, WAVELENGTH_GREEN_NM, WAVELENGTH_RED_NM,
};
pub use ripple::{
    InteractiveRippleBuffer, RIPPLE_GRID_DIM, RIPPLE_TOTAL_CELLS,
};
pub use input::{
    MultiTouchTracker, TouchContact, VirtualTouchStick, ZeroHeapButtonInput, BUTTON_MIDDLE,
    BUTTON_PRIMARY, BUTTON_SECONDARY, INPUT_WORDS_4, KEY_A, KEY_ARROW_DOWN, KEY_ARROW_LEFT,
    KEY_ARROW_RIGHT, KEY_ARROW_UP, KEY_D, KEY_E, KEY_ESCAPE, KEY_Q, KEY_S, KEY_SHIFT, KEY_SPACE,
    KEY_TAB, KEY_W, MAX_TOUCH_POINTS,
};
pub use kinematics::{RoverKinematics, VehicleChassisConfig, VehicleState, WheelConfig};
pub use navmesh::{NavPolygon, Point2D, ZeroHeapNavMesh, MAX_NAVMESH_NODES, MAX_PATH_WAYPOINTS};
pub use time::{
    FixedTimestepAccumulator, DEFAULT_MAX_SUBSTEPS, FIXED_DELTA_NANOS, FIXED_DELTA_SECONDS,
    FIXED_HZ,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_pipeline_integration() {
        let mut sim = FixedTimestepAccumulator::new();
        let mut input = ZeroHeapButtonInput::<4>::new();
        let mut cam = SpringChaseCamera::default();

        // Simulate 1 frame of 16.6ms
        let substeps = sim.advance(0.0166);
        assert_eq!(substeps, 1);

        input.press(KEY_W);
        assert!(input.is_pressed(KEY_W));

        for _ in 0..substeps {
            cam.update([0.0, 1.0, 0.0], [0.0, 5.0, 0.0], 0.0, FIXED_DELTA_SECONDS);
        }

        let alpha = sim.interpolation_alpha();
        assert!(alpha >= 0.0 && alpha <= 1.0);
    }

    #[test]
    fn test_hydrological_character_ripple_pipeline() {
        let mut character = CharacterController::new(CharacterConfig::default(), [0.0, 0.0, 0.0]);
        let mut hysteresis = HydrologicalHysteresisGate::default();
        let mut ripple = InteractiveRippleBuffer::new(0.0, 0.0, 16.0, 2.5, 0.985);
        let baro = BarometricState::default();
        let abl = BoundaryLayerProfile::default();

        let dt = FIXED_DELTA_SECONDS as f32;

        // Character enters water at depth 0.6m (wading)
        let waves = SimdGerstnerSpectrum::default();
        let _surface_h = waves.sample_surface(0.0, 0.0, 0.0);

        // Step character with water physics
        let sub = character.step_with_water(
            [0.0, 1.0],
            false,
            false,
            false,
            dt,
            |_x, _z| (0.0, [0.0, 1.0, 0.0]),
            |_x, _z| 0.6,
        );
        assert_eq!(sub, Some(SubmersionState::Wading));

        // Hysteresis confirms wading transition and computes wetness
        let _transition = hysteresis.update(0.6, 1, dt);
        assert!(hysteresis.wetness_ratio > 0.0);

        // Footstep generates ripple impulse at character position
        ripple.inject_impulse(character.state.position[0], character.state.position[2], 0.15, 0.3);
        ripple.step(dt);

        let wave_h = ripple.sample_height(character.state.position[0], character.state.position[2]);
        assert!(wave_h > 0.0);

        // Atmospheric layer confirms air density and surface wind
        let rho = baro.air_density(0.0);
        assert!(rho > 1.15 && rho < 1.30);

        let wind_at_char = abl.sample_wind_speed(character.state.position[1] + 1.0);
        assert!(wind_at_char > 0.0);
    }

    #[test]
    fn test_flora_biophysics_and_ground_cover_pipeline() {
        // 1. Leaf vascular skeleton with Murray's Law
        let leaf_profile = LeafEconomicsProfile::eucalyptus_sclerophyll();
        let leaf = LeafVascularSkeleton::new(leaf_profile, 1.2);
        assert!(leaf.edges[0].radius_mm > leaf.edges[4].radius_mm);

        // 2. Metamer tree with Shinozaki pipe model
        let phenology = ThermalPhenology::default();
        let mut tree = MetamerPlantGraph::new(phenology);
        tree.solve_growth_cycle(1.0, 0.8);
        assert!(tree.radius_m(0) > tree.radius_m(tree.count - 1));

        // 3. Raunkiær life form frost dieback
        let mut herb = FloraBiomechanicsEvaluator::new(FloraArchetypeConfig::culinary_herb(), 0.5);
        herb.step_environment(-5.0, 0.5, 0.5);
        assert!(herb.is_dieback_active);
        assert!(herb.crown_height_m <= 0.05);

        // 4. Ground cover toroidal clipmap with trample injection
        let mut clipmap = ToroidalGroundCoverClipmap::default();
        clipmap.inject_trample(10.0, 10.0, 1.0, 0.0, 0.9);
        let cell_idx = clipmap.cell_index_at(5, 5);
        let fiber = clipmap.evaluate_sggx_fiber_direction(&clipmap.cells[cell_idx]);
        assert!(fiber[0] > 0.8);
        assert!(fiber[1] < 0.1);
    }
}
