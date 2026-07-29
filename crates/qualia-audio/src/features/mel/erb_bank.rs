//! `build_erb_bank` — triangular filterbank on the ERB-rate scale (Glasberg &
//! Moore 1990). A gammatone-ish auditory bank: centres are equally spaced on the
//! Equivalent-Rectangular-Bandwidth rate axis, so filter width grows with centre
//! frequency the way cochlear filters do.

use crate::types::AudioError;

/// Hz → ERB-rate number: `E = 21.4 * log10(1 + 0.00437*f)` (Glasberg & Moore).
#[inline]
fn hz_to_erb_rate(hz: f32) -> f32 {
    let f = hz.max(0.0);
    21.4 * (1.0 + 0.00437 * f).log10()
}

/// ERB-rate → Hz, inverse of [`hz_to_erb_rate`]: `f = (10^(E/21.4) - 1) / 0.00437`.
#[inline]
fn erb_rate_to_hz(erb: f32) -> f32 {
    (10.0f32.powf(erb / 21.4) - 1.0) / 0.00437
}

/// Build a triangular ERB filterbank as a row-major `n_bands × n_fft_bins` weight
/// table. Same geometry as the mel/bark banks (peak 1.0 at centre, overlapping,
/// non-negative), with centres equally spaced on the ERB-rate scale — the triangular
/// approximation of a gammatone auditory filterbank.
pub fn build_erb_bank(
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

    let e_lo = hz_to_erb_rate(f_lo);
    let e_hi = hz_to_erb_rate(f_hi);
    let step = (e_hi - e_lo) / (n_bands as f32 + 1.0);

    let n_fft = 2 * (n_fft_bins - 1);
    let hz_per_bin = sample_rate / n_fft as f32;

    weights_out[..n_bands * n_fft_bins].fill(0.0);

    for m in 0..n_bands {
        let left = erb_rate_to_hz(e_lo + m as f32 * step);
        let center = erb_rate_to_hz(e_lo + (m as f32 + 1.0) * step);
        let right = erb_rate_to_hz(e_lo + (m as f32 + 2.0) * step);
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
    fn erb_scale_round_trips() {
        for &hz in &[100.0f32, 500.0, 1000.0, 4000.0] {
            let back = erb_rate_to_hz(hz_to_erb_rate(hz));
            assert!((back - hz).abs() / hz < 1e-3, "{hz} -> {back}");
        }
    }

    #[test]
    fn erb_bandwidth_grows_with_frequency() {
        // Higher ERB bands should be wider in Hz than lower ones.
        let n_bins = 513;
        let n_bands = 32;
        let sr = 22_050.0;
        let mut w = vec![0.0f32; n_bands * n_bins];
        build_erb_bank(n_bins, n_bands, sr, 50.0, 0.0, &mut w).unwrap();
        let width = |m: usize| {
            w[m * n_bins..(m + 1) * n_bins]
                .iter()
                .filter(|&&v| v > 1e-4)
                .count()
        };
        assert!(
            width(n_bands - 2) > width(1),
            "ERB bands do not widen with freq"
        );
    }

    #[test]
    fn erb_filters_peak_at_one() {
        // High FFT resolution so even the narrow low-frequency ERB triangles are
        // well sampled: each well-resolved band reaches ≈1.0, none exceeds 1.0.
        let n_bins = 2049; // n_fft = 4096
        let n_bands = 24;
        let sr = 16_000.0;
        let mut w = vec![0.0f32; n_bands * n_bins];
        build_erb_bank(n_bins, n_bands, sr, 0.0, 8000.0, &mut w).unwrap();
        for m in 0..n_bands {
            let row = &w[m * n_bins..(m + 1) * n_bins];
            let peak = row.iter().cloned().fold(0.0f32, f32::max);
            let active = row.iter().filter(|&&v| v > 1e-4).count();
            assert!(peak <= 1.0001, "band {m} peak exceeds 1: {peak}");
            assert!(row.iter().all(|&v| v >= 0.0));
            if active >= 4 {
                assert!(
                    peak > 0.9,
                    "band {m} peak too low: {peak} (active {active})"
                );
            }
        }
    }
}
