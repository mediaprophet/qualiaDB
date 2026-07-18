//! Explicit framing geometry: how many complete frames a signal yields, and a
//! caller-buffered cutter that copies one frame into a supplied buffer.
//!
//! Zero-heap: [`frame_count`] is pure integer arithmetic; [`cut_frame`] copies
//! into a caller-owned `&mut [f32]` with no allocation.

use crate::types::AudioError;

/// Number of **complete** frames of length `frame_size` obtainable from a signal
/// of `signal_len` samples advancing by `hop` each step.
///
/// Matches the standard STFT geometry `(signal_len − frame_size) / hop + 1` when
/// the signal is at least one frame long, and `0` otherwise. Partial trailing
/// frames (fewer than `frame_size` samples) are not counted.
///
/// A `hop` of `0` is meaningless (frames never advance); it returns `0` rather
/// than looping forever. A `frame_size` of `0` also returns `0`.
#[must_use]
pub fn frame_count(signal_len: usize, frame_size: usize, hop: usize) -> usize {
    if frame_size == 0 || hop == 0 || signal_len < frame_size {
        return 0;
    }
    (signal_len - frame_size) / hop + 1
}

/// Copy frame `index` of a signal (window position `index * hop`) into `out`.
///
/// - `signal`: the source samples.
/// - `frame_size`: number of samples per frame; `out` must be at least this long.
/// - `hop`: sample advance between consecutive frames.
/// - `index`: which frame to extract (0-based).
///
/// Only the first `frame_size` slots of `out` are written. Returns
/// [`AudioError::InvalidParameter`] for a zero `frame_size`/`hop` or if the
/// requested frame runs past the end of `signal`, and
/// [`AudioError::OutputBufferTooSmall`] if `out` is shorter than `frame_size`.
pub fn cut_frame(
    signal: &[f32],
    frame_size: usize,
    hop: usize,
    index: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if frame_size == 0 || hop == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < frame_size {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let start = index.saturating_mul(hop);
    let end = start
        .checked_add(frame_size)
        .ok_or(AudioError::InvalidParameter)?;
    if end > signal.len() {
        return Err(AudioError::InvalidParameter);
    }
    out[..frame_size].copy_from_slice(&signal[start..end]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_count_matches_closed_form() {
        // Golden: (len - frame) / hop + 1 for len >= frame.
        for &(len, frame, hop) in &[
            (1024usize, 256usize, 128usize),
            (1000, 256, 128),
            (256, 256, 128),
            (300, 128, 64),
            (48_000, 1024, 512),
        ] {
            let expected = (len - frame) / hop + 1;
            assert_eq!(frame_count(len, frame, hop), expected, "len={len} frame={frame} hop={hop}");
        }
    }

    #[test]
    fn shorter_than_a_frame_yields_zero() {
        assert_eq!(frame_count(100, 256, 128), 0);
        assert_eq!(frame_count(0, 256, 128), 0);
    }

    #[test]
    fn degenerate_params_yield_zero() {
        assert_eq!(frame_count(1000, 0, 128), 0);
        assert_eq!(frame_count(1000, 256, 0), 0);
    }

    #[test]
    fn cut_frame_copies_correct_window() {
        let signal: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let mut out = [0.0f32; 8];
        cut_frame(&signal, 8, 4, 2, &mut out).unwrap(); // start = 8
        let expected: Vec<f32> = (8..16).map(|i| i as f32).collect();
        assert_eq!(&out[..8], expected.as_slice());
    }

    #[test]
    fn cut_frame_writes_only_frame_size_slots() {
        let signal: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let mut out = [-1.0f32; 10];
        cut_frame(&signal, 6, 3, 0, &mut out).unwrap();
        assert_eq!(&out[..6], &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(&out[6..], &[-1.0, -1.0, -1.0, -1.0]); // untouched
    }

    #[test]
    fn cut_frame_past_end_is_invalid() {
        let signal = [0.0f32; 10];
        let mut out = [0.0f32; 8];
        assert_eq!(cut_frame(&signal, 8, 4, 1, &mut out), Err(AudioError::InvalidParameter));
    }

    #[test]
    fn cut_frame_short_output_is_too_small() {
        let signal = [0.0f32; 16];
        let mut out = [0.0f32; 4];
        assert_eq!(cut_frame(&signal, 8, 4, 0, &mut out), Err(AudioError::OutputBufferTooSmall));
    }

    #[test]
    fn cut_frame_covers_all_frame_count_frames() {
        // Every index in 0..frame_count must succeed; the next one must fail.
        let signal: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let (frame, hop) = (256usize, 128usize);
        let n = frame_count(signal.len(), frame, hop);
        let mut out = vec![0.0f32; frame];
        for i in 0..n {
            cut_frame(&signal, frame, hop, i, &mut out).expect("frame in range");
        }
        assert_eq!(cut_frame(&signal, frame, hop, n, &mut out), Err(AudioError::InvalidParameter));
    }
}
