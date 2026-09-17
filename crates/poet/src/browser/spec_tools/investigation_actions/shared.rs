//! Shared DOM attribute helpers for investigation actions.

use web_sys::Element;

pub(crate) fn append_csv_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else {
        format!("{current},{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

pub(crate) fn append_semicolon_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else {
        format!("{current};{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

pub(crate) fn count_selector(document: &web_sys::Document, selector: &str) -> Result<u32, String> {
    document
        .query_selector_all(selector)
        .map(|list| list.length())
        .map_err(|_| format!("Failed to query {selector}."))
}
