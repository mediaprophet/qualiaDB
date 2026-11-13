//! Bounded, explicitly authored native review shots.
use super::{SystemTelemetry, VolumetricRenderer, ReviewGeometry};
use image::ImageEncoder;
use qualia_core_db::render::lighting::environment_lighting::{EnvironmentProbe, EnvironmentProbeSet, MAX_ENVIRONMENT_PROBES};
use super::volumetric_native_preview::CappedPngWriter;
fn default_true() -> bool { true }
fn default_radius() -> f32 { 0.05 }
fn default_strength() -> f32 { 0.75 }
fn default_bias() -> f32 { 0.001 }
fn default_samples() -> u32 { 12 }
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShotProbe {
    pub position: [f32; 3], pub radius: f32, pub irradiance: [f32; 3],
    pub specular_levels: [[f32; 3]; 5], pub dominant_direction: [f32; 3],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeShot {
    pub side: u32,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub camera_target: [f32; 3],
    pub sun_direction: [f32; 3],
    pub direct_intensity: f32,
    pub ambient_intensity: f32,
    pub clear_color: [f32; 4],
    #[serde(default = "default_true")] pub shadows: bool,
    #[serde(default = "default_true")] pub ao_enabled: bool,
    #[serde(default = "default_radius")] pub ao_radius: f32,
    #[serde(default = "default_strength")] pub ao_strength: f32,
    #[serde(default = "default_bias")] pub ao_bias: f32,
    #[serde(default = "default_samples")] pub ao_samples: u32,
    #[serde(default)] pub environment_probes: Vec<ShotProbe>,
    #[serde(default)] pub environment_fallback_rgb: [f32; 3],
    /// Omit for a static still; otherwise one output per time using this SKL1 clip.
    pub clip_id: Option<u64>,
    pub times: Vec<f64>,
}
impl NativeShot {
    pub fn validate(&self) -> Result<(), String> {
        let finite = self.camera_target.iter().chain(self.sun_direction.iter()).chain(self.clear_color.iter()).all(|x| x.is_finite());
        let norm = self.sun_direction.iter().map(|v| v*v).sum::<f32>();
        let probes_valid = self.environment_probes.len() <= MAX_ENVIRONMENT_PROBES && self.environment_fallback_rgb.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0) && self.environment_probes.iter().all(|p| {
            p.radius.is_finite() && p.radius > 0.0 && p.radius <= 1e8 && p.position.iter().chain(p.dominant_direction.iter()).all(|v| v.is_finite() && v.abs() <= 1e8)
            && p.irradiance.iter().chain(p.specular_levels.iter().flatten()).all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
            && p.dominant_direction.iter().map(|v| v*v).sum::<f32>() > 1e-12
        });
        if !probes_valid || !finite || self.camera_target.iter().any(|v| v.abs() > 1e8) || !self.camera_yaw.is_finite() || self.camera_yaw.abs() > 1e6 || !self.camera_pitch.is_finite() || !self.camera_distance.is_finite()
            || !self.direct_intensity.is_finite() || !self.ambient_intensity.is_finite()
            || self.side == 0 || self.side > 2048 || self.times.is_empty() || self.times.len() > 16
            || (self.side as u64 * self.side as u64 * 4 * self.times.len() as u64) > 64 * 1024 * 1024
            || !(-1.45..=1.45).contains(&self.camera_pitch)
            || self.times.iter().any(|t| !t.is_finite() || *t < 0.0 || *t > 86400.0)
            || self.times.windows(2).any(|p| p[0] > p[1])
            || !(0.0001..=1e8).contains(&norm)
            || !(0.05..=2.0).contains(&self.ao_radius) || !(0.0..=1.0).contains(&self.ao_strength) || !(0.0..=0.2).contains(&self.ao_bias) || ![4,8,12].contains(&self.ao_samples)
            || !(0.0..=100.0).contains(&self.direct_intensity) || !(0.0..=100.0).contains(&self.ambient_intensity)
            || self.clear_color.iter().any(|v| !(0.0..=1.0).contains(v))
            || !(qualia_core_db::render::camera::CAMERA_MIN_ZOOM..=qualia_core_db::render::camera::CAMERA_MAX_ZOOM).contains(&self.camera_distance)
 {
            return Err("invalid or out-of-budget native shot".into());
        }
        Ok(())
    }
}
impl VolumetricRenderer {
    /// Render explicit timestamps from immutable bind geometry. Texture resources,
    /// when used, must first be admitted with `load_hmc_asset_with_texture_budget`.
    pub fn render_10d_shot(&mut self, bytes: &[u8], shot: &NativeShot) -> Result<(ReviewGeometry, Vec<Vec<u8>>), String> {
        shot.validate()?;
        let pixels = (shot.side as usize).checked_mul(shot.side as usize).ok_or("shot dimensions overflow")?;
        let len = pixels.checked_mul(4).ok_or("shot frame overflow")?;
        if self.required_rgba8_bytes() != len { return Err("shot renderer target size differs".into()); }
        self.inner.set_tensor_projection_enabled(false);
        self.inner.set_ambient_enabled(false);
        self.inner.set_screen_space_ao_enabled(shot.ao_enabled);
        self.inner.set_screen_space_ao(shot.ao_radius, shot.ao_strength, shot.ao_bias);
        self.inner.set_screen_space_ao_sample_count(shot.ao_samples);
        self.inner.set_shadows_enabled(shot.shadows);
        self.inner.set_clear_color(shot.clear_color[0] as f64, shot.clear_color[1] as f64, shot.clear_color[2] as f64, shot.clear_color[3] as f64);
        self.inner.set_lighting(shot.sun_direction[0], shot.sun_direction[1], shot.sun_direction[2], shot.direct_intensity, shot.ambient_intensity);
        let mut probes = EnvironmentProbeSet::new();
        for p in &shot.environment_probes {
            if !probes.push(EnvironmentProbe { position: p.position, radius: p.radius, irradiance: p.irradiance, specular_levels: p.specular_levels, dominant_direction: p.dominant_direction, valid: true }) { return Err("environment probe limit exceeded".into()); }
        }
        self.inner.set_camera_target(shot.camera_yaw, shot.camera_pitch, shot.camera_distance, shot.camera_target[0], shot.camera_target[1], shot.camera_target[2]);
        let mut rgba = vec![0; len];
        let mut frames = Vec::with_capacity(shot.times.len());
        let mut encoded_total = 0usize;
        let mut geometry = None;
        for &time in &shot.times {
            let mut g = if let Some(clip) = shot.clip_id { self.load_10d_shot_pose(bytes, clip, time)? } else { self.load_10d_review_asset(bytes)?.0 };
            g.camera_target = shot.camera_target;
            g.requested_camera_distance = shot.camera_distance;
            g.camera_distance = shot.camera_distance;
            g.lighting.sun_direction = shot.sun_direction;
            g.lighting.direct_intensity = shot.direct_intensity;
            g.lighting.ambient_intensity = shot.ambient_intensity;
            g.lighting.clear_color = shot.clear_color;
            g.lighting.environment_additive_fallback_rgb = shot.environment_fallback_rgb;
            g.screen_space_ao_radius_m = shot.ao_radius;
            g.screen_space_ao_strength = shot.ao_strength;
            g.screen_space_ao_samples = shot.ao_samples;
            g.shadows_available = self.inner.shadows_enabled();
            // Upload replaces material bindings, so probes must be set after each frame upload.
            self.inner.set_environment_probes(Some(&probes), shot.environment_fallback_rgb);
            self.render(time as f32, &SystemTelemetry::default())?;
            if self.read_rgba8_into(&mut rgba)? != len { return Err("shot readback size mismatch".into()); }
            let mut png = CappedPngWriter::new(pixels.checked_mul(5).ok_or("PNG cap overflow")?);
            image::codecs::png::PngEncoder::new(&mut png).write_image(&rgba, shot.side, shot.side, image::ExtendedColorType::Rgba8).map_err(|e| format!("shot PNG: {e}"))?;
            let png = png.into_inner();
            encoded_total = encoded_total.checked_add(png.len()).ok_or("shot PNG total overflow")?;
            if encoded_total > 64 * 1024 * 1024 { return Err("shot PNGs exceed aggregate output budget".into()); }
            frames.push(png);
            geometry = Some(g);
        }
        Ok((geometry.ok_or("empty shot")?, frames))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shot_admission_rejects_invalid_frames_and_camera() {
        let mut s = NativeShot { side: 64, camera_yaw: 0.0, camera_pitch: 0.0, camera_distance: 2.0, camera_target: [0.0;3], sun_direction: [0.0,1.0,0.0], direct_intensity: 1.0, ambient_intensity: 0.2, clear_color: [0.0,0.0,0.0,1.0], shadows:true, ao_enabled:true, ao_radius:0.05, ao_strength:0.75, ao_bias:0.001, ao_samples:12, environment_probes:vec![], environment_fallback_rgb:[0.0;3], clip_id: Some(1), times: vec![0.0,0.5,1.0] };
        assert!(s.validate().is_ok());
        s.times = vec![1.0,0.0]; assert!(s.validate().is_err());
        s.times = vec![0.0]; s.sun_direction = [0.0;3]; assert!(s.validate().is_err());
    }
}
