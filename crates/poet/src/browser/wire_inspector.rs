//! Wire inspector — click a connection wire to see its details in a
//! floating inspector panel. Ports the wire inspector concept from the
//! Canvas_Workbench mockup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

/// Wire up wire click handlers — clicking a wire path selects it and shows
/// the wire inspector panel with connection details. Double-clicking a wire
/// label lets you edit it inline.
pub fn wire_wire_inspector(document: &Document) {
    let paths = document.query_selector_all(".wire-overlay path").unwrap();
    for i in 0..paths.length() {
        let path = paths.get(i).unwrap();
        let path_el: Element = path.dyn_into().unwrap();
        let wire_id = path_el.get_attribute("data-id").unwrap_or_default();
        let path_clone = path_el.clone();

        let closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();

            // Deselect all other wires
            let all_paths = doc.query_selector_all(".wire-overlay path").unwrap();
            for j in 0..all_paths.length() {
                let p = all_paths.get(j).unwrap();
                let pe: Element = p.dyn_into().unwrap();
                pe.class_list().remove_1("wire-selected").unwrap();
            }

            // Select this wire
            path_clone.class_list().add_1("wire-selected").unwrap();

            // Also deselect all containers (wires and containers are mutually exclusive selection)
            let all_containers = doc.query_selector_all(".canvas-container-node").unwrap();
            for j in 0..all_containers.length() {
                let c = all_containers.get(j).unwrap();
                let ce: Element = c.dyn_into().unwrap();
                ce.class_list().remove_1("selected").unwrap();
            }

            show_inspector(&doc, &wire_id);
        }) as Box<dyn FnMut(MouseEvent)>);

        path_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire label text clicks (select the corresponding wire) and double-clicks (edit label)
    let labels = document.query_selector_all(".wire-label-text").unwrap();
    for i in 0..labels.length() {
        let label = labels.get(i).unwrap();
        let label_el: Element = label.dyn_into().unwrap();
        let label_clone = label_el.clone();

        // Single click — select the wire and show inspector
        let click_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();

            // Deselect all wires
            let all_paths = doc.query_selector_all(".wire-overlay path").unwrap();
            for j in 0..all_paths.length() {
                let p = all_paths.get(j).unwrap();
                let pe: Element = p.dyn_into().unwrap();
                pe.class_list().remove_1("wire-selected").unwrap();
            }

            // Select the wire at the same index as this label
            // (labels and paths are in the same order in the SVG)
            let label_idx = get_element_index(&label_clone);
            if let Some(svg) = doc.query_selector(".wire-overlay").unwrap() {
                let paths = svg.query_selector_all("path").unwrap();
                if label_idx < paths.length() as u32 {
                    let path = paths.get(label_idx).unwrap();
                    let path_el: Element = path.dyn_into().unwrap();
                    path_el.class_list().add_1("wire-selected").unwrap();
                    let wire_id = path_el.get_attribute("data-id").unwrap_or_default();
                    show_inspector(&doc, &wire_id);
                }
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        label_el
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        // Double click — edit the label inline
        let label_clone2 = label_el.clone();
        let dblclick_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            edit_wire_label(&doc, &label_clone2);
        }) as Box<dyn FnMut(MouseEvent)>);

        label_el
            .add_event_listener_with_callback("dblclick", dblclick_closure.as_ref().unchecked_ref())
            .unwrap();
        dblclick_closure.forget();
    }
}

/// Get the index of an element among its siblings of the same type.
fn get_element_index(el: &Element) -> u32 {
    let mut idx = 0u32;
    let mut sibling = el.previous_element_sibling();
    while let Some(s) = sibling {
        if s.tag_name() == el.tag_name() {
            idx += 1;
        }
        sibling = s.previous_element_sibling();
    }
    idx
}

/// Replace a wire label <text> element with an inline <foreignObject> input
/// for editing. On Enter or blur, commit the new label text.
fn edit_wire_label(document: &Document, label_el: &Element) {
    let current_text = label_el.text_content().unwrap_or_default();
    let y = label_el.get_attribute("y").unwrap_or_default();

    // Get the parent SVG
    let parent = match label_el.parent_element() {
        Some(p) => p,
        None => return,
    };

    // Create a foreignObject with an HTML input
    let fo = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "foreignObject")
        .unwrap();
    fo.set_attribute("x", "0").unwrap();
    fo.set_attribute("y", &format!("{}", y.parse::<f32>().unwrap_or(0.0) - 14.0))
        .unwrap();
    fo.set_attribute("width", "300").unwrap();
    fo.set_attribute("height", "28").unwrap();
    fo.set_attribute("class", "wire-label-editor").unwrap();

    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").unwrap();
    input.set_attribute("value", &current_text).unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.style().set_css_text(
        "width: 100%; padding: 2px 6px; font-size: 11px; font-family: var(--font-mono); \
         background: var(--surface-glass-heavy); border: 1px solid var(--accent-cyan); \
         border-radius: var(--radius-xs); color: var(--text-primary); outline: none;",
    );

    fo.append_child(&input).unwrap();

    // Replace the label with the editor
    let label_clone = label_el.clone();
    if let Some(parent_svg) = parent.dyn_ref::<web_sys::Element>() {
        parent_svg.replace_child(&fo, label_el).unwrap();
    }

    // Focus the input
    let _ = input_el.focus();
    input_el.select();

    // Commit on Enter
    let input_for_enter: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    let fo_for_enter = fo.clone();
    let label_for_enter = label_clone.clone();
    let enter_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" {
            let doc = web_sys::window().unwrap().document().unwrap();
            let new_text = input_for_enter.value();
            commit_label_edit(&doc, &fo_for_enter, &label_for_enter, &new_text);
        } else if e.key() == "Escape" {
            let doc = web_sys::window().unwrap().document().unwrap();
            commit_label_edit(&doc, &fo_for_enter, &label_for_enter, &current_text);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    input
        .add_event_listener_with_callback("keydown", enter_closure.as_ref().unchecked_ref())
        .unwrap();
    enter_closure.forget();

    // Commit on blur
    let input_for_blur: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    let fo_for_blur = fo.clone();
    let label_for_blur = label_clone.clone();
    let blur_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        let new_text = input_for_blur.value();
        commit_label_edit(&doc, &fo_for_blur, &label_for_blur, &new_text);
    }) as Box<dyn FnMut(web_sys::Event)>);
    input
        .add_event_listener_with_callback("blur", blur_closure.as_ref().unchecked_ref())
        .unwrap();
    blur_closure.forget();
}

/// Commit a wire label edit — replace the foreignObject input with a
/// <text> element containing the new label.
fn commit_label_edit(document: &Document, fo: &Element, original_label: &Element, new_text: &str) {
    let parent = match fo.parent_element() {
        Some(p) => p,
        None => return,
    };

    // Create a new <text> element with the updated label
    let text = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "text")
        .unwrap();
    text.set_attribute("x", &original_label.get_attribute("x").unwrap_or_default())
        .unwrap();
    text.set_attribute("y", &original_label.get_attribute("y").unwrap_or_default())
        .unwrap();
    text.set_attribute("class", "wire-label-text").unwrap();
    text.set_text_content(Some(new_text));

    // Replace the foreignObject with the new text
    parent.replace_child(&text, fo).unwrap();

    // Re-wire the wire inspector for the new label
    wire_wire_inspector(document);

    // Push a history frame
    super::history::push_current_frame("edit wire label");

    // Show notification
    super::interactions::show_tool_notification(document, "wire-label-edit", "Wire label updated");
}

fn show_inspector(document: &Document, wire_id: &str) {
    // Remove existing inspector
    hide_inspector(document);

    let panel = document.create_element("div").unwrap();
    panel.set_id("wire-inspector");
    panel.set_class_name("wire-inspector-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 300px; width: 320px; \
         background: var(--surface-glass-heavy); backdrop-filter: blur(20px); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 14px; box-shadow: var(--shadow-lg); z-index: 600; \
         display: flex; flex-direction: column; gap: 8px;",
    );

    // Header
    let header = document.create_element("div").unwrap();
    header
        .set_attribute(
            "style",
            "display: flex; align-items: center; justify-content: space-between;",
        )
        .unwrap();
    let title = document.create_element("span").unwrap();
    title.set_attribute("style", "font-size: 11px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; letter-spacing: 0.5px;").unwrap();
    title.set_text_content(Some("\u{1F4A1} Wire Inspector"));
    header.append_child(&title).unwrap();

    let close = document.create_element("button").unwrap();
    close.set_text_content(Some("\u{2715}"));
    let close_el: HtmlElement = close.clone().dyn_into().unwrap();
    close_el.style().set_css_text("background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 14px;");
    header.append_child(&close).unwrap();
    panel.append_child(&header).unwrap();

    // Wire ID
    let id_row = build_info_row(document, "Wire ID", wire_id);
    panel.append_child(&id_row).unwrap();

    // Connection type (extract from wire_id if possible)
    let conn_type = if wire_id.contains("active") {
        "active"
    } else if wire_id.contains("event") {
        "event"
    } else if wire_id.contains("ontology") {
        "ontology"
    } else if wire_id.contains("subjective") {
        "subjective"
    } else if wire_id.contains("objective") {
        "objective"
    } else {
        "active"
    };
    let type_row = build_info_row(document, "Type", conn_type);
    panel.append_child(&type_row).unwrap();

    // Honesty badge
    let honesty_row = document.create_element("div").unwrap();
    honesty_row
        .set_attribute("style", "display: flex; align-items: center; gap: 8px;")
        .unwrap();
    let honesty_label = document.create_element("span").unwrap();
    honesty_label
        .set_attribute(
            "style",
            "font-size: 10px; color: var(--text-muted); min-width: 60px;",
        )
        .unwrap();
    honesty_label.set_text_content(Some("Honesty"));
    honesty_row.append_child(&honesty_label).unwrap();
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("honesty-badge honesty-partial");
    badge.set_text_content(Some("partial"));
    honesty_row.append_child(&badge).unwrap();
    panel.append_child(&honesty_row).unwrap();

    // Description
    let desc = document.create_element("div").unwrap();
    desc.set_attribute("style", "font-size: 11px; color: var(--text-secondary); line-height: 1.5; padding-top: 4px; border-top: 1px solid var(--border-subtle);").unwrap();
    desc.set_text_content(Some(
        "Visual connection between containers. Wire routing is live (SVG bezier). \
         Semantic grounding (RDF triple, provenance) is partial \u{2014} awaiting backend ontology wiring."
    ));
    panel.append_child(&desc).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions
        .set_attribute("style", "display: flex; gap: 6px; padding-top: 4px;")
        .unwrap();
    for (label, action) in &[
        ("Delete", "delete"),
        ("Edit Label", "edit"),
        ("Trace Provenance", "trace"),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("wire-inspector-btn");
        btn.set_attribute("data-action", action).unwrap();
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    panel.append_child(&actions).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&panel).unwrap();
    }

    // Wire close button
    let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    close
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire action buttons
    let action_btns = panel.query_selector_all(".wire-inspector-btn").unwrap();
    for i in 0..action_btns.length() {
        let btn = action_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let action = btn_el.get_attribute("data-action").unwrap_or_default();
        let label = btn_el.text_content().unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_action_notification(&doc, &label, &action);
        }) as Box<dyn FnMut(MouseEvent)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn build_info_row(document: &Document, label: &str, value: &str) -> Element {
    let row = document.create_element("div").unwrap();
    row.set_attribute("style", "display: flex; align-items: center; gap: 8px;")
        .unwrap();
    let lbl = document.create_element("span").unwrap();
    lbl.set_attribute(
        "style",
        "font-size: 10px; color: var(--text-muted); min-width: 60px;",
    )
    .unwrap();
    lbl.set_text_content(Some(label));
    row.append_child(&lbl).unwrap();
    let val = document.create_element("span").unwrap();
    val.set_attribute(
        "style",
        "font-size: 11px; color: var(--text-primary); font-family: var(--font-mono);",
    )
    .unwrap();
    val.set_text_content(Some(value));
    row.append_child(&val).unwrap();
    row
}

fn show_action_notification(document: &Document, label: &str, _action: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 700; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!(
        "\u{1F4A1} {} \u{2014} present, engine wiring pending",
        label
    )));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

fn hide_inspector(document: &Document) {
    if let Some(existing) = document.get_element_by_id("wire-inspector") {
        existing.remove();
    }
}

/// Public hide — removes the wire inspector panel if present.
/// Used by interactions.rs when deleting wires.
pub fn hide() {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(existing) = doc.get_element_by_id("wire-inspector") {
                existing.remove();
            }
        }
    }
}
