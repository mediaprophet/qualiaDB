//! Canonical MIDI 1.0 message model + a zero-alloc streaming parser with
//! **running status**.
//!
//! [`MidiMessage`] is the `Copy` enum every other lane reconciles to. It covers
//! the channel-voice messages, system-common, and system-real-time messages.
//! SysEx payloads are variable-length and are *not* carried in this `Copy` enum;
//! the streaming parser consumes SysEx bytes (emitting nothing) so a stream that
//! interleaves SysEx still yields the surrounding channel messages correctly.
//! Use [`crate::midi::message::sysex`] to frame/unframe SysEx over buffers.
//!
//! [`MessageParser`] is a byte-at-a-time state machine. It carries running
//! status (a data byte with no preceding status re-uses the last channel-voice
//! status), decodes system-real-time bytes that are interleaved mid-message
//! without disturbing running status, and never allocates.

use crate::types::AudioError;

use super::channel_pressure::ChannelPressure;
use super::control_change::ControlChange;
use super::note::{NoteOff, NoteOn};
use super::pitch_bend::PitchBend;
use super::poly_pressure::PolyPressure;
use super::program_change::ProgramChange;

/// The canonical MIDI 1.0 message model (excluding SysEx payload bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiMessage {
    // ---- Channel voice ----
    /// Note Off (`0x8n`).
    NoteOff(NoteOff),
    /// Note On (`0x9n`). Velocity 0 is an implied Note Off.
    NoteOn(NoteOn),
    /// Polyphonic Key Pressure (`0xAn`).
    PolyPressure(PolyPressure),
    /// Control Change (`0xBn`) — includes channel-mode controllers 120..=127.
    ControlChange(ControlChange),
    /// Program Change (`0xCn`).
    ProgramChange(ProgramChange),
    /// Channel Pressure (`0xDn`).
    ChannelPressure(ChannelPressure),
    /// Pitch Bend (`0xEn`), 14-bit.
    PitchBend(PitchBend),

    // ---- System common ----
    /// MTC Quarter Frame (`0xF1`); payload is the 7-bit data byte.
    QuarterFrame(u8),
    /// Song Position Pointer (`0xF2`), 14-bit.
    SongPositionPointer(u16),
    /// Song Select (`0xF3`); payload is the song number.
    SongSelect(u8),
    /// Tune Request (`0xF6`).
    TuneRequest,

    // ---- System real-time ----
    /// Timing Clock (`0xF8`).
    TimingClock,
    /// Start (`0xFA`).
    Start,
    /// Continue (`0xFB`).
    Continue,
    /// Stop (`0xFC`).
    Stop,
    /// Active Sensing (`0xFE`).
    ActiveSensing,
    /// System Reset (`0xFF`).
    SystemReset,
}

/// A zero-alloc, byte-at-a-time MIDI 1.0 stream parser with running status.
#[derive(Debug, Clone, Copy)]
pub struct MessageParser {
    /// Current running status byte (0 = none held), for channel-voice + `0xF1..0xF3`.
    status: u8,
    /// Data bytes collected so far for the in-progress message.
    data: [u8; 2],
    /// Count of data bytes collected.
    data_len: u8,
    /// Data bytes expected for the current status.
    expected: u8,
    /// Whether we are inside a SysEx (`F0`..`F7`) region.
    in_sysex: bool,
}

impl Default for MessageParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Data-byte count for a channel-voice / system-common status byte, or `None`
/// if `status` is not one that collects data bytes here.
fn expected_data_bytes(status: u8) -> Option<u8> {
    match status & 0xF0 {
        0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => Some(2),
        0xC0 | 0xD0 => Some(1),
        0xF0 => match status {
            0xF1 => Some(1), // MTC quarter frame
            0xF2 => Some(2), // song position pointer
            0xF3 => Some(1), // song select
            _ => None,       // F0/F4/F5/F6/F7 handled directly, not via data collection
        },
        _ => None,
    }
}

/// Decode a single-byte system real-time status (`0xF8..=0xFF`).
fn decode_real_time(status: u8) -> Option<MidiMessage> {
    match status {
        0xF8 => Some(MidiMessage::TimingClock),
        0xFA => Some(MidiMessage::Start),
        0xFB => Some(MidiMessage::Continue),
        0xFC => Some(MidiMessage::Stop),
        0xFE => Some(MidiMessage::ActiveSensing),
        0xFF => Some(MidiMessage::SystemReset),
        _ => None, // 0xF9, 0xFD are undefined real-time
    }
}

impl MessageParser {
    /// A fresh parser with no running status held.
    pub const fn new() -> Self {
        Self { status: 0, data: [0; 2], data_len: 0, expected: 0, in_sysex: false }
    }

    /// Build the message for the current (complete) status + data bytes.
    fn build(&self) -> Option<MidiMessage> {
        let ch = self.status & 0x0F;
        let d0 = self.data[0];
        let d1 = self.data[1];
        match self.status & 0xF0 {
            0x80 => Some(MidiMessage::NoteOff(NoteOff { channel: ch, note: d0, velocity: d1 })),
            0x90 => Some(MidiMessage::NoteOn(NoteOn { channel: ch, note: d0, velocity: d1 })),
            0xA0 => Some(MidiMessage::PolyPressure(PolyPressure {
                channel: ch,
                note: d0,
                pressure: d1,
            })),
            0xB0 => Some(MidiMessage::ControlChange(ControlChange {
                channel: ch,
                controller: d0,
                value: d1,
            })),
            0xC0 => Some(MidiMessage::ProgramChange(ProgramChange { channel: ch, program: d0 })),
            0xD0 => {
                Some(MidiMessage::ChannelPressure(ChannelPressure { channel: ch, pressure: d0 }))
            }
            0xE0 => Some(MidiMessage::PitchBend(PitchBend {
                channel: ch,
                value: ((d1 as u16) << 7) | (d0 as u16),
            })),
            0xF0 => match self.status {
                0xF1 => Some(MidiMessage::QuarterFrame(d0)),
                0xF2 => Some(MidiMessage::SongPositionPointer(((d1 as u16) << 7) | (d0 as u16))),
                0xF3 => Some(MidiMessage::SongSelect(d0)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Feed one byte. Returns `Some(message)` when a complete message has been
    /// decoded, otherwise `None`. Never allocates.
    pub fn push(&mut self, byte: u8) -> Option<MidiMessage> {
        if byte >= 0x80 {
            // ---- Status byte ----
            if byte >= 0xF8 {
                // System real-time: does NOT affect running status or an
                // in-progress message; decode and emit immediately.
                return decode_real_time(byte);
            }
            match byte {
                0xF0 => {
                    // SysEx start. Clears running status (system message).
                    self.in_sysex = true;
                    self.status = 0;
                    self.data_len = 0;
                    None
                }
                0xF7 => {
                    // End of SysEx. Running status remains cleared.
                    self.in_sysex = false;
                    self.data_len = 0;
                    None
                }
                0xF6 => {
                    // Tune Request: no data bytes, clears running status.
                    self.status = 0;
                    self.data_len = 0;
                    self.in_sysex = false;
                    Some(MidiMessage::TuneRequest)
                }
                _ => {
                    // Channel-voice status, or F1/F2/F3 system-common.
                    self.in_sysex = false;
                    match expected_data_bytes(byte) {
                        Some(exp) => {
                            self.status = byte;
                            self.expected = exp;
                            self.data_len = 0;
                            None
                        }
                        // Undefined (F4/F5) or otherwise: clear and ignore.
                        None => {
                            self.status = 0;
                            self.data_len = 0;
                            None
                        }
                    }
                }
            }
        } else {
            // ---- Data byte (< 0x80) ----
            if self.in_sysex {
                // Consumed as SysEx payload; not surfaced here.
                return None;
            }
            if self.status == 0 {
                // Stray data byte with no running status: ignore.
                return None;
            }
            self.data[self.data_len as usize] = byte;
            self.data_len += 1;
            if self.data_len >= self.expected {
                let msg = self.build();
                // Retain `status` for running status; reset data collection.
                self.data_len = 0;
                return msg;
            }
            None
        }
    }

    /// Reset the parser to its initial state (drop any running status / SysEx).
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Parse a full byte slice, invoking `on_msg` for each complete message.
/// Zero-alloc: state lives in a stack [`MessageParser`]; no buffers are grown.
pub fn parse_stream<F: FnMut(MidiMessage)>(bytes: &[u8], mut on_msg: F) {
    let mut parser = MessageParser::new();
    for &b in bytes {
        if let Some(m) = parser.push(b) {
            on_msg(m);
        }
    }
}

/// Convenience: parse into a caller-provided output buffer, returning the count
/// written. Errors [`AudioError::OutputBufferTooSmall`] if more messages are
/// produced than fit. Zero-alloc.
pub fn parse_into(bytes: &[u8], out: &mut [MidiMessage]) -> Result<usize, AudioError> {
    let mut n = 0usize;
    let mut overflow = false;
    parse_stream(bytes, |m| {
        if n < out.len() {
            out[n] = m;
            n += 1;
        } else {
            overflow = true;
        }
    });
    if overflow {
        return Err(AudioError::OutputBufferTooSmall);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_single_note_on() {
        let mut msgs = [MidiMessage::TuneRequest; 4];
        let n = parse_into(&[0x90, 0x3C, 0x64], &mut msgs).unwrap();
        assert_eq!(n, 1);
        match msgs[0] {
            MidiMessage::NoteOn(no) => {
                assert_eq!(no.channel, 0);
                assert_eq!(no.note, 60);
                assert_eq!(no.velocity, 100);
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
    }

    #[test]
    fn golden_running_status_two_note_ons() {
        // 0x90 0x3C 0x64 0x3E 0x64 -> TWO note-ons; second reuses status 0x90.
        let mut msgs = [MidiMessage::TuneRequest; 4];
        let n = parse_into(&[0x90, 0x3C, 0x64, 0x3E, 0x64], &mut msgs).unwrap();
        assert_eq!(n, 2);
        let notes: [u8; 2] = match (msgs[0], msgs[1]) {
            (MidiMessage::NoteOn(a), MidiMessage::NoteOn(b)) => {
                assert_eq!(a.channel, 0);
                assert_eq!(a.velocity, 100);
                assert_eq!(b.channel, 0);
                assert_eq!(b.velocity, 100);
                [a.note, b.note]
            }
            other => panic!("expected two NoteOns, got {other:?}"),
        };
        assert_eq!(notes, [60, 62]);
    }

    #[test]
    fn golden_pitch_bend_center_via_stream() {
        let mut msgs = [MidiMessage::TuneRequest; 2];
        let n = parse_into(&[0xE0, 0x00, 0x40], &mut msgs).unwrap();
        assert_eq!(n, 1);
        match msgs[0] {
            MidiMessage::PitchBend(pb) => assert_eq!(pb.value, 8192),
            other => panic!("expected PitchBend, got {other:?}"),
        }
    }

    #[test]
    fn realtime_interleaved_midmessage() {
        // Timing clock (0xF8) arrives between the data bytes of a note-on; it
        // must be emitted without disturbing the note-on being assembled.
        let mut got = [MidiMessage::TuneRequest; 4];
        let mut i = 0;
        parse_stream(&[0x90, 0x3C, 0xF8, 0x64], |m| {
            got[i] = m;
            i += 1;
        });
        assert_eq!(i, 2);
        assert_eq!(got[0], MidiMessage::TimingClock);
        match got[1] {
            MidiMessage::NoteOn(no) => {
                assert_eq!(no.note, 60);
                assert_eq!(no.velocity, 100);
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
    }

    #[test]
    fn sysex_is_skipped_but_surrounding_survives() {
        // NoteOn, then a SysEx blob, then a program change (new status).
        let stream = [0x90, 0x3C, 0x64, 0xF0, 0x43, 0x12, 0xF7, 0xC0, 0x28];
        let mut msgs = [MidiMessage::TuneRequest; 4];
        let n = parse_into(&stream, &mut msgs).unwrap();
        assert_eq!(n, 2);
        assert!(matches!(msgs[0], MidiMessage::NoteOn(_)));
        match msgs[1] {
            MidiMessage::ProgramChange(pc) => assert_eq!(pc.program, 40),
            other => panic!("expected ProgramChange, got {other:?}"),
        }
    }

    #[test]
    fn running_status_program_change_and_song_position() {
        // Program change with running status (two programs), then song position.
        let mut msgs = [MidiMessage::TuneRequest; 4];
        let n = parse_into(&[0xC0, 0x01, 0x02, 0xF2, 0x00, 0x40], &mut msgs).unwrap();
        assert_eq!(n, 3);
        assert_eq!(msgs[0], MidiMessage::ProgramChange(ProgramChange { channel: 0, program: 1 }));
        assert_eq!(msgs[1], MidiMessage::ProgramChange(ProgramChange { channel: 0, program: 2 }));
        assert_eq!(msgs[2], MidiMessage::SongPositionPointer(8192));
    }

    #[test]
    fn overflow_reported() {
        let mut msgs = [MidiMessage::TuneRequest; 1];
        assert_eq!(
            parse_into(&[0x90, 0x3C, 0x64, 0x3E, 0x64], &mut msgs),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
