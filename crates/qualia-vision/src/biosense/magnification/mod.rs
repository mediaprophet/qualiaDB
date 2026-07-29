//! Eulerian Video Magnification (excellence-grade TODO-EVM1).
//!
//! Building blocks: Gaussian/Laplacian pyramid, temporal IIR band-pass,
//! YIQ colour EVM, multi-scale motion EVM, SNR abstain, consent gate.

pub mod colour_evm_yiq;
pub mod eulerian_color_magnify;
pub mod eulerian_motion_magnify;
pub mod evm_snr_gate;
pub mod evm_with_consent;
pub mod gaussian_pyramid_build;
pub mod gaussian_pyramid_level;
pub mod laplacian_pyramid_build;
pub mod pyramid_reconstruct;
pub mod temporal_bandpass;
pub mod temporal_bandpass_iir;

pub use colour_evm_yiq::{colour_evm_yiq, ColourEvmParams};
pub use eulerian_color_magnify::{eulerian_color_magnify, eulerian_color_magnify_ex};
pub use eulerian_motion_magnify::{
    eulerian_motion_magnify, eulerian_motion_magnify_ex, MotionEvmParams,
};
pub use evm_snr_gate::{
    band_energy_snr, energy_ms, evm_snr_gate, evm_snr_gate_energies, evm_snr_gate_trace, EvmRefuse,
    EvmSnrVerdict, DEFAULT_EVM_MIN_SNR,
};
pub use evm_with_consent::{eulerian_color_magnify_consented, eulerian_motion_magnify_consented};
pub use gaussian_pyramid_build::{
    gaussian_pyramid_build, gaussian_pyramid_scratch_elems, PyramidLevelMeta, MAX_PYRAMID_LEVELS,
};
pub use gaussian_pyramid_level::gaussian_pyramid_down_u8;
pub use laplacian_pyramid_build::laplacian_pyramid_build;
pub use pyramid_reconstruct::pyramid_reconstruct;
pub use temporal_bandpass::{
    bandpass_step, design_bandpass_iir, temporal_bandpass_planes, temporal_bandpass_series,
    BandpassIir, LowpassAlpha,
};
pub use temporal_bandpass_iir::{temporal_bandpass_iir, BandpassState};

// ---- Hz-explicit aliases expected by biosense re-exports / recipes ----

/// Colour EVM with explicit sample rate and band (Hz).
pub fn eulerian_color_magnify_hz(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    fps: f32,
    f_lo_hz: f32,
    f_hi_hz: f32,
    alpha: f32,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    eulerian_color_magnify_ex(
        frames,
        n_frames,
        width,
        height,
        ColourEvmParams {
            fps,
            f_lo_hz,
            f_hi_hz,
            alpha_chroma: alpha,
            require_snr: true,
            ..ColourEvmParams::default()
        },
        out,
    )
}

/// Motion EVM with explicit sample rate and band (Hz).
pub fn eulerian_motion_magnify_hz(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    fps: f32,
    f_lo_hz: f32,
    f_hi_hz: f32,
    alpha: f32,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    eulerian_motion_magnify_ex(
        frames,
        n_frames,
        width,
        height,
        MotionEvmParams {
            fps,
            f_lo_hz,
            f_hi_hz,
            alpha,
            require_snr: true,
            ..MotionEvmParams::default()
        },
        out,
    )
}
