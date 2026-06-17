//! Parametric DSP kernel — epistemic τ / FM modulation for U3 AcousticPlane.
//!
//! Hot path: stack `ParametricVoiceState`; no heap. Worklet mirrors this logic in JS.

use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;

/// Single-voice state for parametric sonification (stack-allocated).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametricVoiceState {
    pub phase: f32,
    pub frequency_hz: f32,
    pub gain: f32,
    pub fm_index: f32,
}

impl Default for ParametricVoiceState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            frequency_hz: 440.0,
            gain: 0.0,
            fm_index: 0.0,
        }
    }
}

/// Epistemic temperature τ from superposition index `q` (B4.1 synergy).
#[inline]
pub fn epistemic_temperature_from_q(q: f32) -> f32 {
    (q * q).clamp(0.0, 4.0)
}

/// FM index from epistemic `q` and phase carrier `μ`.
#[inline]
pub fn epistemic_fm_index(q: f32, mu: f32) -> f32 {
    let tau = epistemic_temperature_from_q(q);
    (tau * 0.25 + mu.abs() * 0.5).clamp(0.0, 8.0)
}

/// Map σ preview bin energy to fundamental frequency (Hz).
#[inline]
pub fn sigma_dominant_frequency(bins: &[f32; SPECTRAL_PREVIEW_BINS], base_hz: f32) -> f32 {
    let mut peak = 0usize;
    let mut max_e = 0.0_f32;
    for (i, &e) in bins.iter().enumerate() {
        let a = e.abs();
        if a > max_e {
            max_e = a;
            peak = i;
        }
    }
    let ratio = (peak as f32 + 1.0) / SPECTRAL_PREVIEW_BINS as f32;
    (base_hz * (0.5 + ratio * 3.0)).clamp(55.0, 8_000.0)
}

/// Advance voice one sample at `sample_rate` (default 48 kHz).
#[inline]
pub fn parametric_sample(state: &mut ParametricVoiceState, sample_rate: f32) -> f32 {
    let dt = 1.0 / sample_rate.max(1.0);
    let mod_phase = state.phase * (1.0 + state.fm_index * 0.01);
    let sample = (mod_phase * std::f32::consts::TAU).sin() * state.gain;
    state.phase += state.frequency_hz * dt;
    if state.phase > 1.0 {
        state.phase -= state.phase.floor();
    }
    sample
}

/// Configure voice from tensor channels and preview bins.
#[inline]
pub fn configure_voice_from_tensor(
    state: &mut ParametricVoiceState,
    q: f32,
    mu: f32,
    alpha: f32,
    bins: &[f32; SPECTRAL_PREVIEW_BINS],
) {
    state.frequency_hz = sigma_dominant_frequency(bins, 220.0);
    state.gain = alpha.clamp(0.0, 1.0) * 0.35;
    state.fm_index = epistemic_fm_index(q, mu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epistemic_temperature_monotonic() {
        assert!(epistemic_temperature_from_q(1.0) > epistemic_temperature_from_q(0.5));
    }

    #[test]
    fn parametric_sample_bounded() {
        let mut v = ParametricVoiceState {
            gain: 1.0,
            frequency_hz: 440.0,
            ..Default::default()
        };
        for _ in 0..512 {
            let s = parametric_sample(&mut v, 48_000.0);
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn sigma_frequency_in_audible_range() {
        let bins = [0.0_f32; SPECTRAL_PREVIEW_BINS];
        let mut hot = bins;
        hot[32] = 1.0;
        let hz = sigma_dominant_frequency(&hot, 220.0);
        assert!(hz >= 55.0 && hz <= 8_000.0);
    }
}