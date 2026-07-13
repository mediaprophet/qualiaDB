//! Phenomenal U2/U3 spectral parity — shared σ truth for vision and AcousticPlane (P-F2).
//!
//! Visual: `portal_spectral::sigma_to_cie_xyz` / `spectral.wgsl` (λ ≈ 400–700 nm).
//! Audio: `sigma_to_center_frequency_hz` folds the same λ band into audible Hz for parametric synth.

use crate::audio::acoustic_plane::AcousticParams;
use crate::audio::audio_spectral_sheet::{preview_bins_from_tensor, SPECTRAL_PREVIEW_BINS};
use crate::audio::dsp_kernel::epistemic_fm_index;
use crate::tensor::Tensor10D;

/// Flat uniform scalar count pushed to AudioWorklet (`acoustic_uniform_floats`).
pub const ACOUSTIC_UNIFORM_SCALAR_COUNT: usize = 18;
pub const ACOUSTIC_UNIFORM_FLOAT_COUNT: usize =
    ACOUSTIC_UNIFORM_SCALAR_COUNT + SPECTRAL_PREVIEW_BINS;

#[inline]
fn fract_sigma(sigma: f32) -> f32 {
    sigma - sigma.floor()
}

/// Wavelength (nm) — must stay aligned with `portal_spectral::sigma_to_cie_xyz`.
#[inline]
pub fn sigma_to_wavelength_nm(sigma: f32) -> f32 {
    400.0 + fract_sigma(sigma) * 300.0
}

/// Phenomenal audio twin: map σ → center frequency (Hz) for parametric voice.
///
/// Linear fold of the same 400–700 nm band used by `portal_spectral` into 1760–110 Hz
/// (short λ / blue → higher pitch; long λ / red → lower pitch).
#[inline]
pub fn sigma_to_center_frequency_hz(sigma: f32) -> f32 {
    let lambda = sigma_to_wavelength_nm(sigma);
    let t = ((lambda - 400.0) / 300.0).clamp(0.0, 1.0);
    (1760.0 * (1.0 - t) + 110.0 * t).clamp(55.0, 8_000.0)
}

/// Build phenomenal acoustic params — σ oracle drives frequency; preview bins carry tensor lerp.
#[inline]
pub fn phenomenal_acoustic_params(t: &Tensor10D) -> AcousticParams {
    let mut bins = preview_bins_from_tensor(t);
    let peak = ((fract_sigma(t.sigma) * (SPECTRAL_PREVIEW_BINS - 1) as f32).round() as usize)
        .min(SPECTRAL_PREVIEW_BINS - 1);
    bins[peak] = t.alpha.max(bins[peak]);
    AcousticParams {
        alpha: t.alpha,
        mu: t.mu,
        position: [t.x, t.y, t.z],
        track_v: t.v,
        manifold_w: t.w,
        epistemic_q: t.q,
        preview_bins: bins,
    }
}

/// Frequency + FM for worklet — uses phenomenal σ mapping (not bin peak alone).
#[inline]
pub fn phenomenal_voice_frequency_hz(t: &Tensor10D) -> f32 {
    let sigma_hz = sigma_to_center_frequency_hz(t.sigma);
    let bin_hz =
        crate::audio::dsp_kernel::sigma_dominant_frequency(&preview_bins_from_tensor(t), 220.0);
    sigma_hz * 0.72 + bin_hz * 0.28
}

#[inline]
pub fn phenomenal_fm_index(t: &Tensor10D) -> f32 {
    epistemic_fm_index(t.q, t.mu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigma_wavelength_matches_spectral_fract() {
        let s = 0.42;
        let lambda = sigma_to_wavelength_nm(s);
        assert!((lambda - 526.0).abs() < 1.0);
        assert!((sigma_to_wavelength_nm(s + 1.0) - lambda).abs() < 1e-3);
    }

    #[test]
    fn sigma_frequency_monotonic_in_fract_band() {
        let blue = sigma_to_center_frequency_hz(0.1);
        let red = sigma_to_center_frequency_hz(0.9);
        assert!(blue > red, "blue {blue} should exceed red {red}");
        assert!(blue >= 55.0 && red <= 8_000.0);
        assert!((sigma_to_center_frequency_hz(0.0) - 1760.0).abs() < 1.0);
        assert!((sigma_to_center_frequency_hz(0.99) - 110.0).abs() < 50.0);
    }

    #[test]
    fn phenomenal_params_preserves_alpha_mu() {
        let t = Tensor10D::new(0.3, 1.0, 2.0, 0.1, 0.2, 0.3, 0.5, 0.8, 0.15, 0.6);
        let p = phenomenal_acoustic_params(&t);
        assert_eq!(p.alpha, 0.8);
        assert_eq!(p.mu, 0.15);
        assert_eq!(p.preview_bins.len(), SPECTRAL_PREVIEW_BINS);
    }

    #[test]
    fn uniform_float_count_matches_portal() {
        assert_eq!(ACOUSTIC_UNIFORM_FLOAT_COUNT, 82);
        assert_eq!(ACOUSTIC_UNIFORM_SCALAR_COUNT, 18);
    }
}
