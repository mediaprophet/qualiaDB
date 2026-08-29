//! Shared lifecycle chrome for every Poet canvas container.

use std::collections::BTreeMap;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, MouseEvent};

use crate::tool_chest::core::registry::SeedContainer;

const COLLAPSED_KEY: &str = "poet:collapsed";

pub fn build_header_actions(document: &Document) -> Element {
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("container-actions");

    for (action, class_name, glyph, title, pressed) in [
        (
            "settings",
            "settings-btn",
            "\u{2699}",
            "Container settings",
            None,
        ),
        (
            "minimize",
            "minimize-btn",
            "\u{2212}",
            "Minimise container",
            Some("false"),
        ),
        ("delete", "delete-btn", "\u{2715}", "Delete container", None),
    ] {
        let button = document.create_element("button").unwrap();
        button.set_class_name(&format!("container-action-btn {class_name}"));
        button.set_attribute("type", "button").unwrap();
        button
            .set_attribute("data-container-action", action)
            .unwrap();
        button.set_attribute("title", title).unwrap();
        button.set_attribute("aria-label", title).unwrap();
        if let Some(pressed) = pressed {
            button.set_attribute("aria-pressed", pressed).unwrap();
        }
        button.set_text_content(Some(glyph));
        actions.append_child(&button).unwrap();
    }
    actions
}

pub fn restore_chrome_state(element: &Element, container: &SeedContainer) {
    if container
        .tool_settings
        .get(COLLAPSED_KEY)
        .map(String::as_str)
        == Some("true")
    {
        set_collapsed(element, true);
    }
}

/// One delegated listener covers seeded, duplicated, and newly placed containers.
pub fn wire_container_chrome(document: &Document) {
    let closure = Closure::wrap(Box::new(move |event: Event| {
        let Ok(mouse) = event.dyn_into::<MouseEvent>() else {
            return;
        };
        let Some(target) = mouse
            .target()
            .and_then(|target| target.dyn_into::<Element>().ok())
        else {
            return;
        };
        let Some(button) = target.closest("[data-container-action]").ok().flatten() else {
            return;
        };
        let Some(container) = button.closest(".canvas-container-node").ok().flatten() else {
            return;
        };
        mouse.stop_propagation();
        match button
            .get_attribute("data-container-action")
            .unwrap_or_default()
            .as_str()
        {
            "settings" => {
                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    open_settings_dialog(&document, &container);
                }
            }
            "minimize" => {
                let collapsed = !container.class_list().contains("container-minimized");
                set_collapsed(&container, collapsed);
                persist_collapsed(&container, collapsed);
                super::history::push_current_frame(if collapsed {
                    "minimise container"
                } else {
                    "restore container"
                });
            }
            _ => {}
        }
    }) as Box<dyn FnMut(Event)>);
    document
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn set_collapsed(container: &Element, collapsed: bool) {
    let _ = container
        .class_list()
        .toggle_with_force("container-minimized", collapsed);
    if let Ok(Some(button)) = container.query_selector(".minimize-btn") {
        let _ = button.set_attribute("aria-pressed", if collapsed { "true" } else { "false" });
        let title = if collapsed {
            "Restore container"
        } else {
            "Minimise container"
        };
        let _ = button.set_attribute("title", title);
        let _ = button.set_attribute("aria-label", title);
        button.set_text_content(Some(if collapsed { "\u{25A1}" } else { "\u{2212}" }));
    }
}

fn persist_collapsed(container: &Element, collapsed: bool) {
    let mut settings = container
        .get_attribute("data-tool-settings")
        .and_then(|json| serde_json::from_str::<BTreeMap<String, String>>(&json).ok())
        .unwrap_or_default();
    settings.insert(COLLAPSED_KEY.to_string(), collapsed.to_string());
    if let Ok(json) = serde_json::to_string(&settings) {
        let _ = container.set_attribute("data-tool-settings", &json);
    }
}

fn open_settings_dialog(document: &Document, container: &Element) {
    if let Some(existing) = document.get_element_by_id("container-settings-dialog") {
        existing.remove();
    }
    let model = super::canvas_state::container_from_element(container);
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("container-settings-dialog");
    overlay.set_class_name("dialog-overlay");

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dialog-panel container-settings-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();
    panel
        .set_attribute("aria-labelledby", "container-settings-title")
        .unwrap();

    let header = document.create_element("div").unwrap();
    header.set_class_name("dialog-header");
    let title = document.create_element("div").unwrap();
    title.set_id("container-settings-title");
    title.set_class_name("dialog-title");
    title.set_text_content(Some("\u{2699} Container settings"));
    header.append_child(&title).unwrap();
    let close = document.create_element("button").unwrap();
    close.set_class_name("dialog-close-btn");
    close.set_attribute("type", "button").unwrap();
    close
        .set_attribute("aria-label", "Close container settings")
        .unwrap();
    close.set_text_content(Some("\u{2715}"));
    header.append_child(&close).unwrap();
    panel.append_child(&header).unwrap();

    let body = document.create_element("div").unwrap();
    body.set_class_name("dialog-body");
    append_field(
        document,
        &body,
        "container-setting-title",
        "Title",
        &model.title,
        "text",
    );
    append_field(
        document,
        &body,
        "container-setting-semantic-type",
        "Semantic type",
        &model.semantic_type,
        "text",
    );
    append_field(
        document,
        &body,
        "container-setting-semantic-uri",
        "Semantic URI",
        &model.semantic_uri,
        "url",
    );
    let size = document.create_element("div").unwrap();
    size.set_class_name("container-settings-size-row");
    append_field(
        document,
        &size,
        "container-setting-width",
        "Width",
        &model.width.round().to_string(),
        "number",
    );
    append_field(
        document,
        &size,
        "container-setting-height",
        "Height",
        &model.height.round().to_string(),
        "number",
    );
    body.append_child(&size).unwrap();
    panel.append_child(&body).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_class_name("dialog-footer");
    let cancel = document.create_element("button").unwrap();
    cancel.set_class_name("btn btn-secondary");
    cancel.set_text_content(Some("Cancel"));
    let apply = document.create_element("button").unwrap();
    apply.set_class_name("btn btn-primary");
    apply.set_text_content(Some("Apply changes"));
    footer.append_child(&cancel).unwrap();
    footer.append_child(&apply).unwrap();
    panel.append_child(&footer).unwrap();
    overlay.append_child(&panel).unwrap();
    document.body().unwrap().append_child(&overlay).unwrap();

    for button in [close, cancel] {
        let overlay = overlay.clone();
        let closure =
            Closure::wrap(Box::new(move |_event: Event| overlay.remove()) as Box<dyn FnMut(Event)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let container = container.clone();
    let overlay_for_apply = overlay.clone();
    let closure = Closure::wrap(Box::new(move |_event: Event| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let value = |id: &str| -> String {
            document
                .get_element_by_id(id)
                .and_then(|node| node.dyn_into::<HtmlInputElement>().ok())
                .map(|input| input.value())
                .unwrap_or_default()
        };
        let title_value = value("container-setting-title");
        if title_value.trim().is_empty() {
            super::interactions::show_tool_status(
                &document,
                "Container settings",
                "Title cannot be empty.",
                "error",
            );
            return;
        }
        let width = value("container-setting-width")
            .parse::<f32>()
            .unwrap_or(400.0)
            .max(280.0);
        let height = value("container-setting-height")
            .parse::<f32>()
            .unwrap_or(300.0)
            .max(180.0);
        if let Ok(Some(title)) = container.query_selector(".container-title") {
            title.set_text_content(Some(title_value.trim()));
        }
        let _ = container.set_attribute(
            "data-semantic-type",
            value("container-setting-semantic-type").trim(),
        );
        let _ = container.set_attribute(
            "data-semantic-uri",
            value("container-setting-semantic-uri").trim(),
        );
        if let Ok(html) = container.clone().dyn_into::<HtmlElement>() {
            let _ = html
                .style()
                .set_property("width", &format!("{}px", width.round()));
            let _ = html
                .style()
                .set_property("height", &format!("{}px", height.round()));
        }
        overlay_for_apply.remove();
        super::interactions::update_all_wires(&document);
        super::history::push_current_frame("edit container settings");
        super::interactions::show_tool_status(
            &document,
            "Container settings",
            "Container metadata and dimensions updated.",
            "success",
        );
    }) as Box<dyn FnMut(Event)>);
    apply
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    if let Some(input) = document.get_element_by_id("container-setting-title") {
        let _ = input
            .dyn_into::<HtmlInputElement>()
            .map(|input| input.focus());
    }
}

fn append_field(
    document: &Document,
    parent: &Element,
    id: &str,
    label: &str,
    value: &str,
    input_type: &str,
) {
    let group = document.create_element("label").unwrap();
    group.set_class_name("form-group");
    let text = document.create_element("span").unwrap();
    text.set_class_name("form-label");
    text.set_text_content(Some(label));
    let input = document.create_element("input").unwrap();
    input.set_id(id);
    input.set_class_name("form-input");
    input.set_attribute("type", input_type).unwrap();
    input.set_attribute("value", value).unwrap();
    if input_type == "number" {
        input.set_attribute("min", "180").unwrap();
        input.set_attribute("step", "10").unwrap();
    }
    group.append_child(&text).unwrap();
    group.append_child(&input).unwrap();
    parent.append_child(&group).unwrap();
}
