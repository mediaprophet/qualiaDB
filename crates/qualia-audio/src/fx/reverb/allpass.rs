//! Schroeder allpass filter (Freeverb variant).
//!
//! `y[n] = -x[n] + buf[n]`, `buf'[n] = x[n] + feedback · buf[n]`. Diffuses the
//! signal (flat magnitude, dispersed phase) — used in series after the combs to
//! thicken the reverb tail without adding coloration.
//!
//! Delay buffer allocated **once** in [`Allpass::new`]; `process_sample` /
//! `process` never allocate, lock, or touch the FS.

use crate::types::AudioError;

/// Schroeder allpass diffuser — fixed delay + feedback.
#[derive(Debug, Clone)]
pub struct Allpass {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl Allpass {
    /// Construct with delay `delay_samples` (≥ 1) and `feedback` (typ. 0.5).
    pub fn new(delay_samples: usize, feedback: f32) -> Result<Self, AudioError> {
        if !feedback.is_finite() {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            buffer: vec![0.0; delay_samples.max(1)],
            index: 0,
            feedback: feedback.clamp(-0.999, 0.999),
        })
    }

    /// Delay length in samples.
    #[inline]
    pub fn delay(&self) -> usize {
        self.buffer.len()
    }

    /// Zero internal state. No allocation.
    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
    }

    /// Process one sample. **Zero-alloc, no locks.**
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let bufout = self.buffer[self.index];
        let output = -input + bufout;
        self.buffer[self.index] = input + bufout * self.feedback;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        output
    }

    /// Block process. **Zero-alloc.** Errors if `output` is shorter than `input`.
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
    fn impulse_response_is_finite_and_diffuses() {
        let d = 9usize;
        let mut a = Allpass::new(d, 0.5).unwrap();
        let n = 6 * d;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n];
        a.process(&input, &mut out).unwrap();

        // Immediate inverted feedthrough: y[0] = -x[0] + 0 = -1.
        assert!((out[0] + 1.0).abs() < 1e-6, "y[0] should be -1, got {}", out[0]);
        // First recirculation appears at the delay tap and is non-zero.
        assert!(out[d].abs() > 1e-6, "expected energy at delay tap, got {}", out[d]);
        assert!(out.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn energy_preserving_ish_and_decays() {
        // A stable allpass conserves total energy for a passive impulse but the
        // recirculating taps decay geometrically with feedback.
        let mut a = Allpass::new(13, 0.5).unwrap();
        let n = 2000;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n];
        a.process(&input, &mut out).unwrap();
        let early: f32 = out[..500].iter().map(|x| x * x).sum();
        let late: f32 = out[1500..].iter().map(|x| x * x).sum();
        assert!(late < early, "late energy {} should be < early {}", late, early);
        assert!(out.iter().all(|x| x.abs() < 10.0));
    }

    #[test]
    fn output_too_small_errors() {
        let mut a = Allpass::new(4, 0.5).unwrap();
        let input = [0.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(a.process(&input, &mut out), Err(AudioError::OutputBufferTooSmall));
    }
}
