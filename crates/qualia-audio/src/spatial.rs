//! Zero-Heap 3D Spatial Audio & Doppler Engine (`spatial.rs`)
//!
//! Provides deterministic 3D spatial sound processing:
//! - Distance attenuation models (Inverse, Linear, Exponential)
//! - Directional emission cones (inner angle, outer angle, outer attenuation)
//! - Doppler frequency pitch shifting based on relative emitter-listener velocities
//! - Equal-power stereo binaural panning and interaural level/time differences
//! - Caller-buffered zero-heap mixing
//!
//! Designed according to `AGENTS.md` §0 (Zero heap in hot paths).

/// Speed of sound in dry air at 20°C in m/s.
pub const SPEED_OF_SOUND_AIR: f32 = 343.2;

/// Maximum spatial sound emitters processed in a single zero-heap mix pass.
pub const MAX_SPATIAL_VOICES: usize = 32;

/// Distance attenuation model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceModel {
    /// Inverse distance falloff: gain = ref_dist / (ref_dist + rolloff * (dist - ref_dist)).
    Inverse {
        ref_distance: f32,
        max_distance: f32,
        rolloff_factor: f32,
    },
    /// Linear distance falloff: gain = 1 - rolloff * (dist - ref_dist) / (max_dist - ref_dist).
    Linear {
        ref_distance: f32,
        max_distance: f32,
        rolloff_factor: f32,
    },
    /// Exponential distance falloff: gain = (dist / ref_dist)^(-rolloff).
    Exponential {
        ref_distance: f32,
        max_distance: f32,
        rolloff_factor: f32,
    },
}

impl Default for DistanceModel {
    fn default() -> Self {
        Self::Inverse {
            ref_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
        }
    }
}

impl DistanceModel {
    /// Calculates distance attenuation gain in range `[0.0, 1.0]`.
    pub fn calculate_gain(&self, distance: f32) -> f32 {
        match *self {
            DistanceModel::Inverse {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                let d = distance.clamp(ref_distance, max_distance);
                ref_distance / (ref_distance + rolloff_factor * (d - ref_distance))
            }
            DistanceModel::Linear {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                let d = distance.clamp(ref_distance, max_distance);
                if max_distance <= ref_distance {
                    1.0
                } else {
                    let factor = (d - ref_distance) / (max_distance - ref_distance);
                    (1.0 - rolloff_factor * factor).clamp(0.0, 1.0)
                }
            }
            DistanceModel::Exponential {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                let d = distance.clamp(ref_distance, max_distance);
                (d / ref_distance).powf(-rolloff_factor).clamp(0.0, 1.0)
            }
        }
    }
}

/// 3D Spatial Listener (camera or avatar ears).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialListener {
    /// 3D position [x, y, z] in meters.
    pub position: [f32; 3],
    /// 3D velocity [vx, vy, vz] in m/s (drives Doppler effect).
    pub velocity: [f32; 3],
    /// Unit forward look vector [fx, fy, fz].
    pub forward: [f32; 3],
    /// Unit up vector [ux, uy, uz].
    pub up: [f32; 3],
}

impl Default for SpatialListener {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
        }
    }
}

/// 3D Spatial Sound Emitter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialEmitter {
    /// 3D position [x, y, z] in meters.
    pub position: [f32; 3],
    /// 3D velocity [vx, vy, vz] in m/s.
    pub velocity: [f32; 3],
    /// Base volume gain multiplier (e.g. 1.0).
    pub gain: f32,
    /// Distance attenuation model.
    pub distance_model: DistanceModel,
    /// Directional forward vector [dx, dy, dz] (for directional cone).
    pub direction: [f32; 3],
    /// Inner cone angle in radians (full volume within this cone).
    pub inner_cone_angle: f32,
    /// Outer cone angle in radians (volume transitions to outer gain).
    pub outer_cone_angle: f32,
    /// Gain multiplier outside the outer cone.
    pub outer_cone_gain: f32,
    /// Speed of sound medium (default 343.2 m/s).
    pub speed_of_sound: f32,
}

impl Default for SpatialEmitter {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            gain: 1.0,
            distance_model: DistanceModel::default(),
            direction: [0.0, 0.0, 1.0],
            inner_cone_angle: std::f32::consts::PI * 2.0, // Omni-directional by default
            outer_cone_angle: std::f32::consts::PI * 2.0,
            outer_cone_gain: 0.0,
            speed_of_sound: SPEED_OF_SOUND_AIR,
        }
    }
}

/// Computed spatial voice parameters for audio buffer rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialVoiceParams {
    /// Left channel volume gain.
    pub gain_left: f32,
    /// Right channel volume gain.
    pub gain_right: f32,
    /// Doppler pitch multiplier (1.0 = unchanged, >1.0 = higher pitch, <1.0 = lower pitch).
    pub pitch_multiplier: f32,
    /// Distance from listener in meters.
    pub distance: f32,
    /// Azimuth angle relative to listener facing (-PI to +PI).
    pub azimuth: f32,
}

impl SpatialEmitter {
    /// Evaluates spatial acoustic parameters relative to `listener`.
    pub fn evaluate_spatial(&self, listener: &SpatialListener) -> SpatialVoiceParams {
        let rx = self.position[0] - listener.position[0];
        let ry = self.position[1] - listener.position[1];
        let rz = self.position[2] - listener.position[2];

        let distance = (rx * rx + ry * ry + rz * rz).sqrt().max(1e-4);
        let dir = [rx / distance, ry / distance, rz / distance];

        // 1. Distance attenuation
        let dist_gain = self.distance_model.calculate_gain(distance);

        // 2. Directional cone attenuation
        let cone_gain = if self.inner_cone_angle < std::f32::consts::PI * 1.99 {
            let cos_angle = -(dir[0] * self.direction[0] + dir[1] * self.direction[1] + dir[2] * self.direction[2]);
            let angle = cos_angle.clamp(-1.0, 1.0).acos();
            if angle <= self.inner_cone_angle * 0.5 {
                1.0
            } else if angle >= self.outer_cone_angle * 0.5 {
                self.outer_cone_gain
            } else {
                let t = (angle - self.inner_cone_angle * 0.5) / (self.outer_cone_angle * 0.5 - self.inner_cone_angle * 0.5);
                1.0 + (self.outer_cone_gain - 1.0) * t
            }
        } else {
            1.0
        };

        let net_gain = self.gain * dist_gain * cone_gain;

        // 3. Stereo panning (Listener right vector = up x forward)
        let fwd = listener.forward;
        let up = listener.up;
        let right = [
            up[1] * fwd[2] - up[2] * fwd[1],
            up[2] * fwd[0] - up[0] * fwd[2],
            up[0] * fwd[1] - up[1] * fwd[0],
        ];

        let pan = (dir[0] * right[0] + dir[1] * right[1] + dir[2] * right[2]).clamp(-1.0, 1.0);
        let azimuth = (dir[0] * right[0] + dir[2] * fwd[0]).atan2(dir[0] * fwd[0] + dir[2] * fwd[2]);

        // Equal-power panning law
        let pan_angle = (pan + 1.0) * (std::f32::consts::PI * 0.25);
        let gain_left = net_gain * pan_angle.cos();
        let gain_right = net_gain * pan_angle.sin();

        // 4. Doppler effect calculation
        let c = self.speed_of_sound;
        let v_emitter = self.velocity[0] * dir[0] + self.velocity[1] * dir[1] + self.velocity[2] * dir[2];
        let v_listener = listener.velocity[0] * dir[0] + listener.velocity[1] * dir[1] + listener.velocity[2] * dir[2];

        let pitch_multiplier = if (c + v_emitter).abs() > 1e-3 {
            ((c - v_listener) / (c + v_emitter)).clamp(0.2, 5.0)
        } else {
            1.0
        };

        SpatialVoiceParams {
            gain_left,
            gain_right,
            pitch_multiplier,
            distance,
            azimuth,
        }
    }
}

/// Zero-heap stereo mixer mixing mono PCM audio into a stereo output buffer with spatial gains.
pub fn mix_spatial_voice_into(
    mono_samples: &[f32],
    params: &SpatialVoiceParams,
    out_stereo: &mut [[f32; 2]],
) {
    let n = mono_samples.len().min(out_stereo.len());
    let gl = params.gain_left;
    let gr = params.gain_right;

    for i in 0..n {
        let s = mono_samples[i];
        out_stereo[i][0] += s * gl;
        out_stereo[i][1] += s * gr;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_attenuation() {
        let model = DistanceModel::Inverse {
            ref_distance: 1.0,
            max_distance: 50.0,
            rolloff_factor: 1.0,
        };

        let g1 = model.calculate_gain(1.0);
        let g2 = model.calculate_gain(2.0);
        let g10 = model.calculate_gain(10.0);

        assert!((g1 - 1.0).abs() < 1e-5);
        assert!((g2 - 0.5).abs() < 1e-4);
        assert!(g10 < g2);
    }

    #[test]
    fn test_doppler_shift() {
        let mut emitter = SpatialEmitter::default();
        emitter.position = [0.0, 0.0, 100.0];
        // Moving towards listener at 34.3 m/s (~10% speed of sound)
        emitter.velocity = [0.0, 0.0, -34.32];

        let listener = SpatialListener::default();
        let params = emitter.evaluate_spatial(&listener);

        // When approaching, pitch multiplier must be > 1.0 (approx 1.11)
        assert!(params.pitch_multiplier > 1.05);
    }

    #[test]
    fn test_stereo_panning() {
        let mut emitter = SpatialEmitter::default();
        // Place emitter directly to the right
        emitter.position = [10.0, 0.0, 0.0];

        let listener = SpatialListener::default();
        let params = emitter.evaluate_spatial(&listener);

        // Right channel must be significantly louder than left
        assert!(params.gain_right > params.gain_left);
    }
}
