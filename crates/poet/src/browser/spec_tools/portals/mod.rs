//! Directory-backed portal worlds tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod physics_inspect;
mod portals_avatars;
mod worlds_objects;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        worlds_objects::ROWS
            .iter()
            .chain(portals_avatars::ROWS.iter())
            .chain(physics_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
