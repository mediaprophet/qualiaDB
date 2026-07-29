//! ADSR amplitude envelope — sample-rate based, linear segments, zero-alloc.
//!
//! `process_sample()` advances the envelope by one sample and returns the current
//! gain in `0.0..=1.0`. All state is inline (`Copy`); no heap, no locks, no FS.

/// Envelope segment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A four-stage attack/decay/sustain/release amplitude envelope.
///
/// Segment lengths are stored in samples (derived from seconds × sample-rate at
/// construction). Attack ramps `0 → 1`, decay ramps `1 → sustain`, sustain holds,
/// and release ramps the level captured at `note_off()` down to `0`.
#[derive(Clone, Copy, Debug)]
pub struct AdsrEnvelope {
    attack_samples: f32,
    decay_samples: f32,
    sustain_level: f32,
    release_samples: f32,
    stage: Stage,
    level: f32,
    release_start_level: f32,
}

impl AdsrEnvelope {
    /// Build an envelope. Times are in seconds; `sustain` is a `0..=1` level.
    /// Non-finite / negative inputs are clamped to safe values.
    pub fn new(
        sample_rate: f32,
        attack_s: f32,
        decay_s: f32,
        sustain: f32,
        release_s: f32,
    ) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            1.0
        };
        let to_samples = |secs: f32| -> f32 {
            if secs.is_finite() && secs > 0.0 {
                secs * sr
            } else {
                0.0
            }
        };
        let sustain_level = if sustain.is_finite() {
            sustain.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            attack_samples: to_samples(attack_s),
            decay_samples: to_samples(decay_s),
            sustain_level,
            release_samples: to_samples(release_s),
            stage: Stage::Idle,
            level: 0.0,
            release_start_level: 0.0,
        }
    }

    /// Trigger the envelope: enter the attack stage (rising from the current level).
    pub fn note_on(&mut self) {
        self.stage = Stage::Attack;
    }

    /// Release the envelope: ramp the current level down to zero over the release time.
    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle {
            self.release_start_level = self.level;
            self.stage = Stage::Release;
        }
    }

    /// `true` while the envelope is producing (or will produce) a non-zero contribution.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    /// Current sustain level (`0..=1`).
    #[inline]
    pub fn sustain_level(&self) -> f32 {
        self.sustain_level
    }

    /// Advance one sample and return the current gain in `0.0..=1.0`.
    pub fn process_sample(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {
                self.level = 0.0;
            }
            Stage::Attack => {
                if self.attack_samples <= 0.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                } else {
                    self.level += 1.0 / self.attack_samples;
                    if self.level >= 1.0 {
                        self.level = 1.0;
                        self.stage = Stage::Decay;
                    }
                }
            }
            Stage::Decay => {
                if self.decay_samples <= 0.0 {
                    self.level = self.sustain_level;
                    self.stage = Stage::Sustain;
                } else {
                    self.level -= (1.0 - self.sustain_level) / self.decay_samples;
                    if self.level <= self.sustain_level {
                        self.level = self.sustain_level;
                        self.stage = Stage::Sustain;
                    }
                }
            }
            Stage::Sustain => {
                self.level = self.sustain_level;
            }
            Stage::Release => {
                if self.release_samples <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                } else {
                    self.level -= self.release_start_level / self.release_samples;
                    if self.level <= 0.0 {
                        self.level = 0.0;
                        self.stage = Stage::Idle;
                    }
                }
            }
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    #[test]
    fn attack_rises_monotonically_from_near_zero() {
        // 10 ms attack = 480 samples at 48 kHz.
        let mut env = AdsrEnvelope::new(SR, 0.010, 0.010, 0.5, 0.010);
        env.note_on();
        let mut prev = env.process_sample();
        assert!(
            prev > 0.0 && prev < 0.01,
            "first attack gain ~0, got {prev}"
        );
        // Stay within the attack region (< 480 samples) and assert strict rise.
        for _ in 0..460 {
            let g = env.process_sample();
            assert!(g > prev, "attack must rise monotonically: {g} !> {prev}");
            assert!(g <= 1.0 + 1e-6, "gain bounded by 1.0, got {g}");
            prev = g;
        }
    }

    #[test]
    fn sustain_holds_at_sustain_level() {
        let mut env = AdsrEnvelope::new(SR, 0.010, 0.010, 0.5, 0.010);
        env.note_on();
        // Run well past attack+decay (480+480) into sustain.
        let mut g = 0.0;
        for _ in 0..2_000 {
            g = env.process_sample();
        }
        assert!(
            (g - 0.5).abs() < 1e-3,
            "sustain should hold at 0.5, got {g}"
        );
        // Continues to hold.
        for _ in 0..1_000 {
            g = env.process_sample();
        }
        assert!((g - 0.5).abs() < 1e-3, "sustain still 0.5, got {g}");
    }

    #[test]
    fn release_falls_to_zero_within_release_time() {
        let mut env = AdsrEnvelope::new(SR, 0.010, 0.010, 0.5, 0.010);
        env.note_on();
        for _ in 0..2_000 {
            env.process_sample();
        }
        // At sustain (~0.5) now release.
        env.note_off();
        // 10 ms release = 480 samples; give a small margin.
        let mut g = 1.0;
        for _ in 0..500 {
            g = env.process_sample();
        }
        assert!(
            g <= 1e-3,
            "release should reach ~0 within release time, got {g}"
        );
        assert!(!env.is_active(), "envelope idle after release completes");
    }

    #[test]
    fn instant_attack_and_release_are_safe() {
        let mut env = AdsrEnvelope::new(SR, 0.0, 0.0, 0.7, 0.0);
        env.note_on();
        let g = env.process_sample();
        assert!((g - 1.0).abs() < 1e-6 || (g - 0.7).abs() < 1e-6);
        env.note_off();
        let g2 = env.process_sample();
        assert_eq!(g2, 0.0);
        assert!(!env.is_active());
    }
}
