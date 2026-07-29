//! Standard MIDI File (formats 0/1/2) read/write + tempo map. Re-exports only.
//!
//! Lane AU-MIDI-FILE. Native, dependency-free SMF I/O under the Qualia ABI:
//!
//! - [`read_smf`] parses an SMF byte buffer into an [`SmfFile`] (expanding
//!   running status); [`write_smf`] serializes it back with deterministic,
//!   round-trippable bytes.
//! - [`build_tempo_map`] / [`TempoMap`] convert ticks ↔ seconds given the file's
//!   [`Division`].
//! - [`read_vlq`] / [`write_vlq`] and [`parse_meta`] / [`serialize_meta`] are the
//!   low-level codecs.

mod meta_event;
mod read;
mod tempo_map;
mod vlq;
mod write;

pub use meta_event::{
    parse_meta, serialize_meta, MetaEvent, META_END_OF_TRACK, META_KEY_SIGNATURE,
    META_SEQUENCE_NUMBER, META_TEMPO, META_TIME_SIGNATURE, META_TRACK_NAME,
};
pub use read::{read_smf, Division, Event, SmfFile, Track, TrackEvent};
pub use tempo_map::{build_tempo_map, TempoEntry, TempoMap, DEFAULT_US_PER_QUARTER};
pub use vlq::{read_vlq, write_vlq, VLQ_MAX};
pub use write::write_smf;
