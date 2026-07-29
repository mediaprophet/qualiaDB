//! MIDI clock / MTC / MMC / song-position. Re-exports only (AU-MIDI-SEQ).

pub mod midi_clock;
pub mod mmc;
pub mod mtc;
pub mod song_position;

pub use midi_clock::{bpm_from_clock_period, clock_period_seconds, ClockSync, CLOCKS_PER_QUARTER};
pub use mmc::{build as build_mmc, parse as parse_mmc, MmcCommand};
pub use mtc::{decode_quarter_frames, encode_quarter_frames, FrameRate, Timecode};
pub use song_position::{decode_song_position, encode_song_position, MAX_BEATS};
