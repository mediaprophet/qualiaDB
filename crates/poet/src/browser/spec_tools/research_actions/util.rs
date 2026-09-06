//! Shared DOM helpers for Poet research tool actions.

use web_sys::{Document, Element};

pub(crate) fn append_csv_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
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

pub(crate) fn append_nested(
    document: &Document,
    container: &Element,
    tag: &str,
    marker_attr: &str,
    marker_value: &str,
    extras: &[(&str, &str)],
) -> Result<(), String> {
    let el = document
        .create_element(tag)
        .map_err(|_| "Failed to create research node.".to_string())?;
    el.set_attribute(marker_attr, marker_value)
        .map_err(|_| format!("Failed to set {marker_attr}."))?;
    for (key, value) in extras {
        let _ = el.set_attribute(key, value);
    }
    let _ = el.set_attribute("class", "poet-research-node");
    container
        .append_child(&el)
        .map_err(|_| "Failed to append research node.".to_string())?;
    Ok(())
}

pub(crate) fn next_confidence(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("low") => "moderate",
        Some("moderate") => "high",
        Some("high") => "very_high",
        Some("very_high") => "low",
        _ => "low",
    }
}

pub(crate) fn count_within(container: &Element, selector: &str) -> Result<u32, String> {
    container
        .query_selector_all(selector)
        .map(|list| list.length())
        .map_err(|_| format!("Failed to query {selector}."))
}

pub(crate) fn count_document(document: &Document, selector: &str) -> Result<u32, String> {
    document
        .query_selector_all(selector)
        .map(|list| list.length())
        .map_err(|_| format!("Failed to query {selector}."))
}
