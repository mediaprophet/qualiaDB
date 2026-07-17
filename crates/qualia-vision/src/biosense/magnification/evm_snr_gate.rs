//! SNR / band-energy gate for EVM — abstain rather than invent magnification.

use super::temporal_bandpass::{design_bandpass_iir, temporal_bandpass_series};
use crate::cv::error::CvError;

/// Scalar energy gate verdict (band vs residual).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvmSnrVerdict {
    Ok { snr: f32 },
    Abstain { snr: f32 },
}

/// Why EVM processing was refused (structured, for Result paths).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvmRefuse {
    ConsentDenied,
    BufferTooSmall,
    InvalidParameter,
    EmptyInput,
    /// Band residual energy too weak / junk relative to full signal.
    SnrTooLow { snr: f32, threshold: f32 },
    InsufficientFrames { got: usize, need: usize },
}

impl From<CvError> for EvmRefuse {
    fn from(e: CvError) -> Self {
        match e {
            CvError::BufferTooSmall => Self::BufferTooSmall,
            CvError::EmptyInput => Self::EmptyInput,
            CvError::InvalidParameter | CvError::DimensionMismatch => Self::InvalidParameter,
        }
    }
}

impl core::fmt::Display for EvmRefuse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConsentDenied => write!(f, "biosense consent denied"),
            Self::BufferTooSmall => write!(f, "output or scratch buffer too small"),
            Self::InvalidParameter => write!(f, "invalid EVM parameter"),
            Self::EmptyInput => write!(f, "empty input"),
            Self::SnrTooLow { snr, threshold } => {
                write!(f, "band SNR {snr:.3} below threshold {threshold:.3}")
            }
            Self::InsufficientFrames { got, need } => {
                write!(f, "need at least {need} frames, got {got}")
            }
        }
    }
}

/// Compare band energy to residual (out-of-band proxy).
pub fn evm_snr_gate_energies(band_energy: f32, residual_energy: f32, min_snr: f32) -> EvmSnrVerdict {
    let res = residual_energy.max(1e-12);
    let snr = band_energy.max(0.0) / res;
    if snr >= min_snr {
        EvmSnrVerdict::Ok { snr }
    } else {
        EvmSnrVerdict::Abstain { snr }
    }
}

/// Mean square of a slice.
pub fn energy_ms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32
}

/// Default minimum band SNR (fraction of total demeaned RMS in-band).
pub const DEFAULT_EVM_MIN_SNR: f32 = 0.08;

/// Estimate SNR as band residual RMS / total demeaned RMS (after IIR band-pass).
///
/// `scratch` ≥ `trace.len()`.
pub fn band_energy_snr(
    trace: &[f32],
    fps: f32,
    f_lo_hz: f32,
    f_hi_hz: f32,
    scratch: &mut [f32],
) -> Result<f32, EvmRefuse> {
    if trace.len() < 8 {
        return Err(EvmRefuse::InsufficientFrames {
            got: trace.len(),
            need: 8,
        });
    }
    if scratch.len() < trace.len() {
        return Err(EvmRefuse::BufferTooSmall);
    }
    let bp = design_bandpass_iir(fps, f_lo_hz, f_hi_hz)?;
    temporal_bandpass_series(trace, &bp, scratch)?;
    let n = trace.len();
    let mean: f32 = trace.iter().sum::<f32>() / n as f32;
    let mut total_e = 0.0f32;
    let mut band_e = 0.0f32;
    let skip = ((fps * 0.5) as usize).min(n / 4).max(1);
    let count = (n - skip).max(1) as f32;
    for i in skip..n {
        let d = trace[i] - mean;
        total_e += d * d;
        let b = scratch[i];
        band_e += b * b;
    }
    let total_rms = (total_e / count).sqrt();
    let band_rms = (band_e / count).sqrt();
    if total_rms < 1e-8 {
        return Ok(0.0);
    }
    Ok(band_rms / total_rms)
}

/// Gate on a precomputed energy pair (compat thin name used by early stubs).
pub fn evm_snr_gate(band_energy: f32, residual_energy: f32, min_snr: f32) -> EvmSnrVerdict {
    evm_snr_gate_energies(band_energy, residual_energy, min_snr)
}

/// Gate on a temporal trace: `Ok(snr)` if usable; else `Err(SnrTooLow|…)`.
pub fn evm_snr_gate_trace(
    trace: &[f32],
    fps: f32,
    f_lo_hz: f32,
    f_hi_hz: f32,
    min_snr: f32,
    scratch: &mut [f32],
) -> Result<f32, EvmRefuse> {
    let snr = band_energy_snr(trace, fps, f_lo_hz, f_hi_hz, scratch)?;
    if snr < min_snr {
        return Err(EvmRefuse::SnrTooLow {
            snr,
            threshold: min_snr,
        });
    }
    Ok(snr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abstain_low() {
        assert!(matches!(
            evm_snr_gate(0.1, 10.0, 1.0),
            EvmSnrVerdict::Abstain { .. }
        ));
    }

    #[test]
    fn ok_high() {
        assert!(matches!(
            evm_snr_gate(10.0, 1.0, 2.0),
            EvmSnrVerdict::Ok { .. }
        ));
    }

    #[test]
    fn abstains_on_junk_noise() {
        let fps = 30.0f32;
        let n = 120;
        // Pure DC: no temporal structure → band SNR ≈ 0 after demean.
        let t = vec![42.0f32; n];
        let mut scratch = vec![0.0f32; n];
        let r = evm_snr_gate_trace(&t, fps, 0.7, 4.0, 0.08, &mut scratch);
        assert!(
            matches!(r, Err(EvmRefuse::SnrTooLow { .. })),
            "expected abstain, got {r:?}"
        );
        // Out-of-band high tone only (well above 4 Hz) should also be weak in-band.
        let mut hi = vec![0.0f32; n];
        for i in 0..n {
            let tt = i as f32 / fps;
            hi[i] = 5.0 * (core::f32::consts::TAU * 12.0 * tt).sin();
        }
        let r2 = evm_snr_gate_trace(&hi, fps, 0.7, 4.0, 0.85, &mut scratch);
        assert!(
            matches!(r2, Err(EvmRefuse::SnrTooLow { .. })),
            "expected high-tone abstain, got {r2:?}"
        );
    }

    #[test]
    fn accepts_in_band_sinusoid() {
        let fps = 30.0f32;
        let n = 180;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            let tt = i as f32 / fps;
            t[i] = 10.0 + 2.0 * (core::f32::consts::TAU * 1.2 * tt).sin();
        }
        let mut scratch = vec![0.0f32; n];
        let snr = evm_snr_gate_trace(&t, fps, 0.7, 4.0, 0.15, &mut scratch).unwrap();
        assert!(snr > 0.15, "snr={snr}");
    }
}
