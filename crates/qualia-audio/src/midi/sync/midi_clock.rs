//! 24-PPQN MIDI clock — generation and consumption.
//!
//! MIDI transmits 24 timing-clock bytes (`0xF8`) per quarter note. One clock
//! therefore lasts `(60 / bpm) / 24` seconds. This module both *generates* the
//! stream (how many clock pulses fall inside a wall-clock time slice) and
//! *consumes* it (estimate BPM from the interval between received pulses,
//! smoothed over a small fixed window so jitter on a single interval does not
//! throw the tempo). No allocation; all state lives on the stack.

use crate::types::AudioError;

/// MIDI System Real-Time: Timing Clock (24 per quarter note).
pub const TIMING_CLOCK: u8 = 0xF8;
/// MIDI System Real-Time: Start.
pub const START: u8 = 0xFA;
/// MIDI System Real-Time: Continue.
pub const CONTINUE: u8 = 0xFB;
/// MIDI System Real-Time: Stop.
pub const STOP: u8 = 0xFC;

/// Clock pulses per quarter note, per the MIDI spec.
pub const CLOCKS_PER_QUARTER: u32 = 24;

/// Seconds between consecutive clock pulses at a given tempo.
///
/// Errors if `bpm` is not a positive finite number.
#[inline]
pub fn clock_period_seconds(bpm: f64) -> Result<f64, AudioError> {
    if !(bpm > 0.0) || !bpm.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    Ok((60.0 / bpm) / CLOCKS_PER_QUARTER as f64)
}

/// Recover BPM from the interval (seconds) between two clock pulses.
///
/// Errors if `period_seconds` is not a positive finite number.
#[inline]
pub fn bpm_from_clock_period(period_seconds: f64) -> Result<f64, AudioError> {
    if !(period_seconds > 0.0) || !period_seconds.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    Ok(60.0 / (period_seconds * CLOCKS_PER_QUARTER as f64))
}

/// A 24-PPQN clock that both emits pulses over time and estimates incoming BPM.
///
/// Generation: [`ClockSync::advance`] accumulates elapsed seconds and returns
/// how many whole clock pulses should be transmitted. Consumption:
/// [`ClockSync::on_pulse`] feeds pulse timestamps and returns the current
/// smoothed BPM estimate once enough intervals are known.
#[derive(Debug, Clone, Copy)]
pub struct ClockSync {
    period: f64,
    accum: f64,
    // Ring of recent inter-pulse intervals for smoothing.
    intervals: [f64; 8],
    filled: usize,
    write: usize,
    last_pulse: f64,
    have_last: bool,
}

impl ClockSync {
    /// Build a generator/estimator anchored at `bpm`. Errors on bad tempo.
    pub fn new(bpm: f64) -> Result<Self, AudioError> {
        Ok(Self {
            period: clock_period_seconds(bpm)?,
            accum: 0.0,
            intervals: [0.0; 8],
            filled: 0,
            write: 0,
            last_pulse: 0.0,
            have_last: false,
        })
    }

    /// Retune the generator's tempo. Errors on bad tempo.
    pub fn set_bpm(&mut self, bpm: f64) -> Result<(), AudioError> {
        self.period = clock_period_seconds(bpm)?;
        Ok(())
    }

    /// Generation: advance by `delta_seconds`, returning the number of whole
    /// clock pulses to transmit in that slice. Errors if `delta_seconds` is
    /// negative or not finite.
    pub fn advance(&mut self, delta_seconds: f64) -> Result<u32, AudioError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(AudioError::InvalidParameter);
        }
        self.accum += delta_seconds;
        let mut pulses = 0u32;
        while self.accum >= self.period {
            self.accum -= self.period;
            pulses += 1;
        }
        Ok(pulses)
    }

    /// Consumption: feed the absolute timestamp (seconds) of a received clock
    /// pulse. Returns `Some(bpm)` once at least one interval is known, using
    /// the average of the recent-interval window; `None` on the first pulse.
    pub fn on_pulse(&mut self, timestamp_seconds: f64) -> Option<f64> {
        if !timestamp_seconds.is_finite() {
            return None;
        }
        if self.have_last {
            let dt = timestamp_seconds - self.last_pulse;
            if dt > 0.0 && dt.is_finite() {
                self.intervals[self.write] = dt;
                self.write = (self.write + 1) % self.intervals.len();
                if self.filled < self.intervals.len() {
                    self.filled += 1;
                }
            }
        }
        self.last_pulse = timestamp_seconds;
        self.have_last = true;

        if self.filled == 0 {
            return None;
        }
        let mut sum = 0.0;
        for &v in &self.intervals[..self.filled] {
            sum += v;
        }
        let avg = sum / self.filled as f64;
        bpm_from_clock_period(avg).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_matches_120bpm() {
        // 120 BPM: quarter = 0.5s, /24 = 0.0208333...s per clock.
        let p = clock_period_seconds(120.0).unwrap();
        assert!((p - 0.5 / 24.0).abs() < 1e-9, "got {p}");
        assert!((bpm_from_clock_period(p).unwrap() - 120.0).abs() < 1e-6);
    }

    #[test]
    fn generates_24_pulses_per_quarter() {
        let mut c = ClockSync::new(120.0).unwrap();
        // Half a second at 120 BPM is exactly one quarter note = 24 pulses.
        let pulses = c.advance(0.5).unwrap();
        assert_eq!(pulses, 24);
    }

    #[test]
    fn estimates_bpm_from_pulses() {
        let mut c = ClockSync::new(100.0).unwrap();
        let period = clock_period_seconds(140.0).unwrap();
        let mut t = 0.0;
        c.on_pulse(t);
        let mut last = None;
        for _ in 0..8 {
            t += period;
            last = c.on_pulse(t);
        }
        let bpm = last.expect("estimate available");
        assert!((bpm - 140.0).abs() < 1e-3, "got {bpm}");
    }
}
