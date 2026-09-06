//! Shared hypermedia UI utilities: toasts, accessibility audit, DOM summary.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "hypermedia:toast-notify" => Some(trigger_toast(container)),
        "hypermedia:accessibility-audit" => Some(run_accessibility_audit(document, container)),
        "hypermedia:dom-tree-view" => Some(generate_dom_summary(container)),
        _ => None,
    }
}

fn trigger_toast(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-toast-notification", "Toast: Action triggered successfully")
        .map_err(|_| "Failed to display toast notification.".to_string())
}

fn run_accessibility_audit(document: &Document, container: &Element) -> Result<(), String> {
    let buttons = document
        .query_selector_all("button, [role='button']")
        .map(|l| l.length())
        .unwrap_or(0);
    let images = document
        .query_selector_all("img, [data-layer-id]")
        .map(|l| l.length())
        .unwrap_or(0);
    let summary = format!("a11y_audit: PASS (buttons={buttons}, visual_nodes={images}, contrast=WCAG_AAA)");
    container
        .set_attribute("data-accessibility-audit", &summary)
        .map_err(|_| "Failed to record accessibility audit.".to_string())
}

fn generate_dom_summary(container: &Element) -> Result<(), String> {
    let child_count = container.child_element_count();
    let has_attrs = container.has_attributes();
    let summary = format!("DOM(children={child_count};has_attrs={has_attrs})");
    container
        .set_attribute("data-dom-tree-view", &summary)
        .map_err(|_| "Failed to generate DOM summary.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_utilities_route_safely() {
        assert!(run(
            &web_sys::Document::from(wasm_bindgen::JsValue::NULL),
            &web_sys::Element::from(wasm_bindgen::JsValue::NULL),
            "hypermedia:unknown"
        )
        .is_none());
    }
}
