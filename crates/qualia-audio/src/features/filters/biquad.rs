//! Biquad (second-order IIR) engine — the shared core all EQ designs feed.
//!
//! `BiquadCoeffs` are normalized (a0 folded to 1). `BiquadState` runs Direct
//! Form I: it keeps two input and two output history taps on the stack, so the
//! hot path allocates nothing. Design helpers in sibling files produce coeffs
//! via the RBJ Audio-EQ-Cookbook; this file only *runs* them.

use crate::types::AudioError;

/// Normalized biquad coefficients (transfer function with `a0 == 1`).
///
/// `y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    /// Identity (pass-through) filter: `y[n] = x[n]`.
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Build from raw (un-normalized) cookbook coefficients by dividing through
    /// by `a0`. Falls back to identity if `a0` is not finite/non-zero.
    pub(crate) fn from_raw(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        if a0 == 0.0 || !a0.is_finite() {
            return Self::identity();
        }
        let inv = 1.0 / a0;
        Self {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
        }
    }
}

/// Running filter state (two-tap history). Direct Form I — one instance per
/// mono channel. Zero-heap: all state lives inline on the stack.
#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    /// Fresh state with all history taps at zero.
    pub const fn new() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Clear history taps (e.g. on discontinuity / seek).
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Process one sample; advances history. No allocation.
    #[inline]
    pub fn process_sample(&mut self, c: &BiquadCoeffs, x: f32) -> f32 {
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    /// Process a whole block into a caller-supplied output buffer.
    ///
    /// `out` must be at least as long as `x`; extra `out` slots are untouched.
    pub fn process(
        &mut self,
        c: &BiquadCoeffs,
        x: &[f32],
        out: &mut [f32],
    ) -> Result<(), AudioError> {
        if out.len() < x.len() {
            return Err(AudioError::OutputBufferTooSmall);
        }
        for (i, &xi) in x.iter().enumerate() {
            out[i] = self.process_sample(c, xi);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_input_stays_zero() {
        let c = BiquadCoeffs {
            b0: 0.5,
            b1: -0.3,
            b2: 0.2,
            a1: -1.1,
            a2: 0.4,
        };
        let mut st = BiquadState::new();
        let x = [0.0f32; 16];
        let mut out = [9.0f32; 16];
        st.process(&c, &x, &mut out).expect("buffer big enough");
        for &y in &out {
            assert_eq!(y, 0.0);
        }
    }

    #[test]
    fn impulse_response_first_taps_fir() {
        // Pure FIR (a1=a2=0): impulse response is exactly [b0, b1, b2, 0, ...].
        let c = BiquadCoeffs {
            b0: 0.25,
            b1: 0.5,
            b2: -0.125,
            a1: 0.0,
            a2: 0.0,
        };
        let mut st = BiquadState::new();
        let mut x = [0.0f32; 6];
        x[0] = 1.0;
        let mut out = [0.0f32; 6];
        st.process(&c, &x, &mut out).expect("buffer big enough");
        assert!((out[0] - 0.25).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-6);
        assert!((out[2] - (-0.125)).abs() < 1e-6);
        assert!((out[3]).abs() < 1e-6);
        assert!((out[4]).abs() < 1e-6);
    }

    #[test]
    fn impulse_response_with_feedback() {
        // y[n] = b0*x + b1*x1 - a1*y1.  Impulse: y0=b0, y1=b1-a1*b0,
        // y2 = -a1*y1, y3 = -a1*y2 ...
        let c = BiquadCoeffs {
            b0: 1.0,
            b1: 0.5,
            b2: 0.0,
            a1: -0.5,
            a2: 0.0,
        };
        let mut st = BiquadState::new();
        let mut x = [0.0f32; 5];
        x[0] = 1.0;
        let mut out = [0.0f32; 5];
        st.process(&c, &x, &mut out).expect("buffer big enough");
        let y0 = 1.0f32;
        let y1 = 0.5 - (-0.5) * y0; // = 1.0
        let y2 = -(-0.5) * y1; // = 0.5
        let y3 = -(-0.5) * y2; // = 0.25
        assert!((out[0] - y0).abs() < 1e-6);
        assert!((out[1] - y1).abs() < 1e-6);
        assert!((out[2] - y2).abs() < 1e-6);
        assert!((out[3] - y3).abs() < 1e-6);
    }

    #[test]
    fn short_output_buffer_errors() {
        let c = BiquadCoeffs::identity();
        let mut st = BiquadState::new();
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            st.process(&c, &x, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn identity_passes_through() {
        let c = BiquadCoeffs::identity();
        let mut st = BiquadState::new();
        let x = [0.1, -0.4, 0.7, -0.9, 0.2];
        let mut out = [0.0f32; 5];
        st.process(&c, &x, &mut out).expect("buffer big enough");
        for i in 0..x.len() {
            assert!((out[i] - x[i]).abs() < 1e-6);
        }
    }
}
