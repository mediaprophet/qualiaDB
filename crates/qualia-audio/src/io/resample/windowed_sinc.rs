//! Band-limited windowed-sinc (Lanczos) resampler — the quality path.
//!
//! The stock [`crate::resample::resample_linear_mono`] uses linear
//! interpolation, which **aliases** on downsample: content above the
//! destination Nyquist folds back into the band as spurious tones. This
//! resampler instead reconstructs each output sample from a windowed-sinc
//! kernel whose bandwidth is limited to the destination Nyquist
//! ([`antialias_cutoff`]), so out-of-band energy is filtered out rather than
//! aliased.
//!
//! # Zero-heap hot path
//! [`resample_sinc`] performs **no allocation**: the output is caller-buffered
//! and every kernel tap is evaluated on the fly from stack scalars. (A phase
//! table could be precomputed once, but direct evaluation is already
//! allocation-free and keeps the kernel exact for arbitrary rate ratios.)

use crate::io::resample::anti_alias::antialias_cutoff;
use crate::types::AudioError;

/// Resample mono f32 `src` (`src_rate`) → `dst_rate` into caller-buffered `out`,
/// using a Lanczos-windowed-sinc kernel with `half_taps` on each side.
///
/// - `half_taps`: kernel half-width (in *scaled* taps); larger = sharper
///   transition and deeper stopband. Typical range 8–64. Must be non-zero.
/// - Returns the number of output frames written (clamped to `out.len()`).
///
/// Errors: [`AudioError::InvalidParameter`] for a zero rate or `half_taps == 0`;
/// [`AudioError::MalformedAudio`] for empty `src`.
///
/// The kernel is DC-normalised (taps divided by their in-bounds sum), so
/// constant / ramp signals are preserved and in-band tones keep ~unity gain.
pub fn resample_sinc(
    src: &[f32],
    src_rate: u32,
    dst_rate: u32,
    out: &mut [f32],
    half_taps: usize,
) -> Result<usize, AudioError> {
    if src_rate == 0 || dst_rate == 0 || half_taps == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if src.is_empty() {
        return Err(AudioError::MalformedAudio);
    }
    // Identity rate: straight copy.
    if src_rate == dst_rate {
        let n = src.len().min(out.len());
        out[..n].copy_from_slice(&src[..n]);
        return Ok(n);
    }

    let out_len_total = (src.len() as u64 * dst_rate as u64 / src_rate as u64) as usize;
    let out_n = out_len_total.min(out.len());
    if out_n == 0 {
        return Ok(0);
    }

    // Sinc time-scale: 1.0 when upsampling (full band), < 1.0 when downsampling
    // (bandwidth compressed to the destination Nyquist).
    let filt_scale = (2.0 * antialias_cutoff(src_rate, dst_rate)) as f64; // = min(1, dst/src)
                                                                          // Source samples per output sample (step through the input).
    let step = src_rate as f64 / dst_rate as f64;
    // Kernel support half-width in *source* samples (widens on downsample).
    let radius = half_taps as f64 / filt_scale;

    for (i, o) in out.iter_mut().enumerate().take(out_n) {
        let center = i as f64 * step; // position in source samples
        *o = kernel_interp(src, center, filt_scale, radius);
    }
    Ok(out_n)
}

/// Evaluate the DC-normalised windowed-sinc kernel at fractional source
/// position `center`, summing over the in-bounds neighbourhood.
#[inline]
fn kernel_interp(src: &[f32], center: f64, filt_scale: f64, radius: f64) -> f32 {
    let j_min = (center - radius).ceil() as i64;
    let j_max = (center + radius).floor() as i64;
    let mut acc = 0.0f64;
    let mut wsum = 0.0f64;
    let len = src.len() as i64;
    let mut j = j_min;
    while j <= j_max {
        if j >= 0 && j < len {
            let d = center - j as f64;
            // Lowpass sinc with cutoff filt_scale/2, times a Lanczos window of
            // half-width `radius`. `filt_scale` prefactor gives ~unity passband.
            let tap = filt_scale * sinc(filt_scale * d) * sinc(d / radius);
            // SAFETY note: bounds already checked above.
            acc += src[j as usize] as f64 * tap;
            wsum += tap;
        }
        j += 1;
    }
    if wsum.abs() < 1e-12 {
        0.0
    } else {
        (acc / wsum) as f32
    }
}

/// Normalised sinc: `sin(πx) / (πx)`, with `sinc(0) = 1`.
#[inline]
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let px = core::f64::consts::PI * x;
        px.sin() / px
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::TAU;

    /// Goertzel amplitude estimate at `freq` Hz over `sig` sampled at `fs`.
    fn goertzel_amp(sig: &[f32], freq: f64, fs: f64) -> f64 {
        let n = sig.len();
        if n == 0 {
            return 0.0;
        }
        let w = TAU * freq / fs;
        let coeff = 2.0 * w.cos();
        let mut s1 = 0.0f64;
        let mut s2 = 0.0f64;
        for &x in sig {
            let s0 = x as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        power.max(0.0).sqrt() * 2.0 / n as f64
    }

    fn rms(sig: &[f32]) -> f64 {
        if sig.is_empty() {
            return 0.0;
        }
        let s: f64 = sig.iter().map(|&x| x as f64 * x as f64).sum();
        (s / sig.len() as f64).sqrt()
    }

    fn sine(freq: f64, amp: f64, n: usize, fs: f64) -> Vec<f32> {
        (0..n)
            .map(|i| (amp * (TAU * freq * i as f64 / fs).sin()) as f32)
            .collect()
    }

    #[test]
    fn identity_rate_returns_input() {
        let s = [0.1f32, 0.2, 0.3, 0.4];
        let mut o = [0.0f32; 4];
        let n = resample_sinc(&s, 16000, 16000, &mut o, 16).unwrap();
        assert_eq!(n, 4);
        assert_eq!(o, s);
    }

    #[test]
    fn dc_is_preserved() {
        let s = vec![0.5f32; 256];
        let mut o = vec![0.0f32; 128];
        let n = resample_sinc(&s, 48000, 16000, &mut o, 16).unwrap();
        assert!(n > 0);
        for (k, &v) in o[..n].iter().enumerate() {
            assert!((v - 0.5).abs() < 1e-3, "bin {k} = {v}, expected 0.5");
        }
    }

    #[test]
    fn ramp_value_is_preserved_interior() {
        // src[j] = 0.01·j ; downsample 48k→24k (factor 2). Interior output i
        // maps to source position 2·i, which the symmetric kernel preserves.
        let s: Vec<f32> = (0..200).map(|j| 0.01 * j as f32).collect();
        let mut o = vec![0.0f32; 100];
        let half = 16usize;
        let n = resample_sinc(&s, 48000, 24000, &mut o, half).unwrap();
        // radius = 16 / 0.5 = 32 source samples; safe interior: center in [40,160].
        for i in 25..=75 {
            let center = 2.0 * i as f32;
            let expected = 0.01 * center;
            assert!(
                (o[i] - expected).abs() < 1e-2,
                "ramp[{i}] = {}, expected {expected}",
                o[i]
            );
        }
        let _ = n;
    }

    #[test]
    fn anti_aliasing_downsample_rejects_out_of_band_tone() {
        // 48 kHz → 8 kHz (dst Nyquist = 4000 Hz). half_taps large for sharpness.
        let src_rate = 48000u32;
        let dst_rate = 8000u32;
        let fs_out = dst_rate as f64;
        let n_in = 4800usize; // 0.1 s
        let amp = 0.8;
        let half = 32usize;

        // (a) In-band tone at 1 kHz survives with ~unity amplitude.
        let in_band = sine(1000.0, amp, n_in, src_rate as f64);
        let mut out_in = vec![0.0f32; 900];
        let n_in_out = resample_sinc(&in_band, src_rate, dst_rate, &mut out_in, half).unwrap();
        let mid_in = &out_in[100..n_in_out.saturating_sub(100)];
        let amp_1k = goertzel_amp(mid_in, 1000.0, fs_out);
        assert!(
            amp_1k > 0.6,
            "in-band 1 kHz should survive ~unity (0.8), got {amp_1k}"
        );

        // (b) Out-of-band tone at 6 kHz (> 4 kHz dst Nyquist). Naive/linear
        // decimation would alias it to |6000 - 8000| = 2000 Hz. A band-limited
        // resampler removes it → both the alias tone and total energy stay low.
        let out_band = sine(6000.0, amp, n_in, src_rate as f64);
        let mut out_ob = vec![0.0f32; 900];
        let n_ob_out = resample_sinc(&out_band, src_rate, dst_rate, &mut out_ob, half).unwrap();
        let mid_ob = &out_ob[100..n_ob_out.saturating_sub(100)];

        let alias_2k = goertzel_amp(mid_ob, 2000.0, fs_out);
        assert!(
            alias_2k < 0.1,
            "aliased 2 kHz tone should be suppressed (< 0.1), got {alias_2k}"
        );
        // Total residual energy far below the in-band pass-through case.
        let rms_ob = rms(mid_ob);
        let rms_in = rms(mid_in);
        assert!(
            rms_ob < 0.15 * rms_in,
            "out-of-band residual rms {rms_ob} should be ≪ in-band rms {rms_in}"
        );
    }

    #[test]
    fn rejects_bad_params() {
        let s = [0.0f32; 4];
        let mut o = [0.0f32; 4];
        assert_eq!(
            resample_sinc(&s, 0, 8000, &mut o, 8),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            resample_sinc(&s, 8000, 8000, &mut o, 0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            resample_sinc(&[], 8000, 16000, &mut o, 8),
            Err(AudioError::MalformedAudio)
        );
    }
}
