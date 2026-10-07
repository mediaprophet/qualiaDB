//! Shared authored sky, sun, ambient and analytic-fog state for renderer backends.
//!
//! Preset values are art-directed scene-space controls, not calibrated atmospheric units. The
//! analytic sky and mesh-fog shaders consume the same profile as the mesh-lighting path.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmospherePreset {
    pub id: u32,
    pub clear_rgba: [f32; 4],
    pub sun_direction: [f32; 3],
    pub sun_radiance: f32,
    pub ambient_irradiance: f32,
    pub fog_rgb: [f32; 3],
    pub fog_density: f32,
}

impl AtmospherePreset {
    /// Resolve a public preset id to one complete lighting/atmosphere state.
    /// Unknown ids use the stable deep-void fallback (preset 0).
    pub const fn from_id(id: u32) -> Self {
        match id {
            1 => Self {
                id: 1,
                clear_rgba: [0.45, 0.68, 0.92, 1.0],
                sun_direction: [0.45, 0.85, 0.35],
                sun_radiance: 1.25,
                ambient_irradiance: 0.40,
                fog_rgb: [0.57, 0.71, 0.88],
                fog_density: 0.0012,
            },
            2 => Self {
                id: 2,
                clear_rgba: [0.93, 0.62, 0.38, 1.0],
                sun_direction: [0.85, 0.25, 0.45],
                sun_radiance: 1.15,
                ambient_irradiance: 0.35,
                fog_rgb: [0.91, 0.48, 0.27],
                fog_density: 0.0018,
            },
            3 => Self {
                id: 3,
                clear_rgba: [0.06, 0.08, 0.16, 1.0],
                sun_direction: [-0.3, 0.9, -0.2],
                sun_radiance: 0.45,
                ambient_irradiance: 0.12,
                fog_rgb: [0.045, 0.055, 0.11],
                fog_density: 0.0007,
            },
            _ => Self {
                id: 0,
                clear_rgba: [0.03, 0.05, 0.08, 1.0],
                sun_direction: [0.45, 0.8, 0.55],
                sun_radiance: 1.0,
                ambient_irradiance: 0.25,
                fog_rgb: [0.035, 0.045, 0.075],
                fog_density: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AtmospherePreset;

    #[test]
    fn preset_state_is_complete_and_unknown_ids_are_stable() {
        for id in 0..=3 {
            let preset = AtmospherePreset::from_id(id);
            assert_eq!(preset.id, id);
            assert!(preset.clear_rgba.into_iter().all(f32::is_finite));
            assert!(preset.sun_direction.into_iter().all(f32::is_finite));
            assert!(preset.fog_rgb.into_iter().all(f32::is_finite));
            assert!(preset.sun_radiance.is_finite() && preset.sun_radiance >= 0.0);
            assert!(preset.ambient_irradiance.is_finite() && preset.ambient_irradiance >= 0.0);
            assert!(preset.fog_density.is_finite() && (0.0..=0.05).contains(&preset.fog_density));
        }
        assert_eq!(AtmospherePreset::from_id(99), AtmospherePreset::from_id(0));
    }

    #[test]
    fn authored_sky_presets_have_distinct_lighting_and_horizon_palettes() {
        let daylight = AtmospherePreset::from_id(1);
        let sunset = AtmospherePreset::from_id(2);
        let night = AtmospherePreset::from_id(3);
        assert_ne!(daylight.clear_rgba, sunset.clear_rgba);
        assert_ne!(sunset.fog_rgb, night.fog_rgb);
        assert!(daylight.ambient_irradiance > night.ambient_irradiance);
        assert!(daylight.fog_density > 0.0 && night.fog_density > 0.0);
    }
}
