//! Segment boundaries from novelty-curve peaks (emitted as PROPOSALS).
//!
//! Peaks of the Foote novelty curve are candidate structural boundaries. This
//! layer picks them with the shared [`crate::features::peaks::detect_peaks`]
//! local-maximum picker and rounds each sub-sample peak position to a frame
//! index. The results are epistemic **proposals** — "a section change is
//! plausible near frame N" — not ground-truth cuts; downstream governance
//! decides what to do with them.

use crate::features::peaks::detect_peaks;
use crate::types::AudioError;

/// Propose segment boundaries as novelty-curve peak frame indices.
///
/// `novelty` is a per-frame novelty curve (as produced by
/// [`super::novelty::novelty_curve`]); only its first `n_frames` values are
/// examined. A frame is a boundary candidate when the novelty curve has a local
/// maximum there of at least `threshold`, with peaks closer than `min_distance`
/// frames resolved in favour of the taller one (see `detect_peaks`). Ascending
/// boundary frame indices are written to `out_boundaries`; the count is
/// returned.
///
/// # Caller-buffer contract (zero-heap)
/// `pos_scratch` and `mag_scratch` are caller-owned working buffers handed
/// straight to `detect_peaks` (each must hold at least as many entries as the
/// number of peaks you expect to accept). The rounded frame indices are then
/// written into `out_boundaries`. No heap allocation is performed.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_frames == 0`, `n_frames` exceeds
///   `novelty.len()`, or `threshold` is not finite.
/// - [`AudioError::OutputBufferTooSmall`] if `out_boundaries` cannot hold every
///   accepted peak (propagated from `detect_peaks` for the scratch buffers).
pub fn segment_boundaries(
    novelty: &[f32],
    n_frames: usize,
    threshold: f32,
    min_distance: usize,
    pos_scratch: &mut [f32],
    mag_scratch: &mut [f32],
    out_boundaries: &mut [usize],
) -> Result<usize, AudioError> {
    if n_frames == 0 || n_frames > novelty.len() || !threshold.is_finite() {
        return Err(AudioError::InvalidParameter);
    }

    let count = detect_peaks(
        &novelty[..n_frames],
        threshold,
        min_distance,
        pos_scratch,
        mag_scratch,
    )?;

    if out_boundaries.len() < count {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for k in 0..count {
        // detect_peaks returns parabolically-interpolated positions; a boundary
        // is a whole frame, so round to the nearest index. round() ties away
        // from zero, and positions are always in [1, n_frames-2].
        out_boundaries[k] = pos_scratch[k].round() as usize;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::super::novelty::novelty_curve;
    use super::super::ssm::self_similarity;
    use super::*;

    fn two_section_ssm(n_frames: usize, half: usize) -> Vec<f32> {
        let dims = 2;
        let mut f = vec![0.0f32; n_frames * dims];
        for frame in 0..n_frames {
            if frame < half {
                f[frame * dims] = 1.0;
            } else {
                f[frame * dims + 1] = 1.0;
            }
        }
        let mut ssm = vec![0.0f32; n_frames * n_frames];
        self_similarity(&f, n_frames, dims, &mut ssm).expect("ssm");
        ssm
    }

    /// Golden end-to-end: features -> SSM -> novelty -> boundary at the true
    /// midpoint (frame 4 of an 8-frame, two-section signal).
    #[test]
    fn golden_boundary_at_midpoint() {
        let n = 8;
        let ssm = two_section_ssm(n, 4);
        let mut nov = vec![0.0f32; n];
        novelty_curve(&ssm, n, 4, &mut nov).expect("novelty");

        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let mut boundaries = [0usize; 8];
        // Threshold 4.0 admits the boundary peak (8.0) and rejects the flanks.
        let k = segment_boundaries(&nov, n, 4.0, 1, &mut pos, &mut mag, &mut boundaries)
            .expect("segment");
        assert_eq!(k, 1, "one boundary; curve={nov:?}");
        assert_eq!(boundaries[0], 4, "boundary frame");
    }

    /// A 16-frame signal split at frame 8 yields a boundary at frame 8.
    #[test]
    fn boundary_tracks_true_split() {
        let n = 16;
        let ssm = two_section_ssm(n, 8);
        let mut nov = vec![0.0f32; n];
        novelty_curve(&ssm, n, 4, &mut nov).expect("novelty");
        let mut pos = [0.0f32; 16];
        let mut mag = [0.0f32; 16];
        let mut boundaries = [0usize; 16];
        let k =
            segment_boundaries(&nov, n, 4.0, 2, &mut pos, &mut mag, &mut boundaries).expect("seg");
        assert_eq!(k, 1, "curve={nov:?}");
        assert_eq!(boundaries[0], 8);
    }

    /// A high threshold suppresses all peaks -> no proposed boundaries.
    #[test]
    fn high_threshold_yields_no_boundaries() {
        let n = 8;
        let ssm = two_section_ssm(n, 4);
        let mut nov = vec![0.0f32; n];
        novelty_curve(&ssm, n, 4, &mut nov).expect("novelty");
        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let mut boundaries = [0usize; 8];
        let k = segment_boundaries(&nov, n, 100.0, 1, &mut pos, &mut mag, &mut boundaries)
            .expect("seg");
        assert_eq!(k, 0);
    }

    #[test]
    fn rejects_bad_params() {
        let nov = [0.0f32; 4];
        let mut pos = [0.0f32; 4];
        let mut mag = [0.0f32; 4];
        let mut out = [0usize; 4];
        assert_eq!(
            segment_boundaries(&nov, 0, 1.0, 1, &mut pos, &mut mag, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // n_frames beyond the slice.
        assert_eq!(
            segment_boundaries(&nov, 5, 1.0, 1, &mut pos, &mut mag, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // non-finite threshold.
        assert_eq!(
            segment_boundaries(&nov, 4, f32::NAN, 1, &mut pos, &mut mag, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }
}
