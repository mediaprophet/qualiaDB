//! Freeverb / Schroeder reverb — parallel combs → series allpasses.
//!
//! Eight feedback [`CombFilter`]s run in parallel (their outputs summed), then
//! the sum passes through four series [`Allpass`] diffusers. `room_size` sets the
//! comb feedback (tail length), `damping` the in-loop lowpass (tail brightness),
//! and `wet`/`dry` the mix.
//!
//! All delay buffers are allocated **once** in [`SchroederReverb::new`]. The
//! `process_sample` / `process` paths never allocate, lock, or touch the FS.

use crate::fx::reverb::allpass::Allpass;
use crate::fx::reverb::comb::CombFilter;
use crate::types::AudioError;

/// Freeverb comb delay tunings (samples @ 44.1 kHz), scaled to the target rate.
const COMB_TUNINGS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
/// Freeverb allpass delay tunings (samples @ 44.1 kHz).
const ALLPASS_TUNINGS: [usize; 4] = [556, 441, 341, 225];

/// Freeverb parameter scaling constants.
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;
const SCALE_DAMP: f32 = 0.4;
const FIXED_GAIN: f32 = 0.015;

/// Schroeder/Freeverb-style reverb (8 parallel combs + 4 series allpasses).
#[derive(Debug, Clone)]
pub struct SchroederReverb {
    combs: [CombFilter; 8],
    allpasses: [Allpass; 4],
    wet: f32,
    dry: f32,
}

impl SchroederReverb {
    /// Construct for `sample_rate`. `room_size` and `damping` in `[0, 1]`;
    /// `wet`/`dry` are the linear mix gains. Allocates all delay lines once.
    pub fn new(
        sample_rate: u32,
        room_size: f32,
        damping: f32,
        wet: f32,
        dry: f32,
    ) -> Result<Self, AudioError> {
        if sample_rate == 0
            || !room_size.is_finite()
            || !damping.is_finite()
            || !wet.is_finite()
            || !dry.is_finite()
        {
            return Err(AudioError::InvalidParameter);
        }
        let feedback = room_size.clamp(0.0, 1.0) * SCALE_ROOM + OFFSET_ROOM;
        let damp = damping.clamp(0.0, 1.0) * SCALE_DAMP;
        // Scale the 44.1k reference tunings to the working sample rate.
        let scale = sample_rate as f32 / 44_100.0;
        let d = |t: usize| ((t as f32 * scale) as usize).max(1);

        // Build each filter once (constructor allocation only).
        let combs = [
            CombFilter::new(d(COMB_TUNINGS[0]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[1]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[2]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[3]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[4]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[5]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[6]), feedback, damp)?,
            CombFilter::new(d(COMB_TUNINGS[7]), feedback, damp)?,
        ];
        let allpasses = [
            Allpass::new(d(ALLPASS_TUNINGS[0]), 0.5)?,
            Allpass::new(d(ALLPASS_TUNINGS[1]), 0.5)?,
            Allpass::new(d(ALLPASS_TUNINGS[2]), 0.5)?,
            Allpass::new(d(ALLPASS_TUNINGS[3]), 0.5)?,
        ];
        Ok(Self {
            combs,
            allpasses,
            wet,
            dry,
        })
    }

    /// Live-update room size (comb feedback) across all combs.
    pub fn set_room_size(&mut self, room_size: f32) {
        if room_size.is_finite() {
            let fb = room_size.clamp(0.0, 1.0) * SCALE_ROOM + OFFSET_ROOM;
            for c in self.combs.iter_mut() {
                c.set_feedback(fb);
            }
        }
    }

    /// Live-update damping across all combs.
    pub fn set_damping(&mut self, damping: f32) {
        if damping.is_finite() {
            let damp = damping.clamp(0.0, 1.0) * SCALE_DAMP;
            for c in self.combs.iter_mut() {
                c.set_damp(damp);
            }
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

    /// Clear all delay/state buffers. No allocation.
    pub fn reset(&mut self) {
        for c in self.combs.iter_mut() {
            c.reset();
        }
        for a in self.allpasses.iter_mut() {
            a.reset();
        }
    }

    /// Process one sample through the full network. **Zero-alloc, no locks.**
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let scaled = input * FIXED_GAIN;
        // Parallel combs, summed.
        let mut acc = 0.0f32;
        for c in self.combs.iter_mut() {
            acc += c.process_sample(scaled);
        }
        // Series allpasses.
        for a in self.allpasses.iter_mut() {
            acc = a.process_sample(acc);
        }
        acc * self.wet + input * self.dry
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
        // Fully wet so we measure only the reverberant tail.
        let mut r = SchroederReverb::new(44_100, 0.8, 0.3, 1.0, 0.0).unwrap();
        let n = 44_100; // 1 second
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n];
        r.process(&input, &mut out).unwrap();

        // Everything finite & bounded (stable network, feedback < 1).
        assert!(out.iter().all(|x| x.is_finite()), "reverb produced NaN/Inf");
        assert!(out.iter().all(|x| x.abs() < 4.0), "reverb tail not bounded");

        // RT-style decay: energy in a late window is well below an early window.
        let win = 4_410; // 100 ms
        let early: f32 = out[win..2 * win].iter().map(|x| x * x).sum();
        let late: f32 = out[8 * win..9 * win].iter().map(|x| x * x).sum();
        assert!(early > 0.0, "expected non-zero early tail energy");
        assert!(late < early, "late energy {} must be < early {}", late, early);
        // Meaningful decay, not a plateau.
        assert!(late < early * 0.5, "tail did not decay enough: late {} early {}", late, early);
    }

    #[test]
    fn dry_only_passes_input_through() {
        let mut r = SchroederReverb::new(48_000, 0.5, 0.5, 0.0, 1.0).unwrap();
        let input: Vec<f32> = (0..256)
            .map(|i| (2.0 * core::f32::consts::PI * 440.0 * i as f32 / 48_000.0).sin())
            .collect();
        let mut out = vec![0.0f32; 256];
        r.process(&input, &mut out).unwrap();
        for (o, x) in out.iter().zip(input.iter()) {
            assert!((o - x).abs() < 1e-6, "dry path should be identity");
        }
    }

    #[test]
    fn sample_rate_scales_delays() {
        let r44 = SchroederReverb::new(44_100, 0.5, 0.5, 1.0, 0.0).unwrap();
        let r88 = SchroederReverb::new(88_200, 0.5, 0.5, 1.0, 0.0).unwrap();
        // Doubling the rate roughly doubles each comb delay length.
        assert!(r88.combs[0].delay() > r44.combs[0].delay());
    }

    #[test]
    fn invalid_params_rejected() {
        assert_eq!(
            SchroederReverb::new(0, 0.5, 0.5, 1.0, 0.0).err(),
            Some(AudioError::InvalidParameter)
        );
        assert_eq!(
            SchroederReverb::new(44_100, f32::NAN, 0.5, 1.0, 0.0).err(),
            Some(AudioError::InvalidParameter)
        );
    }
}
