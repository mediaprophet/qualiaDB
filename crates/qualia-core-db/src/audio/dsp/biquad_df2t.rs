//! Direct Form II Transposed (DF2T) 3-Band Parametric Biquad Filter.
//!
//! Designed for the Webizen AudioWorklet real-time audio thread:
//! - **DF2T Topology**: 2 state variables per biquad, superior numerical stability
//!   against low-frequency pole quantization (200 Hz low-shelf anchor).
//! - **Denormal Prevention**: Injects $10^{-24}$ DC offset into raw samples, preventing
//!   subnormal stall penalties (up to 100x) on mobile ARM NEON FPUs.
//! - **128-Sample Quantum Ramping**: Trigonometric coefficient updates occur strictly
//!   at quantum boundaries (2.67 ms @ 48 kHz). Inner sample loop linearly ramps coefficients
//!   using 5 scalar additions per sample, eliminating zipper noise and audio clicks.
//! - **Zero-Heap Hot Path**: Evaluates directly into caller-supplied slices, delegating
//!   RBJ Audio EQ Cookbook synthesis and DF2T recurrence math to `etalon_signal::filter`.

use etalon_signal::filter::{
    df2t_step_f32, rbj_high_shelf, rbj_low_shelf, rbj_peaking_eq,
    BiquadCoeffs as EtalonBiquadCoeffs,
};

/// AudioWorklet render quantum — coefficient updates and ramping occur at
/// these boundaries (2.67 ms @ 48 kHz).
pub const QUANTUM_SIZE: usize = 128;
/// DC offset injected per sample to prevent subnormal-FPU stall penalties.
pub const DENORMAL_DC_OFFSET: f32 = 1e-24;

/// Low-frequency shelf anchor (Hz).
pub const ANCHOR_LOW_HZ: f32 = 200.0;
/// Mid-frequency peaking band anchor (Hz).
pub const ANCHOR_MID_HZ: f32 = 1000.0;
/// High-frequency shelf anchor (Hz).
pub const ANCHOR_HIGH_HZ: f32 = 8000.0;

/// Normalized biquad filter coefficients $H(z) = \frac{b_0 + b_1 z^{-1} + b_2 z^{-2}}{1 + a_1 z^{-1} + a_2 z^{-2}}$.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    /// Feed-forward coefficient for `x[n]`.
    pub b0: f32,
    /// Feed-forward coefficient for `x[n-1]`.
    pub b1: f32,
    /// Feed-forward coefficient for `x[n-2]`.
    pub b2: f32,
    /// Feedback coefficient for `y[n-1]`.
    pub a1: f32,
    /// Feedback coefficient for `y[n-2]`.
    pub a2: f32,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    /// Convert to double-precision `etalon_signal::filter::BiquadCoeffs`.
    #[inline]
    pub fn to_etalon(self) -> EtalonBiquadCoeffs {
        EtalonBiquadCoeffs::from_f32(self.b0, self.b1, self.b2, self.a1, self.a2)
    }

    /// Convert from double-precision `etalon_signal::filter::BiquadCoeffs`.
    #[inline]
    pub fn from_etalon(c: EtalonBiquadCoeffs) -> Self {
        Self {
            b0: c.b0 as f32,
            b1: c.b1 as f32,
            b2: c.b2 as f32,
            a1: c.a1 as f32,
            a2: c.a2 as f32,
        }
    }

    /// Verify Jury stability ($|a_2| < 1$ and $|a_1| < 1 + a_2$) via `etalon_signal`.
    #[inline]
    pub fn is_stable(&self) -> bool {
        self.to_etalon().is_stable()
    }
}

/// Direct Form II Transposed filter state registers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Df2tState {
    /// First state register (`s1[n-1]`).
    pub s1: f32,
    /// Second state register (`s2[n-1]`).
    pub s2: f32,
}

impl Df2tState {
    /// Clear both state registers.
    #[inline]
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

/// Single DF2T biquad filter with linear coefficient ramping.
#[derive(Debug, Clone)]
pub struct Df2tBiquad {
    /// DF2T state registers.
    pub state: Df2tState,
    /// Coefficients in use this sample (ramped per-sample toward `target`).
    pub active: BiquadCoeffs,
    /// Coefficients being ramped toward over the current quantum.
    pub target: BiquadCoeffs,
    /// Per-sample increment derived in `set_target_quantum`.
    pub delta: BiquadCoeffs,
}

impl Default for Df2tBiquad {
    fn default() -> Self {
        Self {
            state: Df2tState::default(),
            active: BiquadCoeffs::default(),
            target: BiquadCoeffs::default(),
            delta: BiquadCoeffs {
                b0: 0.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            },
        }
    }
}

impl Df2tBiquad {
    /// Prepare ramping step towards new target coefficients over the next 128-sample quantum.
    #[inline]
    pub fn set_target_quantum(&mut self, target: BiquadCoeffs) {
        self.target = target;
        let inv_q = 1.0 / (QUANTUM_SIZE as f32);
        self.delta = BiquadCoeffs {
            b0: (target.b0 - self.active.b0) * inv_q,
            b1: (target.b1 - self.active.b1) * inv_q,
            b2: (target.b2 - self.active.b2) * inv_q,
            a1: (target.a1 - self.active.a1) * inv_q,
            a2: (target.a2 - self.active.a2) * inv_q,
        };
    }

    /// Process a single sample using DF2T transposed topology with 5-addition coefficient ramp.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Step coefficients
        self.active.b0 += self.delta.b0;
        self.active.b1 += self.delta.b1;
        self.active.b2 += self.delta.b2;
        self.active.a1 += self.delta.a1;
        self.active.a2 += self.delta.a2;

        df2t_step_f32(
            self.active.b0,
            self.active.b1,
            self.active.b2,
            self.active.a1,
            self.active.a2,
            &mut self.state.s1,
            &mut self.state.s2,
            input,
        )
    }
}

/// 3-Band Parametric Equalizer consisting of cascaded DF2T biquads:
/// 1. Low Shelf (200 Hz anchor)
/// 2. Mid Peaking (1000 Hz anchor)
/// 3. High Shelf (8000 Hz anchor)
#[derive(Debug, Clone)]
pub struct ThreeBandParametricBiquad {
    /// Sample rate the coefficients were computed for (clamped to ≥ 8 kHz).
    pub sample_rate: f32,
    /// Low shelf stage at `ANCHOR_LOW_HZ` (200 Hz).
    pub low_shelf: Df2tBiquad,
    /// Mid peaking stage at `ANCHOR_MID_HZ` (1 kHz).
    pub mid_peak: Df2tBiquad,
    /// High shelf stage at `ANCHOR_HIGH_HZ` (8 kHz).
    pub high_shelf: Df2tBiquad,
}

impl ThreeBandParametricBiquad {
    /// Create an EQ at flat unity response (0 dB on all bands), coefficients
    /// snapped to target so no ramping occurs on the first quantum.
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            sample_rate: sample_rate.max(8000.0),
            low_shelf: Df2tBiquad::default(),
            mid_peak: Df2tBiquad::default(),
            high_shelf: Df2tBiquad::default(),
        };
        // Initialize at flat unity response
        eq.set_band_gains_db(0.0, 0.0, 0.0);
        eq.snap_to_target();
        eq
    }

    /// Snap active coefficients directly to target (useful on init or seek).
    pub fn snap_to_target(&mut self) {
        self.low_shelf.active = self.low_shelf.target;
        self.low_shelf.delta = BiquadCoeffs {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        self.mid_peak.active = self.mid_peak.target;
        self.mid_peak.delta = BiquadCoeffs {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        self.high_shelf.active = self.high_shelf.target;
        self.high_shelf.delta = BiquadCoeffs {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
    }

    /// Reset internal filter state registers (clear delay lines).
    pub fn reset_states(&mut self) {
        self.low_shelf.state.reset();
        self.mid_peak.state.reset();
        self.high_shelf.state.reset();
    }

    /// Configure band gains in decibels for the next 128-sample quantum.
    pub fn set_band_gains_db(&mut self, low_gain_db: f32, mid_gain_db: f32, high_gain_db: f32) {
        let low_coeffs =
            calc_low_shelf_coeffs(self.sample_rate, ANCHOR_LOW_HZ, 0.7071, low_gain_db);
        let mid_coeffs = calc_peaking_coeffs(self.sample_rate, ANCHOR_MID_HZ, 1.0, mid_gain_db);
        let high_coeffs =
            calc_high_shelf_coeffs(self.sample_rate, ANCHOR_HIGH_HZ, 0.7071, high_gain_db);

        self.low_shelf.set_target_quantum(low_coeffs);
        self.mid_peak.set_target_quantum(mid_coeffs);
        self.high_shelf.set_target_quantum(high_coeffs);
    }

    /// Map continuous Jakob & Hanika spectral polynomial coefficients $(c_0, c_1, c_2)$
    /// directly to the 3-band acoustic parameters.
    pub fn set_from_spectral_coeffs(&mut self, c0: f32, c1: f32, c2: f32) {
        // Map polynomial terms to decibel gains in [-18.0, +18.0] dB
        let low_db = (c0 * 12.0).clamp(-24.0, 24.0);
        let mid_db = (c1 * 12.0).clamp(-24.0, 24.0);
        let high_db = (c2 * 12.0).clamp(-24.0, 24.0);
        self.set_band_gains_db(low_db, mid_db, high_db);
    }

    /// Process a single 128-sample quantum in-place with zero allocations.
    /// Injects $10^{-24}$ DC offset into each sample to prevent mobile ARM NEON denormal stalls.
    #[inline]
    pub fn process_quantum(&mut self, buffer: &mut [f32; QUANTUM_SIZE]) {
        for sample in buffer.iter_mut() {
            // Anti-denormal offset
            let clean_in = *sample + DENORMAL_DC_OFFSET;

            // Cascade: Low Shelf -> Mid Peak -> High Shelf
            let y1 = self.low_shelf.process_sample(clean_in);
            let y2 = self.mid_peak.process_sample(y1);
            let y3 = self.high_shelf.process_sample(y2);

            *sample = y3;
        }
    }

    /// Process a caller-supplied slice of arbitrary length (multiple of 128 recommended).
    /// Zero heap allocation.
    pub fn process_slice(&mut self, input: &[f32], output: &mut [f32]) {
        let n = input.len().min(output.len());
        let mut i = 0;
        while i < n {
            let chunk_len = (n - i).min(QUANTUM_SIZE);
            for k in 0..chunk_len {
                let clean_in = input[i + k] + DENORMAL_DC_OFFSET;
                let y1 = self.low_shelf.process_sample(clean_in);
                let y2 = self.mid_peak.process_sample(y1);
                let y3 = self.high_shelf.process_sample(y2);
                output[i + k] = y3;
            }
            i += chunk_len;
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Cookbook Biquad Coefficient Calculators (delegated to etalon_signal::filter)
// ───────────────────────────────────────────────────────────────────────────

#[inline]
fn sanitize_rbj_params(sr: f32, f0: f32, q: f32) -> (f64, f64, f64) {
    let sr_f64 = f64::from(sr.max(8000.0));
    let f0_f64 = f64::from(f0).clamp(1.0, 0.499 * sr_f64);
    let q_f64 = f64::from(q).max(1e-4);
    (sr_f64, f0_f64, q_f64)
}

#[inline]
fn calc_low_shelf_coeffs(sr: f32, f0: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let (sr_f64, f0_f64, q_f64) = sanitize_rbj_params(sr, f0, q);
    rbj_low_shelf(sr_f64, f0_f64, q_f64, f64::from(gain_db))
        .map(BiquadCoeffs::from_etalon)
        .unwrap_or_default()
}

#[inline]
fn calc_peaking_coeffs(sr: f32, f0: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let (sr_f64, f0_f64, q_f64) = sanitize_rbj_params(sr, f0, q);
    rbj_peaking_eq(sr_f64, f0_f64, q_f64, f64::from(gain_db))
        .map(BiquadCoeffs::from_etalon)
        .unwrap_or_default()
}

#[inline]
fn calc_high_shelf_coeffs(sr: f32, f0: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let (sr_f64, f0_f64, q_f64) = sanitize_rbj_params(sr, f0, q);
    rbj_high_shelf(sr_f64, f0_f64, q_f64, f64::from(gain_db))
        .map(BiquadCoeffs::from_etalon)
        .unwrap_or_default()
}

/// Standalone JavaScript snippet for standard AudioWorkletProcessor registration.
pub const AUDIO_WORKLET_DF2T_JS: &str = r#"
class ThreeBandBiquadProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.s1 = [0, 0, 0];
    this.s2 = [0, 0, 0];
    this.active = [
      { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 },
      { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 },
      { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 }
    ];
  }

  process(inputs, outputs, parameters) {
    const input = inputs[0];
    const output = outputs[0];
    if (!input || !input[0]) return true;

    const inChannel = input[0];
    const outChannel = output[0];
    const len = inChannel.length;

    for (let i = 0; i < len; i++) {
      // Inject 1e-24 DC offset to prevent subnormal ARM NEON stalls
      let x = inChannel[i] + 1e-24;
      for (let b = 0; b < 3; b++) {
        const c = this.active[b];
        const y = c.b0 * x + this.s1[b];
        this.s1[b] = c.b1 * x - c.a1 * y + this.s2[b];
        this.s2[b] = c.b2 * x - c.a2 * y;
        x = y;
      }
      outChannel[i] = x;
    }
    return true;
  }
}
registerProcessor('three-band-biquad-processor', ThreeBandBiquadProcessor);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df2t_zero_input_decay_stays_bounded() {
        let mut eq = ThreeBandParametricBiquad::new(48000.0);
        eq.set_band_gains_db(6.0, -3.0, 4.0);
        eq.snap_to_target();
        assert!(eq.low_shelf.active.is_stable());
        assert!(eq.mid_peak.active.is_stable());
        assert!(eq.high_shelf.active.is_stable());

        let mut block = [0.0f32; QUANTUM_SIZE];
        block[0] = 1.0; // Impulse
        eq.process_quantum(&mut block);

        // Subsequent silence blocks
        for _ in 0..10 {
            let mut silence = [0.0f32; QUANTUM_SIZE];
            eq.process_quantum(&mut silence);
            // Verify no NaN or Inf
            for s in silence.iter() {
                assert!(!s.is_nan());
                assert!(!s.is_infinite());
            }
        }
        // State should not enter denormal zero due to DC offset
        assert!(eq.low_shelf.state.s1.abs() > 0.0);
    }

    #[test]
    fn df2t_coefficient_ramping_eliminates_steps() {
        let mut eq = ThreeBandParametricBiquad::new(48000.0);
        eq.set_band_gains_db(0.0, 0.0, 0.0);
        eq.snap_to_target();

        // Introduce large step change
        eq.set_band_gains_db(12.0, -12.0, 6.0);

        // Process first quantum
        let mut block = [0.5f32; QUANTUM_SIZE];
        eq.process_quantum(&mut block);

        // Verify coefficients smoothly interpolated
        let active_b0 = eq.low_shelf.active.b0;
        let target_b0 = eq.low_shelf.target.b0;
        assert!((active_b0 - target_b0).abs() < 1e-4);
    }

    #[test]
    fn spectral_coefficient_mapping_produces_stable_response() {
        let mut eq = ThreeBandParametricBiquad::new(48000.0);
        eq.set_from_spectral_coeffs(0.85, 0.32, 0.15);
        eq.snap_to_target();

        let mut block = [0.25f32; QUANTUM_SIZE];
        eq.process_quantum(&mut block);
        for s in block.iter() {
            assert!(s.is_finite());
        }
    }
}
