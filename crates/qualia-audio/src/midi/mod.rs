//! Native MIDI engine — MIDI 1.0 + MIDI 2.0 (UMP), Standard MIDI File I/O, sequencer/transport,
//! microtuning (Scala/MTS), and a zero-alloc synth. Symbolic authoring/production, under the Qualia ABI.
//!
//! External projects (SST tuning-library, midi2-dev, sfizz, Plaits) are REFERENCE resources only —
//! the engine is native. Instrument/sample CONTENT is user-supplied (hypermedia library / vendor dir),
//! never bundled. See `docs/plans/audio-algorithms-catalogue-delivery-plan-2026.md` Wave M.

pub mod message;
pub mod ump;
pub mod smf;
pub mod sequencer;
pub mod sync;
pub mod mpe;
pub mod tuning;
pub mod synth;
pub mod instrument;
pub mod bridge;
