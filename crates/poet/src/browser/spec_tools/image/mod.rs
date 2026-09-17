//! Directory-backed image tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod layers_brushes;
mod masks_colour;
mod selection_filters;
mod vector_inspect;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        layers_brushes::ROWS
            .iter()
            .chain(selection_filters::ROWS.iter())
            .chain(masks_colour::ROWS.iter())
            .chain(vector_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
