//! Segment a per-frame `(f0, confidence)` pitch track into MIDI note ON/OFF
//! events.
//!
//! Each voiced frame is quantised to a MIDI note number (via
//! [`hz_to_midi`](super::pitch_to_midi::hz_to_midi)). A *note* is a maximal run
//! of consecutive frames that quantise to the **same** note number; the run ends
//! when the pitch crosses the half-semitone quantisation boundary into a
//! different note (the pitch-change threshold) or when a low-confidence /
//! unvoiced *gap* interrupts voicing. Runs shorter than `min_note_frames` are
//! discarded as transients, so a single glitchy frame does not become a note.
//!
//! Epistemic contract: every emitted [`NoteEvent`] is a *proposal*, not an
//! authoritative transcription. It carries the mean voicing `confidence` of its
//! frames, and `velocity` is derived from that same confidence — downstream code
//! can weight or reject a proposal on it. Transcribed MIDI must never be treated
//! as ground truth the way imported MIDI is.
//!
//! Zero-heap hot path: note events are written into the caller-supplied `out`
//! slice; the function allocates nothing.

use crate::features::pitch_midi::pitch_to_midi::hz_to_midi;
use crate::types::AudioError;

/// Minimum voicing confidence for a frame to count as pitched. Frames below this
/// (or with `f0 <= 0`) are treated as an unvoiced gap that ends the current note.
pub const MIN_VOICED_CONFIDENCE: f32 = 0.5;

/// One transcribed note proposal.
///
/// `start_frame` is the index of the first frame of the note; `end_frame` is the
/// index one past its last frame (exclusive), so `end_frame − start_frame` is the
/// note's duration in frames. `note` is the MIDI note number, `velocity` is a
/// `1..=127` value derived from voicing confidence, and `confidence` is the mean
/// voicing confidence over the note's frames.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    pub start_frame: u32,
    pub end_frame: u32,
    pub confidence: f32,
}

impl NoteEvent {
    /// A zeroed placeholder, useful for initialising a caller-owned `out` array.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            note: 0,
            velocity: 0,
            start_frame: 0,
            end_frame: 0,
            confidence: 0.0,
        }
    }
}

/// Map a mean voicing confidence in `[0, 1]` to a MIDI velocity in `1..=127`.
#[inline]
fn confidence_to_velocity(mean_conf: f32) -> u8 {
    let c = if mean_conf.is_finite() {
        mean_conf.clamp(0.0, 1.0)
    } else {
        0.0
    };
    // 0 → 1, 1 → 127. A voiced note always has velocity ≥ 1 (audible).
    (c * 126.0 + 1.0).round() as u8
}

/// An in-progress note accumulator (reset between notes; lives on the stack).
struct OpenNote {
    note: u8,
    start_frame: u32,
    end_frame: u32,
    conf_sum: f32,
    conf_n: u32,
}

/// Segment a per-frame pitch track into note events.
///
/// - `f0_hz`: per-frame fundamental in Hz (`0.0` = unvoiced). Length ≥ `n_frames`.
/// - `confidence`: per-frame voicing confidence in `[0, 1]`. Length ≥ `n_frames`.
/// - `n_frames`: number of valid frames to read from the two tracks.
/// - `ref_a4_hz`: reference tuning for A4 (default `440.0`).
/// - `min_note_frames`: minimum note length in frames; shorter runs are dropped
///   (a value of `0` is treated as `1`).
/// - `out`: caller-owned destination for the note events.
///
/// Returns the number of notes written into `out`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if either input track is shorter than
///   `n_frames`, or `ref_a4_hz` is not positive/finite.
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold every note.
pub fn segment_notes(
    f0_hz: &[f32],
    confidence: &[f32],
    n_frames: usize,
    ref_a4_hz: f32,
    min_note_frames: usize,
    out: &mut [NoteEvent],
) -> Result<usize, AudioError> {
    if f0_hz.len() < n_frames || confidence.len() < n_frames {
        return Err(AudioError::InvalidParameter);
    }
    if !ref_a4_hz.is_finite() || ref_a4_hz <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let min_len = min_note_frames.max(1) as u32;

    let mut count = 0usize;
    let mut open: Option<OpenNote> = None;

    for i in 0..n_frames {
        let f0 = f0_hz[i];
        let conf = confidence[i];
        let voiced =
            f0.is_finite() && f0 > 0.0 && conf.is_finite() && conf >= MIN_VOICED_CONFIDENCE;

        if voiced {
            let (note, _cents) = hz_to_midi(f0, ref_a4_hz);
            match open.as_mut() {
                // Continue the current note: same quantised pitch.
                Some(cur) if cur.note == note => {
                    cur.end_frame = i as u32 + 1;
                    cur.conf_sum += conf;
                    cur.conf_n += 1;
                }
                // Pitch changed (or nothing open): flush then open a fresh note.
                _ => {
                    if let Some(cur) = open.take() {
                        flush(&cur, min_len, out, &mut count)?;
                    }
                    open = Some(OpenNote {
                        note,
                        start_frame: i as u32,
                        end_frame: i as u32 + 1,
                        conf_sum: conf,
                        conf_n: 1,
                    });
                }
            }
        } else {
            // Low-confidence / unvoiced gap ends the current note.
            if let Some(cur) = open.take() {
                flush(&cur, min_len, out, &mut count)?;
            }
        }
    }

    // Flush a note still open at the end of the track.
    if let Some(cur) = open.take() {
        flush(&cur, min_len, out, &mut count)?;
    }

    Ok(count)
}

/// Emit `cur` into `out[*count]` if it meets the minimum duration.
#[inline]
fn flush(
    cur: &OpenNote,
    min_len: u32,
    out: &mut [NoteEvent],
    count: &mut usize,
) -> Result<(), AudioError> {
    let duration = cur.end_frame - cur.start_frame;
    if duration < min_len {
        return Ok(());
    }
    if *count >= out.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let mean_conf = if cur.conf_n > 0 {
        cur.conf_sum / cur.conf_n as f32
    } else {
        0.0
    };
    out[*count] = NoteEvent {
        note: cur.note,
        velocity: confidence_to_velocity(mean_conf),
        start_frame: cur.start_frame,
        end_frame: cur.end_frame,
        confidence: mean_conf,
    };
    *count += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a constant-confidence track holding `f_a` for `n_a` frames then
    /// `f_b` for `n_b` frames.
    fn two_tone(f_a: f32, n_a: usize, f_b: f32, n_b: usize, conf: f32) -> (Vec<f32>, Vec<f32>) {
        let mut f0 = Vec::new();
        let mut c = Vec::new();
        for _ in 0..n_a {
            f0.push(f_a);
            c.push(conf);
        }
        for _ in 0..n_b {
            f0.push(f_b);
            c.push(conf);
        }
        (f0, c)
    }

    #[test]
    fn jump_a4_to_b4_yields_two_notes() {
        // 440 Hz (A4=69) for 10 frames, then 493.88 Hz (B4=71) for 8 frames.
        let (f0, conf) = two_tone(440.0, 10, 493.88, 8, 0.9);
        let mut out = [NoteEvent::empty(); 8];
        let n = segment_notes(&f0, &conf, 18, 440.0, 3, &mut out).expect("segment");
        assert_eq!(n, 2, "expected exactly two notes");

        assert_eq!(out[0].note, 69);
        assert_eq!(out[0].start_frame, 0);
        assert_eq!(out[0].end_frame, 10);

        assert_eq!(out[1].note, 71);
        assert_eq!(out[1].start_frame, 10);
        assert_eq!(out[1].end_frame, 18);
    }

    #[test]
    fn low_confidence_gap_ends_the_note() {
        // A4 for 6 frames, a 3-frame low-confidence gap, then A4 again for 6.
        let mut f0 = vec![440.0f32; 15];
        let mut conf = vec![0.9f32; 15];
        for i in 6..9 {
            conf[i] = 0.1; // below MIN_VOICED_CONFIDENCE
        }
        // f0 during the gap is irrelevant, but zero it to mimic unvoiced.
        for i in 6..9 {
            f0[i] = 0.0;
        }
        let mut out = [NoteEvent::empty(); 8];
        let n = segment_notes(&f0, &conf, 15, 440.0, 3, &mut out).expect("segment");
        assert_eq!(n, 2, "gap must split into two notes");
        assert_eq!(out[0].note, 69);
        assert_eq!(out[0].start_frame, 0);
        assert_eq!(out[0].end_frame, 6);
        assert_eq!(out[1].note, 69);
        assert_eq!(out[1].start_frame, 9);
        assert_eq!(out[1].end_frame, 15);
    }

    #[test]
    fn short_run_is_discarded() {
        // A 2-frame note below min_note_frames = 4 is dropped.
        let (f0, conf) = two_tone(440.0, 2, 493.88, 8, 0.9);
        let mut out = [NoteEvent::empty(); 8];
        let n = segment_notes(&f0, &conf, 10, 440.0, 4, &mut out).expect("segment");
        assert_eq!(n, 1, "only the long note survives");
        assert_eq!(out[0].note, 71);
        assert_eq!(out[0].start_frame, 2);
    }

    #[test]
    fn velocity_and_confidence_track_voicing() {
        let (f0, conf) = two_tone(440.0, 8, 440.0, 0, 1.0);
        let mut out = [NoteEvent::empty(); 4];
        let n = segment_notes(&f0, &conf, 8, 440.0, 3, &mut out).expect("segment");
        assert_eq!(n, 1);
        assert_eq!(out[0].velocity, 127, "full confidence → max velocity");
        assert!((out[0].confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn output_buffer_too_small_is_reported() {
        // Three distinct notes but out capacity 1.
        let mut f0 = Vec::new();
        let mut conf = Vec::new();
        for &f in &[440.0f32, 493.88, 523.25] {
            for _ in 0..4 {
                f0.push(f);
                conf.push(0.9);
            }
        }
        let mut out = [NoteEvent::empty(); 1];
        let err = segment_notes(&f0, &conf, 12, 440.0, 3, &mut out).unwrap_err();
        assert_eq!(err, AudioError::OutputBufferTooSmall);
    }

    #[test]
    fn short_input_track_is_rejected() {
        let f0 = [440.0f32; 4];
        let conf = [0.9f32; 4];
        let mut out = [NoteEvent::empty(); 4];
        let err = segment_notes(&f0, &conf, 8, 440.0, 3, &mut out).unwrap_err();
        assert_eq!(err, AudioError::InvalidParameter);
    }
}
