//! Container metadata → provenance. Re-exports only (AU-IO).
//!
//! - [`read_wav_tags`] parses the RIFF `LIST`/`INFO` tag chunks of a WAV.
//! - [`tags_to_provenance`] maps them to bounded `(key, value)` provenance pairs.

pub mod riff_tags;
pub mod to_provenance;

pub use riff_tags::{read_wav_tags, WavTags};
pub use to_provenance::{tags_to_provenance, MAX_PROVENANCE_PAIRS};
