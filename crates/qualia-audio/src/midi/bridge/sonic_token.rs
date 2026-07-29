//! A packed `u64` **SonicToken** for MIDI — the qualia-audio-side bridge type.
//!
//! Lane AU-MIDI-BRIDGE. This mirrors the bit layout of core-db's
//! `net/sonic_token.rs` so a later cross-crate reconciliation can map this token
//! to that one field-for-field, but it is defined here independently (this crate
//! takes no dependency on qualia-core-db). It exists so a canonical
//! [`crate::midi::message::MidiMessage`] can be squeezed into a single 8-byte
//! `Copy` word for the AcousticPlane hot path and recovered losslessly.
//!
//! # Bit layout (mirrors core-db `SonicToken::pack`)
//! ```text
//! bits  0.. 8  delta_time      (u8)   — inter-event tick delta (0 here)
//! bits  8..12  event_type      (4b)   — 0 NoteOn, 1 NoteOff, 2 ControlChange, 3 Parametric
//! bits 12..16  channel         (4b)   — MIDI channel 0..=15
//! bits 16..24  note / data     (u8)   — note number, or CC controller number
//! bits 24..32  velocity/value  (u8)   — velocity, or CC value
//! bits 32..48  tensor_index    (u16)  — graph node hint (0 here)
//! bits 48..64  flags           (u16)  — provenance flags (0 here)
//! ```
//!
//! Only the three MIDI-mappable event types round-trip: `NoteOn`, `NoteOff`, and
//! `ControlChange`. `Parametric` has no MIDI-1.0 equivalent, so
//! [`sonic_token_to_midi`] returns `None` for it, and non-note/CC messages (pitch
//! bend, pressure, system) return `None` from [`midi_to_sonic_token`].
//!
//! Epistemic note: a `SonicToken` is a *transport encoding*, not a truth claim.
//! Whether the note it carries is a transcription proposal or an authoritative
//! authored event is a property of its source stream (see [`super::from_pitch_midi`]
//! vs [`super::to_note_events`]); the token itself carries no confidence.

use crate::midi::message::control_change::ControlChange;
use crate::midi::message::note::{NoteOff, NoteOn};
use crate::midi::message::MidiMessage;

/// Provenance magic byte mirrored from core-db (`'S'`), packed into `flags`.
pub const SONIC_MAGIC: u8 = 0x53;

/// The MIDI-relevant sonic event types (values match core-db `SonicEventType`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SonicEventType {
    /// Note On (`0x9n`, velocity > 0).
    NoteOn = 0,
    /// Note Off (`0x8n`, or `0x9n` velocity 0).
    NoteOff = 1,
    /// Control Change (`0xBn`).
    ControlChange = 2,
    /// Non-MIDI parametric pulse (no MIDI-1.0 equivalent).
    Parametric = 3,
}

impl SonicEventType {
    /// Decode the 4-bit event-type field.
    #[inline]
    fn from_bits(bits: u64) -> Self {
        match bits & 0x0F {
            1 => SonicEventType::NoteOff,
            2 => SonicEventType::ControlChange,
            3 => SonicEventType::Parametric,
            _ => SonicEventType::NoteOn,
        }
    }
}

/// Pack the fields into the mirrored `u64` layout.
///
/// `note` doubles as the CC controller number; `velocity` doubles as the CC
/// value. `tensor_index` and `flags` are 16-bit graph/provenance hints (0 for a
/// pure MIDI bridge token).
#[inline]
pub fn pack(
    delta_time: u8,
    event_type: SonicEventType,
    channel: u8,
    note: u8,
    velocity: u8,
    tensor_index: u16,
    flags: u16,
) -> u64 {
    let et = event_type as u64;
    (delta_time as u64)
        | ((et & 0x0F) << 8)
        | (((channel as u64) & 0x0F) << 12)
        | ((note as u64) << 16)
        | ((velocity as u64) << 24)
        | (((tensor_index as u64) & 0xFFFF) << 32)
        | (((flags as u64) & 0xFFFF) << 48)
}

/// The 4-bit event type carried by `token`.
#[inline]
pub fn event_type(token: u64) -> SonicEventType {
    SonicEventType::from_bits(token >> 8)
}

/// The 8-bit delta-time field.
#[inline]
pub fn delta_time(token: u64) -> u8 {
    token as u8
}

/// The 4-bit channel field (0..=15).
#[inline]
pub fn channel(token: u64) -> u8 {
    ((token >> 12) & 0x0F) as u8
}

/// The 8-bit note / CC-controller field.
#[inline]
pub fn note(token: u64) -> u8 {
    ((token >> 16) & 0xFF) as u8
}

/// The 8-bit velocity / CC-value field.
#[inline]
pub fn velocity(token: u64) -> u8 {
    ((token >> 24) & 0xFF) as u8
}

/// The 16-bit tensor-index (graph node) field.
#[inline]
pub fn tensor_index(token: u64) -> u16 {
    ((token >> 32) & 0xFFFF) as u16
}

/// The 16-bit flags field.
#[inline]
pub fn flags(token: u64) -> u16 {
    (token >> 48) as u16
}

/// Encode a [`MidiMessage`] into a packed sonic-token `u64`.
///
/// Returns `None` for messages with no sonic-token event type (everything except
/// note-on, note-off, and control change). A velocity-0 note-on is preserved as
/// a `NoteOn` (not rewritten to `NoteOff`) so the round-trip is byte-faithful to
/// the input message; interpret it as a note-off downstream as usual.
pub fn midi_to_sonic_token(msg: &MidiMessage) -> Option<u64> {
    match *msg {
        MidiMessage::NoteOn(NoteOn {
            channel: ch,
            note: n,
            velocity: v,
        }) => Some(pack(0, SonicEventType::NoteOn, ch, n, v, 0, 0)),
        MidiMessage::NoteOff(NoteOff {
            channel: ch,
            note: n,
            velocity: v,
        }) => Some(pack(0, SonicEventType::NoteOff, ch, n, v, 0, 0)),
        MidiMessage::ControlChange(ControlChange {
            channel: ch,
            controller,
            value,
        }) => Some(pack(
            0,
            SonicEventType::ControlChange,
            ch,
            controller,
            value,
            0,
            0,
        )),
        _ => None,
    }
}

/// Decode a packed sonic-token `u64` back into a [`MidiMessage`].
///
/// Returns `None` if the token's event type is [`SonicEventType::Parametric`]
/// (no MIDI equivalent) or if a decoded field is out of MIDI range (note or
/// value `> 127`). Channel is always in range (4-bit field).
pub fn sonic_token_to_midi(token: u64) -> Option<MidiMessage> {
    let ch = channel(token);
    let d0 = note(token);
    let d1 = velocity(token);
    match event_type(token) {
        SonicEventType::NoteOn => NoteOn::new(ch, d0, d1).ok().map(MidiMessage::NoteOn),
        SonicEventType::NoteOff => NoteOff::new(ch, d0, d1).ok().map(MidiMessage::NoteOff),
        SonicEventType::ControlChange => ControlChange::new(ch, d0, d1)
            .ok()
            .map(MidiMessage::ControlChange),
        SonicEventType::Parametric => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_note_on_round_trip() {
        // midi_to_sonic_token(NoteOn ch0 note60 vel100) → u64 → same NoteOn.
        let msg = MidiMessage::NoteOn(NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        let token = midi_to_sonic_token(&msg).expect("encodable");
        // Fields land in the mirrored positions.
        assert_eq!(event_type(token), SonicEventType::NoteOn);
        assert_eq!(channel(token), 0);
        assert_eq!(note(token), 60);
        assert_eq!(velocity(token), 100);
        // Full round-trip.
        assert_eq!(sonic_token_to_midi(token), Some(msg));
    }

    #[test]
    fn note_off_round_trip() {
        let msg = MidiMessage::NoteOff(NoteOff {
            channel: 9,
            note: 64,
            velocity: 40,
        });
        let token = midi_to_sonic_token(&msg).expect("encodable");
        assert_eq!(event_type(token), SonicEventType::NoteOff);
        assert_eq!(channel(token), 9);
        assert_eq!(sonic_token_to_midi(token), Some(msg));
    }

    #[test]
    fn control_change_round_trip() {
        // CC controller in the note field, value in the velocity field.
        let msg = MidiMessage::ControlChange(ControlChange {
            channel: 3,
            controller: 7,
            value: 127,
        });
        let token = midi_to_sonic_token(&msg).expect("encodable");
        assert_eq!(event_type(token), SonicEventType::ControlChange);
        assert_eq!(channel(token), 3);
        assert_eq!(note(token), 7); // controller
        assert_eq!(velocity(token), 127); // value
        assert_eq!(sonic_token_to_midi(token), Some(msg));
    }

    #[test]
    fn pack_layout_matches_core_db_positions() {
        // Same arguments as core-db's pack_roundtrip, checking the shared layout.
        let t = pack(3, SonicEventType::NoteOn, 2, 60, 100, 42, 0x0053);
        assert_eq!(delta_time(t), 3);
        assert_eq!(event_type(t), SonicEventType::NoteOn);
        assert_eq!(channel(t), 2);
        assert_eq!(note(t), 60);
        assert_eq!(velocity(t), 100);
        assert_eq!(tensor_index(t), 42);
        assert_eq!(flags(t), 0x0053);
    }

    #[test]
    fn non_mappable_message_returns_none() {
        assert_eq!(midi_to_sonic_token(&MidiMessage::TimingClock), None);
        assert_eq!(midi_to_sonic_token(&MidiMessage::TuneRequest), None);
        assert_eq!(midi_to_sonic_token(&MidiMessage::SongSelect(3)), None);
    }

    #[test]
    fn parametric_token_has_no_midi() {
        let t = pack(0, SonicEventType::Parametric, 0, 0, 100, 0, 0);
        assert_eq!(sonic_token_to_midi(t), None);
    }
}
