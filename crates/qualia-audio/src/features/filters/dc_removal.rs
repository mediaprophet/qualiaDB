//! First-order DC-blocking high-pass filter.
//!
//! `y[n] = x[n] - x[n-1] + R * y[n-1]` — the classic single-pole DC blocker.
//! `R` (just below 1.0) sets the corner: closer to 1.0 = lower corner. State is
//! two scalars carried on the stack, so block processing is zero-heap.

use crate::types::AudioError;

/// Streaming DC blocker. One instance per mono channel.
#[derive(Debug, Clone, Copy)]
pub struct DcBlocker {
    r: f32,
    x1: f32,
    y1: f32,
}

impl DcBlocker {
    /// Create with an explicit pole radius `r` in `[0, 1)`. Values are clamped
    /// into a stable range. `0.995` is a good default for audio rates.
    pub fn new(r: f32) -> Self {
        let r = if r.is_finite() {
            r.clamp(0.0, 0.999_99)
        } else {
            0.995
        };
        Self {
            r,
            x1: 0.0,
            y1: 0.0,
        }
    }

    /// Default DC blocker (`R = 0.995`).
    pub fn default_audio() -> Self {
        Self::new(0.995)
    }

    /// Reset history taps.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    /// Process one sample; advances state.
    #[inline]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    /// Process a block into a caller-supplied buffer (`out.len() >= x.len()`).
    pub fn process(&mut self, x: &[f32], out: &mut [f32]) -> Result<(), AudioError> {
        if out.len() < x.len() {
            return Err(AudioError::OutputBufferTooSmall);
        }
        for (i, &xi) in x.iter().enumerate() {
            out[i] = self.process_sample(xi);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_dc_converges_to_zero() {
        let mut b = DcBlocker::new(0.995);
        let x = [0.7f32; 20_000];
        let mut out = [0.0f32; 20_000];
        b.process(&x, &mut out).expect("buffer big enough");
        // After the pole settles, output should be ~0 (DC removed).
        let tail = &out[out.len() - 100..];
        let mean: f32 = tail.iter().sum::<f32>() / tail.len() as f32;
        assert!(mean.abs() < 1e-3, "residual DC {mean}");
    }

    #[test]
    fn passes_alternating_signal() {
        // A rapidly alternating (high-freq) signal should survive largely intact.
        let mut b = DcBlocker::new(0.995);
        let mut x = [0.0f32; 4000];
        for (i, v) in x.iter_mut().enumerate() {
            *v = if i % 2 == 0 { 1.0 } else { -1.0 };
        }
        let mut out = [0.0f32; 4000];
        b.process(&x, &mut out).expect("buffer big enough");
        let tail = &out[out.len() - 200..];
        let rms: f32 = (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt();
        assert!(rms > 0.9, "high-freq attenuated too much: {rms}");
    }

    #[test]
    fn short_buffer_errors() {
        let mut b = DcBlocker::default_audio();
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            b.process(&x, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
