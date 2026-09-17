//! Directory-backed interactive hypermedia tool rows, kept in focused chain modules.

use super::row::SpecTool;
use std::sync::OnceLock;

mod interactive_screens;
mod social_packaging;
mod sync_inspect;

pub fn rows() -> &'static [SpecTool] {
    static ROWS: OnceLock<Vec<SpecTool>> = OnceLock::new();
    ROWS.get_or_init(|| {
        interactive_screens::ROWS
            .iter()
            .chain(social_packaging::ROWS.iter())
            .chain(sync_inspect::ROWS.iter())
            .copied()
            .collect()
    })
    .as_slice()
}
