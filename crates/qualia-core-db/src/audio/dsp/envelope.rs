//! ADSR envelope — attack, decay, sustain, release.
//!
//! Generates amplitude envelopes for note shaping. The envelope progresses
//! through four stages:
//! - Attack: 0 → 1 over `attack` seconds
//! - Decay: 1 → `sustain` over `decay` seconds
//! - Sustain: holds at `sustain` level until release
//! - Release: current → 0 over `release` seconds

/// Envelope stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// ADSR envelope generator.
#[derive(Debug, Clone)]
pub struct AdsrEnvelope {
    pub attack: f64,  // seconds
    pub decay: f64,   // seconds
    pub sustain: f64, // level [0, 1]
    pub release: f64, // seconds
    pub sample_rate: f64,
    stage: EnvStage,
    value: f64,
    samples_in_stage: u64,
}

impl AdsrEnvelope {
    pub fn new(attack: f64, decay: f64, sustain: f64, release: f64, sample_rate: f64) -> Self {
        Self {
            attack,
            decay,
            sustain: sustain.clamp(0.0, 1.0),
            release,
            sample_rate,
            stage: EnvStage::Idle,
            value: 0.0,
            samples_in_stage: 0,
        }
    }

    /// Start note-on (trigger attack).
    pub fn note_on(&mut self) {
        self.stage = EnvStage::Attack;
        self.samples_in_stage = 0;
    }

    /// Start note-off (trigger release).
    pub fn note_off(&mut self) {
        self.stage = EnvStage::Release;
        self.samples_in_stage = 0;
    }

    /// Current stage.
    pub fn stage(&self) -> EnvStage {
        self.stage
    }

    /// Current envelope value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Advance one sample and return the envelope value.
    pub fn tick(&mut self) -> f64 {
        let sr = self.sample_rate;
        match self.stage {
            EnvStage::Idle => {
                self.value = 0.0;
            }
            EnvStage::Attack => {
                let attack_samples = (self.attack * sr).max(1.0);
                self.value = self.samples_in_stage as f64 / attack_samples;
                self.samples_in_stage += 1;
                if self.samples_in_stage as f64 >= attack_samples {
                    self.stage = EnvStage::Decay;
                    self.samples_in_stage = 0;
                    self.value = 1.0;
                }
            }
            EnvStage::Decay => {
                let decay_samples = (self.decay * sr).max(1.0);
                let t = self.samples_in_stage as f64 / decay_samples;
                self.value = 1.0 - t * (1.0 - self.sustain);
                self.samples_in_stage += 1;
                if self.samples_in_stage as f64 >= decay_samples {
                    self.stage = EnvStage::Sustain;
                    self.samples_in_stage = 0;
                    self.value = self.sustain;
                }
            }
            EnvStage::Sustain => {
                self.value = self.sustain;
            }
            EnvStage::Release => {
                let release_samples = (self.release * sr).max(1.0);
                let start_value = if self.samples_in_stage == 0 {
                    self.value
                } else {
                    self.value
                };
                let t = self.samples_in_stage as f64 / release_samples;
                self.value = start_value * (1.0 - t);
                self.samples_in_stage += 1;
                if self.samples_in_stage as f64 >= release_samples {
                    self.stage = EnvStage::Idle;
                    self.value = 0.0;
                }
            }
        }
        self.value
    }

    /// Render `n` samples into `out`.
    pub fn render(&mut self, out: &mut [f64]) {
        for s in out.iter_mut() {
            *s = self.tick();
        }
    }

    /// Reset to idle.
    pub fn reset(&mut self) {
        self.stage = EnvStage::Idle;
        self.value = 0.0;
        self.samples_in_stage = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adsr_attack_stage() {
        let mut env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01, 44100.0);
        env.note_on();
        assert_eq!(env.stage(), EnvStage::Attack);
        // After enough samples, should move to decay.
        for _ in 0..500 {
            env.tick();
        }
        assert_eq!(env.stage(), EnvStage::Decay);
    }

    #[test]
    fn adsr_sustain_level() {
        let mut env = AdsrEnvelope::new(0.001, 0.001, 0.7, 0.01, 44100.0);
        env.note_on();
        // Skip attack + decay.
        for _ in 0..200 {
            env.tick();
        }
        assert_eq!(env.stage(), EnvStage::Sustain);
        let v = env.tick();
        assert!((v - 0.7).abs() < 0.01);
    }

    #[test]
    fn adsr_release_stage() {
        let mut env = AdsrEnvelope::new(0.001, 0.001, 0.8, 0.01, 44100.0);
        env.note_on();
        for _ in 0..200 {
            env.tick();
        }
        env.note_off();
        assert_eq!(env.stage(), EnvStage::Release);
        // After release time, should be idle.
        for _ in 0..500 {
            env.tick();
        }
        assert_eq!(env.stage(), EnvStage::Idle);
    }

    #[test]
    fn adsr_idle_is_zero() {
        let mut env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01, 44100.0);
        let v = env.tick();
        assert_eq!(v, 0.0);
    }

    #[test]
    fn adsr_attack_ramps_up() {
        let mut env = AdsrEnvelope::new(0.1, 0.1, 0.5, 0.1, 44100.0);
        env.note_on();
        let v1 = env.tick();
        let v2 = env.tick();
        assert!(v2 > v1);
    }

    #[test]
    fn adsr_reset() {
        let mut env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01, 44100.0);
        env.note_on();
        for _ in 0..100 {
            env.tick();
        }
        env.reset();
        assert_eq!(env.stage(), EnvStage::Idle);
        assert_eq!(env.value(), 0.0);
    }

    #[test]
    fn adsr_sustain_clamped() {
        let env = AdsrEnvelope::new(0.01, 0.01, 1.5, 0.01, 44100.0);
        assert_eq!(env.sustain, 1.0);
    }
}
