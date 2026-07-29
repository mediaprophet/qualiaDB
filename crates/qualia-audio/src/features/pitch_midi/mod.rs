//! Pitch → MIDI note ON/OFF events, Audio2MIDI. Re-exports only (AU-PITCH-MIDI).
//!
//! Frequency ↔ MIDI conversion ([`hz_to_midi`] / [`midi_to_hz`]), note-track
//! segmentation ([`segment_notes`] → [`NoteEvent`]), and the composed
//! end-to-end [`audio_to_midi`]. Transcribed MIDI is an epistemic *proposal*
//! carrying a per-note confidence, never authoritative like imported MIDI.

pub mod audio_to_midi;
pub mod note_segmentation;
pub mod pitch_to_midi;

pub use audio_to_midi::audio_to_midi;
pub use note_segmentation::{segment_notes, NoteEvent, MIN_VOICED_CONFIDENCE};
pub use pitch_to_midi::{hz_to_midi, midi_to_hz, MIDI_MAX, MIDI_MIN};
