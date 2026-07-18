//! Integrated / momentary / short-term loudness per ITU-R BS.1770 & EBU R128.
//!
//! Pipeline: K-weight the mono signal → 400 ms mean-square gating blocks with
//! 75 % overlap → absolute gate (−70 LUFS) → relative gate (−10 LU) →
//! integrated loudness in LUFS.
//!
//! **Zero-heap.** The per-sample hot path runs the K-weighting cascade through
//! [`BiquadState`] with only scalar/stack state. Gating is done in two streaming
//! passes over the samples using a fixed 4-slot ring of 100 ms sub-block
//! energies — no per-call `Vec`, no block-loudness heap buffer.

use crate::features::filters::biquad::BiquadState;
use crate::features::loudness::k_weighting::k_weighting_coeffs;
use crate::types::AudioError;

/// BS.1770 loudness offset: `L = -0.691 + 10*log10(mean_square)`.
const OFFSET: f64 = -0.691;
/// Absolute gate threshold, LUFS.
const ABS_GATE_LUFS: f64 = -70.0;
/// Relative gate, LU below the ungated mean.
const REL_GATE_LU: f64 = 10.0;

/// Convert a block mean-square `z` to block loudness (LUFS).
#[inline]
fn loudness_from_z(z: f64) -> f64 {
    OFFSET + 10.0 * z.log10()
}

/// Stream the K-weighting cascade over `samples` and invoke `on_block(z)` for
/// every 400 ms gating block (mean-square `z`), hopped every 100 ms (75 %
/// overlap). Returns the number of complete blocks emitted.
///
/// Zero-heap: state is the two biquad histories, a `[f64; 4]` sub-block ring,
/// and a handful of scalars.
fn for_each_gating_block<F>(
    samples: &[f32],
    sample_rate: u32,
    mut on_block: F,
) -> Result<usize, AudioError>
where
    F: FnMut(f64),
{
    if sample_rate == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let sub_len = (sample_rate as usize) / 10; // 100 ms sub-block
    if sub_len == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let block_len = (sub_len * 4) as f64; // 400 ms

    let (shelf, hp) = k_weighting_coeffs(sample_rate as f32);
    let mut s1 = BiquadState::new();
    let mut s2 = BiquadState::new();

    let mut ring = [0.0f64; 4];
    let mut ring_pos = 0usize;
    let mut ring_filled = 0usize;
    let mut sub_acc = 0.0f64;
    let mut sub_count = 0usize;
    let mut n_blocks = 0usize;

    for &x in samples {
        let y = s2.process_sample(&hp, s1.process_sample(&shelf, x)) as f64;
        sub_acc += y * y;
        sub_count += 1;
        if sub_count == sub_len {
            ring[ring_pos] = sub_acc;
            ring_pos = (ring_pos + 1) & 3;
            if ring_filled < 4 {
                ring_filled += 1;
            }
            if ring_filled == 4 {
                let block_ss = ring[0] + ring[1] + ring[2] + ring[3];
                on_block(block_ss / block_len);
                n_blocks += 1;
            }
            sub_acc = 0.0;
            sub_count = 0;
        }
    }
    Ok(n_blocks)
}

/// Integrated loudness (LUFS) of a mono signal per BS.1770 / EBU R128.
///
/// Applies the absolute (−70 LUFS) then relative (−10 LU) gate to the K-weighted
/// 400 ms/75 %-overlap block energies. Returns:
/// - `Err(MalformedAudio)` if the signal is shorter than one 400 ms block,
/// - `Ok(f32::NEG_INFINITY)` for silence (no block clears the absolute gate),
/// - otherwise the gated integrated loudness.
pub fn integrated_lufs(samples: &[f32], sample_rate: u32) -> Result<f32, AudioError> {
    let abs_thresh_z = 10f64.powf((ABS_GATE_LUFS - OFFSET) / 10.0);

    // Pass 1: absolute gate → running sum/count of block energies.
    let mut sum_abs = 0.0f64;
    let mut n_abs = 0usize;
    let n_blocks = for_each_gating_block(samples, sample_rate, |z| {
        if z >= abs_thresh_z {
            sum_abs += z;
            n_abs += 1;
        }
    })?;

    if n_blocks == 0 {
        return Err(AudioError::MalformedAudio);
    }
    if n_abs == 0 {
        return Ok(f32::NEG_INFINITY);
    }

    // Relative threshold: mean of absolute-gated energies, −10 LU (÷10 in power).
    let rel_thresh_z = (sum_abs / n_abs as f64) / 10f64.powf(REL_GATE_LU / 10.0);

    // Pass 2: relative gate → integrated energy.
    let mut sum_rel = 0.0f64;
    let mut n_rel = 0usize;
    for_each_gating_block(samples, sample_rate, |z| {
        if z >= abs_thresh_z && z >= rel_thresh_z {
            sum_rel += z;
            n_rel += 1;
        }
    })?;

    if n_rel == 0 {
        return Ok(f32::NEG_INFINITY);
    }
    Ok(loudness_from_z(sum_rel / n_rel as f64) as f32)
}

/// K-weighted loudness (LUFS) over a single window — the ungated measurement
/// underlying momentary and short-term loudness.
///
/// Zero-heap streaming; no gating. `Err(MalformedAudio)` on empty input.
fn window_lufs(samples: &[f32], sample_rate: u32) -> Result<f32, AudioError> {
    if samples.is_empty() {
        return Err(AudioError::MalformedAudio);
    }
    if sample_rate == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let (shelf, hp) = k_weighting_coeffs(sample_rate as f32);
    let mut s1 = BiquadState::new();
    let mut s2 = BiquadState::new();
    let mut acc = 0.0f64;
    for &x in samples {
        let y = s2.process_sample(&hp, s1.process_sample(&shelf, x)) as f64;
        acc += y * y;
    }
    let z = acc / samples.len() as f64;
    if z <= 0.0 {
        return Ok(f32::NEG_INFINITY);
    }
    Ok(loudness_from_z(z) as f32)
}

/// Momentary loudness (LUFS): K-weighted loudness of the supplied window,
/// intended to hold ~400 ms of audio (BS.1770 momentary integration time).
pub fn momentary_lufs(samples: &[f32], sample_rate: u32) -> Result<f32, AudioError> {
    window_lufs(samples, sample_rate)
}

/// Short-term loudness (LUFS): K-weighted loudness of the supplied window,
/// intended to hold ~3 s of audio (EBU R128 short-term integration time).
pub fn short_term_lufs(samples: &[f32], sample_rate: u32) -> Result<f32, AudioError> {
    window_lufs(samples, sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    /// 1 kHz sine at a chosen RMS level (linear full-scale reference 1.0).
    fn sine_at_rms(rms: f32, n: usize) -> Vec<f32> {
        let amp = rms * core::f32::consts::SQRT_2;
        (0..n)
            .map(|i| amp * (2.0 * PI * 1_000.0 * i as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn integrated_minus_20_dbfs_sine_is_near_minus_20_lufs() {
        // GOLDEN: a 1 kHz sine at −20 dBFS RMS (RMS = 0.1) measures ≈ −20 LUFS
        // after K-weighting (BS.1770 calibration; K-gain ≈ +0.7 dB at 1 kHz).
        let s = sine_at_rms(0.1, 48_000 * 3); // 3 s
        let l = integrated_lufs(&s, 48_000).expect("integrated");
        assert!((l - (-20.0)).abs() < 1.0, "integrated {l} LUFS, expected ~ -20");
    }

    #[test]
    fn momentary_matches_calibration() {
        let s = sine_at_rms(0.1, 48_000 / 2); // 500 ms window
        let m = momentary_lufs(&s, 48_000).expect("momentary");
        assert!((m - (-20.0)).abs() < 1.0, "momentary {m}");
    }

    #[test]
    fn level_change_moves_loudness_by_same_db() {
        // Halving amplitude (−6.02 dB) drops loudness by ~6 LU.
        let a = integrated_lufs(&sine_at_rms(0.1, 48_000 * 2), 48_000).unwrap();
        let b = integrated_lufs(&sine_at_rms(0.05, 48_000 * 2), 48_000).unwrap();
        assert!(((a - b) - 6.02).abs() < 0.2, "delta {} LU", a - b);
    }

    #[test]
    fn too_short_is_malformed() {
        let s = sine_at_rms(0.1, 100); // < 400 ms
        assert_eq!(integrated_lufs(&s, 48_000), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn silence_is_neg_infinity() {
        let s = vec![0.0f32; 48_000];
        assert_eq!(integrated_lufs(&s, 48_000), Ok(f32::NEG_INFINITY));
    }

    #[test]
    fn zero_sample_rate_rejected() {
        let s = sine_at_rms(0.1, 48_000);
        assert_eq!(integrated_lufs(&s, 0), Err(AudioError::InvalidParameter));
    }
}
