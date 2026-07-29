//! `bfcc` — Bark-Frequency Cepstral Coefficients: Bark bank → log → DCT-II.
//!
//! Same cepstral core as [`super::mfcc::mfcc`], driven by a Bark bank built with
//! [`super::bark_bank::build_bark_bank`] instead of a mel bank. Kept as a distinct
//! entry point (perceptually a different frequency warping) that reuses the MFCC
//! pipeline rather than duplicating it.

use crate::features::mel::mfcc::mfcc;
use crate::types::AudioError;

/// Compute the first `n_coeffs` Bark cepstral coefficients of one power-spectrum
/// frame. `bark_weights` is an `n_bands × n_fft_bins` bank from
/// [`super::bark_bank::build_bark_bank`]. See [`super::mfcc::mfcc`] for buffer
/// requirements (`out` ≥ `n_coeffs`, `scratch` ≥ `2 * n_bands`); the pipeline is
/// bank → `ln` → orthonormal DCT-II → first `n_coeffs`.
pub fn bfcc(
    power_spectrum: &[f32],
    bark_weights: &[f32],
    n_bands: usize,
    n_coeffs: usize,
    out: &mut [f32],
    scratch: &mut [f32],
) -> Result<(), AudioError> {
    mfcc(
        power_spectrum,
        bark_weights,
        n_bands,
        n_coeffs,
        out,
        scratch,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mel::bark_bank::build_bark_bank;

    #[test]
    fn bfcc_c0_dominates_for_single_tone() {
        let n_bins = 257;
        let n_bands = 24;
        let n_coeffs = 13;
        let sr = 16_000.0;
        let mut bank = vec![0.0f32; n_bands * n_bins];
        build_bark_bank(n_bins, n_bands, sr, 0.0, 8000.0, &mut bank).unwrap();

        let mut power = vec![0.0f32; n_bins];
        power[64] = 100.0;

        let mut out = vec![0.0f32; n_coeffs];
        let mut scratch = vec![0.0f32; 2 * n_bands];
        bfcc(&power, &bank, n_bands, n_coeffs, &mut out, &mut scratch).unwrap();

        let c0 = out[0].abs();
        for (k, &c) in out.iter().enumerate().skip(1) {
            assert!(
                c0 > c.abs(),
                "c0 {c0} does not dominate coeff {k} ({})",
                c.abs()
            );
        }
    }
}
