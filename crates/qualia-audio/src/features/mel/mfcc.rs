//! `mfcc` — Mel-Frequency Cepstral Coefficients: mel bank → log → DCT-II → first
//! `n_coeffs`. This is the cepstral core reused by [`super::bfcc`] and
//! [`super::gfcc`] with Bark / gammatone banks.

use crate::features::mel::dct::dct2;
use crate::types::AudioError;

/// Log-energy floor so silent bands map to a large-negative (finite) log, never `-inf`.
const LOG_FLOOR: f32 = 1e-10;

/// Compute the first `n_coeffs` MFCCs of one power-spectrum frame, allocation-free.
///
/// Pipeline: apply the triangular `bank_weights` (`n_mel × n_fft_bins`, row-major),
/// take `ln(energy + 1e-10)` per band, run an orthonormal DCT-II, and copy the
/// leading `n_coeffs` cepstral coefficients into `out`. Coefficient 0 (`c0`) is the
/// overall log-energy; higher coefficients describe spectral shape.
///
/// - `power_spectrum`: ≥ `n_fft_bins` (= `bank_weights.len()/n_mel`) values.
/// - `bank_weights`: the precomputed bank.
/// - `n_mel`: number of bank rows.
/// - `n_coeffs`: how many leading coefficients to emit (`1..=n_mel`).
/// - `out`: ≥ `n_coeffs` floats.
/// - `scratch`: ≥ `2 * n_mel` floats (log-mel buffer + DCT output); clobbered.
///
/// Returns [`AudioError::InvalidParameter`] for degenerate sizes and
/// [`AudioError::OutputBufferTooSmall`] if `out` or `scratch` are short.
pub fn mfcc(
    power_spectrum: &[f32],
    bank_weights: &[f32],
    n_mel: usize,
    n_coeffs: usize,
    out: &mut [f32],
    scratch: &mut [f32],
) -> Result<(), AudioError> {
    if n_mel == 0
        || n_coeffs == 0
        || n_coeffs > n_mel
        || bank_weights.is_empty()
        || bank_weights.len() % n_mel != 0
    {
        return Err(AudioError::InvalidParameter);
    }
    let n_fft_bins = bank_weights.len() / n_mel;
    if power_spectrum.len() < n_fft_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }
    if out.len() < n_coeffs || scratch.len() < 2 * n_mel {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let (log_mel, dct_out) = scratch.split_at_mut(n_mel);
    let log_mel = &mut log_mel[..n_mel];
    let dct_out = &mut dct_out[..n_mel];

    for m in 0..n_mel {
        let row = &bank_weights[m * n_fft_bins..(m + 1) * n_fft_bins];
        let mut acc = 0.0f32;
        for (w, p) in row.iter().zip(power_spectrum.iter()) {
            acc += w * p;
        }
        log_mel[m] = (acc + LOG_FLOOR).ln();
    }

    dct2(log_mel, dct_out)?;
    out[..n_coeffs].copy_from_slice(&dct_out[..n_coeffs]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mel::mel_bank::build_mel_bank;

    #[test]
    fn c0_dominates_for_single_tone() {
        let n_bins = 257;
        let n_mel = 26;
        let n_coeffs = 13;
        let sr = 16_000.0;
        let mut bank = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, sr, 0.0, 8000.0, &mut bank).unwrap();

        // Single-tone power spectrum: energy concentrated in one FFT bin (~2 kHz).
        let mut power = vec![0.0f32; n_bins];
        power[64] = 100.0;

        let mut out = vec![0.0f32; n_coeffs];
        let mut scratch = vec![0.0f32; 2 * n_mel];
        mfcc(&power, &bank, n_mel, n_coeffs, &mut out, &mut scratch).unwrap();

        // c0 (log-energy) must be the largest-magnitude coefficient.
        let c0 = out[0].abs();
        for (k, &c) in out.iter().enumerate().skip(1) {
            assert!(
                c0 > c.abs(),
                "c0 ({c0}) does not dominate coeff {k} ({})",
                c.abs()
            );
        }
    }

    #[test]
    fn rejects_small_scratch() {
        let n_bins = 5;
        let n_mel = 4;
        let bank = vec![0.1f32; n_mel * n_bins];
        let power = vec![1.0f32; n_bins];
        let mut out = vec![0.0f32; 2];
        let mut scratch = vec![0.0f32; 3]; // needs 2*n_mel = 8
        assert_eq!(
            mfcc(&power, &bank, n_mel, 2, &mut out, &mut scratch),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
