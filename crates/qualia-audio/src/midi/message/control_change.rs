//! MIDI 1.0 Control Change (`0xBn`) channel-voice message.
//!
//! Note: controller numbers 120..=127 are *channel-mode* messages; those are
//! modelled separately in [`crate::midi::message::channel_mode`]. This type
//! carries any controller 0..=127 as a raw CC.

use crate::types::AudioError;

/// Control Change status nibble (`0xBn`).
pub const STATUS_CONTROL_CHANGE: u8 = 0xB0;

/// A MIDI 1.0 Control Change message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlChange {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Controller number, 0..=127.
    pub controller: u8,
    /// Controller value, 0..=127.
    pub value: u8,
}

impl ControlChange {
    /// Construct a validated Control Change.
    pub fn new(channel: u8, controller: u8, value: u8) -> Result<Self, AudioError> {
        if channel > 15 || controller > 127 || value > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, controller, value })
    }

    /// Serialize to `[0xBn, controller, value]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        [STATUS_CONTROL_CHANGE | (self.channel & 0x0F), self.controller, self.value]
    }

    /// Parse from a slice whose first byte is a Control Change status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_CONTROL_CHANGE {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self { channel: bytes[0] & 0x0F, controller: bytes[1], value: bytes[2] })
    }

    /// True if the controller number is in the channel-mode range (120..=127).
    #[inline]
    pub fn is_channel_mode(self) -> bool {
        self.controller >= 120
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cc_roundtrip() {
        let cc = ControlChange::new(1, 7, 100).unwrap(); // channel volume
        assert_eq!(cc.to_bytes(), [0xB1, 7, 100]);
        assert_eq!(ControlChange::parse(&cc.to_bytes()).unwrap(), cc);
    }

    #[test]
    fn channel_mode_range() {
        assert!(ControlChange::new(0, 123, 0).unwrap().is_channel_mode());
        assert!(!ControlChange::new(0, 64, 127).unwrap().is_channel_mode());
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(ControlChange::new(0, 128, 0), Err(AudioError::InvalidParameter));
        assert_eq!(ControlChange::parse(&[0x90, 7, 100]), Err(AudioError::UnsupportedFormat));
    }
}
