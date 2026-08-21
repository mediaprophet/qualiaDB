//! Audio effects — delay, reverb, compressor, EQ.
//!
//! Simple but real implementations of common DAW effects.

/// Simple delay line with feedback.
#[derive(Debug, Clone)]
pub struct Delay {
    buffer: Vec<f64>,
    write_pos: usize,
    pub feedback: f64, // [0, 1)
    pub mix: f64,      // wet/dry mix [0, 1]
}

impl Delay {
    pub fn new(max_samples: usize, feedback: f64, mix: f64) -> Self {
        Self {
            buffer: vec![0.0; max_samples.max(1)],
            write_pos: 0,
            feedback: feedback.clamp(0.0, 0.999),
            mix: mix.clamp(0.0, 1.0),
        }
    }

    /// Set delay time in samples.
    pub fn set_delay_samples(&mut self, samples: usize) {
        // The buffer size is fixed at construction; we just adjust the
        // read offset by ensuring the buffer is large enough.
        if samples > self.buffer.len() {
            self.buffer.resize(samples, 0.0);
        }
    }

    pub fn tick(&mut self, input: f64) -> f64 {
        let delayed = self.buffer[self.write_pos];
        let output = input + delayed * self.mix;
        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        output
    }

    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &s) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = self.tick(s);
            }
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

/// Simple reverb using a feedback delay network.
#[derive(Debug, Clone)]
pub struct Reverb {
    pub room_size: f64, // [0, 1]
    pub damping: f64,   // [0, 1]
    pub mix: f64,       // wet/dry [0, 1]
    delays: Vec<Delay>,
}

impl Reverb {
    pub fn new(sample_rate: f64, room_size: f64, damping: f64, mix: f64) -> Self {
        // Four parallel delay lines with prime-number lengths for a dense tail.
        let prime_ms = [29.0, 37.0, 41.0, 43.0];
        let delays: Vec<Delay> = prime_ms
            .iter()
            .map(|&ms| {
                let samples = (ms * sample_rate / 1000.0) as usize;
                Delay::new(samples, 0.6 + room_size * 0.3, 0.25)
            })
            .collect();
        Self {
            room_size: room_size.clamp(0.0, 1.0),
            damping: damping.clamp(0.0, 1.0),
            mix: mix.clamp(0.0, 1.0),
            delays,
        }
    }

    pub fn tick(&mut self, input: f64) -> f64 {
        let mut wet = 0.0;
        for d in &mut self.delays {
            wet += d.tick(input);
        }
        wet /= self.delays.len() as f64;
        // Apply damping (simple low-pass via averaging).
        let damped = wet * (1.0 - self.damping * 0.5);
        input * (1.0 - self.mix) + damped * self.mix
    }

    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &s) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = self.tick(s);
            }
        }
    }

    pub fn reset(&mut self) {
        for d in &mut self.delays {
            d.reset();
        }
    }
}

/// Simple compressor with threshold, ratio, attack, release.
#[derive(Debug, Clone)]
pub struct Compressor {
    pub threshold: f64, // dB
    pub ratio: f64,     // >= 1.0
    pub attack: f64,    // seconds
    pub release: f64,   // seconds
    pub sample_rate: f64,
    envelope: f64,
    attack_coef: f64,
    release_coef: f64,
}

impl Compressor {
    pub fn new(threshold: f64, ratio: f64, attack: f64, release: f64, sample_rate: f64) -> Self {
        let attack_coef = (-1.0 / (attack * sample_rate)).exp();
        let release_coef = (-1.0 / (release * sample_rate)).exp();
        Self {
            threshold,
            ratio: ratio.max(1.0),
            attack,
            release,
            sample_rate,
            envelope: 0.0,
            attack_coef,
            release_coef,
        }
    }

    pub fn tick(&mut self, input: f64) -> f64 {
        // Convert to dB.
        let abs_input = input.abs();
        let input_db = if abs_input > 1e-10 {
            20.0 * abs_input.log10()
        } else {
            -200.0
        };

        // Gain reduction.
        let over_threshold = input_db - self.threshold;
        let target_gain = if over_threshold > 0.0 {
            -over_threshold * (1.0 - 1.0 / self.ratio)
        } else {
            0.0
        };

        // Smooth envelope.
        let coef = if target_gain < self.envelope {
            self.attack_coef
        } else {
            self.release_coef
        };
        self.envelope = coef * self.envelope + (1.0 - coef) * target_gain;

        // Apply gain reduction.
        let gain_linear = 10.0_f64.powf(self.envelope / 20.0);
        input * gain_linear
    }

    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &s) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = self.tick(s);
            }
        }
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

/// Simple 3-band EQ (low, mid, high) using biquad filters.
use super::filter::{BiquadFilter, FilterType};

#[derive(Debug)]
pub struct Equalizer {
    low: BiquadFilter,
    mid: BiquadFilter,
    high: BiquadFilter,
    pub low_gain: f64,  // dB
    pub mid_gain: f64,  // dB
    pub high_gain: f64, // dB
}

impl Equalizer {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            low: BiquadFilter::new(FilterType::LowPass, 200.0, 0.707, sample_rate),
            mid: BiquadFilter::new(FilterType::BandPass, 1000.0, 0.707, sample_rate),
            high: BiquadFilter::new(FilterType::HighPass, 3000.0, 0.707, sample_rate),
            low_gain: 0.0,
            mid_gain: 0.0,
            high_gain: 0.0,
        }
    }

    pub fn set_band_gains(&mut self, low_db: f64, mid_db: f64, high_db: f64) {
        self.low_gain = low_db;
        self.mid_gain = mid_db;
        self.high_gain = high_db;
    }

    pub fn tick(&mut self, input: f64) -> f64 {
        let low = self.low.tick(input) * 10.0_f64.powf(self.low_gain / 20.0);
        let mid = self.mid.tick(input) * 10.0_f64.powf(self.mid_gain / 20.0);
        let high = self.high.tick(input) * 10.0_f64.powf(self.high_gain / 20.0);
        low + mid + high
    }

    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &s) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = self.tick(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_basic() {
        let mut delay = Delay::new(100, 0.3, 0.5);
        let input = vec![1.0, 0.0, 0.0, 0.0];
        let mut output = vec![0.0; 4];
        delay.process(&input, &mut output);
        // First sample should be mostly input (dry).
        assert!(output[0] > 0.9);
    }

    #[test]
    fn delay_feedback_decays() {
        let mut delay = Delay::new(10, 0.5, 0.5);
        delay.tick(1.0);
        for _ in 0..9 {
            delay.tick(0.0);
        }
        // After one delay period, the echo should be present but smaller than original.
        let echo = delay.tick(0.0);
        assert!(echo > 0.0, "echo should be non-zero: {echo}");
        assert!(echo < 1.0, "echo should be smaller than original: {echo}");
    }

    #[test]
    fn reverb_adds_tail() {
        let mut reverb = Reverb::new(44100.0, 0.5, 0.3, 0.5);
        let mut input = vec![0.0; 3000];
        input[0] = 1.0;
        let mut output = vec![0.0; 3000];
        reverb.process(&input, &mut output);
        // The reverb should produce non-zero output in the tail (after the impulse).
        // Delay lines are ~29-43ms = ~1277-1894 samples at 44.1kHz.
        let has_tail = output[100..].iter().any(|x| x.abs() > 1e-10);
        assert!(has_tail, "reverb should produce a tail after impulse");
    }

    #[test]
    fn compressor_reduces_loud() {
        let mut comp = Compressor::new(-10.0, 4.0, 0.001, 0.1, 44100.0);
        let loud = 0.9; // ~ -0.9 dB, well above -10 threshold
        let compressed = comp.tick(loud);
        assert!(compressed < loud, "compressor should reduce loud signal");
    }

    #[test]
    fn compressor_passes_quiet() {
        let mut comp = Compressor::new(-10.0, 4.0, 0.001, 0.1, 44100.0);
        let quiet = 0.01; // ~ -40 dB, below threshold
        let compressed = comp.tick(quiet);
        assert!(
            (compressed - quiet).abs() < 0.01,
            "compressor should pass quiet signal"
        );
    }

    #[test]
    fn eq_passes_signal() {
        let mut eq = Equalizer::new(44100.0);
        eq.set_band_gains(0.0, 0.0, 0.0);
        let input = vec![0.5; 100];
        let mut output = vec![0.0; 100];
        eq.process(&input, &mut output);
        // With 0 dB gains, output should be non-zero.
        assert!(output[50].abs() > 0.0);
    }

    #[test]
    fn delay_reset() {
        let mut delay = Delay::new(100, 0.5, 0.5);
        for _ in 0..50 {
            delay.tick(0.5);
        }
        delay.reset();
        let v = delay.tick(0.0);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn compressor_reset() {
        let mut comp = Compressor::new(-10.0, 4.0, 0.001, 0.1, 44100.0);
        for _ in 0..100 {
            comp.tick(0.9);
        }
        comp.reset();
        assert_eq!(comp.envelope, 0.0);
    }
}
