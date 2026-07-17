//! Shared respiratory-rate estimate (no training; spectral only).

/// Breaths-per-minute estimate with honest SNR-derived confidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RrEstimate {
    /// Breaths per minute (clinical-style rate).
    pub breaths_per_min: f32,
    /// Peak / mean-band power ratio in the respiratory DFT band.
    pub snr: f32,
    /// Confidence in \[0, 1\], derived from SNR; not calibrated clinical accuracy.
    pub confidence: f32,
}

/// Adult-ish respiratory search band: 0.10–0.50 Hz → 6–30 breaths/min.
pub const RR_F_LO_HZ: f32 = 0.10;
pub const RR_F_HI_HZ: f32 = 0.50;
/// Minimum samples for a usable low-frequency peak (~2 s @ 30 fps is still weak; 64 is floor).
pub const RR_MIN_SAMPLES: usize = 64;
/// Spectral bins across the RR band.
pub const RR_SPECTRAL_STEPS: usize = 96;
/// SNR below this → abstain (fail closed) when a gate is applied.
pub const RR_MIN_SNR_DEFAULT: f32 = 3.0;
/// Map SNR → confidence; conf ≈ 1 when snr ≥ this.
pub const RR_SNR_FOR_FULL_CONF: f32 = 18.0;

#[inline]
pub fn snr_to_confidence(snr: f32) -> f32 {
    (snr / RR_SNR_FOR_FULL_CONF).clamp(0.0, 1.0)
}
