//! Viewport WGSL — migrated from webizen-render; owned by qualia-core-db.

pub const SPECTRAL_WGSL: &str = include_str!("spectral.wgsl");
pub const AMBIENT_WGSL: &str = concat!(include_str!("spectral.wgsl"), include_str!("ambient.wgsl"));
pub const PROJECTOR_WGSL: &str = concat!(
    include_str!("spectral.wgsl"),
    include_str!("projector.wgsl")
);
pub const MESH_WGSL: &str = include_str!("mesh.wgsl");
pub const WATER_WGSL: &str = include_str!("water.wgsl");
pub const HYDROLOGIC_OPTICS_WGSL: &str = include_str!("hydrologic_optics.wgsl");
pub const SUN_SHADOW_WGSL: &str = include_str!("sun_shadow.wgsl");
pub const AO_PREPASS_WGSL: &str = include_str!("ao_prepass.wgsl");
pub const AO_GENERATE_WGSL: &str = include_str!("ao_generate.wgsl");
pub const SKY_WGSL: &str = include_str!("sky.wgsl");
pub const OUTPUT_TRANSFORM_WGSL: &str = include_str!("output_transform.wgsl");
pub const OUTPUT_WGSL: &str = concat!(
    include_str!("output_transform.wgsl"),
    include_str!("output_composite.wgsl")
);
pub const TEMPORAL_RESOLVE_WGSL: &str = include_str!("temporal_resolve.wgsl");
pub const TEMPORAL_DEPTH_WGSL: &str = include_str!("temporal_depth.wgsl");
pub const TEMPORAL_OUTPUT_WGSL: &str = concat!(
    include_str!("output_transform.wgsl"),
    include_str!("temporal_output.wgsl")
);
pub const TEMPORAL_MOTION_WGSL: &str = include_str!("temporal_motion.wgsl");
pub const TEMPORAL_REACTIVE_WGSL: &str = include_str!("temporal_reactive.wgsl");
pub const BLOOM_WGSL: &str = concat!(
    include_str!("output_transform.wgsl"),
    include_str!("bloom.wgsl")
);
pub const EPISTEMIC_WGSL: &str = include_str!("epistemic.wgsl");
pub const SCREEN_WGSL: &str = include_str!("screen.wgsl");
pub const EMF_VOLUMETRIC_WGSL: &str = include_str!("emf_volumetric.wgsl");
pub const SURFEL_INSTANCED_WGSL: &str = include_str!("surfel_instanced.wgsl");
pub const ACOUSTIC_VOXEL_CONE_WGSL: &str = include_str!("acoustic_voxel_cone.wgsl");
pub const DEFERRED_RESOLVE_WGSL: &str = include_str!("deferred_resolve.wgsl");
pub const HOLOGRAPHIC_WGSL: &str = concat!(
    include_str!("spectral.wgsl"),
    include_str!("holographic.wgsl")
);
pub const SWE_SOLVER_WGSL: &str = include_str!("swe_solver.wgsl");
pub const PRECIPITATION_WGSL: &str = include_str!("precipitation.wgsl");
pub const CLUSTER_LIGHT_WGSL: &str = include_str!("cluster_light.wgsl");
pub const SGGX_MICROFLAKE_WGSL: &str = include_str!("sggx_microflake.wgsl");
pub const FOLIAGE_LOD_WGSL: &str = include_str!("foliage_lod.wgsl");
pub const TEMPORAL_WARP_WGSL: &str = include_str!("temporal_warp.wgsl");
pub const TILE_COMPACTION_WGSL: &str = include_str!("tile_compaction.wgsl");
pub const INSTANCE_CULL_INDIRECT_WGSL: &str = include_str!("instance_cull_indirect.wgsl");
