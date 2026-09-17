//! Stateful spreadsheet container.

mod formula;
mod model;
mod ui;
mod view;

use std::collections::BTreeMap;

use web_sys::{Document, Element};

use model::SheetState;

pub fn build_sheet_view(document: &Document, settings: &BTreeMap<String, String>) -> Element {
    view::build(document, settings)
}

/// Import comma- or tab-delimited text into a spreadsheet container from A1.
/// Quoted commas are not split (v0).
pub fn import_delimited_into(container: &Element, text: &str) -> Result<usize, &'static str> {
    let root = container
        .query_selector("[data-sheet-root]")
        .ok()
        .flatten()
        .ok_or("Select a spreadsheet container before importing.")?;
    let settings = container
        .get_attribute("data-tool-settings")
        .and_then(|json| serde_json::from_str::<BTreeMap<String, String>>(&json).ok())
        .unwrap_or_default();
    let mut state = SheetState::from_settings(&settings);
    let tsv = if text.contains('\t') {
        text.to_string()
    } else {
        text.replace(',', "\t")
    };
    let written = state.paste_tsv("A1", &tsv);
    if written == 0 {
        return Err("No cells were imported from the selected file.");
    }
    ui::persist(&root, &state, "import sheet");
    ui::refresh_values(&root, &state);
    Ok(written)
}
