//! Shared DOM updates and persistence for the Sheet UI.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::{
    formula::display_value,
    model::{SheetState, SHEET_STATE_KEY},
};

pub(super) fn refresh_values(root: &Element, state: &SheetState) {
    let Ok(cells) = root.query_selector_all(".sheet-cell") else {
        return;
    };
    let focused = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element());
    for index in 0..cells.length() {
        let Some(node) = cells.get(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<Element>() else {
            continue;
        };
        let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() else {
            continue;
        };
        let reference = element.get_attribute("data-cell-ref").unwrap_or_default();
        let is_focused = focused.as_ref().is_some_and(|active| active == &element);
        if !is_focused {
            input.set_value(&display_value(state, &reference));
        }
        let _ = if state.raw(&reference).starts_with('=') {
            element.class_list().add_1("formula")
        } else {
            element.class_list().remove_1("formula")
        };
    }
}

pub(super) fn update_selection(
    root: &Element,
    formula: &HtmlInputElement,
    name: &HtmlInputElement,
    status: &Element,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
) {
    let selected = active.borrow().clone();
    if let Ok(cells) = root.query_selector_all(".sheet-cell") {
        for index in 0..cells.length() {
            let Some(node) = cells.get(index) else {
                continue;
            };
            let Ok(cell) = node.dyn_into::<Element>() else {
                continue;
            };
            if cell.get_attribute("data-cell-ref").as_deref() == Some(&selected) {
                let _ = cell.class_list().add_1("selected");
            } else {
                let _ = cell.class_list().remove_1("selected");
            }
        }
    }
    name.set_value(&selected);
    formula.set_value(state.borrow().raw(&selected));
    status.set_text_content(Some(&status_text(&state.borrow(), &selected)));
}

pub(super) fn status_text(state: &SheetState, selected: &str) -> String {
    let display = display_value(state, selected);
    let summary = if display.is_empty() {
        "empty".to_string()
    } else {
        format!("value {display}")
    };
    format!(
        "{} rows × {} columns · {selected}: {summary}",
        state.rows, state.cols
    )
}

pub(super) fn focus_cell(root: &Element, reference: &str, state: &SheetState) {
    let selector = format!(".sheet-cell[data-cell-ref=\"{reference}\"]");
    if let Ok(Some(cell)) = root.query_selector(&selector) {
        if let Ok(input) = cell.dyn_into::<HtmlInputElement>() {
            input.set_value(state.raw(reference));
            if let Ok(input) = input.dyn_into::<HtmlElement>() {
                let _ = input.focus();
            }
        }
    }
}

pub(super) fn persist(root: &Element, state: &SheetState, history_label: &str) {
    let Some(container) = root.closest(".canvas-container-node").ok().flatten() else {
        return;
    };
    let mut settings = container
        .get_attribute("data-tool-settings")
        .and_then(|json| serde_json::from_str::<BTreeMap<String, String>>(&json).ok())
        .unwrap_or_default();
    if let Ok(json) = serde_json::to_string(state) {
        settings.insert(SHEET_STATE_KEY.to_string(), json);
    }
    if let Ok(json) = serde_json::to_string(&settings) {
        let _ = container.set_attribute("data-tool-settings", &json);
    }
    super::super::history::push_current_frame(history_label);
}

pub(super) fn element(document: &Document, tag: &str, class_name: &str) -> Element {
    let element = document.create_element(tag).unwrap();
    element.set_class_name(class_name);
    element
}

pub(super) fn button(document: &Document, label: &str, aria_label: &str) -> Element {
    let button = element(document, "button", "sheet-toolbar-button");
    button.set_text_content(Some(label));
    button.set_attribute("type", "button").unwrap();
    button.set_attribute("aria-label", aria_label).unwrap();
    button
}
