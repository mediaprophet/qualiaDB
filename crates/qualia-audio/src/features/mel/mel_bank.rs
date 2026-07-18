//! `build_mel_bank` — precompute a triangular mel filterbank into a caller-owned
//! table. This REPLACES the fake linear-band "log_mel" averaging that lived in
//! `stft_stream.rs`: the bank spaces filters on the perceptual HTK mel scale and
//! places true triangular responses (peak 1.0 at each centre), overlapping into
//! their neighbours — the standard Slaney/HTK filterbank geometry.
//!
//! Build once (cold path); apply per-frame allocation-free via
//! [`super::mel_bands::mel_bands`].

use crate::features::mel::{hz_mel::hz_to_mel, mel_to_hz::mel_to_hz};
use crate::types::AudioError;

/// Build a triangular mel filterbank as a row-major `n_mel × n_fft_bins` table of
/// weights, written into `weights_out`.
///
/// - `n_fft_bins`: number of magnitude/power bins per frame (`n_fft/2 + 1`).
/// - `n_mel`: number of mel filters (rows).
/// - `sample_rate`: sampling rate in Hz (used to map each FFT bin to a frequency).
/// - `fmin` / `fmax`: passband edges in Hz (`fmax <= 0` or above Nyquist → Nyquist).
/// - `weights_out`: at least `n_mel * n_fft_bins` floats; fully overwritten.
///
/// Each filter `m` rises linearly from its lower edge to its centre (peak 1.0),
/// then falls to its upper edge; centres are equally spaced in mel between `fmin`
/// and `fmax`, so adjacent triangles share an edge and overlap. Weights are
/// non-negative. Returns [`AudioError::InvalidParameter`] on degenerate sizes /
/// rates and [`AudioError::OutputBufferTooSmall`] if `weights_out` is short.
pub fn build_mel_bank(
    n_fft_bins: usize,
    n_mel: usize,
    sample_rate: f32,
    fmin: f32,
    fmax: f32,
    weights_out: &mut [f32],
) -> Result<(), AudioError> {
    if n_fft_bins < 2 || n_mel == 0 || sample_rate <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    if weights_out.len() < n_mel * n_fft_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let nyquist = sample_rate * 0.5;
    let f_lo = fmin.max(0.0);
    let f_hi = if fmax <= 0.0 || fmax > nyquist { nyquist } else { fmax };
    if f_lo >= f_hi {
        return Err(AudioError::InvalidParameter);
    }

    // `n_mel + 2` mel-spaced edge frequencies: edges[m], edges[m+1]=centre, edges[m+2].
    let mel_lo = hz_to_mel(f_lo);
    let mel_hi = hz_to_mel(f_hi);
    let mel_step = (mel_hi - mel_lo) / (n_mel as f32 + 1.0);

    // FFT-bin spacing in Hz: bin b sits at b * sample_rate / n_fft, n_fft = 2*(bins-1).
    let n_fft = 2 * (n_fft_bins - 1);
    let hz_per_bin = sample_rate / n_fft as f32;

    weights_out[..n_mel * n_fft_bins].fill(0.0);

    for m in 0..n_mel {
        let left = mel_to_hz(mel_lo + m as f32 * mel_step);
        let center = mel_to_hz(mel_lo + (m as f32 + 1.0) * mel_step);
        let right = mel_to_hz(mel_lo + (m as f32 + 2.0) * mel_step);
        let up = center - left;
        let down = right - center;
        let row = &mut weights_out[m * n_fft_bins..(m + 1) * n_fft_bins];
        for (b, w) in row.iter_mut().enumerate() {
            let f = b as f32 * hz_per_bin;
            let tri = if f >= left && f <= center && up > 0.0 {
                (f - left) / up
            } else if f > center && f <= right && down > 0.0 {
                (right - f) / down
            } else {
                0.0
            };
            *w = tri.max(0.0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argmax(row: &[f32]) -> usize {
        let mut best = 0usize;
        for (b, &v) in row.iter().enumerate() {
            if v > row[best] {
                best = b;
            }
        }
        best
    }

    #[test]
    fn filters_peak_at_one_and_are_nonnegative() {
        // High FFT resolution so the analytic triangle is well sampled: every
        // filter reaches ≈1.0 at its centre and none exceeds 1.0.
        let n_bins = 2049; // n_fft = 4096
        let n_mel = 20;
        let sr = 16_000.0;
        let mut w = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, sr, 0.0, 8000.0, &mut w).unwrap();

        for m in 0..n_mel {
            let row = &w[m * n_bins..(m + 1) * n_bins];
            let peak = row.iter().cloned().fold(0.0f32, f32::max);
            let active = row.iter().filter(|&&v| v > 1e-4).count();
            // Ceiling always holds; non-negativity always holds.
            assert!(peak <= 1.0001, "filter {m} peak exceeds 1: {peak}");
            for &v in row {
                assert!(v >= 0.0, "negative weight in filter {m}");
            }
            // A well-resolved filter (spanning several bins) reaches ~1.0 at its centre.
            if active >= 4 {
                assert!(peak > 0.9, "filter {m} peak too low: {peak} (active {active})");
            }
        }
    }

    #[test]
    fn peak_converges_to_one_at_the_centre() {
        // The analytic triangle peaks at 1.0 at its centre; with fine FFT resolution
        // a bin lands ~on the centre, so the sampled peak → 1.0 (and never exceeds it).
        let n_bins = 8193; // n_fft = 16384, hz/bin ≈ 0.977
        let sr = 16_000.0;
        let mut w = vec![0.0f32; n_bins];
        build_mel_bank(n_bins, 1, sr, 0.0, 8000.0, &mut w).unwrap();
        let peak = w.iter().cloned().fold(0.0f32, f32::max);
        assert!(peak > 0.999 && peak <= 1.0001, "centre peak not ~1.0: {peak}");
    }

    #[test]
    fn adjacent_filters_overlap() {
        let n_bins = 257;
        let n_mel = 26;
        let sr = 16_000.0;
        let mut w = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, sr, 0.0, 8000.0, &mut w).unwrap();

        let mut overlap_found = false;
        for b in 0..n_bins {
            for m in 0..n_mel - 1 {
                if w[m * n_bins + b] > 1e-4 && w[(m + 1) * n_bins + b] > 1e-4 {
                    overlap_found = true;
                }
            }
        }
        assert!(overlap_found, "adjacent mel filters never overlap");
    }

    #[test]
    fn centres_ascend_in_frequency() {
        let n_bins = 513; // n_fft = 1024
        let n_mel = 40;
        let sr = 22_050.0;
        let mut w = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, sr, 0.0, 0.0, &mut w).unwrap();
        let mut last = 0usize;
        for m in 0..n_mel {
            let a = argmax(&w[m * n_bins..(m + 1) * n_bins]);
            assert!(a >= last, "filter {m} centre bin {a} < prev {last}");
            last = a;
        }
    }

    #[test]
    fn rejects_small_output() {
        let mut w = vec![0.0f32; 10];
        assert_eq!(
            build_mel_bank(257, 26, 16_000.0, 0.0, 8000.0, &mut w),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
