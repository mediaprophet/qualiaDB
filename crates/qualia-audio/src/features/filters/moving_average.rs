//! Running (boxcar) mean over a fixed window — a simple FIR low-pass / smoother.
//!
//! Caller-buffered and zero-heap: the running sum is carried in two scalars and
//! the trailing window is read directly from the input buffer, so no per-call
//! allocation is needed.

use crate::types::AudioError;

/// Boxcar moving average with window `window` samples.
///
/// `out[i]` is the mean of the up-to-`window` most recent input samples
/// (`x[i-window+1 ..= i]`), clamped at the start of the buffer where fewer
/// samples are available. This makes it a causal streaming smoother.
///
/// Errors: `InvalidParameter` if `window == 0`; `OutputBufferTooSmall` if
/// `out` is shorter than `x`.
pub fn moving_average(x: &[f32], out: &mut [f32], window: usize) -> Result<(), AudioError> {
    if window == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < x.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let mut sum = 0.0f32;
    for i in 0..x.len() {
        sum += x[i];
        // Once the window is full, drop the sample leaving the window.
        if i >= window {
            sum -= x[i - window];
        }
        let count = if i + 1 < window { i + 1 } else { window };
        out[i] = sum / count as f32;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_input_constant_output() {
        let x = [2.0f32; 32];
        let mut out = [0.0f32; 32];
        moving_average(&x, &mut out, 5).expect("valid");
        for &y in &out {
            assert!((y - 2.0).abs() < 1e-6);
        }
    }

    #[test]
    fn smooths_step_edge() {
        // Step from 0 to 1; window-4 average ramps across 4 samples.
        let mut x = [0.0f32; 16];
        for v in x.iter_mut().skip(8) {
            *v = 1.0;
        }
        let mut out = [0.0f32; 16];
        moving_average(&x, &mut out, 4).expect("valid");
        assert!((out[7]).abs() < 1e-6); // still fully in the 0 region
        assert!((out[8] - 0.25).abs() < 1e-6);
        assert!((out[9] - 0.5).abs() < 1e-6);
        assert!((out[10] - 0.75).abs() < 1e-6);
        assert!((out[11] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn window_one_is_identity() {
        let x = [0.1, -0.4, 0.9, -0.2];
        let mut out = [0.0f32; 4];
        moving_average(&x, &mut out, 1).expect("valid");
        for i in 0..x.len() {
            assert!((out[i] - x[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn zero_window_errors() {
        let x = [1.0f32; 4];
        let mut out = [0.0f32; 4];
        assert_eq!(
            moving_average(&x, &mut out, 0),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn short_output_errors() {
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            moving_average(&x, &mut out, 2),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
