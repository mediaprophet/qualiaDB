//! Viewport Shaders Naga Validation Test Suite (`viewport_shader_naga_tests.rs`)
//!
//! Validates that all viewport WGSL shaders parse and validate cleanly via Naga.

use qualia_core_db::shaders::viewport::*;

#[test]
fn test_all_viewport_shaders_naga_parse() {
    let shaders: &[(&str, &str)] = &[
        ("SPECTRAL", SPECTRAL_WGSL),
        ("AMBIENT", AMBIENT_WGSL),
        ("PROJECTOR", PROJECTOR_WGSL),
        ("MESH", MESH_WGSL),
        ("WATER", WATER_WGSL),
        ("HYDROLOGIC_OPTICS", HYDROLOGIC_OPTICS_WGSL),
        ("SUN_SHADOW", SUN_SHADOW_WGSL),
        ("AO_PREPASS", AO_PREPASS_WGSL),
        ("AO_GENERATE", AO_GENERATE_WGSL),
        ("SKY", SKY_WGSL),
        ("OUTPUT_TRANSFORM", OUTPUT_TRANSFORM_WGSL),
        ("OUTPUT", OUTPUT_WGSL),
        ("TEMPORAL_RESOLVE", TEMPORAL_RESOLVE_WGSL),
        ("TEMPORAL_DEPTH", TEMPORAL_DEPTH_WGSL),
        ("TEMPORAL_OUTPUT", TEMPORAL_OUTPUT_WGSL),
        ("TEMPORAL_MOTION", TEMPORAL_MOTION_WGSL),
        ("TEMPORAL_REACTIVE", TEMPORAL_REACTIVE_WGSL),
        ("BLOOM", BLOOM_WGSL),
        ("EPISTEMIC", EPISTEMIC_WGSL),
        ("SCREEN", SCREEN_WGSL),
        ("EMF_VOLUMETRIC", EMF_VOLUMETRIC_WGSL),
        ("SURFEL_INSTANCED", SURFEL_INSTANCED_WGSL),
        ("ACOUSTIC_VOXEL_CONE", ACOUSTIC_VOXEL_CONE_WGSL),
        ("DEFERRED_RESOLVE", DEFERRED_RESOLVE_WGSL),
        ("HOLOGRAPHIC", HOLOGRAPHIC_WGSL),
        ("SWE_SOLVER", SWE_SOLVER_WGSL),
        ("PRECIPITATION", PRECIPITATION_WGSL),
        ("CLUSTER_LIGHT", CLUSTER_LIGHT_WGSL),
        ("SGGX_MICROFLAKE", SGGX_MICROFLAKE_WGSL),
        ("FOLIAGE_LOD", FOLIAGE_LOD_WGSL),
    ];

    for (name, source) in shaders {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|err| panic!("Shader `{name}` failed Naga WGSL parsing:\n{err:?}"));

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );

        let info = validator
            .validate(&module)
            .unwrap_or_else(|err| panic!("Shader `{name}` failed Naga validation:\n{err:?}"));

        assert!(
            !module.entry_points.is_empty() || source.contains("fn ") || source.contains("struct "),
            "Shader `{name}` produced empty module structure"
        );
        let _ = info;
    }
}
