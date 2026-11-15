//! Viewport WGSL shader source catalog (`RM-03` retired to `etalon_render::shaders`).
pub use etalon_render::shaders::*;

/// Alias for [`INSTANCE_CULL_INDIRECT_WGSL`].
pub const INDIRECT_CULL_WGSL: &str = INSTANCE_CULL_INDIRECT_WGSL;

/// Alias for [`WATER_WGSL`].
pub const WATER_SURFACE_WGSL: &str = WATER_WGSL;
