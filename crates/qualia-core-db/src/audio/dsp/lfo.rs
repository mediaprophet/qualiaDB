//! LFO — low-frequency oscillator for modulation.
//!
//! Wraps an oscillator at sub-audio frequencies (typically 0.1–20 Hz) and
//! provides a smoothed modulation output.

use super::oscillator::{Oscillator, Waveform};

/// LFO configuration.
#[derive(Debug, Clone)]
pub struct Lfo {
    osc: Oscillator,
    pub depth: f64, // Modulation depth [0, 1]
}

impl Lfo {
    pub fn new(waveform: Waveform, frequency: f64, sample_rate: f64, depth: f64) -> Self {
        Self {
            osc: Oscillator::new(waveform, frequency, sample_rate),
            depth: depth.clamp(0.0, 1.0),
        }
    }

    /// Tick the LFO and return a modulation value in [-depth, +depth].
    pub fn tick(&mut self) -> f64 {
        self.osc.tick() * self.depth
    }

    /// Render `n` samples.
    pub fn render(&mut self, out: &mut [f64]) {
        for s in out.iter_mut() {
            *s = self.tick();
        }
    }

    pub fn set_frequency(&mut self, freq: f64) {
        self.osc.set_frequency(freq);
    }

    pub fn set_depth(&mut self, depth: f64) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    pub fn reset(&mut self) {
        self.osc.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfo_output_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 1.0, 44100.0, 0.5);
        for _ in 0..1000 {
            let v = lfo.tick();
            assert!(v.abs() <= 0.5 + 1e-10);
        }
    }

    #[test]
    fn lfo_depth_clamped() {
        let lfo = Lfo::new(Waveform::Sine, 1.0, 44100.0, 2.0);
        assert_eq!(lfo.depth, 1.0);
    }

    #[test]
    fn lfo_frequency_change() {
        let mut lfo = Lfo::new(Waveform::Sine, 1.0, 44100.0, 0.5);
        lfo.set_frequency(5.0);
        assert_eq!(lfo.osc.frequency, 5.0);
    }
}
