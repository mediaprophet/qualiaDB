//! `build_bark_bank` — triangular filterbank on the Bark critical-band scale
//! (Traunmüller 1990), same geometry as the mel bank but with Bark-spaced centres.

use crate::types::AudioError;

/// Hz → Bark (Traunmüller): `z = 26.81*f/(1960+f) - 0.53`.
#[inline]
fn hz_to_bark(hz: f32) -> f32 {
    let f = hz.max(0.0);
    26.81 * f / (1960.0 + f) - 0.53
}

/// Bark → Hz, inverse of [`hz_to_bark`]: `f = 1960*z / (26.81 - z)` with `z = bark + 0.53`.
#[inline]
fn bark_to_hz(bark: f32) -> f32 {
    let z = bark + 0.53;
    let denom = 26.81 - z;
    if denom <= 0.0 {
        return f32::INFINITY;
    }
    1960.0 * z / denom
}

/// Build a triangular Bark filterbank as a row-major `n_bands × n_fft_bins` weight
/// table. Semantics mirror [`super::mel_bank::build_mel_bank`] (peak 1.0 at each
/// centre, overlapping neighbours, non-negative), but centres are equally spaced on
/// the Bark scale.
pub fn build_bark_bank(
    n_fft_bins: usize,
    n_bands: usize,
    sample_rate: f32,
    fmin: f32,
    fmax: f32,
    weights_out: &mut [f32],
) -> Result<(), AudioError> {
    if n_fft_bins < 2 || n_bands == 0 || sample_rate <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    if weights_out.len() < n_bands * n_fft_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let nyquist = sample_rate * 0.5;
    let f_lo = fmin.max(0.0);
    let f_hi = if fmax <= 0.0 || fmax > nyquist {
        nyquist
    } else {
        fmax
    };
    if f_lo >= f_hi {
        return Err(AudioError::InvalidParameter);
    }

    let b_lo = hz_to_bark(f_lo);
    let b_hi = hz_to_bark(f_hi);
    let step = (b_hi - b_lo) / (n_bands as f32 + 1.0);

    let n_fft = 2 * (n_fft_bins - 1);
    let hz_per_bin = sample_rate / n_fft as f32;

    weights_out[..n_bands * n_fft_bins].fill(0.0);

    for m in 0..n_bands {
        let left = bark_to_hz(b_lo + m as f32 * step);
        let center = bark_to_hz(b_lo + (m as f32 + 1.0) * step);
        let right = bark_to_hz(b_lo + (m as f32 + 2.0) * step);
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

    #[test]
    fn bark_scale_round_trips() {
        for &hz in &[100.0f32, 500.0, 1000.0, 4000.0] {
            let back = bark_to_hz(hz_to_bark(hz));
            assert!((back - hz).abs() / hz < 1e-3, "{hz} -> {back}");
        }
    }

    #[test]
    fn bark_filters_peak_at_one_and_overlap() {
        let n_bins = 257;
        let n_bands = 24;
        let sr = 16_000.0;
        let mut w = vec![0.0f32; n_bands * n_bins];
        build_bark_bank(n_bins, n_bands, sr, 0.0, 8000.0, &mut w).unwrap();
        for m in 0..n_bands {
            let row = &w[m * n_bins..(m + 1) * n_bins];
            let peak = row.iter().cloned().fold(0.0f32, f32::max);
            assert!(peak > 0.85 && peak <= 1.0001, "band {m} peak {peak}");
            assert!(row.iter().all(|&v| v >= 0.0));
        }
        let mut overlap = false;
        for b in 0..n_bins {
            for m in 0..n_bands - 1 {
                if w[m * n_bins + b] > 1e-4 && w[(m + 1) * n_bins + b] > 1e-4 {
                    overlap = true;
                }
            }
        }
        assert!(overlap, "bark bands do not overlap");
    }
}
