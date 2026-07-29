//! MIDI 1.0 Polyphonic Key Pressure / Aftertouch (`0xAn`) — a 3-byte message.

use crate::types::AudioError;

/// Poly Key Pressure status nibble (`0xAn`).
pub const STATUS_POLY_PRESSURE: u8 = 0xA0;

/// A MIDI 1.0 Polyphonic Key Pressure message (per-note aftertouch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyPressure {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Note number, 0..=127.
    pub note: u8,
    /// Pressure amount, 0..=127.
    pub pressure: u8,
}

impl PolyPressure {
    /// Construct a validated Poly Key Pressure.
    pub fn new(channel: u8, note: u8, pressure: u8) -> Result<Self, AudioError> {
        if channel > 15 || note > 127 || pressure > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            channel,
            note,
            pressure,
        })
    }

    /// Serialize to `[0xAn, note, pressure]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        [
            STATUS_POLY_PRESSURE | (self.channel & 0x0F),
            self.note,
            self.pressure,
        ]
    }

    /// Parse from a slice whose first byte is a Poly Key Pressure status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_POLY_PRESSURE {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self {
            channel: bytes[0] & 0x0F,
            note: bytes[1],
            pressure: bytes[2],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pp = PolyPressure::new(0, 60, 80).unwrap();
        assert_eq!(pp.to_bytes(), [0xA0, 60, 80]);
        assert_eq!(PolyPressure::parse(&pp.to_bytes()).unwrap(), pp);
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(
            PolyPressure::new(0, 200, 0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            PolyPressure::parse(&[0xB0, 60, 80]),
            Err(AudioError::UnsupportedFormat)
        );
    }
}
