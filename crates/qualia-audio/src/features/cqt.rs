//! CPU constant-Q magnitude spectrum (cold/stream-friendly block).

use crate::types::AudioError;

/// Forward CQT magnitudes for mono `samples` (one vector of `n_bins`).
pub fn forward_cqt_mono(
    samples: &[f32],
    sample_rate: f32,
    f_min: f32,
    bins_per_octave: usize,
    n_bins: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if out.len() < n_bins || bins_per_octave == 0 || sample_rate <= 0.0 || f_min <= 0.0 {
        return Err(AudioError::MalformedAudio);
    }
    if samples.is_empty() {
        out[..n_bins].fill(0.0);
        return Ok(());
    }
    let q = 1.0_f32 / (2.0_f32.powf(1.0 / bins_per_octave as f32) - 1.0);
    for k in 0..n_bins {
        let f_k = f_min * 2.0_f32.powf(k as f32 / bins_per_octave as f32);
        let nk = ((q * sample_rate / f_k).round() as usize).clamp(32, samples.len());
        // Center window in the buffer for better tone response.
        let start = samples.len().saturating_sub(nk) / 2;
        let mut acc_re = 0.0f32;
        let mut acc_im = 0.0f32;
        for j in 0..nk {
            let w = if nk <= 1 {
                1.0
            } else {
                0.5 * (1.0 - (core::f32::consts::TAU * j as f32 / (nk - 1) as f32).cos())
            };
            let theta = -core::f32::consts::TAU * f_k * j as f32 / sample_rate;
            let s = samples[start + j] * w;
            acc_re += s * theta.cos();
            acc_im += s * theta.sin();
        }
        let inv = 1.0 / nk as f32;
        out[k] = ((acc_re * inv).powi(2) + (acc_im * inv).powi(2)).sqrt();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cqt_nonzero_on_tone() {
        let sr = 16000.0f32;
        let n = 4096;
        let mut s = vec![0.0f32; n];
        for i in 0..n {
            s[i] = (core::f32::consts::TAU * 440.0 * i as f32 / sr).sin();
        }
        let mut out = [0.0f32; 24];
        forward_cqt_mono(&s, sr, 55.0, 12, 24, &mut out).unwrap();
        assert!(out.iter().cloned().fold(0.0f32, f32::max) > 1e-6);
    }
}
