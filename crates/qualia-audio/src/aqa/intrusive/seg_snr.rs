//! Segmental signal-to-noise ratio (seg-SNR) between a reference and a degraded
//! signal.
//!
//! Classic intrusive metric: split both signals into fixed-length frames, compute
//! the per-frame SNR in dB (reference energy over error energy), clamp each frame
//! to a sensible dynamic range, and average across frames. Unlike a global SNR,
//! the segmental variant weights quiet and loud passages more evenly, which
//! correlates better with perceived quality.
//!
//! Pure-Rust, no licence encumbrance, zero heap: a single scalar `f32` is
//! returned and the two input slices are read in place.

use crate::types::AudioError;

/// Per-frame SNR is clamped to `[MIN_FRAME_SNR_DB, MAX_FRAME_SNR_DB]` before
/// averaging. These match the conventional seg-SNR range used in speech-quality
/// literature and keep silent/identical frames from dominating the mean with
/// `+inf`.
pub const MAX_FRAME_SNR_DB: f32 = 35.0;
pub const MIN_FRAME_SNR_DB: f32 = -10.0;

/// Segmental SNR in dB between `reference` and `degraded`.
///
/// - `reference`: the clean/reference signal.
/// - `degraded`: the processed/degraded signal, aligned sample-for-sample with
///   `reference`. Must be the same length.
/// - `frame_len`: samples per frame (e.g. 256). Must be non-zero.
///
/// The error signal is `degraded - reference`. For each full frame the SNR is
/// `10 * log10(ref_energy / err_energy)`, clamped to
/// `[MIN_FRAME_SNR_DB, MAX_FRAME_SNR_DB]`; a frame whose error energy is zero
/// (identical) clamps to `MAX_FRAME_SNR_DB`. The returned value is the mean over
/// all full frames. A trailing partial frame shorter than `frame_len` is ignored.
///
/// Returns [`AudioError::InvalidParameter`] if `frame_len` is 0 or the lengths
/// differ, and [`AudioError::MalformedAudio`] if there is not even one full frame.
pub fn segmental_snr(
    reference: &[f32],
    degraded: &[f32],
    frame_len: usize,
) -> Result<f32, AudioError> {
    if frame_len == 0 || reference.len() != degraded.len() {
        return Err(AudioError::InvalidParameter);
    }
    let n_frames = reference.len() / frame_len;
    if n_frames == 0 {
        return Err(AudioError::MalformedAudio);
    }

    // Tiny floor so an all-silent reference frame does not divide by zero; well
    // below any real signal energy.
    const EPS: f32 = 1.0e-12;

    let mut acc_db = 0.0f32;
    for f in 0..n_frames {
        let base = f * frame_len;
        let mut ref_energy = 0.0f32;
        let mut err_energy = 0.0f32;
        for i in 0..frame_len {
            let r = reference[base + i];
            let d = degraded[base + i];
            let e = d - r;
            ref_energy += r * r;
            err_energy += e * e;
        }

        let snr_db = if err_energy <= EPS {
            // Identical (or numerically identical) frame → best case.
            MAX_FRAME_SNR_DB
        } else {
            let ratio = (ref_energy + EPS) / err_energy;
            10.0 * ratio.log10()
        };
        acc_db += snr_db.clamp(MIN_FRAME_SNR_DB, MAX_FRAME_SNR_DB);
    }

    Ok(acc_db / n_frames as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    fn tone(n: usize, freq_bin: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (TAU * freq_bin * i as f32 / n as f32).sin())
            .collect()
    }

    #[test]
    fn identical_signals_hit_clamped_maximum() {
        let x = tone(2048, 20.0);
        let snr = segmental_snr(&x, &x, 256).expect("valid");
        // Identical → every frame clamps to the max.
        assert!((snr - MAX_FRAME_SNR_DB).abs() < 1e-4, "snr={snr}");
        assert!(snr > 30.0);
    }

    #[test]
    fn noisy_signal_has_lower_finite_snr_than_clean() {
        let x = tone(4096, 30.0);

        // Deterministic pseudo-noise added to the reference.
        let mut state: u32 = 0x1234_5678;
        let noisy: Vec<f32> = x
            .iter()
            .map(|&v| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let u = (state >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
                v + (u - 0.5) * 0.4
            })
            .collect();

        let snr_clean = segmental_snr(&x, &x, 256).expect("valid");
        let snr_noisy = segmental_snr(&x, &noisy, 256).expect("valid");

        assert!(snr_noisy.is_finite(), "noisy snr must be finite");
        assert!(
            snr_clean > snr_noisy,
            "clean {snr_clean} should exceed noisy {snr_noisy}"
        );
        // Noisy result stays inside the clamped band.
        assert!(snr_noisy >= MIN_FRAME_SNR_DB && snr_noisy <= MAX_FRAME_SNR_DB);
    }

    #[test]
    fn more_noise_lowers_snr_monotonically() {
        let x = tone(4096, 24.0);
        let make = |amp: f32| -> Vec<f32> {
            let mut state: u32 = 99;
            x.iter()
                .map(|&v| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let u = (state >> 8) as f32 / (1u32 << 24) as f32;
                    v + (u - 0.5) * amp
                })
                .collect()
        };
        let low = segmental_snr(&x, &make(0.1), 256).expect("valid");
        let high = segmental_snr(&x, &make(0.8), 256).expect("valid");
        assert!(low > high, "less noise {low} should beat more noise {high}");
    }

    #[test]
    fn rejects_bad_parameters() {
        let x = tone(512, 10.0);
        assert_eq!(segmental_snr(&x, &x, 0), Err(AudioError::InvalidParameter));
        assert_eq!(
            segmental_snr(&x, &x[..256], 128),
            Err(AudioError::InvalidParameter)
        );
        let short = tone(100, 5.0);
        assert_eq!(
            segmental_snr(&short, &short, 256),
            Err(AudioError::MalformedAudio)
        );
    }
}
