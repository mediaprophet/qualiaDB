//! Directory-backed video tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod effects_generators;
mod sync_inspect_render;
mod transitions_colour;
mod transport_editing;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        transport_editing::ROWS
            .iter()
            .chain(transitions_colour::ROWS.iter())
            .chain(effects_generators::ROWS.iter())
            .chain(sync_inspect_render::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
