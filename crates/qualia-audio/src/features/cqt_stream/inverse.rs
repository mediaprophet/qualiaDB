//! Approximate inverse CQT: time-domain reconstruction from a CQT magnitude
//! spectrogram (AU-CQT-STREAM).
//!
//! This is an **approximate, magnitude-only overlap-add reconstruction**, not
//! an exact inverse. A true inverse CQT requires the complex (phase-bearing)
//! transform and a dual frame; here only per-bin magnitudes are available. We
//! resynthesise each bin as a pure sinusoid at that bin's centre frequency,
//! scaled by its magnitude, and overlap-add the windowed frame contributions.
//!
//! Phase is taken from the **absolute** output sample index `n` (not reset
//! per frame), so overlapping frames add coherently and a steady tone
//! reconstructs as a clean sinusoid at the bin centre frequency. Amplitude is
//! only approximate (a Hann analysis-synthesis window is applied without
//! gain-normalising the overlap), which is sufficient for pitch/frequency
//! reconstruction but does not recover exact levels.
//!
//! Caller-buffered: the reconstruction is written into `out`; no heap
//! allocation is performed.

use crate::types::AudioError;

/// Length of the reconstructed signal for a hopped spectrogram.
fn reconstruction_len(n_frames: usize, frame_len: usize, hop: usize) -> usize {
    if n_frames == 0 {
        0
    } else {
        (n_frames - 1) * hop + frame_len
    }
}

/// Hann window value at sample `j` of a `len`-sample window.
fn hann(j: usize, len: usize) -> f32 {
    if len <= 1 {
        1.0
    } else {
        0.5 * (1.0 - (core::f32::consts::TAU * j as f32 / (len - 1) as f32).cos())
    }
}

/// Approximate inverse CQT via magnitude-weighted sinusoidal overlap-add.
///
/// `spectrogram` is a row-major `[n_frames × n_bins]` magnitude matrix (as
/// produced by [`super::spectrogram::cqt_spectrogram`]). Each bin `k` maps to
/// centre frequency `f_min * 2^(k / bins_per_octave)`. The reconstruction sums,
/// for every frame and bin, `mag * hann * sin(2π f_k n / sr)` into the output at
/// absolute sample index `n`, with frames placed `hop` samples apart.
///
/// Returns the number of samples written:
/// `(n_frames - 1) * hop + frame_len` (0 if `n_frames == 0`). `out` is fully
/// overwritten in `[0, len)` (accumulation starts from zero).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `hop == 0`, `frame_len == 0`,
///   `n_bins == 0`, `bins_per_octave == 0`, `sample_rate <= 0`, or `f_min <= 0`.
/// - [`AudioError::MalformedAudio`] if `spectrogram` is smaller than
///   `n_frames * n_bins`.
/// - [`AudioError::OutputBufferTooSmall`] if `out` is smaller than `len`.
#[allow(clippy::too_many_arguments)]
pub fn inverse_cqt(
    spectrogram: &[f32],
    n_frames: usize,
    n_bins: usize,
    sample_rate: f32,
    f_min: f32,
    bins_per_octave: usize,
    hop: usize,
    frame_len: usize,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    if hop == 0
        || frame_len == 0
        || n_bins == 0
        || bins_per_octave == 0
        || sample_rate <= 0.0
        || f_min <= 0.0
    {
        return Err(AudioError::InvalidParameter);
    }
    if spectrogram.len() < n_frames.saturating_mul(n_bins) {
        return Err(AudioError::MalformedAudio);
    }
    let len = reconstruction_len(n_frames, frame_len, hop);
    if out.len() < len {
        return Err(AudioError::OutputBufferTooSmall);
    }
    if len == 0 {
        return Ok(0);
    }

    out[..len].fill(0.0);

    for f in 0..n_frames {
        let base = f * hop;
        let row = &spectrogram[f * n_bins..(f + 1) * n_bins];
        for (k, &mag) in row.iter().enumerate() {
            if mag == 0.0 {
                continue;
            }
            let f_k = f_min * 2.0_f32.powf(k as f32 / bins_per_octave as f32);
            let omega = core::f32::consts::TAU * f_k / sample_rate;
            for j in 0..frame_len {
                let n = base + j;
                let w = hann(j, frame_len);
                out[n] += mag * w * (omega * n as f32).sin();
            }
        }
    }

    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Estimate dominant frequency (Hz) of `sig` via zero-crossing rate.
    /// crossings ≈ 2 * cycles, so f ≈ crossings / (2 * duration_seconds).
    fn dominant_freq_zc(sig: &[f32], sr: f32) -> f32 {
        let mut crossings = 0usize;
        for w in sig.windows(2) {
            if (w[0] <= 0.0 && w[1] > 0.0) || (w[0] > 0.0 && w[1] <= 0.0) {
                crossings += 1;
            }
        }
        let duration = sig.len() as f32 / sr;
        crossings as f32 / (2.0 * duration)
    }

    #[test]
    fn inverse_single_bin_reconstructs_440hz() {
        let sr = 16000.0f32;
        let f_min = 55.0f32;
        let bpo = 12usize;
        let n_bins = 48usize;
        let hop = 2048usize;
        let frame_len = 4096usize;
        let n_frames = 3usize;

        // Bin 36 -> 55 * 2^3 = 440 Hz.
        let bin = 36usize;
        let f_k = f_min * 2.0f32.powf(bin as f32 / bpo as f32);
        assert!((f_k - 440.0).abs() < 1e-3, "bin 36 centre = {f_k} Hz");

        // Single active bin across all frames.
        let mut spec = vec![0.0f32; n_frames * n_bins];
        for f in 0..n_frames {
            spec[f * n_bins + bin] = 1.0;
        }

        let len = reconstruction_len(n_frames, frame_len, hop);
        let mut out = vec![0.0f32; len];
        let written = inverse_cqt(
            &spec, n_frames, n_bins, sr, f_min, bpo, hop, frame_len, &mut out,
        )
        .unwrap();
        assert_eq!(written, len);
        assert_eq!(len, (n_frames - 1) * hop + frame_len);

        // Measure reconstructed dominant frequency on the coherent central
        // region (avoid the tapered ends where Hann attenuates the signal).
        let lo = frame_len / 2;
        let hi = len - frame_len / 2;
        let f_est = dominant_freq_zc(&out[lo..hi], sr);
        assert!(
            (f_est - 440.0).abs() < 440.0 * 0.03,
            "reconstructed dominant freq {f_est} Hz should be ~440 Hz"
        );
    }

    #[test]
    fn inverse_zero_frames_writes_nothing() {
        let sr = 16000.0f32;
        let mut out = [0.0f32; 16];
        let n = inverse_cqt(&[], 0, 48, sr, 55.0, 12, 2048, 4096, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn inverse_rejects_bad_params_and_buffers() {
        let spec = [0.0f32; 48];
        // hop == 0.
        assert!(matches!(
            inverse_cqt(&spec, 1, 48, 16000.0, 55.0, 12, 0, 4096, &mut [0.0; 4096]),
            Err(AudioError::InvalidParameter)
        ));
        // Spectrogram too small (needs 2*48 = 96).
        assert!(matches!(
            inverse_cqt(
                &spec,
                2,
                48,
                16000.0,
                55.0,
                12,
                2048,
                4096,
                &mut [0.0; 6144]
            ),
            Err(AudioError::MalformedAudio)
        ));
        // Output too small (needs 4096).
        assert!(matches!(
            inverse_cqt(&spec, 1, 48, 16000.0, 55.0, 12, 2048, 4096, &mut [0.0; 100]),
            Err(AudioError::OutputBufferTooSmall)
        ));
    }
}
