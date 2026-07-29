//! Feedback comb filter with in-loop damping lowpass (Freeverb-style).
//!
//! `y[n] = x[n] + feedback · LP(y[n-D])` where `LP` is a one-pole lowpass whose
//! cutoff is set by `damp`. With `damp == 0` the impulse response is a clean train
//! of echoes at multiples of the delay `D` with amplitudes `feedback^k`.
//!
//! The internal delay buffer is allocated **once** in [`CombFilter::new`]; the
//! `process_sample` / `process` paths never allocate, lock, or touch the FS.

use crate::types::AudioError;

/// Feedback comb filter — fixed delay, feedback gain, damping lowpass.
#[derive(Debug, Clone)]
pub struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    /// One-pole damping coefficient in `[0, 1)`. Higher = darker tail.
    damp: f32,
    /// Lowpass state held across the feedback loop.
    filterstore: f32,
}

impl CombFilter {
    /// Construct with a delay of `delay_samples` (clamped to ≥ 1).
    ///
    /// Allocates the delay buffer exactly once. `feedback` is clamped to
    /// `(-1, 1)` for stability; `damp` to `[0, 1)`.
    pub fn new(delay_samples: usize, feedback: f32, damp: f32) -> Result<Self, AudioError> {
        let n = delay_samples.max(1);
        if !feedback.is_finite() || !damp.is_finite() {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            buffer: vec![0.0; n],
            index: 0,
            feedback: feedback.clamp(-0.999, 0.999),
            damp: damp.clamp(0.0, 0.999),
            filterstore: 0.0,
        })
    }

    /// Delay length in samples.
    #[inline]
    pub fn delay(&self) -> usize {
        self.buffer.len()
    }

    /// Update feedback (room size) live. Clamped for stability.
    #[inline]
    pub fn set_feedback(&mut self, feedback: f32) {
        if feedback.is_finite() {
            self.feedback = feedback.clamp(-0.999, 0.999);
        }
    }

    /// Update damping live. Clamped to `[0, 1)`.
    #[inline]
    pub fn set_damp(&mut self, damp: f32) {
        if damp.is_finite() {
            self.damp = damp.clamp(0.0, 0.999);
        }
    }

    /// Zero the delay line and lowpass state. No allocation.
    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
        self.filterstore = 0.0;
    }

    /// Process one sample. **Zero-alloc, no locks.**
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // SAFETY of indexing: `index` is always kept < len below.
        let output = self.buffer[self.index];
        // One-pole lowpass inside the feedback path (damping).
        self.filterstore = output * (1.0 - self.damp) + self.filterstore * self.damp;
        self.buffer[self.index] = input + self.filterstore * self.feedback;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        output
    }

    /// Block process: `output[i] = process_sample(input[i])`. **Zero-alloc.**
    ///
    /// Processes `min(input.len(), output.len())` samples; errors only if
    /// `output` is shorter than `input`.
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), AudioError> {
        if output.len() < input.len() {
            return Err(AudioError::OutputBufferTooSmall);
        }
        for (o, &x) in output.iter_mut().zip(input.iter()) {
            *o = self.process_sample(x);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_echoes_at_delay_period_with_decay() {
        // damp = 0 → clean geometric echo train.
        let d = 7usize;
        let g = 0.6f32;
        let mut c = CombFilter::new(d, g, 0.0).unwrap();
        let n = 4 * d + 1;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n];
        c.process(&input, &mut out).unwrap();

        // Echoes land at D, 2D, 3D with amplitude 1, g, g^2 (Freeverb comb delays
        // the impulse by D, then recirculates with gain g each pass).
        assert!(
            (out[d] - 1.0).abs() < 1e-6,
            "first echo at D should be ~1.0, got {}",
            out[d]
        );
        assert!(
            (out[2 * d] - g).abs() < 1e-6,
            "second echo ~g, got {}",
            out[2 * d]
        );
        assert!(
            (out[3 * d] - g * g).abs() < 1e-6,
            "third echo ~g^2, got {}",
            out[3 * d]
        );
        assert!(
            (out[4 * d] - g * g * g).abs() < 1e-6,
            "fourth echo ~g^3, got {}",
            out[4 * d]
        );

        // Between taps the output is silent.
        assert!(out[1].abs() < 1e-9);
        assert!(out[d - 1].abs() < 1e-9);
        assert!(out[d + 1].abs() < 1e-9);

        // Strictly decaying tap magnitudes.
        assert!(out[2 * d].abs() < out[d].abs());
        assert!(out[3 * d].abs() < out[2 * d].abs());
    }

    #[test]
    fn damping_reduces_tail_energy() {
        let d = 11usize;
        let g = 0.7f32;
        let n = 20 * d;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;

        let mut bright = CombFilter::new(d, g, 0.0).unwrap();
        let mut dark = CombFilter::new(d, g, 0.5).unwrap();
        let mut ob = vec![0.0f32; n];
        let mut od = vec![0.0f32; n];
        bright.process(&input, &mut ob).unwrap();
        dark.process(&input, &mut od).unwrap();

        let eb: f32 = ob.iter().map(|x| x * x).sum();
        let ed: f32 = od.iter().map(|x| x * x).sum();
        assert!(
            ed < eb,
            "damped tail energy {} should be < undamped {}",
            ed,
            eb
        );
        assert!(eb.is_finite() && ed.is_finite());
    }

    #[test]
    fn bounded_and_finite_under_sustained_input() {
        let mut c = CombFilter::new(31, 0.85, 0.2).unwrap();
        let input = vec![0.5f32; 4096];
        let mut out = vec![0.0f32; 4096];
        c.process(&input, &mut out).unwrap();
        assert!(out.iter().all(|x| x.is_finite()));
        // Stable feedback < 1 → bounded steady state.
        assert!(out.iter().all(|x| x.abs() < 100.0));
    }

    #[test]
    fn output_too_small_errors() {
        let mut c = CombFilter::new(4, 0.5, 0.0).unwrap();
        let input = [0.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            c.process(&input, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
