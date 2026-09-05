//! Directory-backed live production tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod cues_control_inspect;
mod dmx_fixtures;
mod lighting_projection;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        dmx_fixtures::ROWS
            .iter()
            .chain(lighting_projection::ROWS.iter())
            .chain(cues_control_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
