//! MIDI 1.0 Note On / Note Off channel-voice messages.
//!
//! Zero-heap: build returns a fixed `[u8; 3]`; parse reads a caller slice and
//! returns a small `Copy` struct. Channel is 0..=15; note/velocity 0..=127.

use crate::types::AudioError;

/// Note Off status nibble (`0x8n`).
pub const STATUS_NOTE_OFF: u8 = 0x80;
/// Note On status nibble (`0x9n`).
pub const STATUS_NOTE_ON: u8 = 0x90;

/// A MIDI 1.0 Note On message. A Note On with `velocity == 0` is, by
/// convention, an implied Note Off (see [`NoteOn::is_effectively_note_off`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteOn {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Note number, 0..=127 (60 = middle C).
    pub note: u8,
    /// Attack velocity, 0..=127.
    pub velocity: u8,
}

/// A MIDI 1.0 Note Off message. `velocity` is release velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteOff {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Note number, 0..=127.
    pub note: u8,
    /// Release velocity, 0..=127.
    pub velocity: u8,
}

impl NoteOn {
    /// Construct a validated Note On. Returns [`AudioError::InvalidParameter`]
    /// if any field is out of range.
    pub fn new(channel: u8, note: u8, velocity: u8) -> Result<Self, AudioError> {
        if channel > 15 || note > 127 || velocity > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, note, velocity })
    }

    /// Serialize to the 3 status/data bytes `[0x9n, note, velocity]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        [STATUS_NOTE_ON | (self.channel & 0x0F), self.note, self.velocity]
    }

    /// Parse from a byte slice whose first byte is a Note On status.
    /// Returns [`AudioError::UnsupportedFormat`] if the status nibble is not
    /// `0x9n`, or [`AudioError::MalformedAudio`] if the slice is too short or a
    /// data byte has its high bit set.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_NOTE_ON {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self { channel: bytes[0] & 0x0F, note: bytes[1], velocity: bytes[2] })
    }

    /// True if this is a running-status "note off" (velocity 0).
    #[inline]
    pub fn is_effectively_note_off(self) -> bool {
        self.velocity == 0
    }
}

impl NoteOff {
    /// Construct a validated Note Off.
    pub fn new(channel: u8, note: u8, velocity: u8) -> Result<Self, AudioError> {
        if channel > 15 || note > 127 || velocity > 127 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { channel, note, velocity })
    }

    /// Serialize to `[0x8n, note, velocity]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        [STATUS_NOTE_OFF | (self.channel & 0x0F), self.note, self.velocity]
    }

    /// Parse from a byte slice whose first byte is a Note Off status.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_NOTE_OFF {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self { channel: bytes[0] & 0x0F, note: bytes[1], velocity: bytes[2] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_note_on_parse() {
        // Canonical golden vector: 0x90 0x3C 0x64 -> NoteOn(ch0, note60, vel100).
        let n = NoteOn::parse(&[0x90, 0x3C, 0x64]).unwrap();
        assert_eq!(n.channel, 0);
        assert_eq!(n.note, 60);
        assert_eq!(n.velocity, 100);
    }

    #[test]
    fn note_on_roundtrip() {
        let n = NoteOn::new(9, 64, 127).unwrap();
        assert_eq!(n.to_bytes(), [0x99, 64, 127]);
        assert_eq!(NoteOn::parse(&n.to_bytes()).unwrap(), n);
    }

    #[test]
    fn note_off_roundtrip() {
        let n = NoteOff::new(0, 60, 0).unwrap();
        assert_eq!(n.to_bytes(), [0x80, 60, 0]);
        assert_eq!(NoteOff::parse(&n.to_bytes()).unwrap(), n);
    }

    #[test]
    fn note_on_zero_velocity_is_note_off() {
        assert!(NoteOn::new(0, 60, 0).unwrap().is_effectively_note_off());
        assert!(!NoteOn::new(0, 60, 1).unwrap().is_effectively_note_off());
    }

    #[test]
    fn out_of_range_rejected() {
        assert_eq!(NoteOn::new(16, 0, 0), Err(AudioError::InvalidParameter));
        assert_eq!(NoteOn::new(0, 128, 0), Err(AudioError::InvalidParameter));
        assert_eq!(NoteOn::parse(&[0x90, 0x3C]), Err(AudioError::MalformedAudio));
        assert_eq!(NoteOn::parse(&[0x80, 0x3C, 0x64]), Err(AudioError::UnsupportedFormat));
    }
}
