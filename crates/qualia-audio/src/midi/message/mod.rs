//! MIDI 1.0 message model — the canonical channel-voice / system message set
//! and a zero-alloc streaming parser with running status (AU-MIDI-MSG).
//!
//! Re-exports only. Each message type lives in its own single-unit leaf module
//! (build to fixed bytes / parse from a caller slice; no heap). The canonical
//! [`MidiMessage`] `Copy` enum and the [`MessageParser`] streaming parser live
//! in [`running_status`].
//!
//! - [`NoteOn`] / [`NoteOff`] — note messages.
//! - [`ControlChange`] — control change (CC).
//! - [`PitchBend`] — 14-bit pitch bend.
//! - [`ChannelPressure`] / [`PolyPressure`] — aftertouch.
//! - [`ProgramChange`] — program change.
//! - [`ChannelMode`] / [`ChannelModeMessage`] — channel-mode messages.
//! - [`frame_sysex`] / [`sysex_payload`] / [`frame_universal`] — SysEx framing.
//! - [`MidiMessage`] / [`MessageParser`] / [`parse_stream`] / [`parse_into`] —
//!   canonical model + running-status stream parser.

pub mod channel_mode;
pub mod channel_pressure;
pub mod control_change;
pub mod note;
pub mod pitch_bend;
pub mod poly_pressure;
pub mod program_change;
pub mod running_status;
pub mod sysex;

pub use channel_mode::{ChannelMode, ChannelModeMessage};
pub use channel_pressure::ChannelPressure;
pub use control_change::ControlChange;
pub use note::{NoteOff, NoteOn};
pub use pitch_bend::PitchBend;
pub use poly_pressure::PolyPressure;
pub use program_change::ProgramChange;
pub use running_status::{parse_into, parse_stream, MessageParser, MidiMessage};
pub use sysex::{frame_sysex, frame_universal, sysex_payload};
