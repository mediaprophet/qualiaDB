//! Bridge — reconcile transcription, authored MIDI, the sequencer, and the
//! sonic-token transport. Re-exports only (AU-MIDI-BRIDGE).
//!
//! This lane wires already-built pieces together without reimplementing them:
//!
//! - [`note_events_to_timed`] — transcribed [`crate::features::pitch_midi::NoteEvent`]
//!   PROPOSALS → sequencer [`crate::midi::sequencer::TimedEvent`] note-on/off pairs.
//! - [`pair_note_events`] — a timed MIDI stream (sequencer / flattened SMF track,
//!   AUTHORITATIVE) → whole [`PairedNote`]s.
//! - [`midi_to_sonic_token`] / [`sonic_token_to_midi`] — canonical
//!   [`crate::midi::message::MidiMessage`] ↔ a packed `u64` [`SonicToken`]-shaped
//!   word mirroring core-db's `net/sonic_token.rs`.
//! - [`extract_smf_provenance`] — a parsed [`crate::midi::smf::SmfFile`] → bounded
//!   [`ProvenancePair`]s for NQuin emission.
//!
//! Epistemic distinction preserved throughout: transcribed MIDI is a *proposal*
//! (carries confidence at its source), authored / imported SMF MIDI is
//! *authoritative*. See each submodule's docs.

pub mod from_pitch_midi;
pub mod smf_provenance;
pub mod sonic_token;
pub mod to_note_events;

pub use from_pitch_midi::{
    note_events_to_timed, NOTE_OFF_VELOCITY, PROPOSAL_CHANNEL, PROPOSAL_NOTE,
};
pub use smf_provenance::{
    extract_smf_provenance, ProvenanceKey, ProvenancePair, MAX_PROV_VALUE, META_COPYRIGHT,
};
pub use sonic_token::{midi_to_sonic_token, sonic_token_to_midi, SonicEventType, SONIC_MAGIC};
pub use to_note_events::{pair_note_events, PairedNote, MAX_OPEN_NOTES};
