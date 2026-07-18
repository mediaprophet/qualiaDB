//! Klapuri-style iterative spectral-subtraction multi-pitch estimation.
//!
//! Repeatedly (1) find the most salient fundamental in the *residual* magnitude
//! spectrum, (2) estimate + cancel its harmonic series, and (3) recurse on the
//! residual — up to `max_polyphony` sources or until the strongest remaining
//! salience falls below `salience_threshold`. This is the classic Klapuri
//! predominant-F0 cancellation loop (Klapuri, "Multiple fundamental frequency
//! estimation by summing harmonic amplitudes", ISMIR 2006).
//!
//! REUSE: the per-iteration salience is built from the shared primitives — the
//! residual's spectral peaks come from
//! [`crate::features::peaks::spectral_peaks`] and are folded into a harmonic
//! pitch-salience curve by [`crate::features::salience::pitch_salience`]. This
//! module adds only the *iteration + cancellation* on top of those.
//!
//! EPISTEMIC RULE (declared): the returned F0s are **proposals**, not ground
//! truth. The **max-polyphony assumption** is explicit — the caller states, via
//! `max_polyphony`, the largest number of concurrent pitches it is willing to
//! posit; the estimator never invents more than that, and it **abstains**
//! (stops early, possibly at zero) as soon as the residual's best salience drops
//! below `salience_threshold`. A single-tone spectrum therefore yields exactly
//! one F0 and a flat/noise spectrum yields none.
//!
//! Zero-heap hot path: the only working buffer is the caller-owned `scratch`,
//! which holds the residual magnitude copy followed by the salience curve; the
//! bounded spectral-peak lists live in fixed stack arrays.

use crate::features::peaks::spectral_peaks;
use crate::features::salience::pitch_salience;
use crate::types::AudioError;

/// Salience-grid resolution (bins per semitone → 10 = 10 cents/bin).
const BINS_PER_SEMITONE: f32 = 10.0;
/// Bounded number of residual spectral peaks folded into the salience curve.
const MAX_PEAKS: usize = 128;
/// Harmonics cancelled per detected source (higher ones are usually silent).
const N_SUBTRACT_HARMONICS: usize = 20;
/// Half-width (semitones) of the magnitude window zeroed around each cancelled
/// harmonic; ±0.5 semitone brackets a spectral peak without touching neighbours.
const SUBTRACT_HALF_WIDTH_SEMITONES: f32 = 0.5;

/// Number of logarithmic salience bins spanning `[f_min, f_max]` at
/// [`BINS_PER_SEMITONE`] resolution. This is exactly the salience-buffer length
/// the caller must reserve in `scratch` **beyond** `mag.len()` (see below).
///
/// `scratch.len()` must be at least `mag.len() + salience_bins(f_min, f_max)`.
#[inline]
pub(crate) fn salience_bins(f_min: f32, f_max: f32) -> usize {
    if !(f_min.is_finite() && f_max.is_finite()) || f_min <= 0.0 || f_max <= f_min {
        return 0;
    }
    1 + (12.0 * BINS_PER_SEMITONE * (f_max / f_min).log2()).floor() as usize
}

/// Sub-bin refinement of the salience maximum at bin `bi` via a 3-point
/// parabola; returns the fractional bin. Boundary maxima fall back to `bi`.
#[inline]
fn parabolic_bin(sal: &[f32], bi: usize, n_bins: usize) -> f32 {
    if bi == 0 || bi + 1 >= n_bins {
        return bi as f32;
    }
    let ym1 = sal[bi - 1];
    let y0 = sal[bi];
    let yp1 = sal[bi + 1];
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom == 0.0 || !denom.is_finite() {
        return bi as f32;
    }
    let off = (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
    bi as f32 + off
}

/// Zero the residual magnitude in a ±half-semitone window around every harmonic
/// `h * f0` (for `h = 1..=N_SUBTRACT_HARMONICS`) that lies below Nyquist — i.e.
/// cancel the detected source so it is neither re-detected nor left to skew the
/// next salience curve.
fn subtract_harmonics(residual: &mut [f32], f0: f32, bin_hz: f32) {
    let n = residual.len();
    if n == 0 || bin_hz <= 0.0 {
        return;
    }
    let nyquist = (n - 1) as f32 * bin_hz;
    let lo_ratio = (-SUBTRACT_HALF_WIDTH_SEMITONES / 12.0).exp2();
    let hi_ratio = (SUBTRACT_HALF_WIDTH_SEMITONES / 12.0).exp2();
    for h in 1..=N_SUBTRACT_HARMONICS {
        let fh = f0 * h as f32;
        if fh > nyquist {
            break;
        }
        let lo = (fh * lo_ratio / bin_hz).floor().max(0.0) as usize;
        let hi_f = (fh * hi_ratio / bin_hz).ceil();
        let hi = if hi_f < 0.0 {
            0
        } else {
            (hi_f as usize).min(n - 1)
        };
        for b in lo..=hi {
            residual[b] = 0.0;
        }
    }
}

/// Iterative spectral-subtraction multi-pitch estimation.
///
/// - `mag`: one-sided magnitude spectrum covering DC..Nyquist inclusive (length
///   `N/2+1` for an FFT of size `2*(mag.len()-1)`), e.g. from
///   [`crate::features::fft::real_fft_magnitude`]. Not modified.
/// - `sample_rate`: Hz; sets the bin spacing `sample_rate / (2*(mag.len()-1))`.
/// - `f_min` / `f_max`: inclusive fundamental-frequency search range in Hz.
/// - `max_polyphony`: the **declared** maximum number of concurrent pitches to
///   propose (the estimator writes at most `min(max_polyphony, out_f0.len())`).
/// - `salience_threshold`: absolute salience floor; the loop stops (abstains) as
///   soon as the residual's strongest salience is below it.
/// - `out_f0`: caller buffer; detected fundamentals are written **salience
///   order** (strongest first).
/// - `scratch`: caller buffer of length `>= mag.len() + salience_bins(f_min,
///   f_max)`; used as the residual magnitude copy followed by the salience curve.
///
/// Returns the number of F0s written.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate`, `f_min`, `f_max` are not
///   positive finite with `f_max > f_min`, `salience_threshold` is negative or
///   non-finite, or `mag.len() < 3`.
/// - [`AudioError::OutputBufferTooSmall`] if `scratch` cannot hold the residual
///   plus the salience curve.
#[allow(clippy::too_many_arguments)]
pub fn multipitch_klapuri(
    mag: &[f32],
    sample_rate: f32,
    f_min: f32,
    f_max: f32,
    max_polyphony: usize,
    salience_threshold: f32,
    out_f0: &mut [f32],
    scratch: &mut [f32],
) -> Result<usize, AudioError> {
    if !(sample_rate.is_finite() && sample_rate > 0.0)
        || !(f_min.is_finite() && f_min > 0.0)
        || !(f_max.is_finite() && f_max > f_min)
        || !(salience_threshold.is_finite() && salience_threshold >= 0.0)
        || mag.len() < 3
    {
        return Err(AudioError::InvalidParameter);
    }

    let mlen = mag.len();
    let n_bins = salience_bins(f_min, f_max);
    if n_bins < 3 || scratch.len() < mlen + n_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let cap = max_polyphony.min(out_f0.len());
    for v in out_f0.iter_mut().take(cap) {
        *v = 0.0;
    }
    if cap == 0 {
        return Ok(0);
    }

    let bin_hz = sample_rate / (2.0 * (mlen - 1) as f32);

    // scratch = [ residual magnitude (mlen) | salience curve (n_bins) ].
    let (residual, tail) = scratch.split_at_mut(mlen);
    let sal = &mut tail[..n_bins];
    residual.copy_from_slice(mag);

    // Bounded per-iteration spectral-peak lists (stack, zero-heap).
    let mut peak_freqs = [0.0f32; MAX_PEAKS];
    let mut peak_mags = [0.0f32; MAX_PEAKS];

    let mut found = 0usize;
    while found < cap {
        // (1) Salience of the current residual, via the shared primitives.
        let n_peaks = spectral_peaks(
            residual,
            sample_rate,
            MAX_PEAKS,
            &mut peak_freqs,
            &mut peak_mags,
        )?;
        if n_peaks == 0 {
            break; // nothing pitched left → abstain
        }
        pitch_salience(
            &peak_freqs,
            &peak_mags,
            n_peaks,
            f_min,
            BINS_PER_SEMITONE,
            n_bins,
            sal,
        )?;

        // (2) Strongest salience ridge.
        let mut bi = 0usize;
        let mut bv = sal[0];
        for b in 1..n_bins {
            if sal[b] > bv {
                bv = sal[b];
                bi = b;
            }
        }
        if bv < salience_threshold {
            break; // abstain: residual no longer confidently pitched
        }

        // (3) Refine → Hz, record, and cancel the source from the residual.
        let refined = parabolic_bin(sal, bi, n_bins);
        let f0 = f_min * (refined / (12.0 * BINS_PER_SEMITONE)).exp2();
        out_f0[found] = f0;
        found += 1;

        subtract_harmonics(residual, f0, bin_hz);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FFT_SIZE: usize = 8192;
    const SR: f32 = 44_100.0;
    const N_BINS_SPEC: usize = FFT_SIZE / 2 + 1; // 4097
    const F_MIN: f32 = 100.0;
    const F_MAX: f32 = 2_000.0;

    fn bin_hz() -> f32 {
        SR / FFT_SIZE as f32
    }

    /// Add a harmonic tone (fundamental `f0`, `n_harm` harmonics with 1/h decay)
    /// into a magnitude spectrum: each harmonic is a 3-bin triangle so it forms a
    /// clean strict local maximum for `spectral_peaks`.
    fn add_tone(mag: &mut [f32], f0: f32, n_harm: usize) {
        let bh = bin_hz();
        for h in 1..=n_harm {
            let fh = f0 * h as f32;
            let k = (fh / bh).round() as usize;
            if k == 0 || k + 1 >= mag.len() {
                continue;
            }
            let amp = 1.0 / h as f32;
            mag[k] += amp;
            mag[k - 1] += 0.4 * amp;
            mag[k + 1] += 0.4 * amp;
        }
    }

    fn semitone_err(a: f32, b: f32) -> f32 {
        12.0 * (a / b).log2().abs()
    }

    fn scratch_buf() -> Vec<f32> {
        vec![0.0f32; N_BINS_SPEC + salience_bins(F_MIN, F_MAX)]
    }

    /// GOLDEN: two simultaneous harmonic tones (440 Hz + 660 Hz). With
    /// `max_polyphony = 3` the estimator recovers BOTH fundamentals (within ~1
    /// semitone) and does NOT invent a spurious third pitch.
    #[test]
    fn recovers_two_simultaneous_tones() {
        let mut mag = vec![0.0f32; N_BINS_SPEC];
        add_tone(&mut mag, 440.0, 5);
        add_tone(&mut mag, 660.0, 5);

        let mut out = [0.0f32; 4];
        let mut scratch = scratch_buf();
        let n = multipitch_klapuri(
            &mag, SR, F_MIN, F_MAX, 3, 0.5, &mut out, &mut scratch,
        )
        .expect("klapuri");

        assert_eq!(n, 2, "two tones present → exactly two F0s (no invented third)");

        // The two recovered F0s must be ~440 and ~660 in some order.
        let (a, b) = (out[0], out[1]);
        let matches_440_660 = (semitone_err(a, 440.0) < 1.0 && semitone_err(b, 660.0) < 1.0)
            || (semitone_err(a, 660.0) < 1.0 && semitone_err(b, 440.0) < 1.0);
        assert!(
            matches_440_660,
            "recovered F0s {a:.2} Hz, {b:.2} Hz do not match 440 & 660"
        );
    }

    /// A single harmonic tone yields exactly one F0 (~440 Hz).
    #[test]
    fn single_tone_returns_one() {
        let mut mag = vec![0.0f32; N_BINS_SPEC];
        add_tone(&mut mag, 440.0, 5);

        let mut out = [0.0f32; 4];
        let mut scratch = scratch_buf();
        let n = multipitch_klapuri(
            &mag, SR, F_MIN, F_MAX, 4, 0.5, &mut out, &mut scratch,
        )
        .expect("klapuri");

        assert_eq!(n, 1, "one tone → one F0");
        assert!(
            semitone_err(out[0], 440.0) < 1.0,
            "recovered {:.2} Hz vs 440",
            out[0]
        );
    }

    /// A flat spectrum (no pitched peaks) abstains entirely.
    #[test]
    fn flat_spectrum_abstains() {
        let mag = vec![0.1f32; N_BINS_SPEC]; // constant → no strict local maxima
        let mut out = [0.0f32; 4];
        let mut scratch = scratch_buf();
        let n = multipitch_klapuri(
            &mag, SR, F_MIN, F_MAX, 4, 0.5, &mut out, &mut scratch,
        )
        .expect("klapuri");
        assert_eq!(n, 0, "flat/noise spectrum → abstain (0 F0s)");
    }

    /// `max_polyphony` caps the count even when more tones are present.
    #[test]
    fn respects_max_polyphony() {
        let mut mag = vec![0.0f32; N_BINS_SPEC];
        add_tone(&mut mag, 261.63, 5); // C4
        add_tone(&mut mag, 329.63, 5); // E4
        add_tone(&mut mag, 392.0, 5); // G4

        let mut out = [0.0f32; 4];
        let mut scratch = scratch_buf();
        let n = multipitch_klapuri(
            &mag, SR, F_MIN, F_MAX, 2, 0.5, &mut out, &mut scratch,
        )
        .expect("klapuri");
        assert_eq!(n, 2, "declared max-polyphony of 2 is honoured");
    }

    #[test]
    fn rejects_bad_params() {
        let mag = vec![0.0f32; N_BINS_SPEC];
        let mut out = [0.0f32; 4];
        let mut scratch = scratch_buf();
        assert_eq!(
            multipitch_klapuri(&mag, 0.0, F_MIN, F_MAX, 2, 0.5, &mut out, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            multipitch_klapuri(&mag, SR, F_MAX, F_MIN, 2, 0.5, &mut out, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_scratch() {
        let mag = vec![0.0f32; N_BINS_SPEC];
        let mut out = [0.0f32; 4];
        let mut scratch = vec![0.0f32; 8]; // far too small
        assert_eq!(
            multipitch_klapuri(&mag, SR, F_MIN, F_MAX, 2, 0.5, &mut out, &mut scratch),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
