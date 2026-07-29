//! Bounded non-power-of-two fallback: a direct DFT floor.
//!
//! This is the documented O(N²) floor used when `N` is not a power of two, so
//! the pipeline always has a correct transform available. It is caller-buffered
//! and zero-heap: the caller supplies both the interleaved-complex input and a
//! separate interleaved-complex output buffer of the same length. For large `N`
//! prefer [`super::radix2::fft_radix2`]; this exists for correctness on awkward
//! sizes, not speed.

use crate::types::AudioError;

/// Direct DFT (or inverse DFT) of arbitrary length `N`.
///
/// `input` and `output` are interleaved complex `[re, im, …]` of length `2 * N`
/// and must be distinct buffers of equal length. Forward uses `e^{-j2πkn/N}`;
/// when `inverse` is true it uses `e^{+j2πkn/N}` and normalises by `1/N`, so a
/// forward followed by an inverse round-trips the signal.
///
/// Returns [`AudioError::InvalidParameter`] if the lengths differ, are odd, or
/// are zero.
pub fn dft_direct(input: &[f32], output: &mut [f32], inverse: bool) -> Result<(), AudioError> {
    if input.len() != output.len() || input.is_empty() || !input.len().is_multiple_of(2) {
        return Err(AudioError::InvalidParameter);
    }
    let n = input.len() / 2;
    let sign: f64 = if inverse { 1.0 } else { -1.0 };
    let base = sign * core::f64::consts::TAU / n as f64;

    for k in 0..n {
        let mut acc_r = 0.0f64;
        let mut acc_i = 0.0f64;
        for m in 0..n {
            let ang = base * (k as f64) * (m as f64);
            let wr = ang.cos();
            let wi = ang.sin();
            let xr = input[2 * m] as f64;
            let xi = input[2 * m + 1] as f64;
            // x · w
            acc_r += xr * wr - xi * wi;
            acc_i += xr * wi + xi * wr;
        }
        if inverse {
            acc_r /= n as f64;
            acc_i /= n as f64;
        }
        output[2 * k] = acc_r as f32;
        output[2 * k + 1] = acc_i as f32;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    #[test]
    fn dft_peak_bin_non_power_of_two() {
        // N = 12 (not a power of two): cosine at k0 must peak at k0.
        let n = 12usize;
        let k0 = 3usize;
        let mut input = vec![0.0f32; 2 * n];
        let mut output = vec![0.0f32; 2 * n];
        for i in 0..n {
            input[2 * i] = (TAU * k0 as f32 * i as f32 / n as f32).cos();
        }
        dft_direct(&input, &mut output, false).unwrap();
        let mags: Vec<f32> = output
            .chunks_exact(2)
            .map(|c| (c[0] * c[0] + c[1] * c[1]).sqrt())
            .collect();
        let mut peak = 1usize;
        for (b, m) in mags.iter().enumerate().take(n / 2 + 1).skip(1) {
            if *m > mags[peak] {
                peak = b;
            }
        }
        assert_eq!(peak, k0);
    }

    #[test]
    fn dft_round_trip() {
        let n = 9usize;
        let orig: Vec<f32> = (0..n).map(|i| (0.7 * i as f32).sin()).collect();
        let mut input = vec![0.0f32; 2 * n];
        let mut fwd = vec![0.0f32; 2 * n];
        let mut back = vec![0.0f32; 2 * n];
        for i in 0..n {
            input[2 * i] = orig[i];
        }
        dft_direct(&input, &mut fwd, false).unwrap();
        dft_direct(&fwd, &mut back, true).unwrap();
        for i in 0..n {
            assert!((back[2 * i] - orig[i]).abs() < 1e-4);
            assert!(back[2 * i + 1].abs() < 1e-4);
        }
    }

    #[test]
    fn rejects_mismatched_lengths() {
        let input = vec![0.0f32; 8];
        let mut output = vec![0.0f32; 6];
        assert_eq!(
            dft_direct(&input, &mut output, false),
            Err(AudioError::InvalidParameter)
        );
    }
}
