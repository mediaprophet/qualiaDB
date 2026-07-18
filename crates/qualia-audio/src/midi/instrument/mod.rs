//! SFZ/SF2/DLS instrument LOADERS + hypermedia/vendor resolver + licence provenance.
//!
//! **Qualia ships NO sample content.** This module implements the open, dependency-free
//! *loaders* and a *resolver* over USER-supplied instrument files (from the user's hypermedia
//! library or a user/vendor content dir). SFZ is parsed from the open plain-text spec (no
//! vendored sfizz); SF2/DLS are enumerated from their RIFF headers WITHOUT decoding sample audio.
//! Each resolved instrument carries a [`LicenceTag`] as provenance so downstream export/share can
//! fail closed on non-commercial / no-redistribution assets. See the delivery plan Wave M
//! ("Engine vs content"). Lane AU-MIDI-INSTRUMENT. Re-exports only.

mod dls;
mod preset;
mod resolver;
mod sample_map;
mod sf2;
mod sfz;

pub use dls::{read_dls_instruments, DlsCollection, DlsInstrument, MAX_INSTRUMENTS};
pub use preset::{InstrumentPreset, PresetFormat, SampleSource};
pub use resolver::{
    resolve_instrument, resolve_instrument_with, LicenceTag, ResolvedInstrument, ResolvedSample,
};
pub use sample_map::region_for;
pub use sf2::{read_sf2_presets, Sf2Preset, Sf2Presets, MAX_PRESETS};
pub use sfz::{parse_sfz, SfzInstrument, SfzRegion, MAX_REGIONS};
