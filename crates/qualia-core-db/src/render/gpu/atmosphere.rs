//! Small static atmosphere uniform shared by opaque and blended mesh passes.

use crate::render::atmosphere::AtmospherePreset;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AtmosphereUniform {
    /// Scene-space fog RGB and base extinction density per world unit.
    pub fog_color_density: [f32; 4],
    /// Base height, exponential height falloff, maximum opacity, enabled.
    pub height: [f32; 4],
}

impl AtmosphereUniform {
    pub fn from_profile(profile: AtmospherePreset) -> Self {
        Self {
            fog_color_density: [
                profile.fog_rgb[0],
                profile.fog_rgb[1],
                profile.fog_rgb[2],
                profile.fog_density.clamp(0.0, 0.05),
            ],
            height: [0.0, 0.012, 0.92, f32::from(profile.fog_density > 0.0)],
        }
    }
}
