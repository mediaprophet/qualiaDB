//! Stateful spreadsheet container.

mod formula;
mod model;
mod ui;
mod view;

use std::collections::BTreeMap;

use web_sys::{Document, Element};

pub fn build_sheet_view(document: &Document, settings: &BTreeMap<String, String>) -> Element {
    view::build(document, settings)
}
