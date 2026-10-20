//! Holosonic Engine — Volumetric Acoustic Wavefield Synthesis & Semantic Sonification.
//!
//! Companion to the Holographic 3D Visual Engine (`docs/Holographic 3D Engine for Semantic Graphs.md`):
//! - Replaces rigid pre-recorded audio triggers with continuous spatial wavefield synthesis.
//! - Evaluates acoustic wave interference: constructive interference generates crisp resonant
//!   harmonics where logical assertions align; destructive interference creates acoustic voids.
//! - Emits sound from volumetric SDF fields (water bodies, wind through canopies, vector corridor).
//! - Evaluates in the Tier-1 zero-heap hot path with caller-supplied buffers.

use crate::audio::hrtf::binaural_from_position;
use crate::sonic_token::{SonicEventType, SonicToken, SONIC_MAGIC};
use bytemuck::{Pod, Zeroable};
use std::f32::consts::TAU;

/// 48-byte Pod telemetry token emitted by the Holosonic wavefield engine (`Q42` U3 telemetry).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct HolosonicSonicToken48 {
    /// 64-bit origin NQuin hash or semantic entity identifier (offset 0..8).
    pub origin_quin_hash: u64,
    /// `HolosonicModality` archetype (0..=4) (offset 8).
    pub archetype: u8,
    /// Deontic permit state (`0 = Permitted`, `1 = Obligatory`, `2 = Forbidden/Escrow`) (offset 9).
    pub deontic_permit: u8,
    /// Magic and status flags (`SONIC_MAGIC`) (offset 10..12).
    pub flags: u16,
    /// Associated 10D tensor node index in `NodePool` (offset 12..16).
    pub tensor_index: u32,
    /// Base fundamental frequency in Hz (offset 16..20).
    pub fundamental_hz: f32,
    /// Instantaneous wavefield phase at the listener in radians (offset 20..24).
    pub phase_rad: f32,
    /// Epistemic certainty `q` in `[0.0, 1.0]` (offset 24..28).
    pub epistemic_q: f32,
    /// Constructive/destructive wave interference gain ratio (offset 28..32).
    pub interference_gain: f32,
    /// Direct-path acoustic occlusion transmission gain `[0.0, 1.0]` (offset 32..36).
    pub occlusion_gain: f32,
    /// World-space emitter position `[x, y, z]` (offset 36..48).
    pub position: [f32; 3],
}

const _: [(); 48] = [(); core::mem::size_of::<HolosonicSonicToken48>()];

impl Default for HolosonicSonicToken48 {
    fn default() -> Self {
        Self::zeroed()
    }
}


/// Maximum number of active acoustic hologram emitters evaluated simultaneously (zero-heap stack array).
pub const MAX_HOLOSONIC_EMITTERS: usize = 32;

/// Speed of sound in ambient air at 20°C (m/s).
pub const SPEED_OF_SOUND_MPS: f32 = 343.0;

/// Semantic modality archetype for holosonic emission.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolosonicModality {
    /// Vector transit corridor (propulsion whine, pavement friction, electromagnetic hum).
    VectorCorridor = 0,
    /// Hydrodynamic fluid resonance (streams, lake shorelines, waterfalls).
    WaterResonance = 1,
    /// Volumetric flora canopy (wind through leaves, pine needle sibilance).
    FloraWind = 2,
    /// Wildlife bioacoustic vocalization (fauna call signatures, movement rustle).
    Bioacoustic = 3,
    /// High-epistemic landmark beacon (resonant singing bowl, crystalline focal harmonic).
    LandmarkBeacon = 4,
}

/// An acoustic emitter contributing to the spatial holosonic field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HolosonicEmitter {
    /// World position [x, y, z] in meters.
    pub position: [f32; 3],
    /// Base fundamental frequency in Hz.
    pub base_freq_hz: f32,
    /// Acoustic pressure amplitude in `[0.0, 1.0]`.
    pub amplitude: f32,
    /// Initial phase offset in radians.
    pub phase_rad: f32,
    /// Epistemic certainty `q` in `[0.0, 1.0]` (controls harmonic purity and constructive focus).
    pub epistemic_q: f32,
    /// Maximum spatial coherence radius in meters.
    pub coherence_radius: f32,
    /// Modality archetype.
    pub modality: HolosonicModality,
}

impl Default for HolosonicEmitter {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            base_freq_hz: 440.0,
            amplitude: 0.0,
            phase_rad: 0.0,
            epistemic_q: 1.0,
            coherence_radius: 25.0,
            modality: HolosonicModality::VectorCorridor,
        }
    }
}

impl HolosonicEmitter {
    /// Constructs a holosonic emitter in the hot path.
    #[inline]
    pub const fn new(
        position: [f32; 3],
        base_freq_hz: f32,
        amplitude: f32,
        phase_rad: f32,
        epistemic_q: f32,
        coherence_radius: f32,
        modality: HolosonicModality,
    ) -> Self {
        Self {
            position,
            base_freq_hz,
            amplitude,
            phase_rad,
            epistemic_q,
            coherence_radius,
            modality,
        }
    }
}

/// Instantaneous binaural field sample at a listener standpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HolosonicPointSample {
    /// Instantaneous acoustic pressure for left ear.
    pub pressure_l: f32,
    /// Instantaneous acoustic pressure for right ear.
    pub pressure_r: f32,
    /// Net wave interference magnitude (constructive gain ratio).
    pub interference_gain: f32,
    /// Number of active acoustic emitters within audible horizon.
    pub active_emitter_count: usize,
}

/// Fixed-capacity holosonic wavefield manager (Tier-1 zero-heap).
pub struct HolosonicWavefield {
    emitters: [HolosonicEmitter; MAX_HOLOSONIC_EMITTERS],
    count: usize,
}

impl Default for HolosonicWavefield {
    fn default() -> Self {
        Self::new()
    }
}

impl HolosonicWavefield {
    /// Creates an empty holosonic wavefield.
    #[inline]
    pub const fn new() -> Self {
        Self {
            emitters: [HolosonicEmitter {
                position: [0.0; 3],
                base_freq_hz: 440.0,
                amplitude: 0.0,
                phase_rad: 0.0,
                epistemic_q: 1.0,
                coherence_radius: 25.0,
                modality: HolosonicModality::VectorCorridor,
            }; MAX_HOLOSONIC_EMITTERS],
            count: 0,
        }
    }

    /// Clears all emitters.
    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Current number of registered emitters.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count
    }

    /// True if no emitters are active.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Adds an emitter to the spatial wavefield. Returns false if capacity exceeded.
    #[inline]
    pub fn push(&mut self, emitter: HolosonicEmitter) -> bool {
        if self.count >= MAX_HOLOSONIC_EMITTERS {
            return false;
        }
        self.emitters[self.count] = emitter;
        self.count += 1;
        true
    }

    /// Returns a slice of the currently active emitters.
    #[inline]
    pub fn as_slice(&self) -> &[HolosonicEmitter] {
        &self.emitters[..self.count]
    }

    /// Emits a 48-byte `HolosonicSonicToken48` telemetry record for an active emitter and sample,
    /// and also packs a 64-bit `SonicToken` for the `Q42` U3 `SonicTokenRing`.
    #[inline]
    pub fn emit_sonic_token(
        &self,
        emitter_idx: usize,
        origin_quin_hash: u64,
        tensor_index: u32,
        deontic_permit: u8,
        occlusion_gain: f32,
        sample: HolosonicPointSample,
        listener_pos: [f32; 3],
        time_s: f32,
    ) -> (HolosonicSonicToken48, SonicToken) {
        let e = if emitter_idx < self.count {
            self.emitters[emitter_idx]
        } else {
            HolosonicEmitter::default()
        };
        let dx = e.position[0] - listener_pos[0];
        let dy = e.position[1] - listener_pos[1];
        let dz = e.position[2] - listener_pos[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let omega = TAU * e.base_freq_hz;
        let phase_rad = e.phase_rad + omega * time_s - (omega / SPEED_OF_SOUND_MPS) * dist;
        let tok48 = HolosonicSonicToken48 {
            origin_quin_hash,
            archetype: e.modality as u8,
            deontic_permit,
            flags: SONIC_MAGIC as u16,
            tensor_index,
            fundamental_hz: e.base_freq_hz,
            phase_rad,
            epistemic_q: e.epistemic_q,
            interference_gain: sample.interference_gain,
            occlusion_gain: (e.amplitude * occlusion_gain).clamp(0.0, 1.0),
            position: e.position,
        };
        let midi_note = (69.0 + 12.0 * (e.base_freq_hz.max(20.0) / 440.0).log2())
            .round()
            .clamp(0.0, 127.0) as u8;
        let velocity = ((e.amplitude * occlusion_gain * sample.interference_gain * 96.0)
            .clamp(1.0, 127.0)) as u8;
        let packed = SonicToken::pack(
            0,
            SonicEventType::Parametric,
            e.modality as u8 & 0x0f,
            midi_note,
            velocity,
            tensor_index,
            (SONIC_MAGIC as u16) | ((deontic_permit as u16) << 8),
        );
        (tok48, packed)
    }

    /// Evaluates the instantaneous spatial acoustic wavefield at the listener standpoint.
    ///
    /// Computes spherical wave propagation, phase delays ($\Delta \phi = k \cdot d$),
    /// wave superposition, and HRTF binaural panning.
    pub fn evaluate_point(
        &self,
        listener_pos: [f32; 3],
        listener_yaw: f32,
        time_s: f32,
    ) -> HolosonicPointSample {
        let mut sum_l = 0.0_f32;
        let mut sum_r = 0.0_f32;
        let mut raw_energy = 0.0_f32;
        let mut active = 0usize;

        for i in 0..self.count {
            let e = &self.emitters[i];
            if e.amplitude <= 0.0 {
                continue;
            }

            let dx = e.position[0] - listener_pos[0];
            let dy = e.position[1] - listener_pos[1];
            let dz = e.position[2] - listener_pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist > e.coherence_radius || dist < 0.001 {
                continue;
            }
            active += 1;

            // Distance attenuation (spherical spreading with smooth rolloff)
            let falloff = (1.0 - (dist / e.coherence_radius)).clamp(0.0, 1.0);
            let atten = (1.0 / (1.0 + dist * 0.45)) * falloff * e.amplitude;

            // Wave phase delay: propagation time tau = dist / c, phase_shift = omega * tau
            let omega = TAU * e.base_freq_hz;
            let wave_number = omega / SPEED_OF_SOUND_MPS;
            let phase = e.phase_rad + omega * time_s - wave_number * dist;

            // Harmonic content modulated by epistemic certainty q
            // High q -> pure fundamental tone; low q -> complex multi-frequency dissonant phase
            let q = e.epistemic_q.clamp(0.0, 1.0);

            // HRTF binaural spatialization (ITD + ILD); pass unit direction so distance
            // attenuation is governed once by the Holosonic spherical spreading formula above.
            let unit_dir = [dx / dist, dy / dist, dz / dist];
            let gains = binaural_from_position(unit_dir, listener_yaw);

            // Left/right ear pressure with interaural phase delay and epistemic harmonics
            let ear_delay_phase = wave_number * gains.itd_seconds * SPEED_OF_SOUND_MPS;
            let phase_l = phase + ear_delay_phase * 0.5;
            let phase_r = phase - ear_delay_phase * 0.5;
            let harmonic_l = (phase_l * 2.0 + 0.5).sin() * (1.0 - q) * 0.35;
            let harmonic_r = (phase_r * 2.0 + 0.5).sin() * (1.0 - q) * 0.35;
            let sample_l = (phase_l.sin() + harmonic_l) * atten * gains.gain_l;
            let sample_r = (phase_r.sin() + harmonic_r) * atten * gains.gain_r;

            sum_l += sample_l;
            sum_r += sample_r;
            raw_energy += atten;
        }

        let net_amplitude = (sum_l * sum_l + sum_r * sum_r).sqrt();
        let interference_gain = if raw_energy > 0.001 {
            net_amplitude / raw_energy
        } else {
            1.0
        };

        HolosonicPointSample {
            pressure_l: sum_l.clamp(-2.0, 2.0),
            pressure_r: sum_r.clamp(-2.0, 2.0),
            interference_gain,
            active_emitter_count: active,
        }
    }

    /// Renders a discrete buffer block of stereo audio samples (zero heap).
    ///
    /// Fills caller-supplied `out_l` and `out_r` slices.
    pub fn render_block(
        &self,
        listener_pos: [f32; 3],
        listener_yaw: f32,
        start_time_s: f32,
        sample_rate: f32,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        let n = out_l.len().min(out_r.len());
        let dt = 1.0 / sample_rate.max(1.0);

        for i in 0..n {
            let t = start_time_s + i as f32 * dt;
            let sample = self.evaluate_point(listener_pos, listener_yaw, t);
            out_l[i] = sample.pressure_l;
            out_r[i] = sample.pressure_r;
        }
    }

    /// Translates constructive interference peaks into 64-bit `SonicToken` events.
    ///
    /// Fills `out_tokens` without heap allocation; returns number of tokens emitted.
    pub fn emit_tokens_from_sample(
        &self,
        sample: HolosonicPointSample,
        out_tokens: &mut [SonicToken],
    ) -> usize {
        if out_tokens.is_empty() {
            return 0;
        }

        // If constructive interference peaks above threshold, trigger symbolic sonic resonance token
        if sample.interference_gain > 1.25 && sample.active_emitter_count > 0 {
            let midi_note = 60u8; // Middle C base
            let velocity = (sample.interference_gain * 40.0).clamp(20.0, 127.0) as u8;
            out_tokens[0] =
                SonicToken::pack(0, SonicEventType::NoteOn, 0, midi_note, velocity, 0, 0);
            1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_wavefield_produces_silence() {
        let wf = HolosonicWavefield::new();
        let sample = wf.evaluate_point([0.0, 0.0, 0.0], 0.0, 0.0);
        assert_eq!(sample.pressure_l, 0.0);
        assert_eq!(sample.pressure_r, 0.0);
        assert_eq!(sample.active_emitter_count, 0);
    }

    #[test]
    fn distance_attenuation_is_monotonic() {
        let mut wf = HolosonicWavefield::new();
        wf.push(HolosonicEmitter::new(
            [0.0, 0.0, 10.0],
            220.0,
            1.0,
            0.0,
            1.0,
            50.0,
            HolosonicModality::LandmarkBeacon,
        ));

        let near = wf.evaluate_point([0.0, 0.0, 8.0], 0.0, 0.0);
        let far = wf.evaluate_point([0.0, 0.0, 0.0], 0.0, 0.0);

        let near_mag = (near.pressure_l.powi(2) + near.pressure_r.powi(2)).sqrt();
        let far_mag = (far.pressure_l.powi(2) + far.pressure_r.powi(2)).sqrt();
        assert!(
            near_mag > far_mag,
            "Near ({near_mag}) should exceed Far ({far_mag})"
        );
    }

    #[test]
    fn constructive_interference_amplifies_signal() {
        let mut wf = HolosonicWavefield::new();
        // Two coherent in-phase emitters equidistant from the listener
        wf.push(HolosonicEmitter::new(
            [2.0, 0.0, 5.0],
            440.0,
            0.5,
            0.0,
            1.0,
            20.0,
            HolosonicModality::LandmarkBeacon,
        ));
        wf.push(HolosonicEmitter::new(
            [-2.0, 0.0, 5.0],
            440.0,
            0.5,
            0.0,
            1.0,
            20.0,
            HolosonicModality::LandmarkBeacon,
        ));

        // Evaluate sample block
        let mut buf_l = [0.0_f32; 64];
        let mut buf_r = [0.0_f32; 64];
        wf.render_block([0.0, 0.0, 0.0], 0.0, 0.0, 48_000.0, &mut buf_l, &mut buf_r);

        let max_val = buf_l
            .iter()
            .chain(buf_r.iter())
            .fold(0.0_f32, |m, &x| m.max(x.abs()));
        assert!(
            max_val > 0.05,
            "Constructive interference should yield audible peak"
        );
    }
}
