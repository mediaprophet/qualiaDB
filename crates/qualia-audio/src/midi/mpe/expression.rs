//! Per-note MPE expression tracking — pitch-bend, pressure, and timbre (CC74)
//! held per member channel.
//!
//! Because each MPE note owns a member channel, its three continuous expression
//! dimensions are just that channel's current state:
//! * **pitch-bend** — 14-bit, centered (stored as a signed offset from center),
//! * **pressure** — channel pressure / aftertouch (0..=127),
//! * **timbre** — CC 74 ("slide"), 0..=127.
//!
//! [`MpeExpression`] is a fixed `[ChannelExpr; 16]` table indexed by channel, so
//! updating a controller value is an allocation-free array write — safe on the
//! real-time thread.

use crate::types::AudioError;

/// Assignable MPE per-note continuous-controller number for timbre (CC 74).
pub const CC_TIMBRE: u8 = 74;

/// The live expression state of one member channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelExpr {
    /// Pitch-bend as a signed offset from center (`-8192..=8191`).
    pub pitch_bend: i16,
    /// Channel pressure / aftertouch (`0..=127`).
    pub pressure: u8,
    /// Timbre — CC 74 value (`0..=127`).
    pub timbre: u8,
}

impl ChannelExpr {
    /// Neutral state: no bend, no pressure, mid timbre.
    pub const NEUTRAL: ChannelExpr = ChannelExpr { pitch_bend: 0, pressure: 0, timbre: 64 };
}

/// Per-member-channel MPE expression table.
#[derive(Debug, Clone)]
pub struct MpeExpression {
    channels: [ChannelExpr; 16],
}

impl Default for MpeExpression {
    fn default() -> Self {
        Self::new()
    }
}

impl MpeExpression {
    /// A table with every channel at [`ChannelExpr::NEUTRAL`].
    pub fn new() -> Self {
        Self { channels: [ChannelExpr::NEUTRAL; 16] }
    }

    /// Set pitch-bend from the raw 14-bit MIDI value (`0..=16383`, center 8192).
    ///
    /// Errors if `channel > 15` or `raw14 > 16383`.
    pub fn set_pitch_bend_raw(&mut self, channel: u8, raw14: u16) -> Result<(), AudioError> {
        if channel > 15 || raw14 > 0x3FFF {
            return Err(AudioError::InvalidParameter);
        }
        self.channels[channel as usize].pitch_bend = raw14 as i16 - 8192;
        Ok(())
    }

    /// Set pitch-bend from two 7-bit MIDI data bytes `[LSB, MSB]`.
    pub fn set_pitch_bend_bytes(&mut self, channel: u8, lsb: u8, msb: u8) -> Result<(), AudioError> {
        if lsb & 0x80 != 0 || msb & 0x80 != 0 {
            return Err(AudioError::InvalidParameter);
        }
        let raw = ((msb as u16) << 7) | lsb as u16;
        self.set_pitch_bend_raw(channel, raw)
    }

    /// Set channel pressure (`0..=127`).
    pub fn set_pressure(&mut self, channel: u8, value: u8) -> Result<(), AudioError> {
        if channel > 15 || value > 127 {
            return Err(AudioError::InvalidParameter);
        }
        self.channels[channel as usize].pressure = value;
        Ok(())
    }

    /// Set timbre from a CC message. Only CC 74 updates timbre; other CCs are
    /// ignored (returns `Ok`). Errors on an out-of-range channel or value.
    pub fn set_timbre_cc(&mut self, channel: u8, cc: u8, value: u8) -> Result<(), AudioError> {
        if channel > 15 || value > 127 {
            return Err(AudioError::InvalidParameter);
        }
        if cc == CC_TIMBRE {
            self.channels[channel as usize].timbre = value;
        }
        Ok(())
    }

    /// Current expression state of `channel`.
    pub fn get(&self, channel: u8) -> Result<ChannelExpr, AudioError> {
        if channel > 15 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(self.channels[channel as usize])
    }

    /// Reset a channel to neutral (e.g. on note-off).
    pub fn reset(&mut self, channel: u8) -> Result<(), AudioError> {
        if channel > 15 {
            return Err(AudioError::InvalidParameter);
        }
        self.channels[channel as usize] = ChannelExpr::NEUTRAL;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_bend_centered() {
        let mut e = MpeExpression::new();
        e.set_pitch_bend_raw(2, 8192).unwrap();
        assert_eq!(e.get(2).unwrap().pitch_bend, 0);
        e.set_pitch_bend_raw(2, 0).unwrap();
        assert_eq!(e.get(2).unwrap().pitch_bend, -8192);
        e.set_pitch_bend_raw(2, 16383).unwrap();
        assert_eq!(e.get(2).unwrap().pitch_bend, 8191);
    }

    #[test]
    fn pitch_bend_from_bytes_matches_raw() {
        let mut e = MpeExpression::new();
        // raw 10000 -> lsb = 10000 & 0x7F, msb = 10000 >> 7
        e.set_pitch_bend_bytes(3, (10000u16 & 0x7F) as u8, (10000u16 >> 7) as u8).unwrap();
        assert_eq!(e.get(3).unwrap().pitch_bend, 10000i16 - 8192);
    }

    #[test]
    fn pressure_and_timbre() {
        let mut e = MpeExpression::new();
        e.set_pressure(5, 100).unwrap();
        e.set_timbre_cc(5, CC_TIMBRE, 20).unwrap();
        e.set_timbre_cc(5, 1, 99).unwrap(); // CC1 must not change timbre
        let s = e.get(5).unwrap();
        assert_eq!(s.pressure, 100);
        assert_eq!(s.timbre, 20);
    }

    #[test]
    fn reset_and_range_checks() {
        let mut e = MpeExpression::new();
        e.set_pressure(1, 64).unwrap();
        e.reset(1).unwrap();
        assert_eq!(e.get(1).unwrap(), ChannelExpr::NEUTRAL);
        assert!(e.set_pressure(16, 0).is_err());
        assert!(e.set_pitch_bend_raw(0, 16384).is_err());
        assert!(e.get(16).is_err());
    }
}
