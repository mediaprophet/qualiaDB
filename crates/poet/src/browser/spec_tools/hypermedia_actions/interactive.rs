//! Interactive overlays, triggers, and second-screen configuration.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "hypermedia:add-trigger" => Some(tag_attr(container, "data-trigger-added", "cue_on_timeline")),
        "hypermedia:trigger-condition" => Some(tag_attr(container, "data-trigger-condition", "predicate_set")),
        "hypermedia:trigger-action" => Some(tag_attr(container, "data-trigger-action", "action_bound")),
        "hypermedia:overlay-create" | "hypermedia:modal-overlay" => Some(add_modal_overlay(container)),
        "hypermedia:overlay-edit" => Some(tag_attr(container, "data-overlay-edit", "content_selected")),
        "hypermedia:overlay-timeline" => Some(tag_attr(container, "data-overlay-timeline", "in_out_set")),
        "hypermedia:interactive-preview" => Some(tag_attr(container, "data-interactive-preview", "audience_view")),
        "hypermedia:interactive-test" => Some(tag_attr(container, "data-interactive-test", "path_walked")),
        "hypermedia:companion-app-config" => Some(tag_attr(container, "data-companion-app", "device_profile_set")),
        "hypermedia:sync-stream-setup" => Some(tag_attr(container, "data-sync-stream", "broadcast_locked")),
        "hypermedia:remote-control-map" => Some(tag_attr(container, "data-remote-map", "keys_bound")),
        "hypermedia:second-screen-preview" => Some(tag_attr(container, "data-second-screen-preview", "companion_view")),
        "hypermedia:button-add" => Some(add_ui_element(document, container, "button", "Interactive Action")),
        "hypermedia:slider-add" => Some(add_ui_element(document, container, "input", "range")),
        "hypermedia:toggle-add" => Some(add_ui_element(document, container, "input", "checkbox")),
        _ => None,
    }
}

fn add_ui_element(document: &Document, container: &Element, tag: &str, label_or_type: &str) -> Result<(), String> {
    let el = document
        .create_element(tag)
        .map_err(|_| "Failed to create UI element.".to_string())?;
    if tag == "input" {
        let _ = el.set_attribute("type", label_or_type);
    } else {
        el.set_text_content(Some(label_or_type));
    }
    let _ = el.set_attribute("class", "poet-interactive-widget");
    container
        .append_child(&el)
        .map_err(|_| "Failed to append UI widget to container.".to_string())?;
    Ok(())
}

fn add_modal_overlay(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-modal-overlay", "active")
        .map_err(|_| "Failed to add modal overlay.".to_string())
}

fn tag_attr(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to set {key}."))
}
