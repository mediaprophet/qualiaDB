//! Fail-closed treatment for leftover prototype-only labels.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

const READ_ONLY_REASON: &str =
    "Read-only prototype: the engine contract named by this surface has not been registered.";

fn declares_prototype(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("mock data")
        || text.contains("structural mock")
        || text.contains("renderer placeholder")
}

/// Disable leftover mock-labelled bodies. Live COP session surfaces are untouched.
pub fn enforce(document: &Document, body: &Element, _container_type: &str) -> bool {
    let text = body.text_content().unwrap_or_default();
    if !declares_prototype(&text) {
        return false;
    }

    body.set_attribute("data-honesty", "read-only-prototype")
        .ok();
    body.class_list().add_1("read-only-prototype").ok();
    if let Some(container) = body.parent_element() {
        container
            .set_attribute("data-effective-honesty", "unavailable")
            .ok();
        if let Ok(Some(badge)) = container.query_selector(".honesty-badge") {
            badge.set_class_name("honesty-badge honesty-missing");
            badge.set_text_content(Some("unavailable"));
            badge.set_attribute("title", READ_ONLY_REASON).ok();
        }
    }

    if let Ok(controls) =
        body.query_selector_all("button, input, select, textarea, [contenteditable=\"true\"]")
    {
        for index in 0..controls.length() {
            let Some(node) = controls.get(index) else {
                continue;
            };
            let Ok(control) = node.dyn_into::<Element>() else {
                continue;
            };
            control.set_attribute("disabled", "").ok();
            control.set_attribute("aria-disabled", "true").ok();
            control.set_attribute("contenteditable", "false").ok();
            control.set_attribute("title", READ_ONLY_REASON).ok();
        }
    }
    let _ = document;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_prototype_markers_are_classified() {
        assert!(declares_prototype("Mock data — engine required"));
        assert!(declares_prototype("structural mock"));
        assert!(!declares_prototype("Unavailable: daemon is offline."));
        assert!(!declares_prototype("Live COP ledger count"));
    }

    #[test]
    fn unbound_specialist_surfaces_have_specific_prerequisites() {
        // Session surfaces persist; leftover mock labels still fail closed.
        assert!(declares_prototype("Mock data — wallet"));
    }
}
