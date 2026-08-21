//! Oscillator — basic waveform synthesis.
//!
//! Generates sine, square, sawtooth, and triangle waveforms at a given
//! frequency and sample rate. Phase-continuous for real-time modulation.

/// Oscillator waveform type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

/// An oscillator with phase accumulation.
#[derive(Debug, Clone)]
pub struct Oscillator {
    pub waveform: Waveform,
    pub frequency: f64,
    pub sample_rate: f64,
    pub gain: f64,
    phase: f64,
}

impl Oscillator {
    pub fn new(waveform: Waveform, frequency: f64, sample_rate: f64) -> Self {
        Self {
            waveform,
            frequency,
            sample_rate,
            gain: 1.0,
            phase: 0.0,
        }
    }

    /// Advance one sample and return the output value in [-1.0, 1.0] * gain.
    pub fn tick(&mut self) -> f64 {
        let phase_increment = self.frequency / self.sample_rate;
        let value = match self.waveform {
            Waveform::Sine => (self.phase * 2.0 * std::f64::consts::PI).sin(),
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Sawtooth => 2.0 * self.phase - 1.0,
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
        };
        self.phase += phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        value * self.gain
    }

    /// Generate `n` samples into `out`.
    pub fn render(&mut self, out: &mut [f64]) {
        for s in out.iter_mut() {
            *s = self.tick();
        }
    }

    /// Reset phase to zero.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Set frequency (Hz).
    pub fn set_frequency(&mut self, freq: f64) {
        self.frequency = freq;
    }

    /// Set gain (0.0 to 1.0 typically).
    pub fn set_gain(&mut self, gain: f64) {
        self.gain = gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_oscillator_range() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0);
        for _ in 0..1000 {
            let v = osc.tick();
            assert!(v.abs() <= 1.0 + 1e-10);
        }
    }

    #[test]
    fn square_oscillator_values() {
        let mut osc = Oscillator::new(Waveform::Square, 1.0, 8.0);
        let mut samples = vec![0.0; 8];
        osc.render(&mut samples);
        // First half should be +1, second half -1.
        assert!(samples[0] > 0.0);
        assert!(samples[4] < 0.0);
    }

    #[test]
    fn sawtooth_oscillator_range() {
        let mut osc = Oscillator::new(Waveform::Sawtooth, 100.0, 44100.0);
        for _ in 0..1000 {
            let v = osc.tick();
            assert!(v.abs() <= 1.0 + 1e-10);
        }
    }

    #[test]
    fn triangle_oscillator_range() {
        let mut osc = Oscillator::new(Waveform::Triangle, 100.0, 44100.0);
        for _ in 0..1000 {
            let v = osc.tick();
            assert!(v.abs() <= 1.0 + 1e-10);
        }
    }

    #[test]
    fn oscillator_reset() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0);
        for _ in 0..100 {
            osc.tick();
        }
        osc.reset();
        // After reset, first sample of sine should be ~0.
        let v = osc.tick();
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn oscillator_gain() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0);
        osc.set_gain(0.5);
        let v = osc.tick();
        // At phase 0, sine is 0, so check a later sample.
        let v2 = osc.tick();
        assert!(v2.abs() <= 0.5 + 1e-10);
    }

    #[test]
    fn oscillator_frequency_change() {
        let mut osc = Oscillator::new(Waveform::Sine, 100.0, 44100.0);
        osc.set_frequency(200.0);
        assert_eq!(osc.frequency, 200.0);
    }
}
