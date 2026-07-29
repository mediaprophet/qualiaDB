//! Deinterleave an interleaved stereo buffer into two mono channels.
//!
//! `left[i] = in[2i]`, `right[i] = in[2i+1]`. Caller-buffered, **zero-alloc**.

use crate::types::AudioError;

/// Split interleaved stereo `interleaved` (L,R,L,R,…) into `left` / `right`.
///
/// Processes `n = min(interleaved.len()/2, left.len(), right.len())` frames and
/// returns the frame count. Errors if `left` or `right` cannot hold the frames
/// available in `interleaved`.
pub fn demux(
    interleaved: &[f32],
    left: &mut [f32],
    right: &mut [f32],
) -> Result<usize, AudioError> {
    let frames = interleaved.len() / 2;
    if left.len() < frames || right.len() < frames {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for i in 0..frames {
        left[i] = interleaved[i * 2];
        right[i] = interleaved[i * 2 + 1];
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinterleaves_in_order() {
        let inter = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut l = [0.0f32; 3];
        let mut r = [0.0f32; 3];
        let n = demux(&inter, &mut l, &mut r).unwrap();
        assert_eq!(n, 3);
        assert_eq!(l, [1.0, 3.0, 5.0]);
        assert_eq!(r, [2.0, 4.0, 6.0]);
    }

    #[test]
    fn out_too_small_errors() {
        let inter = [1.0f32, 2.0, 3.0, 4.0];
        let mut l = [0.0f32; 1];
        let mut r = [0.0f32; 2];
        assert_eq!(
            demux(&inter, &mut l, &mut r),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
