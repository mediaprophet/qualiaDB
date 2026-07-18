//! Decode Universal MIDI Packets back into messages, and translate MIDI 2.0
//! channel-voice → MIDI 1.0 channel-voice (with default resolution down-scaling).

use crate::midi::message::channel_pressure::ChannelPressure;
use crate::midi::message::control_change::ControlChange;
use crate::midi::message::note::{NoteOff, NoteOn};
use crate::midi::message::pitch_bend::PitchBend;
use crate::midi::message::poly_pressure::PolyPressure;
use crate::midi::message::program_change::ProgramChange;
use crate::midi::message::MidiMessage;
use crate::types::AudioError;

use super::packet::{
    Midi2ChannelVoice, UmpPacket, MT_MIDI1_CHANNEL_VOICE, MT_MIDI2_CHANNEL_VOICE,
};
use super::scale::scale_down;

/// The decoded content of a UMP, spanning the message types this lane models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmpDecoded {
    /// MIDI 1.0 channel voice (MT 0x2).
    Midi1(MidiMessage),
    /// MIDI 2.0 channel voice (MT 0x4).
    Midi2(Midi2ChannelVoice),
    /// A message type this decoder does not expand (utility, system, data/SysEx).
    /// Carries the raw Message Type nibble.
    Unhandled(u8),
}

/// Decode a MIDI 1.0 channel-voice UMP (MT 0x2) into a [`MidiMessage`].
/// Errors [`AudioError::UnsupportedFormat`] if the packet is not MT 0x2.
pub fn decode_midi1_channel_voice(packet: UmpPacket) -> Result<MidiMessage, AudioError> {
    if packet.message_type() != MT_MIDI1_CHANNEL_VOICE {
        return Err(AudioError::UnsupportedFormat);
    }
    let w = packet.words[0];
    let status = ((w >> 16) & 0xFF) as u8;
    let channel = status & 0x0F;
    let d1 = ((w >> 8) & 0x7F) as u8;
    let d2 = (w & 0x7F) as u8;
    let msg = match status & 0xF0 {
        0x80 => MidiMessage::NoteOff(NoteOff { channel, note: d1, velocity: d2 }),
        0x90 => MidiMessage::NoteOn(NoteOn { channel, note: d1, velocity: d2 }),
        0xA0 => MidiMessage::PolyPressure(PolyPressure { channel, note: d1, pressure: d2 }),
        0xB0 => MidiMessage::ControlChange(ControlChange { channel, controller: d1, value: d2 }),
        0xC0 => MidiMessage::ProgramChange(ProgramChange { channel, program: d1 }),
        0xD0 => MidiMessage::ChannelPressure(ChannelPressure { channel, pressure: d1 }),
        0xE0 => {
            MidiMessage::PitchBend(PitchBend { channel, value: ((d2 as u16) << 7) | (d1 as u16) })
        }
        _ => return Err(AudioError::UnsupportedFormat),
    };
    Ok(msg)
}

/// Decode a MIDI 2.0 channel-voice UMP (MT 0x4) into a [`Midi2ChannelVoice`].
/// Errors [`AudioError::UnsupportedFormat`] if the packet is not MT 0x4, or
/// [`AudioError::MalformedAudio`] if it does not carry two words.
pub fn decode_midi2_channel_voice(packet: UmpPacket) -> Result<Midi2ChannelVoice, AudioError> {
    if packet.message_type() != MT_MIDI2_CHANNEL_VOICE {
        return Err(AudioError::UnsupportedFormat);
    }
    if packet.word_count < 2 {
        return Err(AudioError::MalformedAudio);
    }
    let w0 = packet.words[0];
    let w1 = packet.words[1];
    let status = ((w0 >> 16) & 0xFF) as u8;
    let channel = status & 0x0F;
    let note = ((w0 >> 8) & 0x7F) as u8;
    let index = ((w0 >> 8) & 0x7F) as u8;
    let cv = match status & 0xF0 {
        0x80 => Midi2ChannelVoice::NoteOff {
            channel,
            note,
            velocity: (w1 >> 16) as u16,
            attribute_type: (w0 & 0xFF) as u8,
            attribute_data: (w1 & 0xFFFF) as u16,
        },
        0x90 => Midi2ChannelVoice::NoteOn {
            channel,
            note,
            velocity: (w1 >> 16) as u16,
            attribute_type: (w0 & 0xFF) as u8,
            attribute_data: (w1 & 0xFFFF) as u16,
        },
        0xA0 => Midi2ChannelVoice::PolyPressure { channel, note, pressure: w1 },
        0xB0 => Midi2ChannelVoice::ControlChange { channel, index, value: w1 },
        0xC0 => Midi2ChannelVoice::ProgramChange {
            channel,
            program: (w1 >> 24) as u8,
            bank_valid: (w0 & 0x01) != 0,
            bank_msb: ((w1 >> 8) & 0x7F) as u8,
            bank_lsb: (w1 & 0x7F) as u8,
        },
        0xD0 => Midi2ChannelVoice::ChannelPressure { channel, pressure: w1 },
        0xE0 => Midi2ChannelVoice::PitchBend { channel, value: w1 },
        _ => return Err(AudioError::UnsupportedFormat),
    };
    Ok(cv)
}

/// Decode any modelled UMP, dispatching on Message Type.
pub fn decode(packet: UmpPacket) -> Result<UmpDecoded, AudioError> {
    match packet.message_type() {
        MT_MIDI1_CHANNEL_VOICE => Ok(UmpDecoded::Midi1(decode_midi1_channel_voice(packet)?)),
        MT_MIDI2_CHANNEL_VOICE => Ok(UmpDecoded::Midi2(decode_midi2_channel_voice(packet)?)),
        mt => Ok(UmpDecoded::Unhandled(mt)),
    }
}

/// Translate a MIDI 2.0 channel-voice message down to the MIDI 1.0 model,
/// applying default resolution down-scaling. Per the MIDI 2.0 default
/// translation, a MIDI 2.0 Note On whose velocity scales down to 0 is clamped to
/// velocity 1 so it is not misread as a MIDI 1.0 implicit note-off.
pub fn translate_midi2_to_midi1(cv: Midi2ChannelVoice) -> Result<MidiMessage, AudioError> {
    let msg = match cv {
        Midi2ChannelVoice::NoteOff { channel, note, velocity, .. } => MidiMessage::NoteOff(
            NoteOff { channel, note, velocity: scale_down(velocity as u32, 16, 7) as u8 },
        ),
        Midi2ChannelVoice::NoteOn { channel, note, velocity, .. } => {
            let mut v7 = scale_down(velocity as u32, 16, 7) as u8;
            if v7 == 0 {
                v7 = 1; // avoid implicit note-off on down-translation
            }
            MidiMessage::NoteOn(NoteOn { channel, note, velocity: v7 })
        }
        Midi2ChannelVoice::PolyPressure { channel, note, pressure } => MidiMessage::PolyPressure(
            PolyPressure { channel, note, pressure: scale_down(pressure, 32, 7) as u8 },
        ),
        Midi2ChannelVoice::ControlChange { channel, index, value } => MidiMessage::ControlChange(
            ControlChange { channel, controller: index, value: scale_down(value, 32, 7) as u8 },
        ),
        Midi2ChannelVoice::ProgramChange { channel, program, .. } => {
            MidiMessage::ProgramChange(ProgramChange { channel, program })
        }
        Midi2ChannelVoice::ChannelPressure { channel, pressure } => MidiMessage::ChannelPressure(
            ChannelPressure { channel, pressure: scale_down(pressure, 32, 7) as u8 },
        ),
        Midi2ChannelVoice::PitchBend { channel, value } => {
            MidiMessage::PitchBend(PitchBend { channel, value: scale_down(value, 32, 14) as u16 })
        }
    };
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::message::note::NoteOn as MsgNoteOn;
    use crate::midi::ump::encode::{
        encode_midi1_channel_voice, encode_midi2_channel_voice, translate_midi1_to_midi2,
    };

    #[test]
    fn golden_ump32_note_on_roundtrip() {
        // Encode a MIDI-1.0 note-on to a 32-bit UMP word, decode back the same.
        let orig = MidiMessage::NoteOn(MsgNoteOn::new(0, 60, 100).unwrap());
        let packet = encode_midi1_channel_voice(0, orig).unwrap();
        let back = decode_midi1_channel_voice(packet).unwrap();
        assert_eq!(back, orig);
        match back {
            MidiMessage::NoteOn(n) => {
                assert_eq!(n.note, 60);
                assert_eq!(n.velocity, 100);
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
    }

    #[test]
    fn golden_velocity_scale_roundtrip_via_translation() {
        // MIDI1 vel 100 -> MIDI2 0xC924 -> MIDI1 100 (through decode of a real packet).
        let orig = MidiMessage::NoteOn(MsgNoteOn::new(5, 72, 100).unwrap());
        let cv = translate_midi1_to_midi2(orig).unwrap();
        let packet = encode_midi2_channel_voice(0, cv).unwrap();
        let cv_back = decode_midi2_channel_voice(packet).unwrap();
        assert_eq!(cv_back, cv);
        match cv_back {
            Midi2ChannelVoice::NoteOn { velocity, .. } => assert_eq!(velocity, 0xC924),
            other => panic!("expected NoteOn, got {other:?}"),
        }
        let back = translate_midi2_to_midi1(cv_back).unwrap();
        assert_eq!(back, orig);
    }

    #[test]
    fn decode_dispatch() {
        let p = encode_midi1_channel_voice(0, MidiMessage::NoteOn(MsgNoteOn::new(0, 60, 100).unwrap()))
            .unwrap();
        assert!(matches!(decode(p).unwrap(), UmpDecoded::Midi1(MidiMessage::NoteOn(_))));
    }

    #[test]
    fn pitch_bend_14_to_32_roundtrip() {
        let orig = MidiMessage::PitchBend(
            crate::midi::message::pitch_bend::PitchBend::new(2, 8192).unwrap(),
        );
        let cv = translate_midi1_to_midi2(orig).unwrap();
        match cv {
            Midi2ChannelVoice::PitchBend { value, .. } => assert_eq!(value, 0x8000_0000),
            other => panic!("expected PitchBend, got {other:?}"),
        }
        assert_eq!(translate_midi2_to_midi1(cv).unwrap(), orig);
    }
}
