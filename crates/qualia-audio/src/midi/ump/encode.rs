//! Encode MIDI messages into Universal MIDI Packets, and translate MIDI 1.0
//! channel-voice → MIDI 2.0 channel-voice (with default resolution up-scaling).

use crate::midi::message::MidiMessage;
use crate::types::AudioError;

use super::packet::{Midi2ChannelVoice, UmpPacket};
use super::scale::scale_up;

/// Build the first word of a channel-voice UMP (MT nibble + group + status).
#[inline]
const fn word0_head(message_type: u8, group: u8, status_byte: u8) -> u32 {
    ((message_type as u32 & 0x0F) << 28)
        | ((group as u32 & 0x0F) << 24)
        | ((status_byte as u32) << 16)
}

/// Encode a MIDI 1.0 **channel-voice** message into a 32-bit UMP (Message Type
/// 0x2) on `group`. Returns [`AudioError::UnsupportedFormat`] for non
/// channel-voice messages (system real-time/common are not MT 0x2).
pub fn encode_midi1_channel_voice(group: u8, msg: MidiMessage) -> Result<UmpPacket, AudioError> {
    if group > 15 {
        return Err(AudioError::InvalidParameter);
    }
    let (status, d1, d2) = match msg {
        MidiMessage::NoteOff(n) => (0x80 | n.channel, n.note, n.velocity),
        MidiMessage::NoteOn(n) => (0x90 | n.channel, n.note, n.velocity),
        MidiMessage::PolyPressure(p) => (0xA0 | p.channel, p.note, p.pressure),
        MidiMessage::ControlChange(c) => (0xB0 | c.channel, c.controller, c.value),
        MidiMessage::ProgramChange(p) => (0xC0 | p.channel, p.program, 0),
        MidiMessage::ChannelPressure(p) => (0xD0 | p.channel, p.pressure, 0),
        MidiMessage::PitchBend(b) => (
            0xE0 | b.channel,
            (b.value & 0x7F) as u8,
            ((b.value >> 7) & 0x7F) as u8,
        ),
        _ => return Err(AudioError::UnsupportedFormat),
    };
    let word = word0_head(0x2, group, status) | ((d1 as u32) << 8) | (d2 as u32);
    Ok(UmpPacket::new32(word))
}

/// Encode a MIDI 2.0 channel-voice message into a 64-bit UMP (Message Type 0x4)
/// on `group`.
pub fn encode_midi2_channel_voice(
    group: u8,
    cv: Midi2ChannelVoice,
) -> Result<UmpPacket, AudioError> {
    if group > 15 {
        return Err(AudioError::InvalidParameter);
    }
    let (word0, word1) = match cv {
        Midi2ChannelVoice::NoteOff {
            channel,
            note,
            velocity,
            attribute_type,
            attribute_data,
        } => (
            word0_head(0x4, group, 0x80 | channel) | ((note as u32) << 8) | attribute_type as u32,
            ((velocity as u32) << 16) | attribute_data as u32,
        ),
        Midi2ChannelVoice::NoteOn {
            channel,
            note,
            velocity,
            attribute_type,
            attribute_data,
        } => (
            word0_head(0x4, group, 0x90 | channel) | ((note as u32) << 8) | attribute_type as u32,
            ((velocity as u32) << 16) | attribute_data as u32,
        ),
        Midi2ChannelVoice::PolyPressure {
            channel,
            note,
            pressure,
        } => (
            word0_head(0x4, group, 0xA0 | channel) | ((note as u32) << 8),
            pressure,
        ),
        Midi2ChannelVoice::ControlChange {
            channel,
            index,
            value,
        } => (
            word0_head(0x4, group, 0xB0 | channel) | ((index as u32) << 8),
            value,
        ),
        Midi2ChannelVoice::ProgramChange {
            channel,
            program,
            bank_valid,
            bank_msb,
            bank_lsb,
        } => (
            word0_head(0x4, group, 0xC0 | channel) | (bank_valid as u32),
            ((program as u32) << 24) | ((bank_msb as u32) << 8) | (bank_lsb as u32),
        ),
        Midi2ChannelVoice::ChannelPressure { channel, pressure } => {
            (word0_head(0x4, group, 0xD0 | channel), pressure)
        }
        Midi2ChannelVoice::PitchBend { channel, value } => {
            (word0_head(0x4, group, 0xE0 | channel), value)
        }
    };
    Ok(UmpPacket::new64(word0, word1))
}

/// Translate a MIDI 1.0 channel-voice message to the MIDI 2.0 channel-voice
/// model, applying default resolution up-scaling (7-bit→16-bit velocity,
/// 7-bit→32-bit controllers/pressure, 14-bit→32-bit pitch bend).
///
/// Per the MIDI 2.0 default translation, a MIDI 1.0 Note On with velocity 0 is
/// translated to a MIDI 2.0 Note Off. Returns [`AudioError::UnsupportedFormat`]
/// for non channel-voice input.
pub fn translate_midi1_to_midi2(msg: MidiMessage) -> Result<Midi2ChannelVoice, AudioError> {
    let cv = match msg {
        MidiMessage::NoteOff(n) => Midi2ChannelVoice::NoteOff {
            channel: n.channel,
            note: n.note,
            velocity: scale_up(n.velocity as u32, 7, 16) as u16,
            attribute_type: 0,
            attribute_data: 0,
        },
        MidiMessage::NoteOn(n) if n.velocity == 0 => Midi2ChannelVoice::NoteOff {
            channel: n.channel,
            note: n.note,
            velocity: 0,
            attribute_type: 0,
            attribute_data: 0,
        },
        MidiMessage::NoteOn(n) => Midi2ChannelVoice::NoteOn {
            channel: n.channel,
            note: n.note,
            velocity: scale_up(n.velocity as u32, 7, 16) as u16,
            attribute_type: 0,
            attribute_data: 0,
        },
        MidiMessage::PolyPressure(p) => Midi2ChannelVoice::PolyPressure {
            channel: p.channel,
            note: p.note,
            pressure: scale_up(p.pressure as u32, 7, 32),
        },
        MidiMessage::ControlChange(c) => Midi2ChannelVoice::ControlChange {
            channel: c.channel,
            index: c.controller,
            value: scale_up(c.value as u32, 7, 32),
        },
        MidiMessage::ProgramChange(p) => Midi2ChannelVoice::ProgramChange {
            channel: p.channel,
            program: p.program,
            bank_valid: false,
            bank_msb: 0,
            bank_lsb: 0,
        },
        MidiMessage::ChannelPressure(p) => Midi2ChannelVoice::ChannelPressure {
            channel: p.channel,
            pressure: scale_up(p.pressure as u32, 7, 32),
        },
        MidiMessage::PitchBend(b) => Midi2ChannelVoice::PitchBend {
            channel: b.channel,
            value: scale_up(b.value as u32, 14, 32),
        },
        _ => return Err(AudioError::UnsupportedFormat),
    };
    Ok(cv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::message::note::NoteOn;

    #[test]
    fn golden_midi1_note_on_to_ump32() {
        // NoteOn ch0 note60 vel100 on group0 -> 0x2090_3C64.
        let msg = MidiMessage::NoteOn(NoteOn::new(0, 60, 100).unwrap());
        let p = encode_midi1_channel_voice(0, msg).unwrap();
        assert_eq!(p.word_count, 1);
        assert_eq!(p.words[0], 0x2090_3C64);
    }

    #[test]
    fn golden_velocity_scale_7_to_16() {
        let msg = MidiMessage::NoteOn(NoteOn::new(0, 60, 100).unwrap());
        match translate_midi1_to_midi2(msg).unwrap() {
            Midi2ChannelVoice::NoteOn {
                velocity,
                note,
                channel,
                ..
            } => {
                assert_eq!(channel, 0);
                assert_eq!(note, 60);
                assert_eq!(velocity, 0xC924);
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
    }

    #[test]
    fn note_on_velocity_zero_becomes_note_off() {
        let msg = MidiMessage::NoteOn(NoteOn::new(3, 40, 0).unwrap());
        assert!(matches!(
            translate_midi1_to_midi2(msg).unwrap(),
            Midi2ChannelVoice::NoteOff {
                channel: 3,
                note: 40,
                ..
            }
        ));
    }

    #[test]
    fn midi2_note_on_encodes_two_words() {
        let cv = Midi2ChannelVoice::NoteOn {
            channel: 0,
            note: 60,
            velocity: 0xC924,
            attribute_type: 0,
            attribute_data: 0,
        };
        let p = encode_midi2_channel_voice(0, cv).unwrap();
        assert_eq!(p.word_count, 2);
        assert_eq!(p.words[0], 0x4090_3C00);
        assert_eq!(p.words[1], 0xC924_0000);
    }
}
