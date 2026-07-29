//! Feedback Delay Network reverb — 4 lines, 4×4 Hadamard feedback matrix.
//!
//! Four delay lines of mutually-detuned lengths are cross-coupled by a normalized
//! Hadamard matrix `0.5·H₄` (orthogonal → lossless mixing). A global `decay`
//! (< 1) sets the tail length; a per-line one-pole `damping` darkens it.
//!
//! Because `0.5·H₄` is orthonormal, the loop gain is exactly `decay`, so the
//! network is unconditionally stable for `decay < 1`.
//!
//! All four delay buffers are allocated **once** in [`FdnReverb::new`]. The
//! `process_sample` / `process` paths never allocate, lock, or touch the FS.

use crate::types::AudioError;

/// Base delay-line lengths (samples @ 44.1 kHz), mutually prime to avoid
/// coincident echoes. Scaled to the working sample rate at construction.
const BASE_DELAYS: [usize; 4] = [1687, 1801, 1949, 2053];

/// 4×4 feedback delay network reverb with Hadamard mixing.
#[derive(Debug, Clone)]
pub struct FdnReverb {
    lines: [Vec<f32>; 4],
    idx: [usize; 4],
    lp: [f32; 4],
    decay: f32,
    damping: f32,
    wet: f32,
    dry: f32,
}

impl FdnReverb {
    /// Construct for `sample_rate`. `decay` in `[0, 1)` (tail length), `damping`
    /// in `[0, 1)`, plus wet/dry linear mix. Allocates all four delay lines once.
    pub fn new(
        sample_rate: u32,
        decay: f32,
        damping: f32,
        wet: f32,
        dry: f32,
    ) -> Result<Self, AudioError> {
        if sample_rate == 0
            || !decay.is_finite()
            || !damping.is_finite()
            || !wet.is_finite()
            || !dry.is_finite()
        {
            return Err(AudioError::InvalidParameter);
        }
        let scale = sample_rate as f32 / 44_100.0;
        let d = |t: usize| ((t as f32 * scale) as usize).max(1);
        let lines = [
            vec![0.0; d(BASE_DELAYS[0])],
            vec![0.0; d(BASE_DELAYS[1])],
            vec![0.0; d(BASE_DELAYS[2])],
            vec![0.0; d(BASE_DELAYS[3])],
        ];
        Ok(Self {
            lines,
            idx: [0; 4],
            lp: [0.0; 4],
            decay: decay.clamp(0.0, 0.999),
            damping: damping.clamp(0.0, 0.999),
            wet,
            dry,
        })
    }

    /// Live-update decay (tail length). Clamped to `[0, 1)`.
    #[inline]
    pub fn set_decay(&mut self, decay: f32) {
        if decay.is_finite() {
            self.decay = decay.clamp(0.0, 0.999);
        }
    }

    /// Live-update damping. Clamped to `[0, 1)`.
    #[inline]
    pub fn set_damping(&mut self, damping: f32) {
        if damping.is_finite() {
            self.damping = damping.clamp(0.0, 0.999);
        }
    }

    /// Set wet/dry linear mix gains.
    pub fn set_mix(&mut self, wet: f32, dry: f32) {
        if wet.is_finite() {
            self.wet = wet;
        }
        if dry.is_finite() {
            self.dry = dry;
        }
    }

    /// Clear all delay lines and lowpass state. No allocation.
    pub fn reset(&mut self) {
        for line in self.lines.iter_mut() {
            for s in line.iter_mut() {
                *s = 0.0;
            }
        }
        self.idx = [0; 4];
        self.lp = [0.0; 4];
    }

    /// Process one sample. **Zero-alloc, no locks.**
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Read the delayed output of each line (oldest sample at idx).
        let d0 = self.lines[0][self.idx[0]];
        let d1 = self.lines[1][self.idx[1]];
        let d2 = self.lines[2][self.idx[2]];
        let d3 = self.lines[3][self.idx[3]];

        // Per-line damping (one-pole lowpass in the feedback path).
        let damp = self.damping;
        let one = 1.0 - damp;
        self.lp[0] = d0 * one + self.lp[0] * damp;
        self.lp[1] = d1 * one + self.lp[1] * damp;
        self.lp[2] = d2 * one + self.lp[2] * damp;
        self.lp[3] = d3 * one + self.lp[3] * damp;
        let (s0, s1, s2, s3) = (self.lp[0], self.lp[1], self.lp[2], self.lp[3]);

        // Normalized Hadamard mix (0.5·H₄, orthonormal → lossless).
        let m0 = 0.5 * (s0 + s1 + s2 + s3);
        let m1 = 0.5 * (s0 - s1 + s2 - s3);
        let m2 = 0.5 * (s0 + s1 - s2 - s3);
        let m3 = 0.5 * (s0 - s1 - s2 + s3);

        // Inject input into every line, add scaled feedback, and write back.
        let g = self.decay;
        self.lines[0][self.idx[0]] = input + g * m0;
        self.lines[1][self.idx[1]] = input + g * m1;
        self.lines[2][self.idx[2]] = input + g * m2;
        self.lines[3][self.idx[3]] = input + g * m3;

        // Advance each ring index.
        for i in 0..4 {
            self.idx[i] += 1;
            if self.idx[i] >= self.lines[i].len() {
                self.idx[i] = 0;
            }
        }

        // Wet output = mean of the raw delayed taps.
        let wet_sig = 0.25 * (d0 + d1 + d2 + d3);
        wet_sig * self.wet + input * self.dry
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
    fn impulse_produces_bounded_decaying_tail() {
        let mut r = FdnReverb::new(44_100, 0.85, 0.2, 1.0, 0.0).unwrap();
        let n = 44_100;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n];
        r.process(&input, &mut out).unwrap();

        assert!(out.iter().all(|x| x.is_finite()), "FDN produced NaN/Inf");
        assert!(out.iter().all(|x| x.abs() < 4.0), "FDN tail not bounded");

        // First echoes only appear after the shortest delay — early window is
        // measured after that so it contains real tail energy.
        let win = 4_410;
        let early: f32 = out[win..2 * win].iter().map(|x| x * x).sum();
        let late: f32 = out[9 * win..10 * win].iter().map(|x| x * x).sum();
        assert!(early > 0.0, "expected non-zero tail energy");
        assert!(
            late < early,
            "late energy {} must be < early {}",
            late,
            early
        );
    }

    #[test]
    fn higher_decay_gives_longer_tail() {
        let n = 44_100;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut short = FdnReverb::new(44_100, 0.5, 0.2, 1.0, 0.0).unwrap();
        let mut long = FdnReverb::new(44_100, 0.9, 0.2, 1.0, 0.0).unwrap();
        let mut os = vec![0.0f32; n];
        let mut ol = vec![0.0f32; n];
        short.process(&input, &mut os).unwrap();
        long.process(&input, &mut ol).unwrap();
        // Late-window energy is greater for the longer decay.
        let es: f32 = os[30_000..].iter().map(|x| x * x).sum();
        let el: f32 = ol[30_000..].iter().map(|x| x * x).sum();
        assert!(
            el > es,
            "long-decay late energy {} should exceed short {}",
            el,
            es
        );
    }

    #[test]
    fn dry_only_passes_input_through() {
        let mut r = FdnReverb::new(48_000, 0.7, 0.3, 0.0, 1.0).unwrap();
        let input = [0.3f32, -0.2, 0.5, 0.1, -0.4];
        let mut out = [0.0f32; 5];
        r.process(&input, &mut out).unwrap();
        for (o, x) in out.iter().zip(input.iter()) {
            assert!((o - x).abs() < 1e-6);
        }
    }

    #[test]
    fn invalid_params_rejected() {
        assert_eq!(
            FdnReverb::new(0, 0.5, 0.5, 1.0, 0.0).err(),
            Some(AudioError::InvalidParameter)
        );
    }
}
