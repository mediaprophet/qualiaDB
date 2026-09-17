//! Biquad filter — low-pass, high-pass, band-pass, notch.
//!
//! Implements a standard biquad filter using direct form I topology.
//! Coefficients are computed from the RBJ Audio EQ Cookbook formulas.

/// Filter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Biquad filter state and coefficients.
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    pub filter_type: FilterType,
    pub cutoff: f64, // Hz
    pub q: f64,      // Quality factor
    pub sample_rate: f64,
    // Coefficients
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    // State (direct form I)
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    pub fn new(filter_type: FilterType, cutoff: f64, q: f64, sample_rate: f64) -> Self {
        let mut filter = Self {
            filter_type,
            cutoff,
            q,
            sample_rate,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        filter.recalculate();
        filter
    }

    /// Recalculate biquad coefficients from current parameters.
    pub fn recalculate(&mut self) {
        let w0 = 2.0 * std::f64::consts::PI * self.cutoff / self.sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                // Constant 0 dB peak gain.
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normalise by a0.
        self.a0 = a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
    }

    /// Process one sample.
    pub fn tick(&mut self, input: f64) -> f64 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    /// Process a buffer.
    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &s) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = self.tick(s);
            }
        }
    }

    /// Set cutoff frequency.
    pub fn set_cutoff(&mut self, cutoff: f64) {
        self.cutoff = cutoff;
        self.recalculate();
    }

    /// Set Q factor.
    pub fn set_q(&mut self, q: f64) {
        self.q = q;
        self.recalculate();
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpass_attenuates_high_freq() {
        let mut lp = BiquadFilter::new(FilterType::LowPass, 100.0, 0.707, 44100.0);
        // Feed a high-frequency signal (5kHz) — should be attenuated.
        let sr = 44100.0;
        let freq = 5000.0;
        let mut input = vec![0.0; 1000];
        for (i, s) in input.iter_mut().enumerate() {
            *s = (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin();
        }
        let mut output = vec![0.0; 1000];
        lp.process(&input, &mut output);
        let in_amp = input[500..].iter().cloned().fold(0.0f64, f64::max);
        let out_amp = output[500..]
            .iter()
            .cloned()
            .map(|x| x.abs())
            .fold(0.0f64, f64::max);
        assert!(
            out_amp < in_amp * 0.5,
            "lowpass should attenuate high freq: out={out_amp}, in={in_amp}"
        );
    }

    #[test]
    fn highpass_attenuates_low_freq() {
        let mut hp = BiquadFilter::new(FilterType::HighPass, 2000.0, 0.707, 44100.0);
        let sr = 44100.0;
        let freq = 50.0; // Low frequency
        let mut input = vec![0.0; 1000];
        for (i, s) in input.iter_mut().enumerate() {
            *s = (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin();
        }
        let mut output = vec![0.0; 1000];
        hp.process(&input, &mut output);
        let out_amp = output[500..].iter().map(|x| x.abs()).fold(0.0f64, f64::max);
        assert!(
            out_amp < 0.5,
            "highpass should attenuate low freq: out={out_amp}"
        );
    }

    #[test]
    fn filter_reset_clears_state() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100.0);
        for _ in 0..100 {
            filter.tick(0.5);
        }
        filter.reset();
        // After reset with zero input, output should be ~0.
        let v = filter.tick(0.0);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn filter_set_cutoff() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100.0);
        filter.set_cutoff(2000.0);
        assert_eq!(filter.cutoff, 2000.0);
    }

    #[test]
    fn filter_set_q() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100.0);
        filter.set_q(1.0);
        assert_eq!(filter.q, 1.0);
    }

    #[test]
    fn bandpass_passes_center_freq() {
        let mut bp = BiquadFilter::new(FilterType::BandPass, 1000.0, 1.0, 44100.0);
        let sr = 44100.0;
        let freq = 1000.0; // Center frequency
        let mut input = vec![0.0; 2000];
        for (i, s) in input.iter_mut().enumerate() {
            *s = (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin();
        }
        let mut output = vec![0.0; 2000];
        bp.process(&input, &mut output);
        let out_amp = output[1000..]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f64, f64::max);
        assert!(
            out_amp > 0.1,
            "bandpass should pass center freq: out={out_amp}"
        );
    }
}
