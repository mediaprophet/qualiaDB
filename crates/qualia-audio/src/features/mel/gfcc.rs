//! `gfcc` — Gammatone-Frequency Cepstral Coefficients: ERB/gammatone bank → log →
//! DCT-II.
//!
//! Same cepstral core as [`super::mfcc::mfcc`], driven by an ERB (gammatone-ish)
//! bank built with [`super::erb_bank::build_erb_bank`]. GFCCs are the noise-robust
//! auditory cousin of MFCCs; here they share the MFCC pipeline (bank → `ln` → DCT-II)
//! rather than duplicating it.

use crate::features::mel::mfcc::mfcc;
use crate::types::AudioError;

/// Compute the first `n_coeffs` gammatone cepstral coefficients of one
/// power-spectrum frame. `erb_weights` is an `n_bands × n_fft_bins` bank from
/// [`super::erb_bank::build_erb_bank`]. See [`super::mfcc::mfcc`] for buffer
/// requirements (`out` ≥ `n_coeffs`, `scratch` ≥ `2 * n_bands`).
pub fn gfcc(
    power_spectrum: &[f32],
    erb_weights: &[f32],
    n_bands: usize,
    n_coeffs: usize,
    out: &mut [f32],
    scratch: &mut [f32],
) -> Result<(), AudioError> {
    mfcc(power_spectrum, erb_weights, n_bands, n_coeffs, out, scratch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mel::erb_bank::build_erb_bank;

    #[test]
    fn gfcc_c0_dominates_for_single_tone() {
        let n_bins = 257;
        let n_bands = 24;
        let n_coeffs = 13;
        let sr = 16_000.0;
        let mut bank = vec![0.0f32; n_bands * n_bins];
        build_erb_bank(n_bins, n_bands, sr, 0.0, 8000.0, &mut bank).unwrap();

        let mut power = vec![0.0f32; n_bins];
        power[64] = 100.0;

        let mut out = vec![0.0f32; n_coeffs];
        let mut scratch = vec![0.0f32; 2 * n_bands];
        gfcc(&power, &bank, n_bands, n_coeffs, &mut out, &mut scratch).unwrap();

        let c0 = out[0].abs();
        for (k, &c) in out.iter().enumerate().skip(1) {
            assert!(c0 > c.abs(), "c0 {c0} does not dominate coeff {k} ({})", c.abs());
        }
    }
}
