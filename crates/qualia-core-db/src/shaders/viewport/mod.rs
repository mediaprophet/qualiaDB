//! Viewport WGSL — migrated from webizen-render; owned by qualia-core-db.

pub const SPECTRAL_WGSL: &str = include_str!("spectral.wgsl");
pub const AMBIENT_WGSL: &str = concat!(include_str!("spectral.wgsl"), include_str!("ambient.wgsl"));
pub const PROJECTOR_WGSL: &str = concat!(
    include_str!("spectral.wgsl"),
    include_str!("projector.wgsl")
);
pub const MESH_WGSL: &str = include_str!("mesh.wgsl");
pub const SUN_SHADOW_WGSL: &str = include_str!("sun_shadow.wgsl");
pub const AO_PREPASS_WGSL: &str = include_str!("ao_prepass.wgsl");
pub const AO_GENERATE_WGSL: &str = include_str!("ao_generate.wgsl");
pub const SKY_WGSL: &str = include_str!("sky.wgsl");
pub const OUTPUT_TRANSFORM_WGSL: &str = include_str!("output_transform.wgsl");
pub const OUTPUT_WGSL: &str = concat!(
    include_str!("output_transform.wgsl"),
    include_str!("output_composite.wgsl")
);
pub const BLOOM_WGSL: &str = concat!(
    include_str!("output_transform.wgsl"),
    include_str!("bloom.wgsl")
);
pub const EPISTEMIC_WGSL: &str = include_str!("epistemic.wgsl");
pub const SCREEN_WGSL: &str = include_str!("screen.wgsl");
pub const EMF_VOLUMETRIC_WGSL: &str = include_str!("emf_volumetric.wgsl");
