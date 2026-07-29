//! Multi-voice pitch-contour tracking (polyphonic extension of Melodia).
//!
//! Melodia's mono predominant-melody stage collapses each frame to a single F0
//! and threads it through time. Polyphonic material has several concurrent
//! lines, so this tracker takes the **per-frame multi-pitch estimates** (e.g.
//! from [`super::multipitch_klapuri`], one row of F0s per analysis frame) and
//! threads them into up to `n_voices` continuous pitch tracks by nearest-pitch
//! continuity — the many-voices analogue of contour tracking.
//!
//! EPISTEMIC RULE (declared): the tracks are **proposals**. The **max-polyphony
//! assumption** is explicit in `n_voices`: at most that many concurrent lines
//! are posited, and when a frame offers more estimates than free voices the
//! surplus is dropped rather than forced onto an unrelated voice. A voice with
//! no matching estimate in a frame **abstains** for that frame (`0.0`), and is
//! freed after [`MAX_GAP`] silent frames so a genuinely new line can reuse the
//! slot. Zero input estimates → zero tracks.
//!
//! Zero-heap hot path: all per-voice / per-estimate state lives in fixed stack
//! arrays bounded by [`MAX_VOICES`] / [`MAX_ESTIMATES`]; the caller owns the
//! `frame_f0` input and the `out_tracks` output. Nothing is allocated.

use crate::types::AudioError;

/// Upper bound on concurrent voices (stack-array sizing).
pub const MAX_VOICES: usize = 16;
/// Upper bound on per-frame F0 estimates (stack-array sizing).
pub const MAX_ESTIMATES: usize = 16;
/// Consecutive unmatched frames after which a voice is freed for reuse.
pub const MAX_GAP: usize = 3;

/// Pitch distance in semitones between two positive frequencies.
#[inline]
fn semitone_distance(a: f32, b: f32) -> f32 {
    12.0 * (a / b).log2().abs()
}

/// Track multiple concurrent pitch contours across frames.
///
/// - `frame_f0`: row-major `n_frames × max_per_frame` buffer of per-frame F0
///   estimates in Hz; a `0.0` (or non-finite) slot means "no estimate". Rows
///   need not be sorted, and estimates may appear in any column order from frame
///   to frame — continuity is resolved by pitch, not by column.
/// - `n_frames` / `max_per_frame`: shape of `frame_f0`.
/// - `n_voices`: **declared** maximum number of concurrent voices to track
///   (`1..=MAX_VOICES`).
/// - `max_jump_semitones`: the largest pitch step (semitones) an estimate may be
///   from a voice's last pitch and still continue that voice.
/// - `out_tracks`: row-major `n_voices × n_frames` buffer; `out_tracks[v *
///   n_frames + f]` is voice `v`'s F0 at frame `f`, or `0.0` where the voice is
///   inactive/abstaining. Cleared on entry.
///
/// Returns the number of voices that were assigned at least one estimate.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_voices` is `0` or `> MAX_VOICES`,
///   `max_per_frame > MAX_ESTIMATES`, `max_jump_semitones` is not positive
///   finite, or `frame_f0.len() < n_frames * max_per_frame`.
/// - [`AudioError::OutputBufferTooSmall`] if `out_tracks.len() < n_voices *
///   n_frames`.
pub fn track_multi_pitch(
    frame_f0: &[f32],
    n_frames: usize,
    max_per_frame: usize,
    n_voices: usize,
    max_jump_semitones: f32,
    out_tracks: &mut [f32],
) -> Result<usize, AudioError> {
    if n_voices == 0
        || n_voices > MAX_VOICES
        || max_per_frame > MAX_ESTIMATES
        || !(max_jump_semitones.is_finite() && max_jump_semitones > 0.0)
        || frame_f0.len() < n_frames.saturating_mul(max_per_frame)
    {
        return Err(AudioError::InvalidParameter);
    }
    if out_tracks.len() < n_voices.saturating_mul(n_frames) {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for v in out_tracks.iter_mut().take(n_voices * n_frames) {
        *v = 0.0;
    }
    if n_frames == 0 || max_per_frame == 0 {
        return Ok(0);
    }

    // Per-voice state: last pitch (0 = free), silent-gap counter, ever-used flag.
    let mut last_f0 = [0.0f32; MAX_VOICES];
    let mut gap = [0usize; MAX_VOICES];
    let mut ever_used = [false; MAX_VOICES];

    for f in 0..n_frames {
        let row = f * max_per_frame;

        // Gather this frame's valid estimates.
        let mut est = [0.0f32; MAX_ESTIMATES];
        let mut est_taken = [false; MAX_ESTIMATES];
        let mut ne = 0usize;
        for c in 0..max_per_frame {
            let v = frame_f0[row + c];
            if v.is_finite() && v > 0.0 {
                est[ne] = v;
                ne += 1;
            }
        }

        let mut v_assigned = [false; MAX_VOICES];

        // (1) Greedily continue active voices: repeatedly bind the globally
        // closest (estimate, active-voice) pair within the jump limit.
        loop {
            let mut best_d = max_jump_semitones;
            let mut best_e = usize::MAX;
            let mut best_v = usize::MAX;
            for e in 0..ne {
                if est_taken[e] {
                    continue;
                }
                for v in 0..n_voices {
                    if v_assigned[v] || last_f0[v] <= 0.0 {
                        continue;
                    }
                    let d = semitone_distance(est[e], last_f0[v]);
                    if d <= best_d {
                        best_d = d;
                        best_e = e;
                        best_v = v;
                    }
                }
            }
            if best_e == usize::MAX {
                break;
            }
            out_tracks[best_v * n_frames + f] = est[best_e];
            last_f0[best_v] = est[best_e];
            gap[best_v] = 0;
            ever_used[best_v] = true;
            est_taken[best_e] = true;
            v_assigned[best_v] = true;
        }

        // (2) Unmatched estimates start new voices (if a free slot exists).
        for e in 0..ne {
            if est_taken[e] {
                continue;
            }
            let mut slot = usize::MAX;
            for v in 0..n_voices {
                if !v_assigned[v] && last_f0[v] <= 0.0 {
                    slot = v;
                    break;
                }
            }
            if slot == usize::MAX {
                continue; // more sources than declared voices → drop the surplus
            }
            out_tracks[slot * n_frames + f] = est[e];
            last_f0[slot] = est[e];
            gap[slot] = 0;
            ever_used[slot] = true;
            est_taken[e] = true;
            v_assigned[slot] = true;
        }

        // (3) Active voices with no estimate abstain this frame; free after a gap.
        for v in 0..n_voices {
            if !v_assigned[v] && last_f0[v] > 0.0 {
                gap[v] += 1;
                if gap[v] > MAX_GAP {
                    last_f0[v] = 0.0;
                    gap[v] = 0;
                }
            }
        }
    }

    let n_active = ever_used.iter().take(n_voices).filter(|&&u| u).count();
    Ok(n_active)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semitone_err(a: f32, b: f32) -> f32 {
        12.0 * (a / b).log2().abs()
    }

    /// GOLDEN: two concurrent lines — a rising ~440→466 Hz voice and a steady
    /// ~660 Hz voice — presented with the two estimates in *different column
    /// order* each frame. The tracker recovers two continuous voices, correctly
    /// separated by pitch continuity rather than by column.
    #[test]
    fn tracks_two_concurrent_voices() {
        // frame layout (max_per_frame = 2), deliberately column-swapped:
        //   f0: [440, 660]
        //   f1: [660, 440]   (swapped)
        //   f2: [466, 662]
        let frame_f0 = [
            440.0f32, 660.0, //
            660.0, 440.0, //
            466.0, 662.0, //
        ];
        let n_frames = 3;
        let mut out = [0.0f32; 4 * 3];
        let n = track_multi_pitch(&frame_f0, n_frames, 2, 4, 2.0, &mut out).expect("track");

        assert_eq!(n, 2, "two concurrent lines → two voices");

        // Voice 0 was seeded at 440 (frame 0 col 0), voice 1 at 660.
        let v0 = &out[0..3];
        let v1 = &out[3..6];
        assert!(semitone_err(v0[0], 440.0) < 0.5, "v0f0={}", v0[0]);
        assert!(semitone_err(v0[1], 440.0) < 0.5, "v0f1={}", v0[1]);
        assert!(semitone_err(v0[2], 466.0) < 0.5, "v0f2={}", v0[2]);
        assert!(semitone_err(v1[0], 660.0) < 0.5, "v1f0={}", v1[0]);
        assert!(semitone_err(v1[1], 660.0) < 0.5, "v1f1={}", v1[1]);
        assert!(semitone_err(v1[2], 662.0) < 0.5, "v1f2={}", v1[2]);

        // Within-track pitch jumps never exceed the declared limit.
        assert!(semitone_err(v0[1], v0[0]) <= 2.0);
        assert!(semitone_err(v0[2], v0[1]) <= 2.0);
    }

    /// A single continuous line uses exactly one voice; a mid-line gap shorter
    /// than MAX_GAP is tolerated and the voice resumes on the same slot.
    #[test]
    fn single_voice_tolerates_short_gap() {
        // 440 → (gap) → 440 over three frames, one estimate per frame.
        let frame_f0 = [440.0f32, 0.0, 442.0];
        let mut out = [0.0f32; 2 * 3];
        let n = track_multi_pitch(&frame_f0, 3, 1, 2, 2.0, &mut out).expect("track");
        assert_eq!(n, 1, "one line → one voice across the gap");
        assert!(semitone_err(out[0], 440.0) < 0.5);
        assert_eq!(out[1], 0.0, "abstains during the gap frame");
        assert!(
            semitone_err(out[2], 442.0) < 0.5,
            "resumes on the same voice"
        );
    }

    /// More sources than declared voices → the surplus is dropped, not forced
    /// onto an unrelated voice (max-polyphony assumption honoured).
    #[test]
    fn drops_surplus_beyond_declared_voices() {
        let frame_f0 = [440.0f32, 660.0]; // two sources, one frame
        let mut out = [0.0f32; 1 * 1];
        let n = track_multi_pitch(&frame_f0, 1, 2, 1, 2.0, &mut out).expect("track");
        assert_eq!(n, 1, "only one voice allowed");
        assert!(
            semitone_err(out[0], 440.0) < 0.5,
            "keeps the first-listed source"
        );
    }

    /// No estimates anywhere → no voices.
    #[test]
    fn all_silent_no_voices() {
        let frame_f0 = [0.0f32; 6];
        let mut out = [9.0f32; 3 * 2];
        let n = track_multi_pitch(&frame_f0, 2, 3, 3, 2.0, &mut out).expect("track");
        assert_eq!(n, 0);
        assert!(
            out.iter().all(|&v| v == 0.0),
            "output cleared to abstention"
        );
    }

    #[test]
    fn rejects_bad_params() {
        let f = [440.0f32];
        let mut out = [0.0f32; 4];
        assert_eq!(
            track_multi_pitch(&f, 1, 1, 0, 2.0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            track_multi_pitch(&f, 1, 1, MAX_VOICES + 1, 2.0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            track_multi_pitch(&f, 1, 1, 2, 0.0, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let f = [440.0f32, 0.0];
        let mut out = [0.0f32; 1]; // needs n_voices * n_frames = 2 * 2 = 4
        assert_eq!(
            track_multi_pitch(&f, 2, 1, 2, 2.0, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
