//! Detect vibrato — a quasi-periodic modulation of the fundamental — from a
//! pitch track, by locating the peak of the pitch-contour spectrum.
//!
//! The track is converted to a cents deviation about its geometric-mean pitch,
//! Hann-windowed, and transformed with [`real_fft_magnitude`]. The strongest
//! bin inside the vibrato band (≈3–12 Hz) gives the rate; its interpolated
//! magnitude gives the extent (peak-to-peak, in cents); the fraction of contour
//! power it carries gives a strength in `[0, 1]`.
//!
//! Zero-heap: one caller `scratch` buffer holds the windowed cents signal and
//! the FFT work area; `out_mags` receives the contour magnitude spectrum.

use crate::features::fft::real_fft::real_fft_magnitude;
use crate::types::AudioError;

/// Vibrato descriptor for a pitch track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vibrato {
    /// Modulation rate in Hz (0.0 = none detected in band).
    pub rate_hz: f32,
    /// Peak-to-peak modulation depth in cents.
    pub extent_cents: f32,
    /// Fraction of contour power concentrated at the rate, in `[0, 1]`.
    pub strength: f32,
}

impl Vibrato {
    #[inline]
    pub const fn none() -> Self {
        Self {
            rate_hz: 0.0,
            extent_cents: 0.0,
            strength: 0.0,
        }
    }
}

const MIN_VIBRATO_HZ: f32 = 3.0;
const MAX_VIBRATO_HZ: f32 = 12.0;
/// Hann coherent gain (Σw / N).
const HANN_CG: f32 = 0.5;

/// Minimum `scratch` length for a track of `track_len` samples.
#[inline]
pub fn vibrato_scratch_len(track_len: usize) -> usize {
    3 * fft_len(track_len)
}

/// Largest power of two ≤ `m` (≥1), the analysis FFT length.
#[inline]
fn fft_len(m: usize) -> usize {
    if m < 1 {
        return 1;
    }
    let mut n = 1usize;
    while n * 2 <= m {
        n *= 2;
    }
    n
}

/// Detect vibrato in `track` sampled at `track_rate_hz` frames/second.
///
/// - `track`: pitch values (Hz); non-positive entries are treated as unvoiced
///   and contribute zero deviation.
/// - `track_rate_hz`: pitch-track frame rate (frames per second), > 0.
/// - `scratch`: work buffer, at least [`vibrato_scratch_len`] floats.
/// - `out_mags`: contour magnitude spectrum, at least `fft_len/2 + 1` floats.
///
/// Returns a [`Vibrato`]; `rate_hz == 0.0` means no in-band modulation was
/// found (e.g. a steady or unvoiced track).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `track_rate_hz ≤ 0`, the usable length
///   is below 16, or the track carries no voiced samples.
/// - [`AudioError::OutputBufferTooSmall`] if `scratch` / `out_mags` are short.
pub fn detect_vibrato(
    track: &[f32],
    track_rate_hz: f32,
    scratch: &mut [f32],
    out_mags: &mut [f32],
) -> Result<Vibrato, AudioError> {
    if !(track_rate_hz.is_finite() && track_rate_hz > 0.0) {
        return Err(AudioError::InvalidParameter);
    }
    let n = fft_len(track.len());
    if n < 16 {
        return Err(AudioError::InvalidParameter);
    }
    let bins = n / 2 + 1;
    if scratch.len() < 3 * n || out_mags.len() < bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Geometric-mean reference pitch over voiced samples.
    let mut ln_sum = 0.0f64;
    let mut voiced = 0usize;
    for &f in &track[..n] {
        if f > 0.0 {
            ln_sum += (f as f64).ln();
            voiced += 1;
        }
    }
    if voiced == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let ref_ln = ln_sum / voiced as f64;

    let (cw, fft_scratch) = scratch.split_at_mut(n);

    // Cents deviation about the reference, Hann-windowed.
    for i in 0..n {
        let f = track[i];
        let dev = if f > 0.0 {
            (1200.0 / core::f64::consts::LN_2 * ((f as f64).ln() - ref_ln)) as f32
        } else {
            0.0
        };
        let hann = 0.5 - 0.5 * (core::f32::consts::TAU * i as f32 / (n - 1) as f32).cos();
        cw[i] = dev * hann;
    }

    real_fft_magnitude(cw, fft_scratch, &mut out_mags[..bins])?;

    let bin_hz = track_rate_hz / n as f32;
    if bin_hz <= 0.0 {
        return Ok(Vibrato::none());
    }
    let lo = ((MIN_VIBRATO_HZ / bin_hz).floor() as usize).max(1);
    let hi = ((MAX_VIBRATO_HZ / bin_hz).ceil() as usize).min(bins - 1);
    if lo > hi {
        return Ok(Vibrato::none());
    }

    // Peak bin in the vibrato band.
    let peak = (lo..=hi).fold(lo, |best, k| {
        if out_mags[k] > out_mags[best] {
            k
        } else {
            best
        }
    });
    if out_mags[peak] <= 0.0 {
        return Ok(Vibrato::none());
    }

    // Parabolic sub-bin refinement of rate and peak magnitude.
    let (offset, peak_mag) = if peak >= 1 && peak + 1 < bins {
        parabolic(out_mags[peak - 1], out_mags[peak], out_mags[peak + 1])
    } else {
        (0.0, out_mags[peak])
    };
    let rate_hz = (peak as f32 + offset) * bin_hz;

    // Extent: |X_peak| ≈ A·N·cg/2 for a windowed sinusoid of amplitude A, so the
    // peak-to-peak swing is 2A = 4·|X_peak| / (N·cg).
    let extent_cents = 4.0 * peak_mag / (n as f32 * HANN_CG);

    // Strength: power fraction carried by the peak (and its two neighbours) out
    // of the whole AC contour spectrum.
    let total_pow: f64 = out_mags[1..bins]
        .iter()
        .map(|&m| (m as f64) * (m as f64))
        .sum();
    let mut peak_pow = (out_mags[peak] as f64).powi(2);
    if peak >= 1 {
        peak_pow += (out_mags[peak - 1] as f64).powi(2);
    }
    if peak + 1 < bins {
        peak_pow += (out_mags[peak + 1] as f64).powi(2);
    }
    let strength = if total_pow > 0.0 {
        (peak_pow / total_pow).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    Ok(Vibrato {
        rate_hz,
        extent_cents,
        strength,
    })
}

/// Parabolic vertex offset and interpolated height for three ordinates centred
/// on a local maximum. Offset in `(-0.5, 0.5)`.
#[inline]
fn parabolic(ym1: f32, y0: f32, yp1: f32) -> (f32, f32) {
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom.abs() <= f32::EPSILON || !denom.is_finite() {
        return (0.0, y0);
    }
    let off = (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
    let peak = y0 - 0.25 * (ym1 - yp1) * off;
    (off, peak)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    /// Synthesize a pitch track with known vibrato and recover it.
    fn synth(rate: f32, extent_pp: f32, track_rate: f32, n: usize, f_ref: f32) -> Vec<f32> {
        let amp = extent_pp * 0.5; // cents amplitude
        (0..n)
            .map(|i| {
                let phase = TAU * rate * i as f32 / track_rate;
                let cents = amp * phase.sin();
                f_ref * 2f32.powf(cents / 1200.0)
            })
            .collect()
    }

    #[test]
    fn recovers_rate_and_extent() {
        let track_rate = 100.0f32;
        let rate = 6.0f32;
        let extent = 60.0f32; // peak-to-peak cents
        let track = synth(rate, extent, track_rate, 512, 440.0);
        let mut scratch = vec![0.0f32; vibrato_scratch_len(512)];
        let mut mags = vec![0.0f32; 512 / 2 + 1];
        let v = detect_vibrato(&track, track_rate, &mut scratch, &mut mags).expect("vibrato");
        assert!((v.rate_hz - rate).abs() < 0.5, "rate={}", v.rate_hz);
        assert!(
            (v.extent_cents - extent).abs() < 15.0,
            "extent={}",
            v.extent_cents
        );
        assert!(v.strength > 0.5, "strength={}", v.strength);
    }

    #[test]
    fn steady_pitch_has_low_strength() {
        let track = vec![330.0f32; 512];
        let mut scratch = vec![0.0f32; vibrato_scratch_len(512)];
        let mut mags = vec![0.0f32; 512 / 2 + 1];
        let v = detect_vibrato(&track, 100.0, &mut scratch, &mut mags).expect("vibrato");
        // No modulation → negligible extent (numerical floor only).
        assert!(v.extent_cents < 1.0, "extent={}", v.extent_cents);
    }

    #[test]
    fn different_rate_recovered() {
        let track_rate = 100.0f32;
        let rate = 5.0f32;
        let track = synth(rate, 40.0, track_rate, 1024, 220.0);
        let mut scratch = vec![0.0f32; vibrato_scratch_len(1024)];
        let mut mags = vec![0.0f32; 1024 / 2 + 1];
        let v = detect_vibrato(&track, track_rate, &mut scratch, &mut mags).expect("vibrato");
        assert!((v.rate_hz - rate).abs() < 0.4, "rate={}", v.rate_hz);
    }

    #[test]
    fn rejects_unvoiced_track() {
        let track = vec![0.0f32; 64];
        let mut scratch = vec![0.0f32; vibrato_scratch_len(64)];
        let mut mags = vec![0.0f32; 64 / 2 + 1];
        assert_eq!(
            detect_vibrato(&track, 100.0, &mut scratch, &mut mags),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_bad_rate() {
        let track = vec![440.0f32; 64];
        let mut scratch = vec![0.0f32; vibrato_scratch_len(64)];
        let mut mags = vec![0.0f32; 64 / 2 + 1];
        assert_eq!(
            detect_vibrato(&track, 0.0, &mut scratch, &mut mags),
            Err(AudioError::InvalidParameter)
        );
    }
}
