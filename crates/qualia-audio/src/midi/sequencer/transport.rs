//! Transport state machine — play / stop / pause, tempo, position, and looping.
//!
//! [`Transport`] holds a fractional tick position and advances it in wall-clock
//! time via [`Transport::advance`]: at `bpm` quarter notes/minute with `ppq`
//! ticks/quarter, `Δticks = Δseconds * (bpm / 60) * ppq`. When looping is
//! enabled and the position reaches `loop_end`, it wraps back to `loop_start`
//! (carrying the remainder), so a long block never skips past the loop. The
//! whole type is `Copy`, stack-only, and allocation-free — safe on the audio
//! thread.

use crate::types::AudioError;

/// Play/stop/pause state of the [`Transport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    /// Stopped; `advance` does nothing and position is anchored.
    Stopped,
    /// Paused; position is held but not reset.
    Paused,
    /// Playing; `advance` moves the position forward.
    Playing,
}

/// A sequencer transport: position, tempo, run-state, and an optional loop.
#[derive(Debug, Clone, Copy)]
pub struct Transport {
    state: TransportState,
    /// Fractional tick position (sub-tick precision across blocks).
    position: f64,
    bpm: f64,
    ppq: u32,
    loop_enabled: bool,
    loop_start: u64,
    loop_end: u64,
}

impl Transport {
    /// A stopped transport at position 0 with the given resolution and tempo.
    ///
    /// Errors if `ppq == 0` or `bpm` is not a positive finite number.
    pub fn new(ppq: u32, bpm: f64) -> Result<Self, AudioError> {
        if ppq == 0 || !(bpm > 0.0) || !bpm.is_finite() {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            state: TransportState::Stopped,
            position: 0.0,
            bpm,
            ppq,
            loop_enabled: false,
            loop_start: 0,
            loop_end: 0,
        })
    }

    /// Current run-state.
    #[inline]
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Current integer tick position (floor of the fractional position).
    #[inline]
    pub fn position_ticks(&self) -> u64 {
        self.position.max(0.0) as u64
    }

    /// Current fractional tick position.
    #[inline]
    pub fn position_ticks_f64(&self) -> f64 {
        self.position
    }

    /// Current tempo in BPM.
    #[inline]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Ticks per quarter note.
    #[inline]
    pub fn ppq(&self) -> u32 {
        self.ppq
    }

    /// Begin playback from the current position.
    #[inline]
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    /// Pause, holding the current position.
    #[inline]
    pub fn pause(&mut self) {
        self.state = TransportState::Paused;
    }

    /// Stop and rewind the position to 0.
    #[inline]
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position = 0.0;
    }

    /// Jump to an absolute tick position (does not change run-state).
    #[inline]
    pub fn seek(&mut self, tick: u64) {
        self.position = tick as f64;
    }

    /// Set the tempo. Errors if `bpm` is not positive/finite.
    pub fn set_tempo(&mut self, bpm: f64) -> Result<(), AudioError> {
        if !(bpm > 0.0) || !bpm.is_finite() {
            return Err(AudioError::InvalidParameter);
        }
        self.bpm = bpm;
        Ok(())
    }

    /// Enable a loop over `[start, end)` ticks. Errors if `end <= start`.
    pub fn set_loop(&mut self, start: u64, end: u64) -> Result<(), AudioError> {
        if end <= start {
            return Err(AudioError::InvalidParameter);
        }
        self.loop_start = start;
        self.loop_end = end;
        self.loop_enabled = true;
        Ok(())
    }

    /// Disable looping.
    #[inline]
    pub fn clear_loop(&mut self) {
        self.loop_enabled = false;
    }

    /// Advance the position by `delta_seconds` of wall-clock time. Only moves
    /// while [`TransportState::Playing`]. Honors the loop if enabled.
    ///
    /// Errors if `delta_seconds` is negative or not finite.
    pub fn advance(&mut self, delta_seconds: f64) -> Result<(), AudioError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(AudioError::InvalidParameter);
        }
        if self.state != TransportState::Playing {
            return Ok(());
        }
        let delta_ticks = delta_seconds * (self.bpm / 60.0) * self.ppq as f64;
        self.position += delta_ticks;

        if self.loop_enabled {
            let start = self.loop_start as f64;
            let end = self.loop_end as f64;
            let span = end - start;
            if span > 0.0 && self.position >= end {
                // Wrap, carrying any overshoot; handles multi-loop long blocks.
                let over = (self.position - start) % span;
                self.position = start + over;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_half_second_at_120bpm_moves_480_ticks() {
        let mut t = Transport::new(480, 120.0).unwrap();
        t.play();
        t.advance(0.5).unwrap();
        assert!(
            (t.position_ticks_f64() - 480.0).abs() < 1e-6,
            "got {}",
            t.position_ticks_f64()
        );
        assert_eq!(t.position_ticks(), 480);
    }

    #[test]
    fn stopped_does_not_advance() {
        let mut t = Transport::new(480, 120.0).unwrap();
        t.advance(1.0).unwrap();
        assert_eq!(t.position_ticks(), 0);
    }

    #[test]
    fn pause_holds_stop_rewinds() {
        let mut t = Transport::new(480, 120.0).unwrap();
        t.play();
        t.advance(0.5).unwrap();
        t.pause();
        t.advance(0.5).unwrap();
        assert_eq!(t.position_ticks(), 480);
        t.stop();
        assert_eq!(t.position_ticks(), 0);
    }

    #[test]
    fn loop_wraps() {
        let mut t = Transport::new(480, 120.0).unwrap();
        t.set_loop(0, 480).unwrap();
        t.play();
        t.advance(0.6).unwrap(); // 576 ticks -> wraps to 96
        assert!(
            (t.position_ticks_f64() - 96.0).abs() < 1e-6,
            "got {}",
            t.position_ticks_f64()
        );
    }
}
