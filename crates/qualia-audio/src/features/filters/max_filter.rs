//! Sliding-window maximum (grey dilation) — envelope / peak-hold helper.
//!
//! Causal: `out[i]` is the maximum of the up-to-`window` most recent input
//! samples (`x[i-window+1 ..= i]`). Bounded naive scan — zero-heap, no
//! allocation, deterministic. For the small windows used in audio framing this
//! is cheaper than maintaining a monotonic deque.

use crate::types::AudioError;

/// Causal sliding maximum with window `window` samples.
///
/// Errors: `InvalidParameter` if `window == 0`; `OutputBufferTooSmall` if
/// `out` is shorter than `x`.
pub fn max_filter(x: &[f32], out: &mut [f32], window: usize) -> Result<(), AudioError> {
    if window == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < x.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for i in 0..x.len() {
        let start = i + 1 - (i + 1).min(window);
        let mut m = x[start];
        for &v in &x[start + 1..=i] {
            if v > m {
                m = v;
            }
        }
        out[i] = m;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_peak_across_window() {
        // Spike at index 3; window-3 max holds it for 3 samples.
        let mut x = [0.0f32; 10];
        x[3] = 5.0;
        let mut out = [0.0f32; 10];
        max_filter(&x, &mut out, 3).expect("valid");
        assert!((out[2]).abs() < 1e-6);
        assert!((out[3] - 5.0).abs() < 1e-6);
        assert!((out[4] - 5.0).abs() < 1e-6);
        assert!((out[5] - 5.0).abs() < 1e-6);
        assert!((out[6]).abs() < 1e-6); // spike has left the window
    }

    #[test]
    fn window_one_is_identity() {
        let x = [0.3, -0.7, 5.0, 0.1];
        let mut out = [0.0f32; 4];
        max_filter(&x, &mut out, 1).expect("valid");
        for i in 0..x.len() {
            assert!((out[i] - x[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn monotonic_increasing_tracks_latest() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f32; 5];
        max_filter(&x, &mut out, 2).expect("valid");
        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[4] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn zero_window_errors() {
        let x = [1.0f32; 4];
        let mut out = [0.0f32; 4];
        assert_eq!(max_filter(&x, &mut out, 0), Err(AudioError::InvalidParameter));
    }

    #[test]
    fn short_output_errors() {
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(max_filter(&x, &mut out, 2), Err(AudioError::OutputBufferTooSmall));
    }
}
