//! Viewport WGSL — migrated from webizen-render; owned by qualia-core-db.

pub const SPECTRAL_WGSL: &str = include_str!("spectral.wgsl");
pub const AMBIENT_WGSL: &str = concat!(include_str!("spectral.wgsl"), include_str!("ambient.wgsl"));
pub const PROJECTOR_WGSL: &str = concat!(
    include_str!("spectral.wgsl"),
    include_str!("projector.wgsl")
);
pub const MESH_WGSL: &str = include_str!("mesh.wgsl");
pub const BLOOM_WGSL: &str = include_str!("bloom.wgsl");
pub const EPISTEMIC_WGSL: &str = include_str!("epistemic.wgsl");
pub const SCREEN_WGSL: &str = include_str!("screen.wgsl");
