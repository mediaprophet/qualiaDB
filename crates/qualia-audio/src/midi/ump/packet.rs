//! MIDI 2.0 Universal MIDI Packet (UMP) container + the MIDI 2.0 channel-voice
//! message model.
//!
//! A UMP is 1..=4 big-endian 32-bit words. The top nibble of word 0 is the
//! Message Type (MT), which fixes the packet size; the next nibble is the Group
//! (0..=15). [`UmpPacket`] is a fixed 4-word `Copy` container tagged with the
//! number of valid words.

use crate::types::AudioError;

/// Utility messages (MT 0x0) — 32-bit.
pub const MT_UTILITY: u8 = 0x0;
/// System real-time / common (MT 0x1) — 32-bit.
pub const MT_SYSTEM: u8 = 0x1;
/// MIDI 1.0 channel voice (MT 0x2) — 32-bit.
pub const MT_MIDI1_CHANNEL_VOICE: u8 = 0x2;
/// Data / SysEx7 (MT 0x3) — 64-bit.
pub const MT_DATA_64: u8 = 0x3;
/// MIDI 2.0 channel voice (MT 0x4) — 64-bit.
pub const MT_MIDI2_CHANNEL_VOICE: u8 = 0x4;
/// Data / SysEx8 & Mixed Data Set (MT 0x5) — 128-bit.
pub const MT_DATA_128: u8 = 0x5;

/// Number of 32-bit words in a UMP for a given Message Type, per the UMP spec.
///
/// `0x0,0x1,0x2,0x6,0x7` → 1 word; `0x3,0x4,0x8,0x9,0xA` → 2 words;
/// `0xB,0xC` → 3 words; `0x5,0xD,0xE,0xF` → 4 words.
pub const fn packet_word_count(message_type: u8) -> usize {
    match message_type & 0x0F {
        0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
        0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
        0xB | 0xC => 3,
        _ => 4, // 0x5, 0xD, 0xE, 0xF
    }
}

/// A fixed-capacity Universal MIDI Packet (1..=4 valid 32-bit words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmpPacket {
    /// Packet words, big-endian per the spec; only `word_count` are valid.
    pub words: [u32; 4],
    /// Number of valid words (1..=4).
    pub word_count: u8,
}

impl UmpPacket {
    /// A 32-bit (1-word) packet.
    #[inline]
    pub const fn new32(word0: u32) -> Self {
        Self {
            words: [word0, 0, 0, 0],
            word_count: 1,
        }
    }

    /// A 64-bit (2-word) packet.
    #[inline]
    pub const fn new64(word0: u32, word1: u32) -> Self {
        Self {
            words: [word0, word1, 0, 0],
            word_count: 2,
        }
    }

    /// A 96-bit (3-word) packet.
    #[inline]
    pub const fn new96(word0: u32, word1: u32, word2: u32) -> Self {
        Self {
            words: [word0, word1, word2, 0],
            word_count: 3,
        }
    }

    /// A 128-bit (4-word) packet.
    #[inline]
    pub const fn new128(word0: u32, word1: u32, word2: u32, word3: u32) -> Self {
        Self {
            words: [word0, word1, word2, word3],
            word_count: 4,
        }
    }

    /// The Message Type (top nibble of word 0), 0x0..=0xF.
    #[inline]
    pub const fn message_type(self) -> u8 {
        (self.words[0] >> 28) as u8 & 0x0F
    }

    /// The Group (second nibble of word 0), 0..=15.
    #[inline]
    pub const fn group(self) -> u8 {
        ((self.words[0] >> 24) & 0x0F) as u8
    }

    /// Valid words as a slice.
    #[inline]
    pub fn as_words(&self) -> &[u32] {
        &self.words[..self.word_count as usize]
    }

    /// Write the valid words as big-endian bytes into `out`; returns bytes
    /// written (`word_count * 4`). Errors [`AudioError::OutputBufferTooSmall`].
    pub fn to_be_bytes(self, out: &mut [u8]) -> Result<usize, AudioError> {
        let n = self.word_count as usize * 4;
        if out.len() < n {
            return Err(AudioError::OutputBufferTooSmall);
        }
        for (i, w) in self.words[..self.word_count as usize].iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        Ok(n)
    }
}

/// A MIDI 2.0 channel-voice message (MT 0x4 payload), decoded into fields.
/// Values are at MIDI 2.0 resolution (16-bit velocity, 32-bit controllers/bend).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Midi2ChannelVoice {
    /// Note Off. `attribute_type`/`attribute_data` carry the optional attribute.
    NoteOff {
        /// Channel 0..=15.
        channel: u8,
        /// Note number 0..=127.
        note: u8,
        /// 16-bit release velocity.
        velocity: u16,
        /// Attribute type (0 = none).
        attribute_type: u8,
        /// 16-bit attribute data.
        attribute_data: u16,
    },
    /// Note On.
    NoteOn {
        /// Channel 0..=15.
        channel: u8,
        /// Note number 0..=127.
        note: u8,
        /// 16-bit attack velocity.
        velocity: u16,
        /// Attribute type (0 = none).
        attribute_type: u8,
        /// 16-bit attribute data.
        attribute_data: u16,
    },
    /// Polyphonic Key Pressure.
    PolyPressure {
        /// Channel 0..=15.
        channel: u8,
        /// Note number 0..=127.
        note: u8,
        /// 32-bit pressure.
        pressure: u32,
    },
    /// Control Change.
    ControlChange {
        /// Channel 0..=15.
        channel: u8,
        /// Controller index 0..=127.
        index: u8,
        /// 32-bit controller value.
        value: u32,
    },
    /// Program Change (with optional bank).
    ProgramChange {
        /// Channel 0..=15.
        channel: u8,
        /// Program number 0..=127.
        program: u8,
        /// Whether the bank fields are valid.
        bank_valid: bool,
        /// Bank MSB 0..=127.
        bank_msb: u8,
        /// Bank LSB 0..=127.
        bank_lsb: u8,
    },
    /// Channel Pressure.
    ChannelPressure {
        /// Channel 0..=15.
        channel: u8,
        /// 32-bit pressure.
        pressure: u32,
    },
    /// Pitch Bend.
    PitchBend {
        /// Channel 0..=15.
        channel: u8,
        /// 32-bit bend value (0x8000_0000 = center).
        value: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_counts() {
        assert_eq!(packet_word_count(MT_MIDI1_CHANNEL_VOICE), 1);
        assert_eq!(packet_word_count(MT_MIDI2_CHANNEL_VOICE), 2);
        assert_eq!(packet_word_count(MT_DATA_128), 4);
        assert_eq!(packet_word_count(0xB), 3);
    }

    #[test]
    fn mt_group_fields() {
        let p = UmpPacket::new32(0x2A90_3C64); // MT=2, group=0xA
        assert_eq!(p.message_type(), 0x2);
        assert_eq!(p.group(), 0xA);
        assert_eq!(p.word_count, 1);
    }

    #[test]
    fn be_bytes() {
        let p = UmpPacket::new32(0x2090_3C64);
        let mut buf = [0u8; 4];
        assert_eq!(p.to_be_bytes(&mut buf).unwrap(), 4);
        assert_eq!(buf, [0x20, 0x90, 0x3C, 0x64]);
        let mut small = [0u8; 3];
        assert_eq!(
            p.to_be_bytes(&mut small),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
