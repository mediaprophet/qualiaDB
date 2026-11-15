//! Binaural HRTF / HRIR Rendering, Fractional-Delay RIR Synthesis & `etalon_signal` Spatial Bridge
//! (`binaural_hrtf.rs`).
//!
//! Adapts `etalon_signal::{audio, modal, spatial}` into QAC Mode `0x0103`
//! (`hrir_impulse_response`), Mode `03` (`ray/RIR contributions`), and HOA-to-binaural
//! virtual-speaker rendering over caller-supplied buffers.

pub use etalon_signal::audio::{
    convolve_fir_into, head_relative_position, pinna_elevation_notch_hz, spherical_ild_db,
    synthesize_hrir_into, woodworth_itd_seconds, DEFAULT_HEAD_RADIUS_M, DEFAULT_SPEED_OF_SOUND_M_S,
};
pub use etalon_signal::modal::{
    FluidImpedanceInterface, IdealAcousticBoundary, OCTAHEDRAL_VIRTUAL_SPEAKERS_XYZ,
};
pub use etalon_signal::spatial::{
    mix_spatial_voice_into, mix_spatial_voice_occluded_into, DistanceModel, SpatialEmitter,
    SpatialListener, SpatialOcclusionBridge, SpatialOcclusionFilterState, SpatialVoiceParams,
    MAX_SPATIAL_VOICES, SPEED_OF_SOUND_AIR,
};

use super::hoa_acn_sn3d::{HoaNormalization, HoaOrder, SpatialDspError};

/// Convolve mono `signal` with a single receiver impulse response `kernel` shifted by
/// `additional_delay_taps` integer samples (applied **once** per C02 Mode `0103` contract).
#[inline]
pub fn convolve_hrir_with_additional_delay_into(
    signal: &[f32],
    kernel: &[f32],
    additional_delay_taps: usize,
    output: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(
        etalon_signal::modal::convolve_hrir_with_additional_delay_into(
            signal,
            kernel,
            additional_delay_taps,
            output,
        )?,
    )
}

/// Convolve mono `signal` with a stereo pair of HRIR/RIR kernels (`left_ir`, `right_ir`)
/// and per-receiver integer `additional_delays_lr = [delay_l, delay_r]` taps (each applied once).
#[inline]
pub fn convolve_stereo_hrir_with_additional_delays_into(
    signal: &[f32],
    left_ir: &[f32],
    right_ir: &[f32],
    additional_delays_lr: [usize; 2],
    out_left: &mut [f32],
    out_right: &mut [f32],
) -> Result<(usize, usize), SpatialDspError> {
    Ok(
        etalon_signal::modal::convolve_stereo_hrir_with_additional_delays_into(
            signal,
            left_ir,
            right_ir,
            additional_delays_lr,
            out_left,
            out_right,
        )?,
    )
}

/// Place a 4-point cubic Lagrange fractional-delay impulse at `delay_samples >= 0.0`
/// with amplitude `gain` into `ir_buffer` ($\le 0.01$ tap alignment error).
#[inline]
pub fn place_cubic_lagrange_fractional_impulse(
    ir_buffer: &mut [f32],
    delay_samples: f64,
    gain: f64,
) -> Result<(), SpatialDspError> {
    Ok(
        etalon_signal::modal::place_cubic_lagrange_fractional_impulse(
            ir_buffer,
            delay_samples,
            gain,
        )?,
    )
}

/// Measure the first-moment (center-of-time) tap index of a finite impulse response.
#[inline]
pub fn impulse_first_moment_tap_f64(ir_buffer: &[f32]) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::modal::impulse_first_moment_tap_f64(
        ir_buffer,
    )?)
}

/// Evaluate normal-incidence fluid acoustic interface coefficients for $Z_1 > 0, Z_2 > 0$.
#[inline]
pub fn normal_incidence_fluid_interface_f64(
    z1_rayl: f64,
    z2_rayl: f64,
) -> Result<FluidImpedanceInterface, SpatialDspError> {
    Ok(etalon_signal::modal::normal_incidence_fluid_interface_f64(
        z1_rayl, z2_rayl,
    )?)
}

/// Compute exact propagation delay in seconds and fractional taps for a homogeneous ray path.
#[inline]
pub fn ray_arrival_delay_taps_f64(
    distance_m: f64,
    speed_of_sound_m_s: f64,
    sample_rate_hz: f64,
) -> Result<(f64, f64), SpatialDspError> {
    Ok(etalon_signal::modal::ray_arrival_delay_taps_f64(
        distance_m,
        speed_of_sound_m_s,
        sample_rate_hz,
    )?)
}

/// Render an interleaved `time-channel` HOA stream (`hoa_interleaved`) to stereo `[L, R]`
/// frames (`out_stereo`) using octahedral virtual loudspeakers.
#[inline]
pub fn render_hoa_stream_binaural_into(
    order: HoaOrder,
    normalization: HoaNormalization,
    hoa_interleaved: &[f32],
    reference_freq_hz: f32,
    out_stereo: &mut [[f32; 2]],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::modal::render_hoa_stream_binaural_into(
        order,
        normalization,
        hoa_interleaved,
        reference_freq_hz,
        out_stereo,
    )?)
}
