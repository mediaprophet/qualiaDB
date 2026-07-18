//! MIDI 1.0 Pitch Bend (`0xEn`) channel-voice message — 14-bit value.
//!
//! The 14-bit value is transmitted as two 7-bit bytes, LSB first:
//! `[0xEn, lsb, msb]` where `value = (msb << 7) | lsb`. Center (no bend) is
//! `8192` (`lsb = 0x00, msb = 0x40`).

use crate::types::AudioError;

/// Pitch Bend status nibble (`0xEn`).
pub const STATUS_PITCH_BEND: u8 = 0xE0;
/// 14-bit center value (no bend).
pub const CENTER: u16 = 8192;
/// Maximum 14-bit value.
pub const MAX: u16 = 0x3FFF;

/// A MIDI 1.0 Pitch Bend message. `value` is the raw 14-bit unsigned value
/// (0..=16383); 8192 is center.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PitchBend {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// 14-bit bend value, 0..=16383. 8192 = center.
    pub value: u16,
}

impl PitchBend {
    /// Construct from a raw 14-bit value (0..=16383).
    pub fn new(channel: u8, value: u16) -> Result<Self, AudioError> {
        if channel > 15 || value > MAX {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, value })
    }

    /// Construct from a signed offset around center (-8192..=8191).
    pub fn from_signed(channel: u8, signed: i16) -> Result<Self, AudioError> {
        let v = signed as i32 + CENTER as i32;
        if !(0..=MAX as i32).contains(&v) {
            return Err(AudioError::InvalidParameter);
        }
        Self::new(channel, v as u16)
    }

    /// Value as a signed offset around center (-8192..=8191).
    #[inline]
    pub fn signed(self) -> i16 {
        self.value as i16 - CENTER as i16
    }

    /// Serialize to `[0xEn, lsb, msb]` (LSB first).
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        [
            STATUS_PITCH_BEND | (self.channel & 0x0F),
            (self.value & 0x7F) as u8,
            ((self.value >> 7) & 0x7F) as u8,
        ]
    }

    /// Parse from a slice whose first byte is a Pitch Bend status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_PITCH_BEND {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        let value = ((bytes[2] as u16) << 7) | (bytes[1] as u16);
        Ok(Self { channel: bytes[0] & 0x0F, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_center() {
        // Golden vector: 0xE0 0x00 0x40 -> 8192 (center).
        let pb = PitchBend::parse(&[0xE0, 0x00, 0x40]).unwrap();
        assert_eq!(pb.channel, 0);
        assert_eq!(pb.value, 8192);
        assert_eq!(pb.signed(), 0);
    }

    #[test]
    fn roundtrip_and_extremes() {
        let center = PitchBend::new(0, CENTER).unwrap();
        assert_eq!(center.to_bytes(), [0xE0, 0x00, 0x40]);

        let max = PitchBend::new(3, MAX).unwrap();
        assert_eq!(max.to_bytes(), [0xE3, 0x7F, 0x7F]);
        assert_eq!(PitchBend::parse(&max.to_bytes()).unwrap(), max);

        let min = PitchBend::new(0, 0).unwrap();
        assert_eq!(min.to_bytes(), [0xE0, 0x00, 0x00]);
        assert_eq!(min.signed(), -8192);
    }

    #[test]
    fn signed_roundtrip() {
        let pb = PitchBend::from_signed(5, -2048).unwrap();
        assert_eq!(pb.value, 8192 - 2048);
        assert_eq!(pb.signed(), -2048);
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(PitchBend::new(0, 16384), Err(AudioError::InvalidParameter));
        assert_eq!(PitchBend::parse(&[0x90, 0, 0x40]), Err(AudioError::UnsupportedFormat));
    }
}
