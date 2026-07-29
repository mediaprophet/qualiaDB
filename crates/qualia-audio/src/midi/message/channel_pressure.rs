//! MIDI 1.0 Channel Pressure / Aftertouch (`0xDn`) — a 2-byte message.

use crate::types::AudioError;

/// Channel Pressure status nibble (`0xDn`).
pub const STATUS_CHANNEL_PRESSURE: u8 = 0xD0;

/// A MIDI 1.0 Channel Pressure (channel aftertouch) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelPressure {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Pressure amount, 0..=127.
    pub pressure: u8,
}

impl ChannelPressure {
    /// Construct a validated Channel Pressure.
    pub fn new(channel: u8, pressure: u8) -> Result<Self, AudioError> {
        if channel > 15 || pressure > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, pressure })
    }

    /// Serialize to the 2 bytes `[0xDn, pressure]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 2] {
        [
            STATUS_CHANNEL_PRESSURE | (self.channel & 0x0F),
            self.pressure,
        ]
    }

    /// Parse from a slice whose first byte is a Channel Pressure status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 2 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_CHANNEL_PRESSURE {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self {
            channel: bytes[0] & 0x0F,
            pressure: bytes[1],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let cp = ChannelPressure::new(2, 90).unwrap();
        assert_eq!(cp.to_bytes(), [0xD2, 90]);
        assert_eq!(ChannelPressure::parse(&cp.to_bytes()).unwrap(), cp);
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(
            ChannelPressure::new(0, 128),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            ChannelPressure::parse(&[0xE0, 10]),
            Err(AudioError::UnsupportedFormat)
        );
        assert_eq!(
            ChannelPressure::parse(&[0xD0]),
            Err(AudioError::MalformedAudio)
        );
    }
}
