//! Overlap-add (OLA) reconstruction: sum a sequence of (already-windowed) frames
//! back into a contiguous signal at a fixed hop.
//!
//! Zero-heap: the accumulation writes directly into a caller-owned `out` buffer.
//! Frames are supplied pre-windowed and concatenated (`num_frames × frame_size`
//! floats); no window is applied here — this is the pure add stage, so it works
//! for any analysis/synthesis window the caller chose. With a COLA-satisfying
//! window+hop (e.g. periodic Hann at hop = frame/2) the interior envelope is flat.

use crate::types::AudioError;

/// Overlap-add `frames` into `out` with stride `hop`.
///
/// - `frames`: `num_frames × frame_size` concatenated frame samples (row-major,
///   frame `i` occupies `frames[i*frame_size .. (i+1)*frame_size]`). Already
///   windowed by the caller if a window is desired.
/// - `frame_size`: samples per frame (`> 0`); `frames.len()` must be a whole
///   multiple of it.
/// - `hop`: sample advance between consecutive frames (`> 0`).
/// - `out`: receives the summed signal; must hold at least the returned length.
///
/// Returns the number of output samples written,
/// `(num_frames − 1) × hop + frame_size`. The used region of `out` is zeroed
/// before accumulation, so stale contents do not leak in.
///
/// Errors: [`AudioError::InvalidParameter`] for a zero `frame_size`/`hop` or a
/// `frames` length that is not a multiple of `frame_size`;
/// [`AudioError::OutputBufferTooSmall`] if `out` cannot hold the result.
pub fn overlap_add(
    frames: &[f32],
    frame_size: usize,
    hop: usize,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    if frame_size == 0 || hop == 0 || !frames.len().is_multiple_of(frame_size) {
        return Err(AudioError::InvalidParameter);
    }
    let num_frames = frames.len() / frame_size;
    if num_frames == 0 {
        return Ok(0);
    }
    let out_len = (num_frames - 1)
        .checked_mul(hop)
        .and_then(|v| v.checked_add(frame_size))
        .ok_or(AudioError::InvalidParameter)?;
    if out.len() < out_len {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for v in out[..out_len].iter_mut() {
        *v = 0.0;
    }
    for i in 0..num_frames {
        let frame = &frames[i * frame_size..(i + 1) * frame_size];
        let base = i * hop;
        for (n, &s) in frame.iter().enumerate() {
            out[base + n] += s;
        }
    }
    Ok(out_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::window::hann_window;

    #[test]
    fn single_frame_is_copied_verbatim() {
        let frames = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 4];
        let n = overlap_add(&frames, 4, 2, &mut out).unwrap();
        assert_eq!(n, 4);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn two_frames_sum_in_the_overlap() {
        // frame_size 4, hop 2 → outputs overlap on indices 2,3.
        let frames = [1.0f32, 1.0, 1.0, 1.0, /* f2 */ 2.0, 2.0, 2.0, 2.0];
        let mut out = [0.0f32; 6];
        let n = overlap_add(&frames, 4, 2, &mut out).unwrap();
        assert_eq!(n, 6);
        // idx0,1 = f1 only; idx2,3 = f1+f2; idx4,5 = f2 only.
        assert_eq!(out, [1.0, 1.0, 3.0, 3.0, 2.0, 2.0]);
    }

    #[test]
    fn hann_cola_constant_envelope() {
        // GOLDEN COLA: overlap-adding periodic Hann frames at hop = N/2 gives a
        // constant unity envelope in the steady-state interior.
        let n = 256usize;
        let hop = n / 2;
        let frames_count = 12usize;
        let mut w = vec![0.0f32; n];
        hann_window(&mut w).unwrap();

        let mut frames = vec![0.0f32; frames_count * n];
        for i in 0..frames_count {
            frames[i * n..(i + 1) * n].copy_from_slice(&w);
        }
        let out_len = (frames_count - 1) * hop + n;
        let mut out = vec![0.0f32; out_len];
        overlap_add(&frames, n, hop, &mut out).unwrap();

        // Steady state: indices [n, out_len - n) see full 2-frame overlap.
        for k in n..out_len - n {
            assert!((out[k] - 1.0).abs() < 1e-5, "envelope at {k} = {}", out[k]);
        }
    }

    #[test]
    fn rejects_ragged_frame_length() {
        let frames = [0.0f32; 10]; // not a multiple of 4
        let mut out = [0.0f32; 32];
        assert_eq!(
            overlap_add(&frames, 4, 2, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_short_output() {
        let frames = [0.0f32; 8]; // 2 frames of 4, hop 2 → needs 6
        let mut out = [0.0f32; 5];
        assert_eq!(
            overlap_add(&frames, 4, 2, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn zero_frames_is_zero_length() {
        let frames: [f32; 0] = [];
        let mut out = [0.0f32; 4];
        assert_eq!(overlap_add(&frames, 4, 2, &mut out), Ok(0));
    }
}
