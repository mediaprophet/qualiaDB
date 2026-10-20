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
pub const MAX_SPATIAL_VOICES: usize = 64;

/// Acoustic occlusion bridge parameters shared with `qualia-core-db::audio::scene_audio_graph`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialOcclusionBridge {
    /// Direct-path amplitude multiplier in `[0.0, 1.0]`.
    pub direct_gain: f32,
    /// Frequency-dependent low-pass cutoff in Hz (`20000.0` = unoccluded).
    pub lowpass_cutoff_hz: f32,
    /// Phase diffraction delay $\Delta\phi$ in radians.
    pub diffraction_phase_rad: f32,
    /// Local canyon/canopy enclosure reverb send level in `[0.0, 1.0]`.
    pub reverb_send: f32,
}

impl Default for SpatialOcclusionBridge {
    fn default() -> Self {
        Self {
            direct_gain: 1.0,
            lowpass_cutoff_hz: 18_000.0,
            diffraction_phase_rad: 0.0,
            reverb_send: 0.1,
        }
    }
}

/// Zero-heap stereo DF2T biquad state for per-voice occlusion low-pass filtering.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpatialOcclusionFilterState {
    pub s1_l: f32,
    pub s2_l: f32,
    pub s1_r: f32,
    pub s2_r: f32,
}


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

impl SpatialListener {
    /// Extracts a `SpatialListener` pose directly from 8-component PGA `Motor` blades `(r, d)`
    /// (`r = [s, e12, e13, e23]`, `d = [e0123, e03, e02, e01]`) and linear velocity.
    #[inline]
    pub fn from_pga_motor_blades(r: [f32; 4], d: [f32; 4], velocity: [f32; 3]) -> Self {
        let position = pga_sandwich_point(r, d, [0.0, 0.0, 0.0]);
        let fwd_pt = pga_sandwich_point(r, d, [0.0, 0.0, 1.0]);
        let up_pt = pga_sandwich_point(r, d, [0.0, 1.0, 0.0]);
        let forward = normalize_vec3(
            [
                fwd_pt[0] - position[0],
                fwd_pt[1] - position[1],
                fwd_pt[2] - position[2],
            ],
            [0.0, 0.0, 1.0],
        );
        let up = normalize_vec3(
            [
                up_pt[0] - position[0],
                up_pt[1] - position[1],
                up_pt[2] - position[2],
            ],
            [0.0, 1.0, 0.0],
        );
        Self {
            position,
            velocity,
            forward,
            up,
        }
    }
}

#[inline]
fn pga_sandwich_point(r: [f32; 4], d: [f32; 4], p: [f32; 3]) -> [f32; 3] {
    let qr = [r[0], -r[3], -r[2], -r[1]];
    let qd = [d[0], -d[3], -d[2], -d[1]];
    let qr_conj = [qr[0], -qr[1], -qr[2], -qr[3]];
    let pq = [0.0, p[0], p[1], p[2]];
    let rot = quat_mul(quat_mul(qr, pq), qr_conj);
    let tq = quat_mul(qd, qr_conj);
    [
        rot[1] + 2.0 * tq[1],
        rot[2] + 2.0 * tq[2],
        rot[3] + 2.0 * tq[3],
    ]
}

#[inline]
fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

#[inline]
fn normalize_vec3(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        fallback
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

    /// Evaluates spatial acoustic parameters modulated by shared SDF + BVH [`SpatialOcclusionBridge`].
    #[inline]
    pub fn evaluate_spatial_with_occlusion(
        &self,
        listener: &SpatialListener,
        occlusion: &SpatialOcclusionBridge,
    ) -> SpatialVoiceParams {
        let mut params = self.evaluate_spatial(listener);
        let occ_gain = occlusion.direct_gain.clamp(0.0, 1.0);
        params.gain_left *= occ_gain;
        params.gain_right *= occ_gain;
        params
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

/// Zero-heap occluded stereo mixer bridging `qualia-audio` voices with `scene_audio_graph`
/// DF2T biquad low-pass occlusion filtering and wet reverb send bus accumulation.
pub fn mix_spatial_voice_occluded_into(
    mono_samples: &[f32],
    params: &SpatialVoiceParams,
    occlusion: &SpatialOcclusionBridge,
    sample_rate: f32,
    filter_state: &mut SpatialOcclusionFilterState,
    out_direct_stereo: &mut [[f32; 2]],
    out_reverb_stereo: &mut [[f32; 2]],
) {
    let n = mono_samples
        .len()
        .min(out_direct_stereo.len())
        .min(out_reverb_stereo.len());
    if n == 0 {
        return;
    }

    // Compute RBJ 2nd-order low-pass biquad coefficients for occlusion cutoff
    let sr = sample_rate.max(8_000.0);
    let fc = occlusion.lowpass_cutoff_hz.clamp(80.0, sr * 0.45);
    let w0 = std::f32::consts::TAU * fc / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * 0.7071);
    let inv_a0 = 1.0 / (1.0 + alpha).max(1e-6);
    let b0 = ((1.0 - cos_w0) * 0.5) * inv_a0;
    let b1 = (1.0 - cos_w0) * inv_a0;
    let b2 = b0;
    let a1 = (-2.0 * cos_w0) * inv_a0;
    let a2 = (1.0 - alpha) * inv_a0;

    let gl = params.gain_left;
    let gr = params.gain_right;
    let rev_send = occlusion.reverb_send.clamp(0.0, 1.0);
    const DC_OFFSET: f32 = 1e-24;

    for i in 0..n {
        let s = mono_samples[i] + DC_OFFSET;
        let xl = s * gl;
        let xr = s * gr;

        let yl = b0 * xl + filter_state.s1_l;
        filter_state.s1_l = b1 * xl - a1 * yl + filter_state.s2_l;
        filter_state.s2_l = b2 * xl - a2 * yl;

        let yr = b0 * xr + filter_state.s1_r;
        filter_state.s1_r = b1 * xr - a1 * yr + filter_state.s2_r;
        filter_state.s2_r = b2 * xr - a2 * yr;

        out_direct_stereo[i][0] += yl;
        out_direct_stereo[i][1] += yr;
        out_reverb_stereo[i][0] += yl * rev_send;
        out_reverb_stereo[i][1] += yr * rev_send;
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

    #[test]
    fn test_pga_motor_listener_and_occluded_mixer_bridge() {
        // Pure translation motor by [0.0, 2.0, -5.0] in PGA blade4 encoding
        let r = [1.0, 0.0, 0.0, 0.0];
        let d = [0.0, 2.5, -1.0, 0.0]; // blade4: [w, -z, -y, -x] for half-translation [0, 1, -2.5]
        let listener = SpatialListener::from_pga_motor_blades(r, d, [0.0, 0.0, 0.0]);
        assert!((listener.position[1] - 2.0).abs() < 1e-4);
        assert!((listener.position[2] + 5.0).abs() < 1e-4);

        let mut emitter = SpatialEmitter::default();
        emitter.position = [4.0, 2.0, -5.0];
        let occlusion = SpatialOcclusionBridge {
            direct_gain: 0.4,
            lowpass_cutoff_hz: 900.0,
            diffraction_phase_rad: 0.25,
            reverb_send: 0.35,
        };
        let params = emitter.evaluate_spatial_with_occlusion(&listener, &occlusion);
        assert!(params.gain_right > params.gain_left);

        let mono = [0.8_f32; 32];
        let mut st = SpatialOcclusionFilterState::default();
        let mut direct = [[0.0_f32; 2]; 32];
        let mut reverb = [[0.0_f32; 2]; 32];
        mix_spatial_voice_occluded_into(
            &mono,
            &params,
            &occlusion,
            48_000.0,
            &mut st,
            &mut direct,
            &mut reverb,
        );
        assert!(direct[31][1] > direct[31][0]);
        assert!(reverb[31][1] > 0.0);
    }
}
