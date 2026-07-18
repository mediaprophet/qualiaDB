//! End-to-end audio → MIDI note transcription.
//!
//! Pipeline: slice the mono signal into overlapping analysis frames → estimate a
//! per-frame fundamental with [`yin_pitch`](crate::features::pitch::yin_pitch) →
//! [`segment_notes`](super::note_segmentation::segment_notes) that pitch track
//! into MIDI note events. This is the composed convenience entry point; the two
//! stages remain usable on their own.
//!
//! Epistemic contract: the emitted notes are *proposals* carrying the mean
//! voicing confidence of their frames — transcribed MIDI is never authoritative
//! the way imported MIDI is.
//!
//! Zero-heap hot path: the caller supplies the frame/hop geometry, the two
//! per-frame track buffers (`f0_scratch`, `conf_scratch`), the YIN work buffer
//! (`yin_scratch`), and the `out` note array. This function allocates nothing.

use crate::features::pitch::yin_pitch;
use crate::features::pitch_midi::note_segmentation::{segment_notes, NoteEvent};
use crate::types::AudioError;

/// Transcribe a mono `samples` buffer into MIDI note events.
///
/// - `samples`: mono PCM in `[-1, 1]`.
/// - `sample_rate`: sampling rate in Hz (> 0).
/// - `frame_size`: analysis window length in samples (≥ 8).
/// - `hop_size`: advance between frame starts in samples (> 0).
/// - `min_hz` / `max_hz`: YIN fundamental search band (`0 < min_hz < max_hz`).
/// - `yin_threshold`: YIN absolute CMND threshold (typical `0.10..0.20`).
/// - `ref_a4_hz`: reference tuning for A4 (default `440.0`).
/// - `min_note_frames`: minimum note length in frames (short runs are dropped).
/// - `f0_scratch` / `conf_scratch`: per-frame track buffers; each must hold at
///   least `n_frames = (samples.len() − frame_size) / hop_size + 1` floats.
/// - `yin_scratch`: YIN difference/CMND work buffer (see [`yin_pitch`]).
/// - `out`: caller-owned destination for the note events.
///
/// Returns the number of notes written into `out`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] for a non-positive rate/hop, a
///   `frame_size < 8`, an inconsistent search band, or a signal shorter than one
///   frame.
/// - [`AudioError::WorkspaceTooSmall`] if `f0_scratch`/`conf_scratch` cannot hold
///   every frame.
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold every note.
/// - Any error propagated by [`yin_pitch`] (e.g. a too-small `yin_scratch`).
#[allow(clippy::too_many_arguments)]
pub fn audio_to_midi(
    samples: &[f32],
    sample_rate: f32,
    frame_size: usize,
    hop_size: usize,
    min_hz: f32,
    max_hz: f32,
    yin_threshold: f32,
    ref_a4_hz: f32,
    min_note_frames: usize,
    f0_scratch: &mut [f32],
    conf_scratch: &mut [f32],
    yin_scratch: &mut [f32],
    out: &mut [NoteEvent],
) -> Result<usize, AudioError> {
    if !sample_rate.is_finite() || sample_rate <= 0.0 || hop_size == 0 || frame_size < 8 {
        return Err(AudioError::InvalidParameter);
    }
    if samples.len() < frame_size {
        return Err(AudioError::InvalidParameter);
    }
    // Number of complete frames that fit with this hop.
    let n_frames = (samples.len() - frame_size) / hop_size + 1;
    if f0_scratch.len() < n_frames || conf_scratch.len() < n_frames {
        return Err(AudioError::WorkspaceTooSmall);
    }

    // Stage 1: per-frame fundamental estimation.
    for f in 0..n_frames {
        let start = f * hop_size;
        let frame = &samples[start..start + frame_size];
        let est = yin_pitch(frame, sample_rate, min_hz, max_hz, yin_threshold, yin_scratch)?;
        f0_scratch[f] = est.f0_hz;
        conf_scratch[f] = est.confidence;
    }

    // Stage 2: segment the pitch track into note proposals.
    segment_notes(
        &f0_scratch[..n_frames],
        &conf_scratch[..n_frames],
        n_frames,
        ref_a4_hz,
        min_note_frames,
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    /// Synthesise a pure sine of `freq` Hz for `n` samples at `sr`.
    fn sine(freq: f32, sr: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * PI * freq * i as f32 / sr).sin()).collect()
    }

    #[test]
    fn synth_a4_tone_yields_one_note_69() {
        let sr = 44_100.0f32;
        let frame_size = 2048usize;
        let hop = 512usize;
        // ~0.35 s of clean 440 Hz — enough for several overlapping frames.
        let samples = sine(440.0, sr, 16_000);

        let n_frames = (samples.len() - frame_size) / hop + 1;
        let mut f0 = vec![0.0f32; n_frames];
        let mut conf = vec![0.0f32; n_frames];
        // YIN scratch: max_lag = sr/min_hz + 1; min_hz = 80 → ~552.
        let mut yin_scratch = vec![0.0f32; frame_size / 2 + 1];
        let mut out = [NoteEvent::empty(); 16];

        let n = audio_to_midi(
            &samples, sr, frame_size, hop, 80.0, 1000.0, 0.15, 440.0, 3, &mut f0, &mut conf,
            &mut yin_scratch, &mut out,
        )
        .expect("audio_to_midi");

        assert_eq!(n, 1, "a single sustained tone → one note");
        assert_eq!(out[0].note, 69, "440 Hz → MIDI 69 (A4)");
        assert!(out[0].confidence > 0.5, "clean sine is confidently voiced");
        assert_eq!(out[0].start_frame, 0);
        assert_eq!(out[0].end_frame, n_frames as u32);
    }

    #[test]
    fn rejects_too_short_signal() {
        let mut f0 = [0.0f32; 4];
        let mut conf = [0.0f32; 4];
        let mut yin_scratch = [0.0f32; 64];
        let mut out = [NoteEvent::empty(); 4];
        let err = audio_to_midi(
            &[0.0f32; 10], 44_100.0, 2048, 512, 80.0, 1000.0, 0.15, 440.0, 3, &mut f0, &mut conf,
            &mut yin_scratch, &mut out,
        )
        .unwrap_err();
        assert_eq!(err, AudioError::InvalidParameter);
    }

    #[test]
    fn rejects_undersized_track_scratch() {
        let sr = 44_100.0f32;
        let samples = sine(440.0, sr, 8000);
        let mut f0 = [0.0f32; 2]; // too small
        let mut conf = [0.0f32; 2];
        let mut yin_scratch = vec![0.0f32; 1025];
        let mut out = [NoteEvent::empty(); 8];
        let err = audio_to_midi(
            &samples, sr, 2048, 512, 80.0, 1000.0, 0.15, 440.0, 3, &mut f0, &mut conf,
            &mut yin_scratch, &mut out,
        )
        .unwrap_err();
        assert_eq!(err, AudioError::WorkspaceTooSmall);
    }
}
