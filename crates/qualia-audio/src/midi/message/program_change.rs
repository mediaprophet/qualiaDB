//! MIDI 1.0 Program Change (`0xCn`) — a 2-byte message.

use crate::types::AudioError;

/// Program Change status nibble (`0xCn`).
pub const STATUS_PROGRAM_CHANGE: u8 = 0xC0;

/// A MIDI 1.0 Program Change message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramChange {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Program number, 0..=127.
    pub program: u8,
}

impl ProgramChange {
    /// Construct a validated Program Change.
    pub fn new(channel: u8, program: u8) -> Result<Self, AudioError> {
        if channel > 15 || program > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, program })
    }

    /// Serialize to the 2 bytes `[0xCn, program]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 2] {
        [STATUS_PROGRAM_CHANGE | (self.channel & 0x0F), self.program]
    }

    /// Parse from a slice whose first byte is a Program Change status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 2 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_PROGRAM_CHANGE {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self {
            channel: bytes[0] & 0x0F,
            program: bytes[1],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pc = ProgramChange::new(0, 40).unwrap(); // e.g. violin
        assert_eq!(pc.to_bytes(), [0xC0, 40]);
        assert_eq!(ProgramChange::parse(&pc.to_bytes()).unwrap(), pc);
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(
            ProgramChange::new(0, 128),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            ProgramChange::parse(&[0xD0, 40]),
            Err(AudioError::UnsupportedFormat)
        );
    }
}
