//! Interleave two mono channels into an interleaved stereo buffer.
//!
//! `out[2i] = left[i]`, `out[2i+1] = right[i]`. Caller-buffered, **zero-alloc**.

use crate::types::AudioError;

/// Interleave `left` and `right` mono channels into `out` (L,R,L,R,…).
///
/// Processes `n = min(left.len(), right.len())` frames. Returns the frame count.
/// Errors with [`AudioError::OutputBufferTooSmall`] if `out.len() < 2·n`.
pub fn mux(left: &[f32], right: &[f32], out: &mut [f32]) -> Result<usize, AudioError> {
    let n = left.len().min(right.len());
    if out.len() < n * 2 {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for i in 0..n {
        out[i * 2] = left[i];
        out[i * 2 + 1] = right[i];
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::stereo::demux::demux;

    #[test]
    fn interleaves_in_order() {
        let l = [1.0f32, 3.0, 5.0];
        let r = [2.0f32, 4.0, 6.0];
        let mut out = [0.0f32; 6];
        let n = mux(&l, &r, &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn mux_then_demux_is_identity() {
        let l: Vec<f32> = (0..64).map(|i| (i as f32 * 0.013).sin()).collect();
        let r: Vec<f32> = (0..64).map(|i| (i as f32 * 0.031).cos()).collect();
        let mut inter = vec![0.0f32; 128];
        let n = mux(&l, &r, &mut inter).unwrap();
        assert_eq!(n, 64);
        let mut l2 = vec![0.0f32; 64];
        let mut r2 = vec![0.0f32; 64];
        let m = demux(&inter, &mut l2, &mut r2).unwrap();
        assert_eq!(m, 64);
        for i in 0..64 {
            assert!((l[i] - l2[i]).abs() < 1e-9);
            assert!((r[i] - r2[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn out_too_small_errors() {
        let l = [1.0f32, 2.0];
        let r = [3.0f32, 4.0];
        let mut out = [0.0f32; 2];
        assert_eq!(mux(&l, &r, &mut out), Err(AudioError::OutputBufferTooSmall));
    }
}
