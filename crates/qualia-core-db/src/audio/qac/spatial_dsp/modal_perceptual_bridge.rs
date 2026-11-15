//! Modal Resonance Bank Synthesizer & Perceptual Filterbank / Mel / Pitch Bridge
//! (`modal_perceptual_bridge.rs`).
//!
//! Adapts `etalon_signal::{modal, bs1770, mel, pitch, dft, window}` into QAC Mode `0x0104`
//! (`modal_resonance`, Schema 206 `RayModal`) and Mode `0x0105` (`perceptual_features`,
//! Schema 203 `FilterbankFeatures`).

pub use etalon_signal::bs1770::{
    CalibratedSplReading, QualityStatus, AIR_REF_PRESSURE_PA, UNDERWATER_REF_PRESSURE_PA,
};
pub use etalon_signal::dft::{
    dft_interleaved_into, fft_radix2_in_place, fft_radix2_interleaved_f32, DftScale,
    MAX_DFT_SAMPLES,
};
pub use etalon_signal::mel::{hz_to_mel, mel_apply_into, mel_to_hz, MelShape, PreparedMel};
pub use etalon_signal::modal::{
    ModalOscillatorState, ModalResonanceRecord, PitchReading, LN_1000,
    MODAL_RESONANCE_RECORD_BYTES,
};
pub use etalon_signal::pitch::{
    cents_deviation, hz_to_midi, hz_to_midi_quantized, midi_note_to_hz, midi_to_hz,
    yin_absolute_threshold, yin_cmndf_in_place, yin_difference_into, yin_estimate_f0_hz,
    yin_parabolic_interp, yin_parabolic_min, yin_pitch_confidence,
};
pub use etalon_signal::window::{
    apply_window_in_place, apply_window_into, blackman_harris_window, blackman_window,
    hamming_window, hann_window,
};

use super::hoa_acn_sn3d::SpatialDspError;

/// Compute $T_{60} = \frac{\ln(1000)}{d}$ for exponential decay rate $d \ge 0\text{ s}^{-1}$.
///
/// Returns `(None, QualityStatus::ModelUnavailable)` when $d == 0.0$ (never $+\infty$).
#[inline]
pub fn modal_t60_seconds(
    decay_rate_s1: f64,
) -> Result<(Option<f64>, QualityStatus), SpatialDspError> {
    Ok(etalon_signal::modal::modal_t60_seconds(decay_rate_s1)?)
}

/// Evaluate a coupled bank of damped modal resonators at time $t \ge 0$ seconds.
#[inline]
pub fn evaluate_modal_bank_at_time_f64(
    modes: &[ModalResonanceRecord],
    mode_shape_weights: &[f32],
    excitation_weights: &[f32],
    time_seconds: f64,
) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::modal::evaluate_modal_bank_at_time_f64(
        modes,
        mode_shape_weights,
        excitation_weights,
        time_seconds,
    )?)
}

/// Compute calibrated RMS pressure and dB-SPL relative to `ref_pressure_pa > 0`.
#[inline]
pub fn calibrated_rms_db_spl(
    pressure_samples_pa: &[f32],
    ref_pressure_pa: f64,
) -> Result<CalibratedSplReading, SpatialDspError> {
    Ok(etalon_signal::bs1770::calibrated_rms_db_spl(
        pressure_samples_pa,
        ref_pressure_pa,
    )?)
}

/// Evaluate exact DC ($\omega = 0$) and Nyquist ($\omega = \pi$) linear amplitude gains
/// of a real FIR filter kernel $h[0..M]$.
#[inline]
pub fn fir_dc_and_nyquist_gain_f64(kernel: &[f32]) -> Result<(f64, f64), SpatialDspError> {
    Ok(etalon_signal::modal::fir_dc_and_nyquist_gain_f64(kernel)?)
}

/// Apply a declared linear FIR filter kernel to `input` into `output`.
#[inline]
pub fn apply_linear_fir_filter_into(
    input: &[f32],
    kernel: &[f32],
    output: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::modal::apply_linear_fir_filter_into(
        input, kernel, output,
    )?)
}

/// Extract log-Mel energies and orthonormal DCT-II Mel-Frequency Cepstral Coefficients (MFCCs)
/// from a single power-of-two time-domain `frame`.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn extract_mel_mfcc_frame_into(
    frame: &[f32],
    n_mel: usize,
    mel_filterbank: &[f32],
    stft_interleaved_scratch: &mut [f32],
    one_sided_power_scratch: &mut [f32],
    out_mel_energies: &mut [f32],
    out_mfcc: &mut [f32],
) -> Result<QualityStatus, SpatialDspError> {
    Ok(etalon_signal::modal::extract_mel_mfcc_frame_into(
        frame,
        n_mel,
        mel_filterbank,
        stft_interleaved_scratch,
        one_sided_power_scratch,
        out_mel_energies,
        out_mfcc,
    )?)
}

/// Estimate fundamental frequency $f_0$ using YIN while enforcing C02-C01 / Mode 05
/// quality rules (`Silence` / `Unvoiced` return `f0_hz: None`, never `0.0 Hz`).
#[inline]
pub fn estimate_pitch_yin_with_status(
    frame: &[f32],
    sample_rate_hz: f32,
    aperiodicity_threshold: f32,
    diff_scratch: &mut [f32],
) -> Result<PitchReading, SpatialDspError> {
    Ok(etalon_signal::modal::estimate_pitch_yin_with_status(
        frame,
        sample_rate_hz,
        aperiodicity_threshold,
        diff_scratch,
    )?)
}
