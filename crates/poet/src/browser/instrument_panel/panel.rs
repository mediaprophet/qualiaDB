//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Show, hide, and wire the contextual instrument panel chrome.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

/// Show or replace the contextual instrument panel for the given container element.
/// The instrument panel appears between the control bar and the canvas workspace.
pub fn show_for_container(document: &Document, container: &Element) {
    let container_type = container
        .get_attribute("data-container-type")
        .unwrap_or_default();

    let tools = super::catalog::tools_for_type(&container_type);
    if tools.is_empty() {
        hide(document);
        return;
    }

    // Remove existing instrument panel
    hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    crate::browser::surface_aspects::mark(&panel, "dwell");
    panel
        .set_attribute("data-container-type", &container_type)
        .unwrap();

    // Context label
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    label.set_text_content(Some(&format!("\u{1F4CB} {} tools", container_type)));
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        configure_tool_button(&btn, &tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    // Insert instrument panel between control bar and workspace
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    wire_instrument_panel(document);
}

/// Hide the contextual instrument panel.
pub fn hide(document: &Document) {
    if let Some(existing) = document
        .query_selector(".contextual-instrument-panel")
        .unwrap()
    {
        existing.remove();
    }
}

pub(super) fn configure_tool_button(button: &Element, tool: &super::ribbon::RibbonTool) {
    if !super::commands::local_instrument_action(tool.id) {
        button.set_attribute("disabled", "").unwrap();
        button.set_attribute("aria-disabled", "true").unwrap();
        button
            .set_attribute(
                "title",
                &format!(
                    "Unavailable in standalone POET: {} requires a dedicated typed runtime contract.",
                    tool.description
                ),
            )
            .unwrap();
        button.set_attribute("data-honesty", "unavailable").unwrap();
    } else if super::commands::instrument_requires_daemon(tool.id) {
        button
            .set_attribute("data-requires-daemon", "true")
            .unwrap();
        button
            .set_attribute("data-enabled-title", tool.description)
            .unwrap();
        if !crate::browser::native_daemon::is_daemon_connected() {
            button.set_attribute("disabled", "").unwrap();
            button.set_attribute("aria-disabled", "true").unwrap();
            button
                .set_attribute(
                    "title",
                    "Unavailable until the local QualiaDB daemon is connected.",
                )
                .unwrap();
        }
    }
}

/// Wire instrument panel button clicks and the close button.
pub(super) fn wire_instrument_panel(document: &Document) {
    // Tool buttons
    let buttons = document
        .query_selector_all(".instrument-panel-tool-btn")
        .unwrap();
    for i in 0..buttons.length() {
        let btn = buttons.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let tool_id = btn_el.get_attribute("data-tool").unwrap_or_default();
        let label = btn_el
            .query_selector(".instrument-panel-tool-label")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::dispatch::dispatch_instrument_action(&doc, &tool_id, &label);
        }) as Box<dyn FnMut(Event)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Close button
    if let Some(close) = document
        .query_selector(".instrument-panel-close-btn")
        .unwrap()
    {
        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            hide(&doc);
        }) as Box<dyn FnMut(Event)>);
        close
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
