//! Real-input forward FFT producing the `N/2 + 1` magnitude spectrum.
//!
//! Built on [`super::radix2::fft_radix2`]. Caller-buffered and zero-heap: the
//! caller supplies a `2 * N` interleaved-complex scratch buffer (overwritten)
//! and an `N/2 + 1` magnitude output buffer.

use crate::features::fft::radix2::fft_radix2;
use crate::types::AudioError;

/// Forward FFT of `N` real samples, writing the magnitude of the first
/// `N/2 + 1` bins (the non-redundant half for real input).
///
/// - `input`: `N` real samples, `N` a power of two.
/// - `scratch`: exactly `2 * N` floats; used as interleaved complex work space
///   and clobbered on return.
/// - `out_mags`: at least `N/2 + 1` floats; magnitudes `|X[k]|` for
///   `k = 0..=N/2` are written into the first `N/2 + 1` slots.
///
/// Returns [`AudioError::InvalidParameter`] if `N` is 0 or not a power of two,
/// or [`AudioError::OutputBufferTooSmall`] if `scratch` or `out_mags` are short.
pub fn real_fft_magnitude(
    input: &[f32],
    scratch: &mut [f32],
    out_mags: &mut [f32],
) -> Result<(), AudioError> {
    let n = input.len();
    if n == 0 || !n.is_power_of_two() {
        return Err(AudioError::InvalidParameter);
    }
    let bins = n / 2 + 1;
    if scratch.len() < 2 * n || out_mags.len() < bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Pack real input into interleaved complex [re, 0].
    for i in 0..n {
        scratch[2 * i] = input[i];
        scratch[2 * i + 1] = 0.0;
    }
    fft_radix2(&mut scratch[..2 * n], false)?;

    for k in 0..bins {
        let re = scratch[2 * k];
        let im = scratch[2 * k + 1];
        out_mags[k] = (re * re + im * im).sqrt();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    #[test]
    fn tone_energy_lands_in_expected_bin() {
        // 440 Hz-like tone: choose N and fs, pick the bin nearest 440 Hz, and
        // synthesise exactly at that bin's centre frequency so it lands cleanly.
        let n = 1024usize;
        let fs = 44_100.0f32;
        let k0 = (440.0 * n as f32 / fs).round() as usize; // ≈ bin 10
        let freq = k0 as f32 * fs / n as f32; // exact bin-centre freq (~430.7 Hz)

        let input: Vec<f32> =
            (0..n).map(|i| (TAU * freq * i as f32 / fs).cos()).collect();
        let mut scratch = vec![0.0f32; 2 * n];
        let mut mags = vec![0.0f32; n / 2 + 1];
        real_fft_magnitude(&input, &mut scratch, &mut mags).unwrap();

        let mut peak = 1usize;
        for (b, m) in mags.iter().enumerate().skip(1) {
            if *m > mags[peak] {
                peak = b;
            }
        }
        assert_eq!(peak, k0, "tone energy expected in bin {k0}, found {peak}");
    }

    #[test]
    fn dc_input_peaks_at_bin_zero() {
        let n = 16usize;
        let input = vec![1.0f32; n];
        let mut scratch = vec![0.0f32; 2 * n];
        let mut mags = vec![0.0f32; n / 2 + 1];
        real_fft_magnitude(&input, &mut scratch, &mut mags).unwrap();
        assert!((mags[0] - n as f32).abs() < 1e-3);
        for m in &mags[1..] {
            assert!(*m < 1e-3);
        }
    }

    #[test]
    fn rejects_short_scratch() {
        let input = vec![0.0f32; 8];
        let mut scratch = vec![0.0f32; 8]; // needs 16
        let mut mags = vec![0.0f32; 5];
        assert_eq!(
            real_fft_magnitude(&input, &mut scratch, &mut mags),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
