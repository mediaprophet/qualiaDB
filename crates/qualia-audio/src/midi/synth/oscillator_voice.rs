//! Single oscillator voice — waveform × ADSR × velocity gain, zero-alloc.
//!
//! One naive (non-band-limited) oscillator running at a set frequency, gated by an
//! [`AdsrEnvelope`] and scaled by a per-note velocity gain. `render_sample()` produces
//! one output sample and advances all internal state. No heap, no locks, no FS.

use super::adsr::AdsrEnvelope;

/// Selectable oscillator waveform.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Waveform {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
}

/// A single monophonic voice: oscillator + envelope + velocity gain.
///
/// Frequency is supplied directly to `note_on` (Hz), so a tuning table can drive it;
/// callers wanting equal temperament compute the frequency upstream.
#[derive(Clone, Copy, Debug)]
pub struct OscillatorVoice {
    sample_rate: f32,
    waveform: Waveform,
    /// Normalized phase in `[0.0, 1.0)`.
    phase: f32,
    /// Phase increment per sample = freq / sample_rate.
    phase_inc: f32,
    /// Velocity-derived gain in `0.0..=1.0`.
    velocity_gain: f32,
    env: AdsrEnvelope,
}

impl OscillatorVoice {
    /// Build an idle voice with the given waveform and envelope shape.
    pub fn new(sample_rate: f32, waveform: Waveform, env: AdsrEnvelope) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            1.0
        };
        Self {
            sample_rate: sr,
            waveform,
            phase: 0.0,
            phase_inc: 0.0,
            velocity_gain: 0.0,
            env,
        }
    }

    /// Start a note at an explicit frequency (Hz) and MIDI velocity (`0..=127`).
    pub fn note_on(&mut self, freq_hz: f32, velocity: u8) {
        let f = if freq_hz.is_finite() && freq_hz > 0.0 {
            freq_hz
        } else {
            0.0
        };
        self.phase = 0.0;
        self.phase_inc = f / self.sample_rate;
        self.velocity_gain = (velocity as f32 / 127.0).clamp(0.0, 1.0);
        self.env.note_on();
    }

    /// Release the note (enters the envelope's release stage).
    pub fn note_off(&mut self) {
        self.env.note_off();
    }

    /// `true` while the voice still contributes signal (attack..release).
    #[inline]
    pub fn is_active(&self) -> bool {
        self.env.is_active()
    }

    /// Velocity gain currently applied (`0..=1`).
    #[inline]
    pub fn velocity_gain(&self) -> f32 {
        self.velocity_gain
    }

    /// Render one sample and advance phase + envelope.
    pub fn render_sample(&mut self) -> f32 {
        let p = self.phase;
        let osc = match self.waveform {
            Waveform::Sine => (core::f32::consts::TAU * p).sin(),
            Waveform::Saw => 2.0 * p - 1.0,
            Waveform::Square => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            // Peak +1 at p=0.5, -1 at the edges.
            Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        };
        self.phase += self.phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            // Guard against freq >= sample_rate (step > 1.0).
            if self.phase >= 1.0 {
                self.phase = self.phase.fract();
            }
        }
        let gain = self.env.process_sample();
        osc * gain * self.velocity_gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn fast_env() -> AdsrEnvelope {
        // Near-instant attack, full sustain so gain settles at ~1.0 quickly.
        AdsrEnvelope::new(SR, 0.001, 0.0, 1.0, 0.010)
    }

    #[test]
    fn sine_dominant_frequency_is_440hz() {
        let mut v = OscillatorVoice::new(SR, Waveform::Sine, fast_env());
        v.note_on(440.0, 127);
        // Render 1.0 s → 440 cycles → 880 zero crossings.
        let n = SR as usize;
        let mut buf = vec![0.0f32; n];
        for s in buf.iter_mut() {
            *s = v.render_sample();
        }
        // Count sign changes (ignoring exact zeros).
        let mut crossings = 0usize;
        let mut prev_sign = 0i8;
        for &x in &buf {
            let sign = if x > 0.0 {
                1
            } else if x < 0.0 {
                -1
            } else {
                0
            };
            if sign != 0 {
                if prev_sign != 0 && sign != prev_sign {
                    crossings += 1;
                }
                prev_sign = sign;
            }
        }
        // 440 Hz over 1 s → ~880 zero crossings.
        assert!(
            (crossings as i64 - 880).abs() <= 4,
            "expected ~880 zero crossings for 440 Hz, got {crossings}"
        );
        // Amplitude bounded by velocity gain (1.0 here).
        let max_abs = buf.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!(max_abs <= 1.0 + 1e-4, "amplitude exceeds velocity bound: {max_abs}");
    }

    #[test]
    fn amplitude_scales_with_velocity() {
        let mut v = OscillatorVoice::new(SR, Waveform::Saw, fast_env());
        v.note_on(220.0, 64);
        let mut max_abs = 0.0f32;
        for _ in 0..SR as usize {
            max_abs = max_abs.max(v.render_sample().abs());
        }
        let bound = 64.0 / 127.0;
        assert!(max_abs <= bound + 1e-3, "saw amplitude {max_abs} exceeds velocity bound {bound}");
        assert!(max_abs > bound - 0.05, "saw should approach velocity bound, got {max_abs}");
    }

    #[test]
    fn inactive_before_note_on() {
        let v = OscillatorVoice::new(SR, Waveform::Square, fast_env());
        assert!(!v.is_active());
    }
}
