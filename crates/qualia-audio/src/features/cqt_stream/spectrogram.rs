//! Streaming multi-frame CQT magnitude spectrogram (AU-CQT-STREAM).
//!
//! Hops a fixed-length analysis window across a mono signal, calling the
//! single-frame [`crate::features::cqt::forward_cqt_mono`] kernel per frame and
//! writing a row-major `[n_frames × n_bins]` magnitude spectrogram into a
//! caller-provided buffer.
//!
//! Zero-heap hot path: this function performs no allocation. Each frame is a
//! borrowed sub-slice of `signal`, and each frame's CQT magnitudes are written
//! directly into the matching row of the caller's `out` buffer. The per-frame
//! kernel itself uses only stack scalars.

use crate::features::cqt::forward_cqt_mono;
use crate::types::AudioError;

/// Number of hopped frames that fit in `signal_len` given `frame_len`/`hop`.
///
/// A frame is counted only if it lies fully within the signal, i.e. the last
/// sample it reads (`start + frame_len - 1`) is in range. Returns 0 when the
/// signal is shorter than a single frame.
fn frame_count(signal_len: usize, frame_len: usize, hop: usize) -> usize {
    if frame_len == 0 || hop == 0 || signal_len < frame_len {
        return 0;
    }
    1 + (signal_len - frame_len) / hop
}

/// Compute a streaming CQT magnitude spectrogram over `signal`.
///
/// The window of length `frame_len` is advanced by `hop` samples per frame;
/// for each frame the constant-Q magnitudes (`n_bins` bins, geometrically
/// spaced from `f_min` at `bins_per_octave`) are computed with
/// [`forward_cqt_mono`] and stored row-major into `out` as
/// `out[frame * n_bins + bin]`.
///
/// Returns the number of frames written. `out` must hold at least
/// `n_frames * n_bins` values, where `n_frames` follows the hop math
/// (`1 + (signal.len() - frame_len) / hop`, or 0 if the signal is too short).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `hop == 0`, `frame_len == 0`,
///   `n_bins == 0`, `bins_per_octave == 0`, `sample_rate <= 0`, or `f_min <= 0`.
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold the spectrogram.
#[allow(clippy::too_many_arguments)]
pub fn cqt_spectrogram(
    signal: &[f32],
    sample_rate: f32,
    f_min: f32,
    bins_per_octave: usize,
    n_bins: usize,
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

    let n_frames = frame_count(signal.len(), frame_len, hop);
    let needed = n_frames.saturating_mul(n_bins);
    if out.len() < needed {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for f in 0..n_frames {
        let start = f * hop;
        let frame = &signal[start..start + frame_len];
        let row = &mut out[f * n_bins..(f + 1) * n_bins];
        forward_cqt_mono(frame, sample_rate, f_min, bins_per_octave, n_bins, row)?;
    }

    Ok(n_frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, sr: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (core::f32::consts::TAU * freq * i as f32 / sr).sin())
            .collect()
    }

    #[test]
    fn cqt_stream_frame_count_matches_hop_math() {
        // 8192 samples, frame 4096, hop 2048 -> 1 + (8192-4096)/2048 = 3.
        assert_eq!(frame_count(8192, 4096, 2048), 3);
        // Exactly one frame.
        assert_eq!(frame_count(4096, 4096, 2048), 1);
        // Signal shorter than a frame -> zero frames.
        assert_eq!(frame_count(4000, 4096, 2048), 0);
        // Non-integer trailing frame is dropped (fully-in-range rule).
        assert_eq!(frame_count(9000, 4096, 2048), 3);
    }

    #[test]
    fn cqt_stream_440_tone_peaks_at_bin_36() {
        let sr = 16000.0f32;
        let n = 8192usize;
        let frame_len = 4096usize;
        let hop = 2048usize;
        let f_min = 55.0f32;
        let bpo = 12usize;
        let n_bins = 48usize; // covers 4 octaves; 440 Hz -> bin 36.

        let s = tone(440.0, sr, n);

        // Expected bin index: 12 * log2(440/55) = 36.
        let expected_bin = (bpo as f32 * (440.0f32 / f_min).log2()).round() as usize;
        assert_eq!(expected_bin, 36);

        let n_frames_expected = 1 + (n - frame_len) / hop; // 3
        let mut out = vec![0.0f32; n_frames_expected * n_bins];

        let n_frames =
            cqt_spectrogram(&s, sr, f_min, bpo, n_bins, hop, frame_len, &mut out).unwrap();
        assert_eq!(n_frames, 3, "frame count must match hop math");

        // Per-frame the max-magnitude bin must be 36.
        for f in 0..n_frames {
            let row = &out[f * n_bins..(f + 1) * n_bins];
            let mut best = 0usize;
            let mut best_v = row[0];
            for (k, &v) in row.iter().enumerate() {
                if v > best_v {
                    best_v = v;
                    best = k;
                }
            }
            assert_eq!(best, 36, "frame {f} peak bin should be 36 (440 Hz)");
        }

        // Summed energy across frames also peaks at bin 36.
        let mut summed = vec![0.0f32; n_bins];
        for f in 0..n_frames {
            for k in 0..n_bins {
                summed[k] += out[f * n_bins + k];
            }
        }
        let mut best = 0usize;
        for k in 1..n_bins {
            if summed[k] > summed[best] {
                best = k;
            }
        }
        assert_eq!(best, 36, "summed-energy peak bin should be 36");
    }

    #[test]
    fn cqt_stream_short_signal_zero_frames() {
        let sr = 16000.0f32;
        let s = tone(440.0, sr, 1000);
        let mut out = [0.0f32; 48];
        let n = cqt_spectrogram(&s, sr, 55.0, 12, 48, 2048, 4096, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn cqt_stream_rejects_bad_params_and_small_out() {
        let s = tone(440.0, 16000.0, 8192);
        // hop == 0.
        assert!(matches!(
            cqt_spectrogram(&s, 16000.0, 55.0, 12, 48, 0, 4096, &mut [0.0; 48]),
            Err(AudioError::InvalidParameter)
        ));
        // Output too small (needs 3*48 = 144).
        let mut small = [0.0f32; 100];
        assert!(matches!(
            cqt_spectrogram(&s, 16000.0, 55.0, 12, 48, 2048, 4096, &mut small),
            Err(AudioError::OutputBufferTooSmall)
        ));
    }
}
