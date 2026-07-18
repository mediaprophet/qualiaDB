//! Salient pitch candidates: the peaks of a pitch-salience curve for one frame.
//!
//! Reuses [`crate::features::peaks::detect_peaks`] to pick local maxima of the
//! salience function produced by
//! [`crate::features::salience::pitch_salience`], keeping only peaks above a
//! fraction of the frame's strongest salience. These are the per-frame pitch
//! candidates that contour tracking later streams across time.
//!
//! EPISTEMIC NOTE: the returned candidates are *proposals* — a polyphonic frame
//! legitimately yields several. No monophonic collapse happens here.
//!
//! Zero-heap hot path: the caller owns the output buffers; the reused
//! `detect_peaks` is itself allocation-free.

use crate::features::peaks::detect_peaks;
use crate::types::AudioError;

/// Pick the salient pitch candidates (salience-curve peaks) of one frame.
///
/// A candidate is a local maximum of `salience[0..n_bins]` whose value is at
/// least `threshold_ratio * max(salience)`; candidates closer than
/// `min_distance_bins` bins are resolved in favour of the stronger (via the
/// reused peak picker). Output is written **bin-ascending**: `out_bins[k]` the
/// sub-bin candidate position and `out_salience[k]` its salience.
///
/// Returns the number of candidates written.
///
/// - `salience`: the frame's salience curve; only `[0, n_bins)` is read.
/// - `threshold_ratio`: relative floor in `[0, 1]` (e.g. `0.1`).
/// - `min_distance_bins`: minimum spacing between accepted candidates.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `threshold_ratio` is not a finite value
///   in `[0, 1]`, or `salience.len() < n_bins`.
/// - [`AudioError::OutputBufferTooSmall`] propagated from the peak picker if more
///   candidates are found than the output buffers can hold.
pub fn salient_pitch_peaks(
    salience: &[f32],
    n_bins: usize,
    threshold_ratio: f32,
    min_distance_bins: usize,
    out_bins: &mut [f32],
    out_salience: &mut [f32],
) -> Result<usize, AudioError> {
    if !threshold_ratio.is_finite()
        || !(0.0..=1.0).contains(&threshold_ratio)
        || salience.len() < n_bins
    {
        return Err(AudioError::InvalidParameter);
    }
    if n_bins < 3 {
        return Ok(0);
    }
    let view = &salience[..n_bins];

    // Absolute threshold from the frame's own strongest salience.
    let mut max_s = 0.0f32;
    for &v in view {
        if v > max_s {
            max_s = v;
        }
    }
    if max_s <= 0.0 {
        return Ok(0); // silent / unpitched frame → no candidates
    }
    let threshold = threshold_ratio * max_s;

    detect_peaks(view, threshold, min_distance_bins, out_bins, out_salience)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two salience ridges → two candidates, weak noise ridge rejected by the
    /// relative threshold.
    #[test]
    fn picks_two_ridges() {
        let mut s = vec![0.0f32; 64];
        // Ridge A at bin 10 (tall), ridge B at bin 40 (medium), noise at bin 25.
        for (c, h) in [(10usize, 4.0f32), (40, 3.0), (25, 0.2)] {
            s[c - 1] = 0.5 * h;
            s[c] = h;
            s[c + 1] = 0.5 * h;
        }
        let mut bins = [0.0f32; 8];
        let mut sal = [0.0f32; 8];
        let n = salient_pitch_peaks(&s, 64, 0.1, 3, &mut bins, &mut sal).expect("peaks");
        assert_eq!(n, 2, "noise ridge (0.2 < 0.1*4=0.4) rejected");
        // Bin-ascending output.
        assert!((bins[0] - 10.0).abs() < 1e-3, "b0={}", bins[0]);
        assert!((bins[1] - 40.0).abs() < 1e-3, "b1={}", bins[1]);
        assert_eq!(sal[0], 4.0);
        assert_eq!(sal[1], 3.0);
    }

    #[test]
    fn silent_frame_no_candidates() {
        let s = vec![0.0f32; 32];
        let mut bins = [0.0f32; 4];
        let mut sal = [0.0f32; 4];
        let n = salient_pitch_peaks(&s, 32, 0.1, 1, &mut bins, &mut sal).expect("ok");
        assert_eq!(n, 0);
    }

    #[test]
    fn rejects_bad_ratio() {
        let s = vec![1.0f32; 8];
        let mut bins = [0.0f32; 4];
        let mut sal = [0.0f32; 4];
        assert_eq!(
            salient_pitch_peaks(&s, 8, 1.5, 1, &mut bins, &mut sal),
            Err(AudioError::InvalidParameter)
        );
    }
}
