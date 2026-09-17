//! Audio meters — waveform, phase correlation, loudness (LUFS).

/// Waveform view — computes peak/RMS envelope of a signal buffer.
#[derive(Debug, Clone)]
pub struct WaveformMeter {
    pub peak: f64,
    pub rms: f64,
}

impl WaveformMeter {
    pub fn new() -> Self {
        Self {
            peak: 0.0,
            rms: 0.0,
        }
    }

    /// Analyse a buffer and update peak/RMS.
    pub fn analyse(&mut self, samples: &[f64]) {
        if samples.is_empty() {
            self.peak = 0.0;
            self.rms = 0.0;
            return;
        }
        let mut sum_sq = 0.0;
        let mut peak = 0.0;
        for &s in samples {
            let abs = s.abs();
            if abs > peak {
                peak = abs;
            }
            sum_sq += s * s;
        }
        self.peak = peak;
        self.rms = (sum_sq / samples.len() as f64).sqrt();
    }

    /// Get a downsampled waveform display (min/max per bucket).
    pub fn waveform_display(samples: &[f64], buckets: usize) -> Vec<(f64, f64)> {
        if samples.is_empty() || buckets == 0 {
            return vec![];
        }
        let bucket_size = samples.len() / buckets;
        if bucket_size == 0 {
            return vec![];
        }
        let mut result = Vec::with_capacity(buckets);
        for i in 0..buckets {
            let start = i * bucket_size;
            let end = ((i + 1) * bucket_size).min(samples.len());
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for &s in &samples[start..end] {
                if s < min {
                    min = s;
                }
                if s > max {
                    max = s;
                }
            }
            if min == f64::INFINITY {
                min = 0.0;
                max = 0.0;
            }
            result.push((min, max));
        }
        result
    }
}

/// Phase correlation meter — measures stereo correlation.
#[derive(Debug, Clone)]
pub struct PhaseMeter {
    pub correlation: f64, // [-1, 1], 1 = mono, 0 = uncorrelated, -1 = out of phase
}

impl PhaseMeter {
    pub fn new() -> Self {
        Self { correlation: 1.0 }
    }

    /// Analyse left/right channels.
    pub fn analyse(&mut self, left: &[f64], right: &[f64]) {
        let n = left.len().min(right.len());
        if n == 0 {
            self.correlation = 1.0;
            return;
        }
        let mut sum_lr = 0.0;
        let mut sum_l2 = 0.0;
        let mut sum_r2 = 0.0;
        for i in 0..n {
            sum_lr += left[i] * right[i];
            sum_l2 += left[i] * left[i];
            sum_r2 += right[i] * right[i];
        }
        let denom = (sum_l2 * sum_r2).sqrt();
        self.correlation = if denom > 1e-10 { sum_lr / denom } else { 1.0 };
    }
}

/// Loudness meter — ITU-R BS.1770 simplified LUFS measurement.
///
/// This is a simplified implementation: K-weighting filter + integrated
/// loudness over a sliding window. Full BS.1770 includes channel weighting
/// and gating; this version handles mono/stereo.
#[derive(Debug, Clone)]
pub struct LoudnessMeter {
    /// Current momentary loudness (LUFS).
    pub momentary_lufs: f64,
    /// Current short-term loudness (LUFS).
    pub short_term_lufs: f64,
    // K-weighting filter state (high-pass shelf + high-pass RLB)
    hp_x1: f64,
    hp_x2: f64,
    hp_y1: f64,
    hp_y2: f64,
    rlb_x1: f64,
    rlb_x2: f64,
    rlb_y1: f64,
    rlb_y2: f64,
    // Integration window
    window: Vec<f64>,
    window_pos: usize,
    window_size: usize,
}

impl LoudnessMeter {
    pub fn new(sample_rate: f64) -> Self {
        // 400ms window for momentary, 3s for short-term.
        let window_size = (0.4 * sample_rate) as usize;
        Self {
            momentary_lufs: -70.0,
            short_term_lufs: -70.0,
            hp_x1: 0.0,
            hp_x2: 0.0,
            hp_y1: 0.0,
            hp_y2: 0.0,
            rlb_x1: 0.0,
            rlb_x2: 0.0,
            rlb_y1: 0.0,
            rlb_y2: 0.0,
            window: vec![0.0; window_size],
            window_pos: 0,
            window_size,
        }
    }

    /// K-weighting filter (ITU-R BS.1770 stage 1 high-shelf + stage 2 high-pass RLB).
    fn k_weight(&mut self, input: f64) -> f64 {
        // Stage 1: high-shelf boost (48kHz biquad approximation).
        let b0 = 1.53512485958697;
        let b1 = -2.69169618940638;
        let b2 = 1.19839281085285;
        let a1 = -1.69065929318241;
        let a2 = 0.73248077421585;

        let hs_out =
            b0 * input + b1 * self.hp_x1 + b2 * self.hp_x2 - a1 * self.hp_y1 - a2 * self.hp_y2;
        self.hp_x2 = self.hp_x1;
        self.hp_x1 = input;
        self.hp_y2 = self.hp_y1;
        self.hp_y1 = hs_out;

        // Stage 2: high-pass (RLB filter) applied to the high-shelf output.
        let rlb_out =
            hs_out - 2.0 * self.rlb_x1 + self.rlb_x2 + 1.99 * self.rlb_y1 - 0.99 * self.rlb_y2;
        self.rlb_x2 = self.rlb_x1;
        self.rlb_x1 = hs_out;
        self.rlb_y2 = self.rlb_y1;
        self.rlb_y1 = rlb_out;

        rlb_out
    }

    /// Process one sample.
    pub fn tick(&mut self, input: f64) -> f64 {
        let weighted = self.k_weight(input);
        let sq = weighted * weighted;
        self.window[self.window_pos] = sq;
        self.window_pos = (self.window_pos + 1) % self.window_size;

        // Compute momentary loudness.
        let mean_sq: f64 = self.window.iter().sum::<f64>() / self.window_size as f64;
        self.momentary_lufs = if mean_sq > 1e-12 {
            -0.691 + 10.0 * mean_sq.log10()
        } else {
            -70.0
        };
        weighted
    }

    /// Process a buffer.
    pub fn process(&mut self, samples: &[f64]) {
        for &s in samples {
            self.tick(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waveform_meter_peak_rms() {
        let mut meter = WaveformMeter::new();
        let samples: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
        meter.analyse(&samples);
        assert!(meter.peak > 0.0);
        assert!(meter.rms > 0.0);
        assert!(meter.rms <= meter.peak);
    }

    #[test]
    fn waveform_meter_silence() {
        let mut meter = WaveformMeter::new();
        meter.analyse(&[0.0; 100]);
        assert_eq!(meter.peak, 0.0);
        assert_eq!(meter.rms, 0.0);
    }

    #[test]
    fn waveform_display_basic() {
        let samples: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.1).sin()).collect();
        let display = WaveformMeter::waveform_display(&samples, 10);
        assert_eq!(display.len(), 10);
        for (min, max) in &display {
            assert!(min <= max);
        }
    }

    #[test]
    fn phase_meter_correlated() {
        let mut meter = PhaseMeter::new();
        let signal: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
        meter.analyse(&signal, &signal);
        assert!((meter.correlation - 1.0).abs() < 0.01);
    }

    #[test]
    fn phase_meter_anticorrelated() {
        let mut meter = PhaseMeter::new();
        let left: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
        let right: Vec<f64> = left.iter().map(|&x| -x).collect();
        meter.analyse(&left, &right);
        assert!((meter.correlation + 1.0).abs() < 0.01);
    }

    #[test]
    fn phase_meter_uncorrelated() {
        let mut meter = PhaseMeter::new();
        let left: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
        let right: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.03).cos()).collect();
        meter.analyse(&left, &right);
        assert!(meter.correlation.abs() < 0.3);
    }

    #[test]
    fn loudness_meter_silence() {
        let mut meter = LoudnessMeter::new(44100.0);
        meter.process(&[0.0; 1000]);
        assert!(meter.momentary_lufs < -60.0);
    }

    #[test]
    fn loudness_meter_loud_signal() {
        let mut meter = LoudnessMeter::new(44100.0);
        // Use a 1kHz sine wave — K-weighting includes a high-pass that removes DC.
        let signal: Vec<f64> = (0..20000)
            .map(|i| (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 44100.0).sin() * 0.5)
            .collect();
        meter.process(&signal);
        // A loud sine should produce a finite, non-silence LUFS.
        assert!(
            meter.momentary_lufs > -70.0,
            "momentary_lufs={}",
            meter.momentary_lufs
        );
    }
}
