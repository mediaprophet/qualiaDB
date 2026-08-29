//! Generic persistence adapter for specialist container controls.

use std::collections::BTreeMap;

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

fn control_key(element: &Element, index: u32) -> String {
    element
        .get_attribute("data-state-key")
        .or_else(|| element.get_attribute("id"))
        .or_else(|| element.get_attribute("name"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("control-{index}"))
}

/// Capture user-editable state without serialising or replacing the view DOM.
pub fn capture(container: &Element) -> BTreeMap<String, String> {
    let mut state = BTreeMap::new();
    let Ok(controls) = container
        .query_selector_all("input, select, textarea, [contenteditable=\"true\"], [aria-pressed]")
    else {
        return state;
    };

    for index in 0..controls.length() {
        let Some(node) = controls.get(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<Element>() else {
            continue;
        };
        if element.class_list().contains("doc-editor") {
            continue;
        }
        let key = control_key(&element, index);
        let encoded = if let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() {
            let input_type = input.type_();
            if input_type == "checkbox" || input_type == "radio" {
                format!("checked:{}", input.checked())
            } else {
                format!("value:{}", input.value())
            }
        } else if let Ok(select) = element.clone().dyn_into::<HtmlSelectElement>() {
            format!("value:{}", select.value())
        } else if let Ok(textarea) = element.clone().dyn_into::<HtmlTextAreaElement>() {
            format!("value:{}", textarea.value())
        } else if element.get_attribute("contenteditable").as_deref() == Some("true") {
            format!("text:{}", element.text_content().unwrap_or_default())
        } else if let Some(pressed) = element.get_attribute("aria-pressed") {
            format!("pressed:{pressed}")
        } else {
            continue;
        };
        state.insert(key, encoded);
    }
    state
}

/// Restore a previously captured state after the specialist view is built.
pub fn restore(container: &Element, state: &BTreeMap<String, String>) {
    if state.is_empty() {
        return;
    }
    let Ok(controls) = container
        .query_selector_all("input, select, textarea, [contenteditable=\"true\"], [aria-pressed]")
    else {
        return;
    };

    for index in 0..controls.length() {
        let Some(node) = controls.get(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<Element>() else {
            continue;
        };
        if element.class_list().contains("doc-editor") {
            continue;
        }
        let key = control_key(&element, index);
        let Some(encoded) = state.get(&key) else {
            continue;
        };

        if let Some(value) = encoded.strip_prefix("checked:") {
            if let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() {
                input.set_checked(value == "true");
            }
        } else if let Some(value) = encoded.strip_prefix("value:") {
            if let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() {
                input.set_value(value);
            } else if let Ok(select) = element.clone().dyn_into::<HtmlSelectElement>() {
                select.set_value(value);
            } else if let Ok(textarea) = element.clone().dyn_into::<HtmlTextAreaElement>() {
                textarea.set_value(value);
            }
        } else if let Some(value) = encoded.strip_prefix("text:") {
            element.set_text_content(Some(value));
        } else if let Some(value) = encoded.strip_prefix("pressed:") {
            let _ = element.set_attribute("aria-pressed", value);
            if value == "true" {
                let _ = element.class_list().add_1("active");
            } else {
                let _ = element.class_list().remove_1("active");
            }
        }
    }
}
