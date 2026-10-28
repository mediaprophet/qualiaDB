//! Scene-Synchronized 3D Holosonic Wavefield & HRTF Audio Graph with Shared SDF + BVH Acoustic Occlusion.
//!
//! Implements Epic 8 of the QualiaDB Superior Game Engine architecture:
//! - Synchronizes observer [`AudioListenerPose`] directly from camera PGA [`Motor`] (`r`, `d`).
//! - Manages up to [`MAX_SCENE_AUDIO_EMITTERS`] = 64 active scene emitters ([`HolosonicNode`]s + point/ambient emitters)
//!   sharing analytical SDF geometry ([`HolographicNodeGpu`] / `PROC1`) and epistemic certainty $q$.
//! - Evaluates zero-heap acoustic occlusion ([`evaluate_acoustic_occlusion_zero_heap`]) against analytical SDF
//!   primitives (sphere, capsule, rounded box) and BVH AABB/triangle obstacles with material-specific
//!   absorption coefficients (rock/lithology, timber/trunk, foliage canopy, water surface).
//! - Computes frequency-dependent RBJ low-pass biquad coefficients, direct-path transmission loss,
//!   Fresnel/grazing phase diffraction delay $\Delta\phi$, and 6-tap early reflection geometry delays.
//! - Executes a zero-heap 4-bus effect graph ([`SceneAudioBusGraph`]):
//!   1. [`DirectBinauralBus`]: Continuous Holosonic wave superposition, per-emitter DF2T biquad occlusion
//!      low-pass filtering, and zero-heap HRIR FIR binaural convolution (ITD + ILD + head-shadow).
//!   2. [`EarlyReflectionsBus`]: 6-tap stereo early reflection delay line driven by obstacle geometry.
//!   3. [`LateReverbFreeverbBus`]: Zero-heap stereo Schroeder/Freeverb 4-comb + 2-allpass reverb network
//!      modulated by canyon/canopy enclosure density and manifold weight $w$.
//!   4. [`HolosonicTelemetryBus`]: Fixed ring buffer of 48-byte [`HolosonicSonicToken48`] records + SPSC
//!      publication into the `Q42` U3 [`SonicTokenRing`].

use std::f32::consts::{PI, TAU};

use crate::audio::acoustic_plane::push_sonic_token;
use crate::audio::dsp::biquad_df2t::{BiquadCoeffs, Df2tState, DENORMAL_DC_OFFSET};
#[allow(unused_imports)]
use crate::audio::holosonic::HolosonicModality;
use crate::audio::holosonic::{
    HolosonicEmitter, HolosonicPointSample, HolosonicSonicToken48, HolosonicWavefield,
    SPEED_OF_SOUND_MPS,
};
use crate::audio::hrtf::{binaural_from_position, BinauralGains};
use crate::container_10d::procedural_synthesis::evaluate_smooth_minimum_sdf;
#[cfg(feature = "gpu-runtime")]
use crate::render::gpu::holographic::HolographicNodeGpu;
use crate::render::pga::{sandwich_point, Motor};
use crate::sonic_token::{SonicEventType, SonicToken, SONIC_MAGIC};

/// Maximum active scene audio emitters processed in the zero-heap audio graph.
pub const MAX_SCENE_AUDIO_EMITTERS: usize = 64;

/// Number of early reflection geometry delay taps per acoustic occlusion evaluation.
pub const EARLY_REFLECTION_TAPS: usize = 6;

/// Zero-heap per-emitter short HRIR FIR tap count for real-time binaural convolution.
pub const ZERO_HEAP_HRIR_TAPS: usize = 16;

/// Fixed capacity of the 48-byte `HolosonicSonicToken48` telemetry ring buffer.
pub const HOLOSONIC_TELEMETRY_RING_CAP: usize = 64;

/// Observer audio listener pose synchronized from the camera's 3D PGA [`Motor`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioListenerPose {
    /// World-space listener position `[x, y, z]` in meters.
    pub position: [f32; 3],
    /// Unit forward look direction `[fx, fy, fz]`.
    pub forward: [f32; 3],
    /// Unit up direction `[ux, uy, uz]`.
    pub up: [f32; 3],
    /// Unit right ear direction `[rx, ry, rz]`.
    pub right: [f32; 3],
    /// Linear velocity `[vx, vy, vz]` in m/s for Doppler shift.
    pub velocity: [f32; 3],
    /// Horizontal yaw angle in radians around +Y.
    pub yaw_rad: f32,
    /// Manifold domain coordinate `w` (modulates high-frequency room damping).
    pub manifold_w: f32,
}

impl Default for AudioListenerPose {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            right: [1.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            yaw_rad: 0.0,
            manifold_w: 0.0,
        }
    }
}

impl AudioListenerPose {
    /// Extracts the world-space listener position and orthonormal basis vectors directly
    /// from a PGA [`Motor`] using the null-vector sandwich product $P' = M P \widetilde{M}$.
    #[inline]
    pub fn from_pga_motor(motor: Motor, velocity: [f32; 3], manifold_w: f32) -> Self {
        let origin = sandwich_point(motor, [0.0, 0.0, 0.0]);
        let px = sandwich_point(motor, [1.0, 0.0, 0.0]);
        let py = sandwich_point(motor, [0.0, 1.0, 0.0]);
        let pz = sandwich_point(motor, [0.0, 0.0, -1.0]);

        let right = normalize3(
            [px[0] - origin[0], px[1] - origin[1], px[2] - origin[2]],
            [1.0, 0.0, 0.0],
        );
        let up = normalize3(
            [py[0] - origin[0], py[1] - origin[1], py[2] - origin[2]],
            [0.0, 1.0, 0.0],
        );
        let forward = normalize3(
            [pz[0] - origin[0], pz[1] - origin[1], pz[2] - origin[2]],
            [0.0, 0.0, -1.0],
        );
        let yaw_rad = forward[0].atan2(-forward[2]);

        Self {
            position: origin,
            forward,
            up,
            right,
            velocity,
            yaw_rad,
            manifold_w,
        }
    }
}

/// Scene-synchronized Holosonic / point / ambient audio emitter node.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HolosonicNode {
    /// Core Holosonic wavefield emitter parameters.
    pub emitter: HolosonicEmitter,
    /// Scene `NodePool` index or entity slot.
    pub node_id: u32,
    /// 64-bit `NQuin` semantic identity hash.
    pub origin_quin_hash: u64,
    /// Deontic permission state (`0 = Permitted`, `1 = Obligatory`, `2 = Forbidden/Escrow`).
    pub deontic_permit: u8,
    /// True if emitter is an omnidirectional ambient bed rather than a localized SDF source.
    pub is_ambient: bool,
    /// Shared `HolographicPrimitive` SDF archetype (`0 = Sphere`, `1 = RoundedBox`, `2 = Capsule`, etc.).
    pub sdf_primitive: u8,
    /// Padding for alignment.
    pub reserved_u8: u8,
    /// World-space emitter velocity `[vx, vy, vz]` in m/s (drives Doppler shift).
    pub velocity: [f32; 3],
    /// Analytical SDF half-extents `[hx, hy, hz]` in meters (for volumetric emission).
    pub sdf_half_extents: [f32; 3],
}

impl Default for HolosonicNode {
    fn default() -> Self {
        Self {
            emitter: HolosonicEmitter::default(),
            node_id: 0,
            origin_quin_hash: 0,
            deontic_permit: 0,
            is_ambient: false,
            sdf_primitive: 0, // HolographicPrimitive::Sphere as u8
            reserved_u8: 0,
            velocity: [0.0; 3],
            sdf_half_extents: [0.5, 0.5, 0.5],
        }
    }
}

impl HolosonicNode {
    /// Synchronizes a [`HolosonicNode`] directly from a [`HolographicNodeGpu`] SDF proxy lens.
    #[inline]
    #[cfg(feature = "gpu-runtime")]
    pub fn from_holographic_node(
        node_id: u32,
        origin_quin_hash: u64,
        holo: &HolographicNodeGpu,
        modality: HolosonicModality,
    ) -> Self {
        let pos = [
            holo.position_scale[0],
            holo.position_scale[1],
            holo.position_scale[2],
        ];
        let scale = holo.position_scale[3].max(0.1);
        let freq_hz = (holo.frequency_amplitude[0] * 110.0).clamp(40.0, 4000.0);
        let amp = holo.frequency_amplitude[1].clamp(0.0, 1.0);
        let phase = holo.frequency_amplitude[2];
        let epistemic_q = holo.manifold_tensor[0].clamp(0.0, 1.0);
        let prim = (holo.half_extents_primitive[3] as u32).min(4) as u8;

        Self {
            emitter: HolosonicEmitter::new(
                pos,
                freq_hz,
                amp,
                phase,
                epistemic_q,
                (scale * 18.0).clamp(5.0, 150.0),
                modality,
            ),
            node_id,
            origin_quin_hash,
            deontic_permit: 0,
            is_ambient: false,
            sdf_primitive: prim,
            reserved_u8: 0,
            velocity: [0.0; 3],
            sdf_half_extents: [
                holo.half_extents_primitive[0] * scale,
                holo.half_extents_primitive[1] * scale,
                holo.half_extents_primitive[2] * scale,
            ],
        }
    }
}

/// Physical surface & volumetric material class governing acoustic absorption, transmission loss,
/// and early reflection scattering.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcousticMaterial {
    /// Dense silicate/granite/sandstone canyon wall: high reflection, strong transmission block.
    RockLithology = 0,
    /// Hardwood tree trunk / structural timber (`Corymbia` / `Eucalyptus`): medium-high transmission loss.
    TimberTrunk = 1,
    /// Volumetric foliage canopy / scrub: soft high-frequency rolloff, porous transmission.
    FoliageCanopy = 2,
    /// River / billabong water surface: high specular acoustic reflection.
    WaterSurface = 3,
}

impl AcousticMaterial {
    /// Direct-path transmission absorption coefficient per meter of obstacle thickness.
    #[inline]
    pub const fn transmission_absorption_per_m(self) -> f32 {
        match self {
            Self::RockLithology => 0.82,
            Self::TimberTrunk => 0.55,
            Self::FoliageCanopy => 0.22,
            Self::WaterSurface => 0.65,
        }
    }

    /// Specular early-reflection albedo coefficient in `[0.0, 1.0]`.
    #[inline]
    pub const fn reflection_albedo(self) -> f32 {
        match self {
            Self::RockLithology => 0.88,
            Self::TimberTrunk => 0.58,
            Self::FoliageCanopy => 0.24,
            Self::WaterSurface => 0.92,
        }
    }

    /// High-frequency low-pass cutoff multiplier when occluded by this material.
    #[inline]
    pub const fn cutoff_attenuation_factor(self) -> f32 {
        match self {
            Self::RockLithology => 0.14,
            Self::TimberTrunk => 0.28,
            Self::FoliageCanopy => 0.58,
            Self::WaterSurface => 0.22,
        }
    }
}

/// Shared analytical SDF and BVH B-Rep obstacle shapes for zero-heap acoustic ray/cone queries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcousticObstacleShape {
    /// Analytical SDF sphere (`sdf(p) = |p - c| - r`), matching `HolographicPrimitive::Sphere`.
    AnalyticalSphere { center: [f32; 3], radius: f32 },
    /// Analytical SDF capsule (`sdf_capsule(p, a, b, r)`), matching `HolographicPrimitive::Capsule` / trunks.
    AnalyticalCapsule {
        a: [f32; 3],
        b: [f32; 3],
        radius: f32,
    },
    /// Analytical SDF rounded box (`sdf_round_box(p - c, b, r)`), matching `HolographicPrimitive::RoundedBox`.
    AnalyticalRoundedBox {
        center: [f32; 3],
        half_extents: [f32; 3],
        round_radius: f32,
    },
    /// BVH axis-aligned bounding box obstacle.
    Aabb { min: [f32; 3], max: [f32; 3] },
    /// B-Rep mesh triangle obstacle (Möller–Trumbore intersection).
    Triangle {
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
    },
}

impl AcousticObstacleShape {
    /// Evaluates the exact signed distance $d = \text{sdf}(\mathbf{p})$ to the obstacle at world point `p`.
    #[inline]
    pub fn signed_distance(&self, p: [f32; 3]) -> f32 {
        match *self {
            Self::AnalyticalSphere { center, radius } => length3(sub3(p, center)) - radius,
            Self::AnalyticalCapsule { a, b, radius } => {
                let pa = sub3(p, a);
                let ba = sub3(b, a);
                let baba = dot3(ba, ba).max(1e-6);
                let h = (dot3(pa, ba) / baba).clamp(0.0, 1.0);
                let closest = [a[0] + ba[0] * h, a[1] + ba[1] * h, a[2] + ba[2] * h];
                length3(sub3(p, closest)) - radius
            }
            Self::AnalyticalRoundedBox {
                center,
                half_extents,
                round_radius,
            } => {
                let q = [
                    (p[0] - center[0]).abs() - half_extents[0],
                    (p[1] - center[1]).abs() - half_extents[1],
                    (p[2] - center[2]).abs() - half_extents[2],
                ];
                let outside = length3([q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)]);
                let inside = q[0].max(q[1].max(q[2])).min(0.0);
                outside + inside - round_radius
            }
            Self::Aabb { min, max } => {
                let center = [
                    0.5 * (min[0] + max[0]),
                    0.5 * (min[1] + max[1]),
                    0.5 * (min[2] + max[2]),
                ];
                let half = [
                    0.5 * (max[0] - min[0]).max(0.0),
                    0.5 * (max[1] - min[1]).max(0.0),
                    0.5 * (max[2] - min[2]).max(0.0),
                ];
                let q = [
                    (p[0] - center[0]).abs() - half[0],
                    (p[1] - center[1]).abs() - half[1],
                    (p[2] - center[2]).abs() - half[2],
                ];
                let outside = length3([q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)]);
                let inside = q[0].max(q[1].max(q[2])).min(0.0);
                outside + inside
            }
            Self::Triangle { v0, v1, v2 } => {
                // Distance from point to triangle plane / centroid proxy
                let e1 = sub3(v1, v0);
                let e2 = sub3(v2, v0);
                let n = normalize3(cross3(e1, e2), [0.0, 1.0, 0.0]);
                dot3(sub3(p, v0), n).abs()
            }
        }
    }

    /// Representative obstacle centroid for early reflection bounce estimation.
    #[inline]
    pub fn centroid(&self) -> [f32; 3] {
        match *self {
            Self::AnalyticalSphere { center, .. } => center,
            Self::AnalyticalCapsule { a, b, .. } => [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ],
            Self::AnalyticalRoundedBox { center, .. } => center,
            Self::Aabb { min, max } => [
                0.5 * (min[0] + max[0]),
                0.5 * (min[1] + max[1]),
                0.5 * (min[2] + max[2]),
            ],
            Self::Triangle { v0, v1, v2 } => [
                (v0[0] + v1[0] + v2[0]) / 3.0,
                (v0[1] + v1[1] + v2[1]) / 3.0,
                (v0[2] + v1[2] + v2[2]) / 3.0,
            ],
        }
    }
}

/// An acoustic obstacle in the shared SDF + BVH scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticObstacle {
    pub shape: AcousticObstacleShape,
    pub material: AcousticMaterial,
    /// Effective acoustic path thickness in meters when penetrated.
    pub thickness_m: f32,
    /// Optional `PROC1` smooth-minimum blend radius $r$ (`evaluate_smooth_minimum_sdf`).
    pub smooth_blend_radius: f32,
}

impl AcousticObstacle {
    /// Creates an obstacle directly from a [`HolographicNodeGpu`] SDF proxy lens so visual
    /// raymarching and acoustic occlusion share the exact same SDF primitive parameters.
    #[inline]
    #[cfg(feature = "gpu-runtime")]
    pub fn from_holographic_node(node: &HolographicNodeGpu, material: AcousticMaterial) -> Self {
        let center = [
            node.position_scale[0],
            node.position_scale[1],
            node.position_scale[2],
        ];
        let scale = node.position_scale[3].max(0.05);
        let he = [
            node.half_extents_primitive[0] * scale,
            node.half_extents_primitive[1] * scale,
            node.half_extents_primitive[2] * scale,
        ];
        let prim_id = node.half_extents_primitive[3] as u32;
        let shape = match prim_id {
            1 => AcousticObstacleShape::AnalyticalRoundedBox {
                center,
                half_extents: he,
                round_radius: 0.1 * scale,
            },
            2 => AcousticObstacleShape::AnalyticalCapsule {
                a: [center[0], center[1] - he[1], center[2]],
                b: [center[0], center[1] + he[1], center[2]],
                radius: he[0].max(he[2]),
            },
            _ => AcousticObstacleShape::AnalyticalSphere {
                center,
                radius: he[0].max(he[1]).max(he[2]),
            },
        };
        Self {
            shape,
            material,
            thickness_m: (2.0f32 * he[0].min(he[1]).min(he[2])).max(0.25f32),
            smooth_blend_radius: 0.25,
        }
    }
}

/// Result of zero-heap acoustic occlusion evaluation between an emitter and the listener.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticOcclusionResult {
    /// Direct-path amplitude multiplier in `[0.0, 1.0]` (`1.0` = unoccluded).
    pub direct_gain: f32,
    /// Frequency-dependent low-pass filter cutoff in Hz (`20000.0` = unoccluded).
    pub lowpass_cutoff_hz: f32,
    /// Precomputed RBJ low-pass biquad filter coefficients at 48 kHz.
    pub lowpass_biquad: BiquadCoeffs,
    /// Phase diffraction delay $\Delta\phi$ in radians induced by obstacle grazing / penetration.
    pub diffraction_phase_rad: f32,
    /// 6-tap early reflection arrival delays in seconds relative to direct sound.
    pub early_reflection_delays_s: [f32; EARLY_REFLECTION_TAPS],
    /// 6-tap early reflection linear gains.
    pub early_reflection_gains: [f32; EARLY_REFLECTION_TAPS],
    /// Local canyon/canopy enclosure metric in `[0.0, 1.0]` driving late Freeverb decay.
    pub enclosure_density: f32,
    /// Number of obstacles intersecting the direct acoustic ray.
    pub hit_count: u32,
}

impl Default for AcousticOcclusionResult {
    fn default() -> Self {
        Self {
            direct_gain: 1.0,
            lowpass_cutoff_hz: 18_000.0,
            lowpass_biquad: calc_rbj_lowpass_coeffs(48_000.0, 18_000.0, 0.7071),
            diffraction_phase_rad: 0.0,
            early_reflection_delays_s: [0.008, 0.014, 0.021, 0.029, 0.038, 0.049],
            early_reflection_gains: [0.0; EARLY_REFLECTION_TAPS],
            enclosure_density: 0.1,
            hit_count: 0,
        }
    }
}

/// Computes normalized RBJ cookbook 2nd-order low-pass [`BiquadCoeffs`] in zero heap.
#[inline]
pub fn calc_rbj_lowpass_coeffs(sample_rate: f32, cutoff_hz: f32, q: f32) -> BiquadCoeffs {
    let sr = sample_rate.max(8_000.0);
    let fc = cutoff_hz.clamp(80.0, sr * 0.45);
    let w0 = TAU * fc / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q.max(0.1));

    let b0 = (1.0 - cos_w0) * 0.5;
    let b1 = 1.0 - cos_w0;
    let b2 = (1.0 - cos_w0) * 0.5;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    let inv_a0 = 1.0 / a0.max(1e-6);
    BiquadCoeffs {
        b0: b0 * inv_a0,
        b1: b1 * inv_a0,
        b2: b2 * inv_a0,
        a1: a1 * inv_a0,
        a2: a2 * inv_a0,
    }
}

/// Evaluates zero-heap acoustic occlusion, Fresnel SDF grazing diffraction, low-pass biquad cutoff,
/// and 6-tap early reflection geometry delays between `listener_pos` and `emitter_pos`.
pub fn evaluate_acoustic_occlusion_zero_heap(
    listener_pos: [f32; 3],
    emitter_pos: [f32; 3],
    obstacles: &[AcousticObstacle],
    out_occlusion: &mut AcousticOcclusionResult,
) {
    let delta = sub3(listener_pos, emitter_pos);
    let direct_dist = length3(delta);
    if direct_dist < 1e-4 {
        *out_occlusion = AcousticOcclusionResult::default();
        return;
    }
    let ray_dir = [
        delta[0] / direct_dist,
        delta[1] / direct_dist,
        delta[2] / direct_dist,
    ];

    let mut total_transmission_gain = 1.0_f32;
    let mut cutoff_hz = 18_000.0_f32;
    let mut diffraction_phase = 0.0_f32;
    let mut hit_count = 0u32;
    let mut enclosure_accum = 0.0_f32;

    // Reset 6-tap early reflections (ground bounce + 5 obstacle/canyon reflections)
    let ground_bounce_extra_m =
        ((emitter_pos[1].max(0.5) + listener_pos[1].max(0.5)).hypot(direct_dist) - direct_dist)
            .max(0.5);
    out_occlusion.early_reflection_delays_s = [
        (ground_bounce_extra_m / SPEED_OF_SOUND_MPS).clamp(0.002, 0.080),
        0.013,
        0.019,
        0.027,
        0.035,
        0.046,
    ];
    out_occlusion.early_reflection_gains = [0.12, 0.0, 0.0, 0.0, 0.0, 0.0];

    let mut reflection_slot = 1usize;
    let mut combined_sdf_min = f32::MAX;

    for obs in obstacles {
        let (hit, penetration_m, min_sdf_dist) = intersect_or_graze_obstacle(
            &obs.shape,
            emitter_pos,
            ray_dir,
            direct_dist,
            obs.thickness_m,
        );

        combined_sdf_min =
            evaluate_smooth_minimum_sdf(combined_sdf_min, min_sdf_dist, obs.smooth_blend_radius);

        if hit {
            hit_count += 1;
            let eff_thickness = penetration_m.max(0.15);
            let absorption = obs.material.transmission_absorption_per_m();
            let trans = (-absorption * eff_thickness).exp().clamp(0.03, 1.0);
            total_transmission_gain *= trans;
            cutoff_hz *= obs.material.cutoff_attenuation_factor();
            // Detour phase delay around/through obstacle
            let detour_m = (eff_thickness * 0.35).min(4.0);
            diffraction_phase += TAU * 440.0 * (detour_m / SPEED_OF_SOUND_MPS);
        } else if min_sdf_dist < 1.5 {
            // Grazing Fresnel diffraction zone near analytical SDF surface
            let graze_t = (1.0 - (min_sdf_dist.max(0.0) / 1.5)).clamp(0.0, 1.0);
            total_transmission_gain *= 1.0 - 0.18 * graze_t;
            cutoff_hz *= 1.0 - 0.25 * graze_t;
            diffraction_phase += 0.45 * graze_t;
        }

        // Evaluate 1st-order specular reflection tap via obstacle centroid
        let c = obs.shape.centroid();
        let d_emit_c = length3(sub3(c, emitter_pos));
        let d_c_list = length3(sub3(listener_pos, c));
        let path_diff_m = (d_emit_c + d_c_list - direct_dist).max(0.35);
        if path_diff_m < 45.0 {
            enclosure_accum += obs.material.reflection_albedo() / (1.0 + 0.08 * path_diff_m);
            if reflection_slot < EARLY_REFLECTION_TAPS {
                let delay_s = (path_diff_m / SPEED_OF_SOUND_MPS).clamp(0.002, 0.085);
                let refl_gain = (obs.material.reflection_albedo()
                    / (1.0 + 0.12 * (d_emit_c + d_c_list)))
                    .clamp(0.01, 0.45);
                out_occlusion.early_reflection_delays_s[reflection_slot] = delay_s;
                out_occlusion.early_reflection_gains[reflection_slot] = refl_gain;
                reflection_slot += 1;
            }
        }
    }

    // If multiple smooth-filleted SDFs form a narrow aperture (`combined_sdf_min < 0.0`),
    // apply smooth fillet occlusion continuity
    if combined_sdf_min < 0.0 && hit_count == 0 {
        total_transmission_gain *= 0.65;
        cutoff_hz *= 0.55;
    }

    let final_cutoff = cutoff_hz.clamp(220.0, 18_000.0);
    out_occlusion.direct_gain = total_transmission_gain.clamp(0.02, 1.0);
    out_occlusion.lowpass_cutoff_hz = final_cutoff;
    out_occlusion.lowpass_biquad = calc_rbj_lowpass_coeffs(48_000.0, final_cutoff, 0.7071);
    out_occlusion.diffraction_phase_rad = diffraction_phase;
    out_occlusion.enclosure_density = (0.12 + enclosure_accum * 0.25).clamp(0.05, 0.98);
    out_occlusion.hit_count = hit_count;
}

/// Tests ray segment `origin + t * dir` (`t in [0, max_dist]`) against an [`AcousticObstacleShape`].
/// Returns `(did_hit, penetration_thickness_m, closest_signed_distance_m)`.
fn intersect_or_graze_obstacle(
    shape: &AcousticObstacleShape,
    origin: [f32; 3],
    dir: [f32; 3],
    max_dist: f32,
    default_thickness: f32,
) -> (bool, f32, f32) {
    match *shape {
        AcousticObstacleShape::AnalyticalSphere { center, radius } => {
            let oc = sub3(origin, center);
            let b = dot3(oc, dir);
            let c = dot3(oc, oc) - radius * radius;
            let t_closest = (-b).clamp(0.0, max_dist);
            let closest_pt = [
                origin[0] + dir[0] * t_closest,
                origin[1] + dir[1] * t_closest,
                origin[2] + dir[2] * t_closest,
            ];
            let min_sdf = length3(sub3(closest_pt, center)) - radius;
            let disc = b * b - c;
            if disc <= 0.0 {
                return (false, 0.0, min_sdf);
            }
            let sqrt_d = disc.sqrt();
            let t0 = (-b - sqrt_d).max(0.0);
            let t1 = (-b + sqrt_d).min(max_dist);
            if t1 > t0 {
                (true, (t1 - t0).max(0.1), min_sdf)
            } else {
                (false, 0.0, min_sdf)
            }
        }
        AcousticObstacleShape::AnalyticalCapsule { a, b, radius }
        | AcousticObstacleShape::AnalyticalRoundedBox {
            center: a,
            half_extents: b,
            round_radius: radius,
        } => {
            // Fixed-step zero-heap sphere-march / sample along the segment [0, max_dist]
            let _ = (a, b, radius);
            const STEPS: usize = 16;
            let dt = max_dist / (STEPS as f32);
            let mut min_sdf = f32::MAX;
            let mut inside_steps = 0usize;
            for s in 0..=STEPS {
                let t = (s as f32) * dt;
                let p = [
                    origin[0] + dir[0] * t,
                    origin[1] + dir[1] * t,
                    origin[2] + dir[2] * t,
                ];
                let d = shape.signed_distance(p);
                if d < min_sdf {
                    min_sdf = d;
                }
                if d <= 0.0 {
                    inside_steps += 1;
                }
            }
            if inside_steps > 0 {
                let chord = (inside_steps as f32 * dt).max(default_thickness * 0.5);
                (true, chord, min_sdf)
            } else {
                (false, 0.0, min_sdf)
            }
        }
        AcousticObstacleShape::Aabb { min, max } => {
            let mut tmin = 0.0_f32;
            let mut tmax = max_dist;
            for axis in 0..3 {
                if dir[axis].abs() < 1e-6 {
                    if origin[axis] < min[axis] || origin[axis] > max[axis] {
                        return (false, 0.0, 2.0);
                    }
                } else {
                    let inv_d = 1.0 / dir[axis];
                    let mut t1 = (min[axis] - origin[axis]) * inv_d;
                    let mut t2 = (max[axis] - origin[axis]) * inv_d;
                    if t1 > t2 {
                        core::mem::swap(&mut t1, &mut t2);
                    }
                    tmin = tmin.max(t1);
                    tmax = tmax.min(t2);
                    if tmin > tmax {
                        return (false, 0.0, 2.0);
                    }
                }
            }
            let chord = (tmax - tmin).max(default_thickness);
            (true, chord, -0.5 * chord)
        }
        AcousticObstacleShape::Triangle { v0, v1, v2 } => {
            // Möller–Trumbore ray-triangle intersection
            let edge1 = sub3(v1, v0);
            let edge2 = sub3(v2, v0);
            let h = cross3(dir, edge2);
            let a = dot3(edge1, h);
            if a.abs() < 1e-6 {
                return (false, 0.0, 2.0);
            }
            let f = 1.0 / a;
            let s = sub3(origin, v0);
            let u = f * dot3(s, h);
            if !(0.0..=1.0).contains(&u) {
                return (false, 0.0, 2.0);
            }
            let q = cross3(s, edge1);
            let v = f * dot3(dir, q);
            if v < 0.0 || u + v > 1.0 {
                return (false, 0.0, 2.0);
            }
            let t = f * dot3(edge2, q);
            if t > 1e-4 && t < max_dist {
                (true, default_thickness.max(0.25), -0.1)
            } else {
                (false, 0.0, 2.0)
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Zero-Heap Multi-Bus Effect Graph
// ─────────────────────────────────────────────────────────────────────────────

/// Bus 1: Direct Binaural Bus — per-emitter DF2T occlusion low-pass filter + zero-heap
/// HRIR FIR convolution (ITD fractional delay + ILD + contralateral head-shadow).
#[derive(Debug, Clone)]
pub struct DirectBinauralBus {
    /// Per-emitter DF2T biquad state registers.
    pub biquad_states: [Df2tState; MAX_SCENE_AUDIO_EMITTERS],
    /// Per-emitter circular mono history buffer for zero-heap HRIR FIR convolution.
    pub fir_history: [[f32; ZERO_HEAP_HRIR_TAPS]; MAX_SCENE_AUDIO_EMITTERS],
    /// Per-emitter write cursor into `fir_history`.
    pub fir_cursor: [u8; MAX_SCENE_AUDIO_EMITTERS],
}

impl Default for DirectBinauralBus {
    fn default() -> Self {
        Self {
            biquad_states: [Df2tState::default(); MAX_SCENE_AUDIO_EMITTERS],
            fir_history: [[0.0; ZERO_HEAP_HRIR_TAPS]; MAX_SCENE_AUDIO_EMITTERS],
            fir_cursor: [0; MAX_SCENE_AUDIO_EMITTERS],
        }
    }
}

impl DirectBinauralBus {
    /// Resets all filter and FIR delay states.
    pub fn reset(&mut self) {
        self.biquad_states = [Df2tState::default(); MAX_SCENE_AUDIO_EMITTERS];
        self.fir_history = [[0.0; ZERO_HEAP_HRIR_TAPS]; MAX_SCENE_AUDIO_EMITTERS];
        self.fir_cursor = [0; MAX_SCENE_AUDIO_EMITTERS];
    }

    /// Filters a single mono emitter sample through its occlusion DF2T biquad and convolves
    /// it with the synthesized left/right HRIR taps (`BinauralGains` from `hrtf.rs`).
    #[inline]
    pub fn process_emitter_sample(
        &mut self,
        emitter_idx: usize,
        mono_input: f32,
        coeffs: &BiquadCoeffs,
        gains: &BinauralGains,
        sample_rate: f32,
    ) -> (f32, f32) {
        let idx = emitter_idx % MAX_SCENE_AUDIO_EMITTERS;
        let st = &mut self.biquad_states[idx];
        let x = mono_input + DENORMAL_DC_OFFSET;
        let filtered = coeffs.b0 * x + st.s1;
        st.s1 = coeffs.b1 * x - coeffs.a1 * filtered + st.s2;
        st.s2 = coeffs.b2 * x - coeffs.a2 * filtered;

        // Write filtered sample into circular FIR history
        let cur = (self.fir_cursor[idx] as usize) % ZERO_HEAP_HRIR_TAPS;
        self.fir_history[idx][cur] = filtered;
        self.fir_cursor[idx] = ((cur + 1) % ZERO_HEAP_HRIR_TAPS) as u8;

        // Fractional ITD sample delay for contralateral ear (clamped to ZERO_HEAP_HRIR_TAPS - 2)
        let delay_samples = (gains.itd_seconds.abs() * sample_rate.max(8_000.0))
            .clamp(0.0, (ZERO_HEAP_HRIR_TAPS - 2) as f32);
        let d_int = delay_samples.floor() as usize;
        let d_frac = delay_samples - d_int as f32;

        let idx0 = (cur + ZERO_HEAP_HRIR_TAPS - d_int) % ZERO_HEAP_HRIR_TAPS;
        let idx1 = (cur + ZERO_HEAP_HRIR_TAPS - (d_int + 1)) % ZERO_HEAP_HRIR_TAPS;
        let delayed =
            self.fir_history[idx][idx0] * (1.0 - d_frac) + self.fir_history[idx][idx1] * d_frac;
        // Gentle contralateral 2-tap head-shadow smoothing
        let shadow_delayed = 0.65 * delayed + 0.35 * self.fir_history[idx][idx1];

        if gains.itd_seconds <= 0.0 {
            // Source to left: left ear immediate, right ear delayed + head-shadowed
            (filtered * gains.gain_l, shadow_delayed * gains.gain_r)
        } else {
            // Source to right: right ear immediate, left ear delayed + head-shadowed
            (shadow_delayed * gains.gain_l, filtered * gains.gain_r)
        }
    }
}

/// Bus 2: 6-Tap Early Reflections Bus — stereo multi-tap delay line driven by scene geometry.
#[derive(Debug, Clone)]
pub struct EarlyReflectionsBus {
    buffer_l: [f32; 4096],
    buffer_r: [f32; 4096],
    write_pos: usize,
}

impl Default for EarlyReflectionsBus {
    fn default() -> Self {
        Self {
            buffer_l: [0.0; 4096],
            buffer_r: [0.0; 4096],
            write_pos: 0,
        }
    }
}

impl EarlyReflectionsBus {
    /// Clears early reflection delay lines.
    pub fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_pos = 0;
    }

    /// Advances one stereo sample through the 6-tap early reflection geometry delay line.
    #[inline]
    pub fn tick(
        &mut self,
        in_l: f32,
        in_r: f32,
        delays_s: &[f32; EARLY_REFLECTION_TAPS],
        gains: &[f32; EARLY_REFLECTION_TAPS],
        sample_rate: f32,
    ) -> (f32, f32) {
        let cap = self.buffer_l.len();
        self.buffer_l[self.write_pos] = in_l + DENORMAL_DC_OFFSET;
        self.buffer_r[self.write_pos] = in_r + DENORMAL_DC_OFFSET;

        let mut out_l = 0.0_f32;
        let mut out_r = 0.0_f32;
        for tap in 0..EARLY_REFLECTION_TAPS {
            let g = gains[tap];
            if g <= 1e-5 {
                continue;
            }
            let d_samples = ((delays_s[tap] * sample_rate) as usize).clamp(1, cap - 1);
            let read_pos = (self.write_pos + cap - d_samples) % cap;
            // Alternate cross-ear lateral reflections for spacious canyon image
            if tap & 1 == 0 {
                out_l += self.buffer_l[read_pos] * g;
                out_r += self.buffer_r[read_pos] * (g * 0.85);
            } else {
                out_l += self.buffer_r[read_pos] * (g * 0.85);
                out_r += self.buffer_l[read_pos] * g;
            }
        }

        self.write_pos = (self.write_pos + 1) % cap;
        (out_l, out_r)
    }
}

/// Zero-heap Schroeder/Moorer low-pass feedback comb filter for Freeverb.
#[derive(Debug, Clone)]
pub struct ZeroHeapCombFilter<const N: usize> {
    buffer: [f32; N],
    idx: usize,
    filter_store: f32,
}

impl<const N: usize> Default for ZeroHeapCombFilter<N> {
    fn default() -> Self {
        Self {
            buffer: [0.0; N],
            idx: 0,
            filter_store: 0.0,
        }
    }
}

impl<const N: usize> ZeroHeapCombFilter<N> {
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filter_store = 0.0;
    }

    #[inline]
    pub fn process(&mut self, input: f32, feedback: f32, damp1: f32, damp2: f32) -> f32 {
        let output = self.buffer[self.idx];
        self.filter_store = output * damp2 + self.filter_store * damp1 + DENORMAL_DC_OFFSET;
        self.buffer[self.idx] = input + self.filter_store * feedback;
        self.idx = (self.idx + 1) % N;
        output
    }
}

/// Zero-heap Schroeder all-pass diffuser stage for Freeverb.
#[derive(Debug, Clone)]
pub struct ZeroHeapAllpassFilter<const N: usize> {
    buffer: [f32; N],
    idx: usize,
}

impl<const N: usize> Default for ZeroHeapAllpassFilter<N> {
    fn default() -> Self {
        Self {
            buffer: [0.0; N],
            idx: 0,
        }
    }
}

impl<const N: usize> ZeroHeapAllpassFilter<N> {
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.idx = 0;
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.idx];
        let output = -input + buf_out;
        self.buffer[self.idx] = input + buf_out * 0.5 + DENORMAL_DC_OFFSET;
        self.idx = (self.idx + 1) % N;
        output
    }
}

/// Bus 3: Zero-Heap Stereo Freeverb Bus (4 parallel comb filters + 2 series all-pass filters per ear)
/// driven by local canyon/canopy enclosure metrics.
#[derive(Debug, Clone, Default)]
pub struct LateReverbFreeverbBus {
    comb_l0: ZeroHeapCombFilter<1116>,
    comb_l1: ZeroHeapCombFilter<1188>,
    comb_l2: ZeroHeapCombFilter<1277>,
    comb_l3: ZeroHeapCombFilter<1356>,
    comb_r0: ZeroHeapCombFilter<1139>,
    comb_r1: ZeroHeapCombFilter<1211>,
    comb_r2: ZeroHeapCombFilter<1300>,
    comb_r3: ZeroHeapCombFilter<1379>,
    allpass_l0: ZeroHeapAllpassFilter<556>,
    allpass_l1: ZeroHeapAllpassFilter<441>,
    allpass_r0: ZeroHeapAllpassFilter<579>,
    allpass_r1: ZeroHeapAllpassFilter<464>,
    pub room_size: f32,
    pub damping: f32,
    pub wet_level: f32,
}

impl LateReverbFreeverbBus {
    /// Creates a new zero-heap stereo Freeverb bus.
    pub fn new(room_size: f32, damping: f32, wet_level: f32) -> Self {
        Self {
            room_size: room_size.clamp(0.1, 0.98),
            damping: damping.clamp(0.05, 0.95),
            wet_level: wet_level.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Updates reverb enclosure parameters from scene canyon/canopy enclosure density.
    #[inline]
    pub fn configure_from_enclosure(&mut self, enclosure_density: f32, manifold_w: f32) {
        let enc = enclosure_density.clamp(0.0, 1.0);
        self.room_size = (0.45 + enc * 0.48).clamp(0.2, 0.95);
        self.damping = (0.25 + manifold_w.clamp(0.0, 4.0) * 0.12).clamp(0.15, 0.85);
        self.wet_level = (0.08 + enc * 0.35).clamp(0.05, 0.55);
    }

    /// Resets all comb and all-pass delay buffers.
    pub fn reset(&mut self) {
        self.comb_l0.reset();
        self.comb_l1.reset();
        self.comb_l2.reset();
        self.comb_l3.reset();
        self.comb_r0.reset();
        self.comb_r1.reset();
        self.comb_r2.reset();
        self.comb_r3.reset();
        self.allpass_l0.reset();
        self.allpass_l1.reset();
        self.allpass_r0.reset();
        self.allpass_r1.reset();
    }

    /// Processes a stereo input pair and returns the wet stereo reverb output.
    #[inline]
    pub fn tick(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let input = (in_l + in_r) * 0.015;
        let feedback = 0.70 + self.room_size * 0.28;
        let damp1 = self.damping * 0.4;
        let damp2 = 1.0 - damp1;

        let mut out_l = self.comb_l0.process(input, feedback, damp1, damp2)
            + self.comb_l1.process(input, feedback, damp1, damp2)
            + self.comb_l2.process(input, feedback, damp1, damp2)
            + self.comb_l3.process(input, feedback, damp1, damp2);

        let mut out_r = self.comb_r0.process(input, feedback, damp1, damp2)
            + self.comb_r1.process(input, feedback, damp1, damp2)
            + self.comb_r2.process(input, feedback, damp1, damp2)
            + self.comb_r3.process(input, feedback, damp1, damp2);

        out_l = self.allpass_l0.process(out_l);
        out_l = self.allpass_l1.process(out_l);
        out_r = self.allpass_r0.process(out_r);
        out_r = self.allpass_r1.process(out_r);

        (out_l * self.wet_level, out_r * self.wet_level)
    }
}

/// Bus 4: Fixed-capacity ring buffer of 48-byte [`HolosonicSonicToken48`] telemetry tokens.
#[derive(Debug, Clone)]
pub struct HolosonicTelemetryBus {
    pub tokens: [HolosonicSonicToken48; HOLOSONIC_TELEMETRY_RING_CAP],
    pub write_idx: usize,
    pub read_idx: usize,
    pub count: usize,
}

impl Default for HolosonicTelemetryBus {
    fn default() -> Self {
        Self {
            tokens: [HolosonicSonicToken48::default(); HOLOSONIC_TELEMETRY_RING_CAP],
            write_idx: 0,
            read_idx: 0,
            count: 0,
        }
    }
}

impl HolosonicTelemetryBus {
    /// Pushes a 48-byte [`HolosonicSonicToken48`] into the local ring and forwards the packed
    /// 64-bit [`SonicToken`] to the process-global U3 [`SonicTokenRing`].
    #[inline]
    pub fn push(&mut self, token48: HolosonicSonicToken48, packed64: SonicToken) {
        self.tokens[self.write_idx] = token48;
        self.write_idx = (self.write_idx + 1) % HOLOSONIC_TELEMETRY_RING_CAP;
        if self.count < HOLOSONIC_TELEMETRY_RING_CAP {
            self.count += 1;
        } else {
            self.read_idx = (self.read_idx + 1) % HOLOSONIC_TELEMETRY_RING_CAP;
        }
        let _ = push_sonic_token(packed64);
    }

    /// Pops the oldest 48-byte [`HolosonicSonicToken48`] from the telemetry ring.
    #[inline]
    pub fn pop(&mut self) -> Option<HolosonicSonicToken48> {
        if self.count == 0 {
            return None;
        }
        let tok = self.tokens[self.read_idx];
        self.read_idx = (self.read_idx + 1) % HOLOSONIC_TELEMETRY_RING_CAP;
        self.count -= 1;
        Some(tok)
    }
}

/// Complete Scene-Synchronized 3D Holosonic Wavefield & Multi-Bus HRTF Audio Graph.
pub struct SceneAudioBusGraph {
    /// Observer listener pose synchronized from camera PGA `Motor`.
    pub listener: AudioListenerPose,
    /// Primary Holosonic wavefield engine (preserved and synchronized with `emitters`).
    pub wavefield: HolosonicWavefield,
    /// Up to 64 active scene audio emitters (`HolosonicNode`s + point/ambient sources).
    pub emitters: [HolosonicNode; MAX_SCENE_AUDIO_EMITTERS],
    /// Cached per-emitter acoustic occlusion state.
    pub occlusions: [AcousticOcclusionResult; MAX_SCENE_AUDIO_EMITTERS],
    /// Number of active emitters in `emitters`.
    pub emitter_count: usize,
    /// Bus 1: Direct binaural FIR + DF2T biquad occlusion bus.
    pub direct_binaural_bus: DirectBinauralBus,
    /// Bus 2: 6-tap early reflections bus.
    pub early_reflections_bus: EarlyReflectionsBus,
    /// Bus 3: Stereo Freeverb late reverb bus.
    pub late_reverb_bus: LateReverbFreeverbBus,
    /// Bus 4: 48-byte `SonicToken` telemetry ring buffer.
    pub telemetry_bus: HolosonicTelemetryBus,
}

impl Default for SceneAudioBusGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneAudioBusGraph {
    /// Creates an empty zero-heap scene audio bus graph.
    pub fn new() -> Self {
        Self {
            listener: AudioListenerPose::default(),
            wavefield: HolosonicWavefield::new(),
            emitters: [HolosonicNode::default(); MAX_SCENE_AUDIO_EMITTERS],
            occlusions: [AcousticOcclusionResult::default(); MAX_SCENE_AUDIO_EMITTERS],
            emitter_count: 0,
            direct_binaural_bus: DirectBinauralBus::default(),
            early_reflections_bus: EarlyReflectionsBus::default(),
            late_reverb_bus: LateReverbFreeverbBus::new(0.65, 0.35, 0.22),
            telemetry_bus: HolosonicTelemetryBus::default(),
        }
    }

    /// Synchronizes the listener pose from the observer camera's PGA [`Motor`].
    #[inline]
    pub fn sync_listener_from_motor(
        &mut self,
        camera_motor: Motor,
        velocity: [f32; 3],
        manifold_w: f32,
    ) {
        self.listener = AudioListenerPose::from_pga_motor(camera_motor, velocity, manifold_w);
    }

    /// Clears all scene emitters for a new scene sync pass.
    #[inline]
    pub fn clear_emitters(&mut self) {
        self.emitter_count = 0;
        self.wavefield.clear();
    }

    /// Registers a [`HolosonicNode`] emitter into the scene graph and syncs [`HolosonicWavefield`].
    #[inline]
    pub fn push_emitter(&mut self, node: HolosonicNode) -> bool {
        if self.emitter_count >= MAX_SCENE_AUDIO_EMITTERS {
            return false;
        }
        self.emitters[self.emitter_count] = node;
        self.emitter_count += 1;
        let _ = self.wavefield.push(node.emitter);
        true
    }

    /// Evaluates shared SDF + BVH acoustic occlusion for all active emitters in zero heap.
    pub fn update_occlusion_zero_heap(&mut self, obstacles: &[AcousticObstacle]) {
        let mut mean_enclosure = 0.0_f32;
        for i in 0..self.emitter_count {
            let emitter_pos = self.emitters[i].emitter.position;
            evaluate_acoustic_occlusion_zero_heap(
                self.listener.position,
                emitter_pos,
                obstacles,
                &mut self.occlusions[i],
            );
            mean_enclosure += self.occlusions[i].enclosure_density;
        }
        if self.emitter_count > 0 {
            mean_enclosure /= self.emitter_count as f32;
            self.late_reverb_bus
                .configure_from_enclosure(mean_enclosure, self.listener.manifold_w);
        }
    }

    /// Synthesizes and mixes a stereo audio block (`out_l`, `out_r`) across all 4 buses in zero heap:
    /// 1. Continuous Holosonic wave superposition ($P_{\text{total}} = \sum p_i(t, \mathbf{x}_L)$)
    ///    with epistemic certainty $q$ harmonic purity modulation and Doppler pitch shift.
    /// 2. Per-emitter DF2T biquad occlusion low-pass filtering + `hrtf.rs` binaural FIR convolution.
    /// 3. 6-tap early reflections + stereo Freeverb late reverb tail.
    /// 4. Emits 48-byte [`HolosonicSonicToken48`] telemetry tokens into [`HolosonicTelemetryBus`].
    pub fn render_quantum_zero_heap(
        &mut self,
        start_time_s: f32,
        sample_rate: f32,
        obstacles: &[AcousticObstacle],
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) -> HolosonicPointSample {
        self.update_occlusion_zero_heap(obstacles);

        let n = out_l.len().min(out_r.len());
        let sr = sample_rate.max(8_000.0);
        let dt = 1.0 / sr;

        // Precompute per-emitter spatial binaural gains, Doppler shift, and distance attenuation
        let mut emitter_gains = [BinauralGains::default(); MAX_SCENE_AUDIO_EMITTERS];
        let mut emitter_atten = [0.0_f32; MAX_SCENE_AUDIO_EMITTERS];
        let mut emitter_omega = [0.0_f32; MAX_SCENE_AUDIO_EMITTERS];
        let mut emitter_phase0 = [0.0_f32; MAX_SCENE_AUDIO_EMITTERS];
        let mut active_count = 0usize;

        for idx in 0..self.emitter_count {
            let node = &self.emitters[idx];
            // Forbidden/Escrow deontic permit silences direct emission
            if node.deontic_permit >= 2 || node.emitter.amplitude <= 0.0 {
                continue;
            }
            let e = &node.emitter;
            let rel = sub3(e.position, self.listener.position);
            let dist = length3(rel);
            if !node.is_ambient && (dist > e.coherence_radius || dist < 0.001) {
                continue;
            }
            active_count += 1;

            let falloff = if node.is_ambient {
                1.0
            } else {
                (1.0 - (dist / e.coherence_radius.max(0.1))).clamp(0.0, 1.0)
            };
            let dist_atten = if node.is_ambient {
                e.amplitude
            } else {
                (1.0 / (1.0 + dist * 0.45)) * falloff * e.amplitude
            };
            emitter_atten[idx] = dist_atten * self.occlusions[idx].direct_gain;

            // Doppler shift from relative radial velocity
            let dir = if dist > 1e-4 {
                [rel[0] / dist, rel[1] / dist, rel[2] / dist]
            } else {
                [0.0, 0.0, -1.0]
            };
            let v_emit = dot3(node.velocity, dir);
            let v_list = dot3(self.listener.velocity, dir);
            let doppler = ((SPEED_OF_SOUND_MPS - v_list) / (SPEED_OF_SOUND_MPS + v_emit).max(20.0))
                .clamp(0.5, 2.0);

            let omega = TAU * e.base_freq_hz * doppler;
            let wave_number = omega / SPEED_OF_SOUND_MPS;
            emitter_omega[idx] = omega;
            emitter_phase0[idx] =
                e.phase_rad - wave_number * dist - self.occlusions[idx].diffraction_phase_rad;
            emitter_gains[idx] = if node.is_ambient {
                BinauralGains {
                    gain_l: 0.707,
                    gain_r: 0.707,
                    itd_seconds: 0.0,
                    azimuth_rad: 0.0,
                    elevation_rad: 0.0,
                    distance: dist.max(1.0),
                }
            } else {
                binaural_from_position(rel, self.listener.yaw_rad)
            };
        }

        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        let mut raw_energy_sum = 0.0_f32;

        // Shared early-reflection tap configuration from emitter 0 (or default)
        let er_delays = if self.emitter_count > 0 {
            self.occlusions[0].early_reflection_delays_s
        } else {
            [0.008, 0.014, 0.021, 0.029, 0.038, 0.049]
        };
        let er_gains = if self.emitter_count > 0 {
            self.occlusions[0].early_reflection_gains
        } else {
            [0.0; EARLY_REFLECTION_TAPS]
        };

        for sample_i in 0..n {
            let t = start_time_s + (sample_i as f32) * dt;
            let mut direct_sum_l = 0.0_f32;
            let mut direct_sum_r = 0.0_f32;

            for idx in 0..self.emitter_count {
                let atten = emitter_atten[idx];
                if atten <= 1e-6 {
                    continue;
                }
                let e = &self.emitters[idx].emitter;
                let phase = emitter_phase0[idx] + emitter_omega[idx] * t;
                let q = e.epistemic_q.clamp(0.0, 1.0);

                // Epistemic certainty q modulates harmonic purity vs FM/overtone dissonance
                let fundamental = phase.sin();
                let overtone_2 = (phase * 2.0 + 0.5).sin() * (1.0 - q) * 0.35;
                let overtone_3 = (phase * 3.01 + 1.1).sin() * (1.0 - q) * 0.18;
                let mono_wave = (fundamental + overtone_2 + overtone_3) * atten;

                let (bin_l, bin_r) = self.direct_binaural_bus.process_emitter_sample(
                    idx,
                    mono_wave,
                    &self.occlusions[idx].lowpass_biquad,
                    &emitter_gains[idx],
                    sr,
                );
                direct_sum_l += bin_l;
                direct_sum_r += bin_r;
                if sample_i == 0 {
                    raw_energy_sum += atten;
                }
            }

            let (er_l, er_r) = self.early_reflections_bus.tick(
                direct_sum_l,
                direct_sum_r,
                &er_delays,
                &er_gains,
                sr,
            );
            let (rev_l, rev_r) = self
                .late_reverb_bus
                .tick(direct_sum_l + er_l, direct_sum_r + er_r);

            let total_l = (direct_sum_l + er_l + rev_l).clamp(-2.0, 2.0);
            let total_r = (direct_sum_r + er_r + rev_r).clamp(-2.0, 2.0);
            out_l[sample_i] = total_l;
            out_r[sample_i] = total_r;

            if total_l.abs() > peak_l.abs() {
                peak_l = total_l;
            }
            if total_r.abs() > peak_r.abs() {
                peak_r = total_r;
            }
        }

        let net_amp = (peak_l * peak_l + peak_r * peak_r).sqrt();
        let interference_gain = if raw_energy_sum > 0.001 {
            net_amp / raw_energy_sum
        } else {
            1.0
        };

        let summary = HolosonicPointSample {
            pressure_l: peak_l,
            pressure_r: peak_r,
            interference_gain,
            active_emitter_count: active_count,
        };

        // Emit 48-byte SonicToken telemetry for active emitters
        for idx in 0..self.emitter_count {
            if emitter_atten[idx] > 1e-4 {
                let node = &self.emitters[idx];
                let (tok48, packed64) = self.wavefield.emit_sonic_token(
                    idx.min(self.wavefield.len().saturating_sub(1)),
                    node.origin_quin_hash,
                    node.node_id,
                    node.deontic_permit,
                    self.occlusions[idx].direct_gain,
                    summary,
                    self.listener.position,
                    start_time_s,
                );
                let _ = (SonicEventType::Parametric, SONIC_MAGIC, PI);
                self.telemetry_bus.push(tok48, packed64);
            }
        }

        summary
    }
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn length3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

#[inline]
fn normalize3(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let l = length3(v);
    if l > 1e-6 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "gpu-runtime")]
    use crate::render::gpu::HolographicPrimitive;
    use crate::render::pga::motor_translate;

    #[test]
    fn listener_pose_from_pga_motor_extracts_position_and_basis() {
        let motor = motor_translate([4.0, 2.5, -7.0]);
        let pose = AudioListenerPose::from_pga_motor(motor, [0.0, 0.0, -2.0], 1.0);
        assert!((pose.position[0] - 4.0).abs() < 1e-4);
        assert!((pose.position[1] - 2.5).abs() < 1e-4);
        assert!((pose.position[2] + 7.0).abs() < 1e-4);
        assert!((pose.forward[2] + 1.0).abs() < 1e-4);
        assert!((pose.up[1] - 1.0).abs() < 1e-4);
        assert!((pose.right[0] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn analytical_sdf_and_bvh_occlusion_attenuates_direct_gain_and_cutoff() {
        let listener = [0.0, 1.5, 0.0];
        let emitter = [0.0, 1.5, -20.0];

        let mut unoccluded = AcousticOcclusionResult::default();
        evaluate_acoustic_occlusion_zero_heap(listener, emitter, &[], &mut unoccluded);
        assert!((unoccluded.direct_gain - 1.0).abs() < 1e-5);
        assert_eq!(unoccluded.hit_count, 0);

        // Place a rock SDF sphere directly between listener and emitter at z = -10
        let rock_sphere = AcousticObstacle {
            shape: AcousticObstacleShape::AnalyticalSphere {
                center: [0.0, 1.5, -10.0],
                radius: 2.0,
            },
            material: AcousticMaterial::RockLithology,
            thickness_m: 4.0,
            smooth_blend_radius: 0.25,
        };
        let canopy_box = AcousticObstacle {
            shape: AcousticObstacleShape::AnalyticalRoundedBox {
                center: [0.0, 1.5, -5.0],
                half_extents: [1.5, 1.5, 1.0],
                round_radius: 0.2,
            },
            material: AcousticMaterial::FoliageCanopy,
            thickness_m: 2.0,
            smooth_blend_radius: 0.25,
        };

        let mut occluded = AcousticOcclusionResult::default();
        evaluate_acoustic_occlusion_zero_heap(
            listener,
            emitter,
            &[rock_sphere, canopy_box],
            &mut occluded,
        );

        assert_eq!(occluded.hit_count, 2);
        assert!(
            occluded.direct_gain < unoccluded.direct_gain * 0.35,
            "Expected strong rock+canopy attenuation, got {}",
            occluded.direct_gain
        );
        assert!(
            occluded.lowpass_cutoff_hz < 3_000.0,
            "Expected low-pass cutoff drop, got {}",
            occluded.lowpass_cutoff_hz
        );
        assert!(occluded.diffraction_phase_rad > 0.0);
        assert!(occluded.early_reflection_gains[1] > 0.0);
    }

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn scene_audio_bus_graph_renders_binaural_reverb_and_emits_48b_sonic_tokens() {
        let mut graph = SceneAudioBusGraph::new();
        graph.sync_listener_from_motor(Motor::identity(), [0.0, 0.0, 0.0], 1.0);

        let holo = HolographicNodeGpu::new(
            [-3.0, 0.0, -5.0],
            1.0,
            [1.0, 1.0, 1.0],
            HolographicPrimitive::Sphere,
            4.0, // 440 Hz
            0.85,
            0.0,
            1.0,
            0.9,
            [0.0, 1.0, 1.0, 1.0],
        );
        let node = HolosonicNode::from_holographic_node(
            7,
            0xCAFE_BABE_1234_5678,
            &holo,
            HolosonicModality::LandmarkBeacon,
        );
        assert!(graph.push_emitter(node));

        let obstacle = AcousticObstacle {
            shape: AcousticObstacleShape::Aabb {
                min: [-6.0, -2.0, -8.0],
                max: [-4.0, 2.0, -6.0],
            },
            material: AcousticMaterial::TimberTrunk,
            thickness_m: 1.0,
            smooth_blend_radius: 0.2,
        };

        let mut out_l = [0.0_f32; 128];
        let mut out_r = [0.0_f32; 128];
        let sample =
            graph.render_quantum_zero_heap(0.0, 48_000.0, &[obstacle], &mut out_l, &mut out_r);

        assert_eq!(sample.active_emitter_count, 1);
        let energy_l: f32 = out_l.iter().map(|x| x * x).sum();
        let energy_r: f32 = out_r.iter().map(|x| x * x).sum();
        assert!(energy_l > 0.001, "Expected audible left channel energy");
        assert!(
            energy_l > energy_r,
            "Left-positioned source (-3, 0, -5) should produce higher left ear energy ({energy_l} vs {energy_r})"
        );

        let tok48 = graph
            .telemetry_bus
            .pop()
            .expect("Expected 48-byte HolosonicSonicToken48 emitted");
        assert_eq!(std::mem::size_of_val(&tok48), 48);
        assert_eq!(tok48.origin_quin_hash, 0xCAFE_BABE_1234_5678);
        assert_eq!(tok48.tensor_index, 7);
        assert_eq!(tok48.archetype, HolosonicModality::LandmarkBeacon as u8);
    }
}
