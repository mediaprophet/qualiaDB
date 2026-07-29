//! `mel_bands` — apply a precomputed triangular bank to one power (or magnitude)
//! spectrum frame, allocation-free (the zero-heap hot path).

use crate::types::AudioError;

/// Apply a `n_mel × n_fft_bins` bank (row-major, from
/// [`super::mel_bank::build_mel_bank`]) to one `power_spectrum` frame.
///
/// For each filter row `m`, computes the dot product `Σ_b bank[m,b] * power[b]`
/// and writes it to `out[m]`. This is a pure matrix-vector apply: no allocation,
/// no logarithm (callers wanting log-mel take `ln` themselves, or use
/// [`super::mfcc::mfcc`]). The number of FFT bins is inferred from
/// `bank_weights.len() / n_mel`, and `power_spectrum` must cover at least that many.
///
/// Returns [`AudioError::InvalidParameter`] if `n_mel == 0` or the bank length is
/// not a positive multiple of `n_mel`, and [`AudioError::OutputBufferTooSmall`] if
/// `out` is shorter than `n_mel` or `power_spectrum` is shorter than the bank width.
pub fn mel_bands(
    power_spectrum: &[f32],
    bank_weights: &[f32],
    n_mel: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if n_mel == 0 || bank_weights.is_empty() || bank_weights.len() % n_mel != 0 {
        return Err(AudioError::InvalidParameter);
    }
    let n_fft_bins = bank_weights.len() / n_mel;
    if out.len() < n_mel || power_spectrum.len() < n_fft_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for m in 0..n_mel {
        let row = &bank_weights[m * n_fft_bins..(m + 1) * n_fft_bins];
        let mut acc = 0.0f32;
        for (w, p) in row.iter().zip(power_spectrum.iter()) {
            acc += w * p;
        }
        out[m] = acc;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mel::mel_bank::build_mel_bank;

    #[test]
    fn flat_spectrum_gives_nonzero_bands() {
        let n_bins = 257;
        let n_mel = 20;
        let mut bank = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, 16_000.0, 0.0, 8000.0, &mut bank).unwrap();
        let power = vec![1.0f32; n_bins];
        let mut out = vec![0.0f32; n_mel];
        mel_bands(&power, &bank, n_mel, &mut out).unwrap();
        for (m, &e) in out.iter().enumerate() {
            assert!(
                e > 0.0,
                "band {m} energy should be positive under flat input"
            );
        }
    }

    #[test]
    fn tone_lands_in_a_single_band_region() {
        // Delta at a specific bin should energise the bands whose triangles cover it.
        let n_bins = 257;
        let n_mel = 20;
        let sr = 16_000.0;
        let mut bank = vec![0.0f32; n_mel * n_bins];
        build_mel_bank(n_bins, n_mel, sr, 0.0, 8000.0, &mut bank).unwrap();
        let mut power = vec![0.0f32; n_bins];
        power[128] = 5.0; // ~4 kHz
        let mut out = vec![0.0f32; n_mel];
        mel_bands(&power, &bank, n_mel, &mut out).unwrap();
        let total: f32 = out.iter().sum();
        assert!(total > 0.0);
        // At most a couple of adjacent bands should carry the delta.
        let active = out.iter().filter(|&&e| e > 1e-4).count();
        assert!(active <= 3, "delta spread across too many bands: {active}");
    }

    #[test]
    fn rejects_short_out() {
        let bank = vec![0.0f32; 40];
        let power = vec![0.0f32; 20];
        let mut out = vec![0.0f32; 1];
        assert_eq!(
            mel_bands(&power, &bank, 2, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
