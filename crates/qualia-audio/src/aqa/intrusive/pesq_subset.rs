//! A small, HONESTLY-LABELLED subset of PESQ-style perceptual processing.
//!
//! # This is NOT PESQ / POLQA
//!
//! ITU-T P.862 (PESQ) and P.863 (POLQA) are large, licence-encumbered, formally
//! validated reference algorithms with time alignment, a full auditory model,
//! asymmetry weighting and a trained mapping to the MOS scale. **This function is
//! none of that.** It reproduces only three of PESQ's *ideas*, on already-aligned,
//! already-framed magnitude spectra, and returns a **raw distortion score** — a
//! non-negative number where 0 is identical and larger is worse. It is **not**
//! calibrated to MOS and must never be reported as a PESQ/POLQA score.
//!
//! The three borrowed steps, applied per matched frame pair:
//! 1. **Level alignment.** Scale the degraded spectrum so its total power matches
//!    the reference (PESQ aligns overall gain before comparison, so a pure volume
//!    change is not counted as distortion).
//! 2. **Bark-domain warp.** Group linear FFT bins into critical bands on the Bark
//!    scale (Traunmüller), summing power per band — a coarse stand-in for PESQ's
//!    pitch-power-density stage.
//! 3. **Loudness compression + difference.** Compress band powers with a Zwicker
//!    style `p^0.23` law (perceived loudness grows sub-linearly), then take the
//!    mean absolute band-loudness difference as the disturbance for that frame.
//!
//! Frames are averaged. Pure Rust, allocation-free (fixed Bark-band accumulators),
//! scalar return.

use crate::types::AudioError;

/// Number of Bark critical bands used for the coarse warp (covers ~0–15.5 Bark,
/// enough for speech/wideband; fixed so the accumulator is stack-allocated).
pub const BARK_BANDS: usize = 24;

/// Hz → Bark (Traunmüller 1990), matching `features::mel::bark_bank`.
#[inline]
fn hz_to_bark(hz: f32) -> f32 {
    let f = hz.max(0.0);
    26.81 * f / (1960.0 + f) - 0.53
}

/// PESQ-*subset* raw distortion score between two magnitude spectra.
///
/// - `reference_mag` / `degraded_mag`: linear magnitude spectra with the same
///   number of bins, laid out as `k = 0..=N/2` (DC..Nyquist), e.g. the output of
///   [`crate::features::fft::real_fft_magnitude`].
/// - `sample_rate`: sampling rate in Hz (used to place bins on the Bark scale).
///
/// Returns a raw disturbance score ≥ 0. `0.0` means the two spectra are identical
/// up to overall level; larger values mean greater perceptual distortion. This is
/// a distortion (lower is better), NOT a MOS. See the module docs: it is a
/// documented subset, not certified PESQ/POLQA.
///
/// Returns [`AudioError::InvalidParameter`] if the spectra are empty, differ in
/// length, or `sample_rate` is not positive.
pub fn pesq_subset(
    reference_mag: &[f32],
    degraded_mag: &[f32],
    sample_rate: f32,
) -> Result<f32, AudioError> {
    if reference_mag.is_empty()
        || reference_mag.len() != degraded_mag.len()
        || sample_rate <= 0.0
    {
        return Err(AudioError::InvalidParameter);
    }

    let bins = reference_mag.len();
    // For a real FFT, bins = N/2 + 1, so N = 2*(bins-1). Guard the 1-bin case.
    let n_fft = if bins > 1 { 2 * (bins - 1) } else { 2 };
    let hz_per_bin = sample_rate / n_fft as f32;

    // --- Step 1: level alignment (match total power). ---
    let mut ref_power = 0.0f32;
    let mut deg_power = 0.0f32;
    for k in 0..bins {
        ref_power += reference_mag[k] * reference_mag[k];
        deg_power += degraded_mag[k] * degraded_mag[k];
    }
    const EPS: f32 = 1.0e-12;
    // Gain applied to degraded power so its total matches the reference.
    let gain_power = if deg_power > EPS {
        (ref_power + EPS) / (deg_power + EPS)
    } else {
        1.0
    };

    // --- Step 2: warp linear power into fixed Bark bands. ---
    let mut ref_band = [0.0f32; BARK_BANDS];
    let mut deg_band = [0.0f32; BARK_BANDS];
    // Bark span mapped onto BARK_BANDS bins.
    let bark_max = hz_to_bark(sample_rate * 0.5).max(EPS);
    let band_width = bark_max / BARK_BANDS as f32;

    for k in 0..bins {
        let hz = k as f32 * hz_per_bin;
        let bark = hz_to_bark(hz);
        let mut b = (bark / band_width) as isize;
        if b < 0 {
            b = 0;
        }
        let b = (b as usize).min(BARK_BANDS - 1);
        ref_band[b] += reference_mag[k] * reference_mag[k];
        deg_band[b] += degraded_mag[k] * degraded_mag[k] * gain_power;
    }

    // --- Step 3: loudness compression + mean absolute band difference. ---
    // Zwicker-style sub-linear loudness growth.
    const LOUDNESS_EXP: f32 = 0.23;
    let mut disturbance = 0.0f32;
    for b in 0..BARK_BANDS {
        let lr = (ref_band[b] + EPS).powf(LOUDNESS_EXP);
        let ld = (deg_band[b] + EPS).powf(LOUDNESS_EXP);
        disturbance += (lr - ld).abs();
    }

    Ok(disturbance / BARK_BANDS as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_mag(bins: usize, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..bins)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / (1u32 << 24) as f32 // [0,1)
            })
            .collect()
    }

    #[test]
    fn identical_signals_score_best_zero() {
        let mag = synth_mag(513, 7);
        let score = pesq_subset(&mag, &mag, 16_000.0).expect("valid");
        assert!(
            score.abs() < 1e-4,
            "identical spectra must give the best (≈0) distortion, got {score}"
        );
    }

    #[test]
    fn pure_level_change_is_forgiven() {
        // Level alignment should make a scaled copy nearly identical.
        let mag = synth_mag(513, 11);
        let louder: Vec<f32> = mag.iter().map(|&v| v * 3.0).collect();
        let score = pesq_subset(&mag, &louder, 16_000.0).expect("valid");
        assert!(
            score < 1e-2,
            "a pure gain change should score near-zero, got {score}"
        );
    }

    #[test]
    fn spectral_distortion_scores_worse_than_identity() {
        let mag = synth_mag(513, 3);
        let identity = pesq_subset(&mag, &mag, 16_000.0).expect("valid");

        // A genuinely different spectrum (different noise realisation).
        let other = synth_mag(513, 999);
        let distorted = pesq_subset(&mag, &other, 16_000.0).expect("valid");

        assert!(
            distorted > identity,
            "distorted {distorted} must score worse than identical {identity}"
        );
        assert!(distorted.is_finite());
    }

    #[test]
    fn more_distortion_scores_worse() {
        let mag = synth_mag(513, 42);
        let mild: Vec<f32> = mag
            .iter()
            .enumerate()
            .map(|(k, &v)| if k % 8 == 0 { v * 1.2 } else { v })
            .collect();
        let heavy: Vec<f32> = mag
            .iter()
            .enumerate()
            .map(|(k, &v)| if k % 2 == 0 { v * 4.0 } else { v * 0.1 })
            .collect();
        let s_mild = pesq_subset(&mag, &mild, 16_000.0).expect("valid");
        let s_heavy = pesq_subset(&mag, &heavy, 16_000.0).expect("valid");
        assert!(
            s_heavy > s_mild,
            "heavier distortion {s_heavy} must exceed milder {s_mild}"
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(
            pesq_subset(&[], &[], 16_000.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            pesq_subset(&[1.0, 2.0], &[1.0], 16_000.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            pesq_subset(&[1.0, 2.0], &[1.0, 2.0], 0.0),
            Err(AudioError::InvalidParameter)
        );
    }
}
