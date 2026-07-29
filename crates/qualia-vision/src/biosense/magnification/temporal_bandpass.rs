//! Temporal band-pass (difference of two 1st-order IIR low-pass filters).
//!
//! Explicit `fps`, `f_lo_hz`, `f_hi_hz` (e.g. 0.7–4 Hz for resting HR band).
//! Zero heap: state is either stack (scalar series) or caller-supplied per-sample state.

use crate::cv::error::CvError;

/// Coefficients for a first-order low-pass: y += α (x − y).
#[derive(Debug, Clone, Copy)]
pub struct LowpassAlpha {
    pub alpha: f32,
}

/// Band-pass as high_lp − low_lp (Wu-style IIR band).
#[derive(Debug, Clone, Copy)]
pub struct BandpassIir {
    pub lo: LowpassAlpha,
    pub hi: LowpassAlpha,
    pub f_lo_hz: f32,
    pub f_hi_hz: f32,
    pub fps: f32,
}

/// Design IIR band-pass from sample rate and cut-offs.
///
/// Requires `0 < f_lo_hz < f_hi_hz < fps/2` (Nyquist).
pub fn design_bandpass_iir(fps: f32, f_lo_hz: f32, f_hi_hz: f32) -> Result<BandpassIir, CvError> {
    if !(fps > 1.0) || !(f_lo_hz > 0.0) || !(f_hi_hz > f_lo_hz) {
        return Err(CvError::InvalidParameter);
    }
    let nyq = fps * 0.5;
    if f_hi_hz >= nyq {
        return Err(CvError::InvalidParameter);
    }
    // α ≈ 1 − exp(−2π f_c / fps) for a mild 1-pole LP
    let alpha_for = |fc: f32| -> f32 {
        let a = 1.0 - (-core::f32::consts::TAU * fc / fps).exp();
        a.clamp(1e-6, 1.0)
    };
    Ok(BandpassIir {
        lo: LowpassAlpha {
            alpha: alpha_for(f_lo_hz),
        },
        hi: LowpassAlpha {
            alpha: alpha_for(f_hi_hz),
        },
        f_lo_hz,
        f_hi_hz,
        fps,
    })
}

/// Filter a scalar series: `out[i] = lp_hi[i] − lp_lo[i]`.
/// DC and frequencies below `f_lo` / above `f_hi` are attenuated.
pub fn temporal_bandpass_series(
    samples: &[f32],
    bp: &BandpassIir,
    out: &mut [f32],
) -> Result<(), CvError> {
    if samples.is_empty() {
        return Err(CvError::EmptyInput);
    }
    if out.len() < samples.len() {
        return Err(CvError::BufferTooSmall);
    }
    let mut y_lo = samples[0];
    let mut y_hi = samples[0];
    for (i, &x) in samples.iter().enumerate() {
        y_lo += bp.lo.alpha * (x - y_lo);
        y_hi += bp.hi.alpha * (x - y_hi);
        out[i] = y_hi - y_lo;
    }
    Ok(())
}

/// One sample step for a multi-pixel path. Caller holds `state_lo` / `state_hi`.
#[inline]
pub fn bandpass_step(x: f32, bp: &BandpassIir, state_lo: &mut f32, state_hi: &mut f32) -> f32 {
    *state_lo += bp.lo.alpha * (x - *state_lo);
    *state_hi += bp.hi.alpha * (x - *state_hi);
    *state_hi - *state_lo
}

/// Band-pass every temporal series along frame axis for a packed buffer.
///
/// Layout: `frames` length ≥ `n_frames * plane_elems`, frame-major.
/// `state_lo` / `state_hi` each length ≥ `plane_elems` (initialized to first frame).
pub fn temporal_bandpass_planes(
    frames: &[f32],
    n_frames: usize,
    plane_elems: usize,
    bp: &BandpassIir,
    state_lo: &mut [f32],
    state_hi: &mut [f32],
    out: &mut [f32],
) -> Result<(), CvError> {
    if n_frames == 0 || plane_elems == 0 {
        return Err(CvError::InvalidParameter);
    }
    let need = n_frames * plane_elems;
    if frames.len() < need
        || out.len() < need
        || state_lo.len() < plane_elems
        || state_hi.len() < plane_elems
    {
        return Err(CvError::BufferTooSmall);
    }
    // Init states from frame 0
    state_lo[..plane_elems].copy_from_slice(&frames[..plane_elems]);
    state_hi[..plane_elems].copy_from_slice(&frames[..plane_elems]);
    for t in 0..n_frames {
        let base = t * plane_elems;
        for p in 0..plane_elems {
            let x = frames[base + p];
            out[base + p] = bandpass_step(x, bp, &mut state_lo[p], &mut state_hi[p]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandpass_passes_sinusoid_rejects_dc() {
        let fps = 30.0f32;
        let f_sig = 1.5f32; // inside 0.7–4
        let bp = design_bandpass_iir(fps, 0.7, 4.0).unwrap();
        let n = 180;
        let mut sig = vec![0.0f32; n];
        for i in 0..n {
            // DC + in-band + high-freq noise
            let t = i as f32 / fps;
            sig[i] = 50.0
                + 3.0 * (core::f32::consts::TAU * f_sig * t).sin()
                + 2.0 * (core::f32::consts::TAU * 12.0 * t).sin();
        }
        let mut out = vec![0.0f32; n];
        temporal_bandpass_series(&sig, &bp, &mut out).unwrap();
        // Skip IIR transient
        let body = &out[60..];
        let mean: f32 = body.iter().sum::<f32>() / body.len() as f32;
        let rms: f32 =
            (body.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / body.len() as f32).sqrt();
        // Band energy should be non-trivial (sinusoid preserved)
        assert!(rms > 0.5, "rms={rms}");
        // Mean of band-passed should be near 0 (DC rejected)
        assert!(mean.abs() < 0.5, "mean={mean}");
    }

    #[test]
    fn design_rejects_bad_band() {
        assert!(design_bandpass_iir(30.0, 4.0, 0.7).is_err());
        assert!(design_bandpass_iir(30.0, 0.7, 20.0).is_err()); // above Nyquist
    }
}
