//! Microtuning — Scala `.scl`/`.kbm` + MIDI Tuning Standard (MTS). Re-exports only (AU-MIDI-TUNE).
//!
//! Tuning is a FIRST-CLASS, declared parameter here — 12-TET is one input among many, never
//! hardcoded. Clean-room implementations from the open Scala scale-file / keyboard-mapping specs
//! and the MMA MIDI Tuning Standard. The note→frequency table is a fixed `[f64; 128]` and lookup
//! is allocation-free; only `.scl`/`.kbm` text parsing (a cold path) may allocate.

pub mod mts;
pub mod scala_kbm;
pub mod scala_scl;

pub use mts::{
    build_tuning_table, decode_single_note_tuning, encode_single_note_tuning, note_frequency,
    SingleNoteTuning, SNTC_LEN,
};
pub use scala_kbm::{parse_kbm, KbmMapping, KBM_UNMAPPED, MAX_KBM_ENTRIES};
pub use scala_scl::{parse_scl, SclEntry, SclScale, MAX_SCALE_NOTES};
