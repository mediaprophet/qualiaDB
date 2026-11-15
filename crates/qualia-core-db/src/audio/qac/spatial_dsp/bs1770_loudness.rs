//! ITU-R BS.1770-5 Gated Programme Loudness, K-Weighting & 4x Oversampled True-Peak Meter
//! (`bs1770_loudness.rs`).
//!
//! Adapts `etalon_signal::{bs1770, filter, loudness}` into QAC Mode `0x0105`
//! (`perceptual_features`) and Schema 203 (`FilterbankFeatures`).

pub use etalon_signal::bs1770::{
    Bs1770ChannelRole, Bs1770LoudnessReading, CalibratedSplReading, QualityStatus,
    TruePeakReading, AIR_REF_PRESSURE_PA, BS1770_ABSOLUTE_GATE_LUFS,
    BS1770_RELATIVE_GATE_OFFSET_LU, MAX_BS1770_CHANNELS, UNDERWATER_REF_PRESSURE_PA,
};
pub use etalon_signal::filter::{
    design_k_weighting_stage1, design_k_weighting_stage2, BiquadCoeffs, BiquadDf2t,
    CascadedBiquadDf2t,
};
pub use etalon_signal::loudness::{
    crest_factor_db, crest_factor_f32, dct_ii_into, energy_to_lufs, rms_dbfs, rms_f32,
};

use super::hoa_acn_sn3d::SpatialDspError;

/// Construct a 2-stage ITU-R BS.1770-5 K-weighting filter cascade at `sample_rate_hz`.
#[inline]
pub fn new_k_weighting_cascade(
    sample_rate_hz: f64,
) -> Result<CascadedBiquadDf2t<2>, SpatialDspError> {
    Ok(etalon_signal::bs1770::new_k_weighting_cascade(
        sample_rate_hz,
    )?)
}

/// Apply the ITU-R BS.1770-5 two-stage gating equations to precomputed 400 ms weighted
/// block energies $E_j = \sum_i G_i z_{i,j}$.
#[inline]
pub fn gate_block_energies_bs1770(
    block_energies: &[f64],
) -> Result<Bs1770LoudnessReading, SpatialDspError> {
    Ok(etalon_signal::bs1770::gate_block_energies_bs1770(
        block_energies,
    )?)
}

/// Measure ITU-R BS.1770-5 integrated gated loudness over an interleaved multi-channel
/// `time-channel` `f32` stream using K-weighting and 400 ms gating blocks (75% overlap).
#[inline]
pub fn measure_bs1770_integrated_loudness(
    interleaved_samples: &[f32],
    channel_roles: &[Bs1770ChannelRole],
    sample_rate_hz: u32,
    block_energies_scratch: &mut [f64],
) -> Result<Bs1770LoudnessReading, SpatialDspError> {
    Ok(etalon_signal::bs1770::measure_bs1770_integrated_loudness(
        interleaved_samples,
        channel_roles,
        sample_rate_hz,
        block_energies_scratch,
    )?)
}

/// Measure $4\times$ oversampled true-peak level (`dBTP`) over a mono `f32` buffer.
#[inline]
pub fn measure_true_peak_4x_f32(samples: &[f32]) -> Result<TruePeakReading, SpatialDspError> {
    Ok(etalon_signal::bs1770::measure_true_peak_4x_f32(samples)?)
}
