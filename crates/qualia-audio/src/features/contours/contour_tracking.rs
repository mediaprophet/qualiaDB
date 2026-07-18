//! Streaming pitch-contour tracker: connect per-frame salience peaks across time.
//!
//! Given the salient pitch candidates of each frame (as bin positions on the
//! logarithmic salience grid), link candidates in consecutive frames that stay
//! within a pitch-continuity tolerance into **contours** — time-connected pitch
//! trajectories. This is the streaming stage before predominant-melody selection
//! (`melodia`/`predominant`).
//!
//! Assignment is greedy one-to-one nearest-neighbour: each candidate in frame `f`
//! extends the closest still-unclaimed contour from frame `f-1` within
//! `pitch_tol_bins`; otherwise it begins a new contour. Peaks are processed in
//! the order given (callers should pass them salience-descending so the strongest
//! candidate claims first).
//!
//! EPISTEMIC NOTE: contours are *proposals* about pitch continuity; several
//! coexist in polyphony. No melody is chosen here.
//!
//! Zero-heap hot path: the per-frame candidate bins live in a caller-owned
//! flat buffer, and the contour id of each candidate is derived by scanning the
//! previous frame's already-written ids — no per-call allocation.

use crate::types::AudioError;

/// A candidate slot that has not been assigned to a contour.
const NO_CONTOUR: i32 = -1;

/// Link per-frame salience peaks into time-continuous pitch contours.
///
/// `peak_bins` is a flat row-major buffer of shape `n_frames × max_peaks_per_frame`
/// (`peak_bins[f * max_peaks_per_frame + k]`), holding the bin position of the
/// `k`-th candidate in frame `f`; `peak_count[f]` gives how many of that frame's
/// `max_peaks_per_frame` slots are valid (candidates occupy slots `0..count`).
///
/// Writes one contour id per candidate slot into `out_contour_id` (same layout as
/// `peak_bins`): candidates sharing an id form one contour; unused slots are set
/// to [`NO_CONTOUR`] (`-1`). Two candidates in consecutive frames whose bins
/// differ by at most `pitch_tol_bins` may join the same contour.
///
/// Returns the number of distinct contours created.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `max_peaks_per_frame == 0`,
///   `pitch_tol_bins` is not a finite non-negative value, or a `peak_count[f]`
///   exceeds `max_peaks_per_frame`.
/// - [`AudioError::OutputBufferTooSmall`] if either flat buffer is shorter than
///   `n_frames * max_peaks_per_frame`.
pub fn track_contours(
    peak_bins: &[f32],
    peak_count: &[usize],
    n_frames: usize,
    max_peaks_per_frame: usize,
    pitch_tol_bins: f32,
    out_contour_id: &mut [i32],
) -> Result<usize, AudioError> {
    if max_peaks_per_frame == 0
        || !pitch_tol_bins.is_finite()
        || pitch_tol_bins < 0.0
        || peak_count.len() < n_frames
    {
        return Err(AudioError::InvalidParameter);
    }
    let need = n_frames.saturating_mul(max_peaks_per_frame);
    if peak_bins.len() < need || out_contour_id.len() < need {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for id in out_contour_id.iter_mut().take(need) {
        *id = NO_CONTOUR;
    }
    if n_frames == 0 {
        return Ok(0);
    }

    let mut next_id: i32 = 0;
    let base = |f: usize| f * max_peaks_per_frame;

    for f in 0..n_frames {
        let count = peak_count[f];
        if count > max_peaks_per_frame {
            return Err(AudioError::InvalidParameter);
        }
        let row = base(f);

        if f == 0 {
            for k in 0..count {
                out_contour_id[row + k] = next_id;
                next_id += 1;
            }
            continue;
        }

        let prev = base(f - 1);
        let prev_count = peak_count[f - 1];
        for k in 0..count {
            let b = peak_bins[row + k];
            // Find the nearest previous candidate within tolerance whose contour
            // has not already been extended into this frame.
            let mut best_j: Option<usize> = None;
            let mut best_d = pitch_tol_bins;
            for j in 0..prev_count {
                let cid = out_contour_id[prev + j];
                if cid == NO_CONTOUR {
                    continue;
                }
                if contour_claimed_this_frame(out_contour_id, row, k, cid) {
                    continue; // one-to-one: this contour already extended here
                }
                let d = (peak_bins[prev + j] - b).abs();
                if d <= best_d {
                    best_d = d;
                    best_j = Some(j);
                }
            }
            match best_j {
                Some(j) => out_contour_id[row + k] = out_contour_id[prev + j],
                None => {
                    out_contour_id[row + k] = next_id;
                    next_id += 1;
                }
            }
        }
    }
    Ok(next_id as usize)
}

/// True if contour `cid` was already assigned to an earlier candidate slot
/// (`0..k`) of the current frame — enforces one-to-one extension per frame.
#[inline]
fn contour_claimed_this_frame(ids: &[i32], row: usize, k: usize, cid: i32) -> bool {
    for kk in 0..k {
        if ids[row + kk] == cid {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single pitch drifting slowly (bins 100, 101, 102) stays one contour.
    #[test]
    fn continuous_pitch_one_contour() {
        let max_p = 2usize;
        let n = 3usize;
        let mut bins = vec![0.0f32; n * max_p];
        bins[0] = 100.0; // f0
        bins[max_p] = 101.0; // f1
        bins[2 * max_p] = 102.0; // f2
        let counts = [1usize, 1, 1];
        let mut ids = vec![0i32; n * max_p];
        let n_contours =
            track_contours(&bins, &counts, n, max_p, 3.0, &mut ids).expect("track");
        assert_eq!(n_contours, 1);
        assert_eq!(ids[0], 0);
        assert_eq!(ids[max_p], 0);
        assert_eq!(ids[2 * max_p], 0);
    }

    /// A jump beyond tolerance breaks the contour (new id after the gap).
    #[test]
    fn discontinuity_starts_new_contour() {
        let max_p = 1usize;
        let n = 3usize;
        let bins = [100.0f32, 200.0, 201.0]; // big jump at frame 1
        let counts = [1usize, 1, 1];
        let mut ids = vec![0i32; n];
        let n_contours =
            track_contours(&bins, &counts, n, max_p, 3.0, &mut ids).expect("track");
        assert_eq!(n_contours, 2);
        assert_eq!(ids[0], 0);
        assert_eq!(ids[1], 1);
        assert_eq!(ids[2], 1, "202≈201 continues the second contour");
    }

    /// Two simultaneous voices track as two parallel contours.
    #[test]
    fn two_voices_two_contours() {
        let max_p = 2usize;
        let n = 3usize;
        // Voice A ≈ bin 100, Voice B ≈ bin 160, both present every frame.
        let mut bins = vec![0.0f32; n * max_p];
        for f in 0..n {
            bins[f * max_p] = 100.0 + f as f32; // A drifts up slightly
            bins[f * max_p + 1] = 160.0 - f as f32; // B drifts down slightly
        }
        let counts = [2usize, 2, 2];
        let mut ids = vec![0i32; n * max_p];
        let n_contours =
            track_contours(&bins, &counts, n, max_p, 3.0, &mut ids).expect("track");
        assert_eq!(n_contours, 2);
        // Each voice keeps a stable contour id across frames.
        assert_eq!(ids[0], ids[max_p]);
        assert_eq!(ids[0], ids[2 * max_p]);
        assert_eq!(ids[1], ids[max_p + 1]);
        assert_ne!(ids[0], ids[1]);
    }

    #[test]
    fn rejects_bad_params() {
        let bins = [0.0f32; 4];
        let counts = [1usize, 1];
        let mut ids = [0i32; 4];
        assert_eq!(
            track_contours(&bins, &counts, 2, 0, 3.0, &mut ids),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            track_contours(&bins, &counts, 2, 2, f32::NAN, &mut ids),
            Err(AudioError::InvalidParameter)
        );
    }
}
