//! Segment a mono signal into voiced regions with hysteresis and hangover.
//!
//! A raw per-frame decision ([`crate::features::vad::frame_is_voiced`]) chatters
//! at speech onsets/offsets and on brief dropouts (stops, glottal gaps). This
//! module turns the frame-level proposal stream into stable `[start, end)`
//! frame-index segments using two duration guards:
//!
//! - **min-speech (onset debounce):** a run of voiced frames must reach a
//!   minimum length before a segment *opens*, rejecting isolated spurious
//!   frames.
//! - **min-silence (hangover):** once open, a segment only *closes* after a
//!   sustained run of unvoiced frames, so a short dropout inside speech does not
//!   split it.
//!
//! The noise floor is adaptive and streamed online: a fixed **stack** ring of
//! the most recent per-frame energies feeds
//! [`crate::features::vad::noise_floor_min_stat`], so the floor tracks the quiet
//! frames without any heap allocation.
//!
//! EPISTEMIC RULE: output segments are *proposals*. Frames at or below the
//! adaptive floor stay unvoiced — silence is never coerced into a segment.

use crate::features::vad::frame_vad::frame_is_voiced;
use crate::features::energy::frame_energy;
use crate::features::vad::noise_estimate::noise_floor_min_stat;
use crate::types::AudioError;

/// Trailing frames used by the online minimum-statistics noise floor.
const NOISE_WIN: usize = 64;
/// Minimum voiced-run duration (ms) to open a segment (onset debounce).
const MIN_SPEECH_MS: u32 = 32;
/// Minimum unvoiced-run duration (ms) to close a segment (hangover).
const MIN_SILENCE_MS: u32 = 120;

/// Convert a duration in milliseconds to a frame count for the given `hop` and
/// `sample_rate`, clamped to at least one frame.
fn ms_to_frames(ms: u32, hop: usize, sample_rate: u32) -> usize {
    if hop == 0 || sample_rate == 0 {
        return 1;
    }
    let frames = (ms as u64 * sample_rate as u64) / (1000u64 * hop as u64);
    (frames as usize).max(1)
}

/// Detect voiced segments in `signal`, writing `[start_frame, end_frame)` index
/// pairs into `out_segments`.
///
/// - `signal`: mono samples in `[-1, 1]` (or any linear scale).
/// - `frame_len`: analysis window length in samples (power-of-two enables the
///   spectral-flatness cue; see [`crate::features::vad::frame_voicing_score`]).
/// - `hop`: advance between frame starts in samples.
/// - `sample_rate`: used only to convert the hysteresis durations to frames.
/// - `out_segments`: caller-owned output; each entry is `(start_frame, end_frame)`
///   with `end` exclusive, in frame-index units.
///
/// Returns the number of segments written.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `frame_len` or `hop` is zero.
/// - [`AudioError::OutputBufferTooSmall`] if there are more segments than
///   `out_segments` can hold (segments written so far are left in place).
///
/// Zero-heap: a single streaming pass over `signal`; the only working state is a
/// fixed stack ring for the adaptive noise floor.
pub fn segment_voiced(
    signal: &[f32],
    frame_len: usize,
    hop: usize,
    sample_rate: u32,
    out_segments: &mut [(u32, u32)],
) -> Result<usize, AudioError> {
    if frame_len == 0 || hop == 0 {
        return Err(AudioError::InvalidParameter);
    }

    let num_frames = if signal.len() < frame_len {
        0
    } else {
        (signal.len() - frame_len) / hop + 1
    };

    let min_speech = ms_to_frames(MIN_SPEECH_MS, hop, sample_rate);
    let min_silence = ms_to_frames(MIN_SILENCE_MS, hop, sample_rate);

    // Online minimum-statistics noise floor over a fixed stack ring.
    let mut ring = [0.0f32; NOISE_WIN];
    let mut ring_len = 0usize; // number of valid entries (saturates at NOISE_WIN)
    let mut ring_pos = 0usize; // next write index

    // Hysteresis state machine.
    let mut in_speech = false;
    let mut seg_start = 0usize;
    let mut voiced_run = 0usize; // consecutive voiced frames while in silence
    let mut silence_run = 0usize; // consecutive unvoiced frames while in speech
    let mut count = 0usize;

    for f in 0..num_frames {
        let s = f * hop;
        let frame = &signal[s..s + frame_len];

        // Update the adaptive floor from recent energies (this frame included so
        // a lone loud frame cannot depress the floor).
        let e = frame_energy(frame);
        ring[ring_pos] = e;
        ring_pos = (ring_pos + 1) % NOISE_WIN;
        if ring_len < NOISE_WIN {
            ring_len += 1;
        }
        let floor = noise_floor_min_stat(&ring[..ring_len], NOISE_WIN);

        let voiced = frame_is_voiced(frame, floor);

        if in_speech {
            if voiced {
                silence_run = 0;
            } else {
                silence_run += 1;
                if silence_run >= min_silence {
                    // Segment ended at the last voiced frame: current frame `f`
                    // is the `silence_run`-th unvoiced frame, so the last voiced
                    // frame was `f - silence_run`; end is exclusive.
                    let end = (f - silence_run + 1) as u32;
                    write_segment(out_segments, &mut count, seg_start as u32, end)?;
                    in_speech = false;
                    silence_run = 0;
                    voiced_run = 0;
                }
            }
        } else if voiced {
            voiced_run += 1;
            if voiced_run >= min_speech {
                in_speech = true;
                // The run began `voiced_run - 1` frames before the current one.
                seg_start = f - (voiced_run - 1);
                voiced_run = 0;
                silence_run = 0;
            }
        } else {
            voiced_run = 0;
        }
    }

    // Flush an open segment at end-of-signal (closes at the last frame).
    if in_speech {
        write_segment(out_segments, &mut count, seg_start as u32, num_frames as u32)?;
    }

    Ok(count)
}

/// Append one `[start, end)` segment, guarding output capacity.
fn write_segment(
    out: &mut [(u32, u32)],
    count: &mut usize,
    start: u32,
    end: u32,
) -> Result<(), AudioError> {
    if *count >= out.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    out[*count] = (start, end);
    *count += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    const SR: u32 = 16_000;
    const FRAME: usize = 512;
    const HOP: usize = 256;

    /// Build silence · tone-burst · silence. Returns (signal, tone_start_sample,
    /// tone_end_sample).
    fn build_burst(lead: usize, burst: usize, trail: usize) -> (Vec<f32>, usize, usize) {
        let mut sig = vec![0.0f32; lead + burst + trail];
        for i in 0..burst {
            let t = i as f32 / SR as f32;
            sig[lead + i] = 0.5 * (TAU * 440.0 * t).sin();
        }
        (sig, lead, lead + burst)
    }

    fn frames_for(len: usize) -> usize {
        if len < FRAME { 0 } else { (len - FRAME) / HOP + 1 }
    }

    #[test]
    fn one_segment_covers_the_burst_not_the_silence() {
        let (sig, tstart, tend) = build_burst(3000, 6000, 3000);
        let n_frames = frames_for(sig.len());
        let mut segs = [(0u32, 0u32); 8];
        let c = segment_voiced(&sig, FRAME, HOP, SR, &mut segs).expect("segment");

        assert_eq!(c, 1, "expected exactly one voiced segment, got {c}: {:?}", &segs[..c]);
        let (start, end) = segs[0];

        // Expected onset/offset in frame indices (a frame at sample s spans
        // [s, s+FRAME)). Allow a few frames of tolerance for the analysis window
        // straddling the boundaries and the onset/hangover guards.
        let onset_frame = (tstart / HOP) as u32;
        let offset_frame = (tend / HOP) as u32;

        assert!(
            start >= 5 && start <= onset_frame + 3,
            "start {start} should exclude lead silence and be near onset {onset_frame}"
        );
        assert!(
            end >= offset_frame - 3 && end < n_frames as u32,
            "end {end} should reach offset {offset_frame} and exclude trailing silence (n={n_frames})"
        );
        // The burst body is inside the segment; lead/trail silence frames are not.
        assert!(start < onset_frame + 2 && end > offset_frame - 2, "segment must cover the burst");
        assert!(start > 2, "early silence frame 2 must be outside the segment");
    }

    #[test]
    fn one_frame_dropout_does_not_split_segment() {
        let (mut sig, tstart, tend) = build_burst(3000, 6000, 3000);
        // Zero one full frame's worth in the middle of the burst → a single
        // silent frame. The hangover must bridge it.
        let mid = tstart + (tend - tstart) / 2;
        for x in sig[mid..mid + FRAME].iter_mut() {
            *x = 0.0;
        }
        let mut segs = [(0u32, 0u32); 8];
        let c = segment_voiced(&sig, FRAME, HOP, SR, &mut segs).expect("segment");
        assert_eq!(c, 1, "a 1-frame dropout must not split the burst, got {c}: {:?}", &segs[..c]);
    }

    #[test]
    fn pure_silence_yields_no_segments() {
        let sig = vec![0.0f32; 12_000];
        let mut segs = [(0u32, 0u32); 8];
        let c = segment_voiced(&sig, FRAME, HOP, SR, &mut segs).expect("segment");
        assert_eq!(c, 0, "silence must not be forced into a voiced segment");
    }

    #[test]
    fn rejects_zero_params() {
        let sig = vec![0.0f32; 1000];
        let mut segs = [(0u32, 0u32); 4];
        assert_eq!(
            segment_voiced(&sig, 0, HOP, SR, &mut segs),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            segment_voiced(&sig, FRAME, 0, SR, &mut segs),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn full_output_buffer_reports_too_small() {
        // Two separated bursts, but room for only one segment.
        let mut sig = vec![0.0f32; 3000];
        let (b1, _, _) = build_burst(0, 4000, 3000);
        sig.extend_from_slice(&b1);
        let (b2, _, _) = build_burst(0, 4000, 3000);
        sig.extend_from_slice(&b2);
        let mut segs = [(0u32, 0u32); 1];
        let r = segment_voiced(&sig, FRAME, HOP, SR, &mut segs);
        assert_eq!(r, Err(AudioError::OutputBufferTooSmall));
    }

    #[test]
    fn signal_shorter_than_frame_is_empty() {
        let sig = vec![0.5f32; FRAME - 1];
        let mut segs = [(0u32, 0u32); 4];
        assert_eq!(segment_voiced(&sig, FRAME, HOP, SR, &mut segs), Ok(0));
    }
}
