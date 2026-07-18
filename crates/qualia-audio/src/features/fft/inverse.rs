//! ISTFT-friendly inverse helper: inverse FFT then extract the real part.
//!
//! Built on [`super::radix2::fft_radix2`]. Caller-buffered and zero-heap: the
//! interleaved-complex spectrum is inverse-transformed in place, then the real
//! components are copied into a caller-supplied real buffer.

use crate::features::fft::radix2::fft_radix2;
use crate::types::AudioError;

/// Inverse FFT of an interleaved-complex spectrum, writing the real part of the
/// time-domain result.
///
/// - `spectrum`: `2 * N` interleaved complex bins, `N` a power of two. Clobbered
///   in place (holds the full complex inverse on return).
/// - `out_real`: at least `N` floats; receives `Re(x[n])` for `n = 0..N`.
///
/// The inverse is `1/N`-normalised (via [`fft_radix2`]), so pairing this with a
/// forward real FFT reconstructs the original samples — the ISTFT synthesis path.
///
/// Returns [`AudioError::InvalidParameter`] if the spectrum length is invalid,
/// or [`AudioError::OutputBufferTooSmall`] if `out_real` is short.
pub fn ifft_to_real(spectrum: &mut [f32], out_real: &mut [f32]) -> Result<(), AudioError> {
    if spectrum.is_empty() || !spectrum.len().is_multiple_of(2) {
        return Err(AudioError::InvalidParameter);
    }
    let n = spectrum.len() / 2;
    if out_real.len() < n {
        return Err(AudioError::OutputBufferTooSmall);
    }
    fft_radix2(spectrum, true)?;
    for i in 0..n {
        out_real[i] = spectrum[2 * i];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::fft::radix2::fft_radix2;
    use core::f32::consts::TAU;

    #[test]
    fn forward_then_ifft_to_real_reconstructs() {
        let n = 64usize;
        let orig: Vec<f32> =
            (0..n).map(|i| (TAU * 3.0 * i as f32 / n as f32).cos() * 0.8).collect();
        let mut spec = vec![0.0f32; 2 * n];
        for i in 0..n {
            spec[2 * i] = orig[i];
        }
        fft_radix2(&mut spec, false).unwrap();

        let mut out = vec![0.0f32; n];
        ifft_to_real(&mut spec, &mut out).unwrap();
        for i in 0..n {
            assert!((out[i] - orig[i]).abs() < 1e-4, "sample {i}: {} vs {}", out[i], orig[i]);
        }
    }

    #[test]
    fn rejects_short_output() {
        let mut spec = vec![0.0f32; 2 * 8];
        let mut out = vec![0.0f32; 4]; // needs 8
        assert_eq!(
            ifft_to_real(&mut spec, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
