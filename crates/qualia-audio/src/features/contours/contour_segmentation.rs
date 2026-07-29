//! Segment a pitch contour into note-like events (stable-pitch runs).
//!
//! A melody contour (a per-frame sequence of pitch positions, in salience bins)
//! is carved into **notes**: maximal runs of frames that stay near a common
//! pitch. A note ends when the pitch departs from the run's running mean by more
//! than a tolerance, or when the contour goes unvoiced (a `0.0` sentinel). Very
//! short runs (below `min_note_frames`) are discarded as transients/glides.
//!
//! EPISTEMIC NOTE: notes are *proposals* about how a continuous pitch track
//! quantises into discrete events; the pitch value reported per note is the mean
//! over its frames, not a snapped scale degree (no tuning system is assumed
//! here).
//!
//! Zero-heap hot path: reads a caller-owned bin sequence and writes note events
//! into caller-owned output slices; allocates nothing.

use crate::types::AudioError;

/// Segment `contour_bins[0..n_points]` into stable-pitch note events.
///
/// `contour_bins[i]` is the pitch of frame `i` on the salience grid (any
/// monotonic pitch scale in units where `stable_tol_bins` is meaningful); a value
/// of `0.0` marks an unvoiced frame that cannot belong to a note. A note is a run
/// whose every frame lies within `stable_tol_bins` of the run's running mean; the
/// run closes when a frame breaks that bound or is unvoiced. Runs of at least
/// `min_note_frames` frames are emitted:
///
/// - `out_start[k]`: first frame index of note `k`.
/// - `out_len[k]`: number of frames in note `k`.
/// - `out_pitch[k]`: mean pitch (bins) over note `k`.
///
/// Returns the number of notes written.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `stable_tol_bins` is not a finite
///   non-negative value, `min_note_frames == 0`, or `contour_bins.len() < n_points`.
/// - [`AudioError::OutputBufferTooSmall`] if more notes are found than any of the
///   output slices can hold.
pub fn segment_contour(
    contour_bins: &[f32],
    n_points: usize,
    stable_tol_bins: f32,
    min_note_frames: usize,
    out_start: &mut [usize],
    out_len: &mut [usize],
    out_pitch: &mut [f32],
) -> Result<usize, AudioError> {
    if !stable_tol_bins.is_finite()
        || stable_tol_bins < 0.0
        || min_note_frames == 0
        || contour_bins.len() < n_points
    {
        return Err(AudioError::InvalidParameter);
    }
    let cap = out_start.len().min(out_len.len()).min(out_pitch.len());

    let mut notes = 0usize;
    let mut run_start = 0usize;
    let mut run_len = 0usize;
    let mut run_sum = 0.0f64; // running mean numerator (f64 for long runs)

    // Emit the current run as a note if it is long enough.
    // Returns Err on output overflow.
    // (Inlined as a closure would borrow the out slices mutably twice; use a macro-free helper.)
    let mut flush =
        |start: usize, len: usize, sum: f64, notes: &mut usize| -> Result<(), AudioError> {
            if len >= min_note_frames {
                if *notes >= cap {
                    return Err(AudioError::OutputBufferTooSmall);
                }
                out_start[*notes] = start;
                out_len[*notes] = len;
                out_pitch[*notes] = (sum / len as f64) as f32;
                *notes += 1;
            }
            Ok(())
        };

    for i in 0..n_points {
        let p = contour_bins[i];
        let voiced = p > 0.0 && p.is_finite();

        if run_len == 0 {
            if voiced {
                run_start = i;
                run_len = 1;
                run_sum = p as f64;
            }
            continue;
        }

        let mean = (run_sum / run_len as f64) as f32;
        if voiced && (p - mean).abs() <= stable_tol_bins {
            run_len += 1;
            run_sum += p as f64;
        } else {
            // Close current run; start a fresh one at this frame if voiced.
            flush(run_start, run_len, run_sum, &mut notes)?;
            if voiced {
                run_start = i;
                run_len = 1;
                run_sum = p as f64;
            } else {
                run_len = 0;
                run_sum = 0.0;
            }
        }
    }
    // Close a trailing run.
    if run_len > 0 {
        flush(run_start, run_len, run_sum, &mut notes)?;
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GOLDEN: a two-note contour — 6 frames near bin 100, then 6 frames near
    /// bin 124 — splits into exactly two notes at frame 6, with correct means.
    #[test]
    fn splits_two_note_contour() {
        let mut c = vec![0.0f32; 12];
        for v in c.iter_mut().take(6) {
            *v = 100.0;
        }
        for v in c.iter_mut().skip(6) {
            *v = 124.0;
        }
        let mut start = [0usize; 4];
        let mut len = [0usize; 4];
        let mut pitch = [0.0f32; 4];
        let n = segment_contour(&c, 12, 0.5, 2, &mut start, &mut len, &mut pitch).expect("segment");
        assert_eq!(n, 2);
        assert_eq!(start[0], 0);
        assert_eq!(len[0], 6);
        assert!((pitch[0] - 100.0).abs() < 1e-3);
        assert_eq!(start[1], 6, "split at the pitch change");
        assert_eq!(len[1], 6);
        assert!((pitch[1] - 124.0).abs() < 1e-3);
    }

    /// An unvoiced gap breaks a note and short runs are discarded.
    #[test]
    fn gap_breaks_and_short_runs_dropped() {
        // 4 frames at 100, one voiced blip at 200 (too short), gap, 4 at 100.
        let c = [
            100.0f32, 100.0, 100.0, 100.0, // note A (len 4)
            200.0, // 1-frame blip → dropped
            0.0, 0.0, // unvoiced gap
            100.0, 100.0, 100.0, 100.0, // note B (len 4)
        ];
        let mut start = [0usize; 4];
        let mut len = [0usize; 4];
        let mut pitch = [0.0f32; 4];
        let n = segment_contour(&c, c.len(), 0.5, 2, &mut start, &mut len, &mut pitch)
            .expect("segment");
        assert_eq!(n, 2, "two long notes; the 1-frame blip is dropped");
        assert_eq!((start[0], len[0]), (0, 4));
        assert_eq!((start[1], len[1]), (7, 4));
    }

    /// A slow glide (drift beyond tolerance) fragments rather than reading as one
    /// note — the running-mean bound trips.
    #[test]
    fn drift_beyond_tolerance_fragments() {
        // Steady climb 100,101,102,... with tol 0.5 → each step breaks the run.
        let c: Vec<f32> = (0..8).map(|i| 100.0 + i as f32).collect();
        let mut start = [0usize; 16];
        let mut len = [0usize; 16];
        let mut pitch = [0.0f32; 16];
        let n = segment_contour(&c, 8, 0.5, 1, &mut start, &mut len, &mut pitch).expect("segment");
        assert_eq!(n, 8, "each 1-cent-step frame is its own micro-note");
    }

    #[test]
    fn rejects_bad_params() {
        let c = [100.0f32; 4];
        let mut s = [0usize; 4];
        let mut l = [0usize; 4];
        let mut p = [0.0f32; 4];
        assert_eq!(
            segment_contour(&c, 4, -1.0, 2, &mut s, &mut l, &mut p),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            segment_contour(&c, 4, 0.5, 0, &mut s, &mut l, &mut p),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn output_overflow_errors() {
        let c = [100.0f32, 100.0, 0.0, 200.0, 200.0, 0.0, 50.0, 50.0];
        let mut s = [0usize; 1];
        let mut l = [0usize; 1];
        let mut p = [0.0f32; 1];
        assert_eq!(
            segment_contour(&c, c.len(), 0.5, 2, &mut s, &mut l, &mut p),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
