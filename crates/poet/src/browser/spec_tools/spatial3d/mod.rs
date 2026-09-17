//! Directory-backed 3D editing tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod materials_cameras;
mod narrative_inspect;
mod object_modelling;
mod rigging_animation;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        object_modelling::ROWS
            .iter()
            .chain(rigging_animation::ROWS.iter())
            .chain(materials_cameras::ROWS.iter())
            .chain(narrative_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
