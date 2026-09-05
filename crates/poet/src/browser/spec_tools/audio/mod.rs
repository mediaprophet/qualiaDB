//! Directory-backed audio tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod editing_synthesis;
mod midi_effects;
mod mixing_inspect;
mod transport_tracks;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        transport_tracks::ROWS
            .iter()
            .chain(editing_synthesis::ROWS.iter())
            .chain(midi_effects::ROWS.iter())
            .chain(mixing_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
