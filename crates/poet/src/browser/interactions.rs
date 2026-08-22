//! Canvas interactions: pan/zoom, container dragging, selection, wire rendering.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, MouseEvent};

use crate::tool_chest::core::registry::ManifoldSeed;

/// JS `window.setTimeout` binding — avoids needing extra web-sys features.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout")]
    pub fn set_timeout(callback: &js_sys::Function, delay: u32) -> i32;
}

/// SVG namespace URI for creating SVG elements.
const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Global z-index counter — increments each time a container is focused.
static HIGHEST_Z: AtomicU32 = AtomicU32::new(100);

// Thread-local for the currently selected container id.
thread_local! {
    static SELECTED_CONTAINER: RefCell<Option<String>> = RefCell::new(None);
}

/// Wire up port-to-port wire drawing. Drag from an output port to an
/// input port to create a connection wire between two containers.
pub fn wire_port_dragging(document: &Document) {
    let out_ports = document.query_selector_all(".port-out").unwrap();
    for i in 0..out_ports.length() {
        let port = out_ports.get(i).unwrap();
        let port_el: Element = port.dyn_into().unwrap();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            me.prevent_default();

            let doc = web_sys::window().unwrap().document().unwrap();
            let target: Element = me.current_target().unwrap().dyn_into().unwrap();
            let source_container = match target.closest(".canvas-container-node").unwrap() {
                Some(c) => c,
                None => return,
            };

            // Get source container position
            let src_style = source_container.get_attribute("style").unwrap_or_default();
            let (src_x, src_y) = parse_position(&src_style);
            let (src_w, src_h) = parse_size(&src_style);
            let start_x = src_x + src_w;
            let start_y = src_y + src_h / 2.0;

            // Create or get the wire-drawing overlay
            let canvas = match doc.get_element_by_id("manifold-canvas") {
                Some(c) => c,
                None => return,
            };

            // Create a temporary SVG for the drag wire
            let drag_svg = doc.create_element_ns(Some(SVG_NS), "svg").unwrap();
            let drag_svg_el = drag_svg.clone();
            drag_svg_el
                .set_attribute("class", "wire-drag-overlay")
                .unwrap();
            drag_svg_el.set_attribute("width", "100%").unwrap();
            drag_svg_el.set_attribute("height", "100%").unwrap();
            let drag_svg_style: HtmlElement = drag_svg.clone().dyn_into().unwrap();
            drag_svg_style.style().set_css_text(
                "position: absolute; top: 0; left: 0; width: 100%; height: 100%; \
                 pointer-events: none; z-index: 200;",
            );

            let drag_path = doc.create_element_ns(Some(SVG_NS), "path").unwrap();
            drag_path
                .set_attribute(
                    "d",
                    &format!("M {} {} L {} {}", start_x, start_y, start_x, start_y),
                )
                .unwrap();
            drag_path
                .set_attribute("class", "wire-active wire-selected")
                .unwrap();
            drag_path
                .set_attribute("stroke", "var(--accent-cyan)")
                .unwrap();
            drag_path.set_attribute("stroke-width", "2").unwrap();
            drag_path.set_attribute("fill", "none").unwrap();
            drag_path.set_attribute("stroke-dasharray", "6 4").unwrap();
            drag_svg_el.append_child(&drag_path).unwrap();

            canvas.append_child(&drag_svg).unwrap();

            // Track mouse move to update the drag path
            let drag_path_clone = drag_path.clone();
            let start_x_clone = start_x;
            let start_y_clone = start_y;
            let canvas_clone = canvas.clone();
            let on_move = Closure::wrap(Box::new(move |ev: Event| {
                let mev: MouseEvent = ev.dyn_into().unwrap();
                // Get canvas rect to compute relative coordinates
                let canvas_rect = canvas_clone.get_bounding_client_rect();
                let rel_x = mev.client_x() as f32 - canvas_rect.left() as f32;
                let rel_y = mev.client_y() as f32 - canvas_rect.top() as f32;
                let dx = ((rel_x - start_x_clone).abs()) * 0.5;
                let path_d = format!(
                    "M {} {} C {} {}, {} {}, {} {}",
                    start_x_clone,
                    start_y_clone,
                    start_x_clone + dx,
                    start_y_clone,
                    rel_x - dx,
                    rel_y,
                    rel_x,
                    rel_y
                );
                drag_path_clone.set_attribute("d", &path_d).unwrap();
            }) as Box<dyn FnMut(Event)>);

            doc.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
                .unwrap();
            on_move.forget();

            // On mouseup, check if we're over a port-in
            let drag_svg_for_up = drag_svg.clone();
            let source_container_for_up = source_container.clone();
            let on_up = Closure::wrap(Box::new(move |ev: Event| {
                let mev: MouseEvent = ev.dyn_into().unwrap();
                let doc = web_sys::window().unwrap().document().unwrap();

                // Get the element under the cursor
                let target_el =
                    doc.element_from_point(mev.client_x() as f32, mev.client_y() as f32);

                // Remove the drag overlay
                drag_svg_for_up.remove();

                if let Some(el) = target_el {
                    let el: Element = el.dyn_into().unwrap();
                    // Check if we dropped on a port-in
                    if el.class_list().contains("port-in") {
                        if let Some(target_container) =
                            el.closest(".canvas-container-node").unwrap()
                        {
                            // Don't connect to self
                            if target_container != source_container_for_up {
                                create_wire(&doc, &source_container_for_up, &target_container);
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(Event)>);

            doc.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
                .unwrap();
            on_up.forget();
        }) as Box<dyn FnMut(Event)>);

        port_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Create a permanent wire between two containers.
fn create_wire(document: &Document, source: &Element, target: &Element) {
    let canvas = match document.get_element_by_id("manifold-canvas") {
        Some(c) => c,
        None => return,
    };

    let src_style = source.get_attribute("style").unwrap_or_default();
    let (src_x, src_y) = parse_position(&src_style);
    let (src_w, src_h) = parse_size(&src_style);
    let start_x = src_x + src_w;
    let start_y = src_y + src_h / 2.0;

    let tgt_style = target.get_attribute("style").unwrap_or_default();
    let (tgt_x, tgt_y) = parse_position(&tgt_style);
    let (_, tgt_h) = parse_size(&tgt_style);
    let end_x = tgt_x;
    let end_y = tgt_y + tgt_h / 2.0;

    let dx = ((end_x - start_x).abs()) * 0.5;
    let path_d = format!(
        "M {} {} C {} {}, {} {}, {} {}",
        start_x,
        start_y,
        start_x + dx,
        start_y,
        end_x - dx,
        end_y,
        end_x,
        end_y
    );

    // Find or create the wire overlay SVG
    let svg = match canvas.query_selector(".wire-overlay").unwrap() {
        Some(s) => s,
        None => {
            let new_svg = document.create_element_ns(Some(SVG_NS), "svg").unwrap();
            new_svg.set_attribute("class", "wire-overlay").unwrap();
            new_svg.set_attribute("width", "100%").unwrap();
            new_svg.set_attribute("height", "100%").unwrap();
            canvas.append_child(&new_svg).unwrap();
            new_svg
        }
    };

    // Generate a unique wire ID
    let wire_id = format!("wire-{}", js_sys::Date::now() as u64);
    let path = document.create_element_ns(Some(SVG_NS), "path").unwrap();
    path.set_attribute("d", &path_d).unwrap();
    path.set_attribute("class", "wire-active").unwrap();
    path.set_attribute("data-id", &wire_id).unwrap();
    svg.append_child(&path).unwrap();

    // Midpoint label
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0 - 6.0;
    let text = document.create_element_ns(Some(SVG_NS), "text").unwrap();
    text.set_attribute("x", &mid_x.to_string()).unwrap();
    text.set_attribute("y", &mid_y.to_string()).unwrap();
    text.set_attribute("class", "wire-label-text").unwrap();
    text.set_text_content(Some("new connection"));
    svg.append_child(&text).unwrap();

    // Re-wire wire inspector for the new wire
    super::wire_inspector::wire_wire_inspector(document);

    // Push a history frame
    super::history::push_current_frame("draw wire");

    // Show notification
    show_tool_notification(document, "wire-draw", "Wire connected");
}

/// Render connection wires as SVG bezier curves between containers.
pub fn render_wires(document: &Document, canvas: &Element, seed: &ManifoldSeed) {
    if seed.connections.is_empty() {
        return;
    }

    let svg = document.create_element_ns(Some(SVG_NS), "svg").unwrap();
    let svg_el = svg.clone();
    svg_el.set_attribute("class", "wire-overlay").unwrap();
    svg_el.set_attribute("width", "100%").unwrap();
    svg_el.set_attribute("height", "100%").unwrap();

    for conn in &seed.connections {
        if conn.from >= seed.containers.len() || conn.to >= seed.containers.len() {
            continue;
        }
        let src = &seed.containers[conn.from];
        let tgt = &seed.containers[conn.to];

        let start_x = src.x + src.width;
        let start_y = src.y + src.height / 2.0;
        let end_x = tgt.x;
        let end_y = tgt.y + tgt.height / 2.0;
        let dx = ((end_x - start_x).abs()) * 0.5;
        let path_d = format!(
            "M {} {} C {} {}, {} {}, {} {}",
            start_x,
            start_y,
            start_x + dx,
            start_y,
            end_x - dx,
            end_y,
            end_x,
            end_y
        );

        let path = document.create_element_ns(Some(SVG_NS), "path").unwrap();
        path.set_attribute("d", &path_d).unwrap();
        path.set_attribute("class", &format!("wire-{}", conn.wire_type))
            .unwrap();
        path.set_attribute("data-id", &conn.id).unwrap();
        svg_el.append_child(&path).unwrap();

        // Midpoint label
        let mid_x = (start_x + end_x) / 2.0;
        let mid_y = (start_y + end_y) / 2.0 - 6.0;

        let text = document.create_element_ns(Some(SVG_NS), "text").unwrap();
        text.set_attribute("x", &mid_x.to_string()).unwrap();
        text.set_attribute("y", &mid_y.to_string()).unwrap();
        text.set_attribute("class", "wire-label-text").unwrap();
        text.set_text_content(Some(&conn.label));
        svg_el.append_child(&text).unwrap();
    }

    canvas.append_child(&svg).unwrap();
}

/// Wire up container selection — clicking a container selects it and
/// brings it to the top z-index (dynamic layering).
pub fn wire_container_selection(document: &Document) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        let el_clone = el.clone();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();

            let doc = web_sys::window().unwrap().document().unwrap();

            // Multi-select: if Shift is held, toggle this container's selection
            // without clearing others. If not held, clear all and select only this.
            if me.shift_key() {
                // Toggle this container's selection
                if el_clone.class_list().contains("selected") {
                    el_clone.class_list().remove_1("selected").unwrap();
                } else {
                    el_clone.class_list().add_1("selected").unwrap();
                }
            } else {
                // Deselect all
                let all = doc.query_selector_all(".canvas-container-node").unwrap();
                for j in 0..all.length() {
                    let n = all.get(j).unwrap();
                    let ne: Element = n.dyn_into().unwrap();
                    ne.class_list().remove_1("selected").unwrap();
                }

                // Select this
                el_clone.class_list().add_1("selected").unwrap();
            }

            // Dynamic z-ordering: bring to front
            let next_z = HIGHEST_Z.fetch_add(1, Ordering::SeqCst) + 1;
            let style = el_clone.get_attribute("style").unwrap_or_default();
            let new_style = update_z_index(&style, next_z);
            el_clone.set_attribute("style", &new_style).unwrap();

            // Track selected container for contextual instrument panel
            let container_type = el_clone
                .get_attribute("data-container-type")
                .unwrap_or_default();
            SELECTED_CONTAINER.with(|sc| {
                *sc.borrow_mut() = Some(container_type);
            });

            // Show contextual instrument panel for this container type
            super::instrument_panel::show_for_container(&doc, &el_clone);
        }) as Box<dyn FnMut(Event)>);

        el.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Wire up container deletion: the ✕ button in the container header
/// removes the container from the canvas. Also wires the Delete keyboard
/// shortcut to remove the currently selected container.
pub fn wire_container_deletion(document: &Document) {
    // Wire the ✕ close button on each container header
    let close_btns = document
        .query_selector_all(".container-action-btn")
        .unwrap();
    for i in 0..close_btns.length() {
        let btn = close_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        // Only wire the ✕ button (second action button), not the ⚙ button
        if btn_el.text_content().as_deref() != Some("\u{2715}") {
            continue;
        }

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            let target: Element = me.current_target().unwrap().dyn_into().unwrap();
            // Find the parent container node
            if let Some(container) = target.closest(".canvas-container-node").unwrap() {
                delete_container(&doc, &container);
            }
        }) as Box<dyn FnMut(Event)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Delete a container from the canvas, push a history frame, and hide
/// the instrument panel if it was showing for this container.
fn delete_container(document: &Document, container: &Element) {
    container.remove();
    // Hide instrument panel since the container it was showing for is gone
    super::instrument_panel::hide(document);
    // Push a history frame
    super::history::push_current_frame("delete container");
    // Show a brief notification
    show_tool_notification(document, "delete-container", "Container deleted");
}

/// Delete a wire (SVG path) from the wire overlay, along with its label.
/// Pushes a history frame and hides the wire inspector.
fn delete_wire(document: &Document, wire_path: &Element) {
    // Try to find and remove the associated label text
    // The label is a sibling <text> element in the same SVG
    if let Some(parent) = wire_path.parent_element() {
        let labels = parent.query_selector_all(".wire-label-text").unwrap();
        for i in 0..labels.length() {
            // Remove all labels — in the current model, labels aren't
            // explicitly linked to paths by ID. A more precise approach
            // would match by data-id, but the current renderer doesn't
            // set data-id on labels. For now, we remove the first label
            // (which corresponds to the first wire in render order).
            // TODO: link labels to paths by data-id for precise deletion.
            let label = labels.get(i).unwrap();
            let label_el: Element = label.dyn_into().unwrap();
            label_el.remove();
            break;
        }
    }
    wire_path.remove();
    // Hide wire inspector
    super::wire_inspector::hide();
    // Push a history frame
    super::history::push_current_frame("delete wire");
    // Show notification
    show_tool_notification(document, "delete-wire", "Wire deleted");
}

/// Duplicate the selected container(s). Ctrl+D triggers this.
/// The duplicate is placed at an offset from the original and inherits
/// all properties (type, title, size, honesty).
pub fn wire_container_duplication(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: Event| {
        let ke: web_sys::KeyboardEvent = e.dyn_into().unwrap();
        // Ctrl+D or Cmd+D
        if !(ke.key() == "d" || ke.key() == "D") {
            return;
        }
        if !ke.ctrl_key() && !ke.meta_key() {
            return;
        }
        let doc = web_sys::window().unwrap().document().unwrap();
        // Don't duplicate if focus is in an input/textarea/contenteditable
        if let Some(active) = doc.active_element() {
            let tag = active.tag_name().to_lowercase();
            if tag == "input"
                || tag == "textarea"
                || active.get_attribute("contenteditable").as_deref() == Some("true")
            {
                return;
            }
        }
        // Find selected container(s)
        let selected = doc
            .query_selector_all(".canvas-container-node.selected")
            .unwrap();
        if selected.length() == 0 {
            return;
        }
        ke.prevent_default();

        let mut count = 0u32;
        for i in 0..selected.length() {
            let node = selected.get(i).unwrap();
            let el: Element = node.dyn_into().unwrap();
            duplicate_container(&doc, &el);
            count += 1;
        }

        // Push a history frame
        super::history::push_current_frame("duplicate container");

        if count > 1 {
            show_tool_notification(
                &doc,
                "dup-multi",
                &format!("{} containers duplicated", count),
            );
        } else {
            show_tool_notification(&doc, "dup", "Container duplicated (Ctrl+D)");
        }

        // Re-wire interactions for the new containers
        wire_container_selection(&doc);
        wire_container_dragging(&doc);
        wire_container_resize(&doc);
        wire_container_deletion(&doc);
        super::wire_inspector::wire_wire_inspector(&doc);
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Duplicate a single container — clone it with a 30px offset.
fn duplicate_container(document: &Document, container: &Element) {
    let clone = container.clone_node_with_deep(true).unwrap();
    let clone_el: Element = clone.dyn_into().unwrap();

    // Deselect the original
    container.class_list().remove_1("selected").unwrap();

    // Offset position by 30px
    let style = container.get_attribute("style").unwrap_or_default();
    let (x, y) = parse_position(&style);
    let new_x = x + 30.0;
    let new_y = y + 30.0;
    let new_style = update_position(&style, new_x, new_y);
    clone_el.set_attribute("style", &new_style).unwrap();

    // Select the clone
    clone_el.class_list().add_1("selected").unwrap();

    // Append to the canvas content layer (or canvas as fallback)
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&clone_el).unwrap();
        } else {
            canvas.append_child(&clone_el).unwrap();
        }
    }
}

/// Wire the Delete keyboard shortcut to remove the selected container(s)
/// or the selected wire. Supports multi-select deletion.
pub fn wire_delete_key(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: Event| {
        let ke: web_sys::KeyboardEvent = e.dyn_into().unwrap();
        // Delete or Backspace (but not when typing in an input/textarea)
        if ke.key() != "Delete" && ke.key() != "Backspace" {
            return;
        }
        let doc = web_sys::window().unwrap().document().unwrap();
        // Don't delete if focus is in an input/textarea/contenteditable
        if let Some(active) = doc.active_element() {
            let tag = active.tag_name().to_lowercase();
            if tag == "input"
                || tag == "textarea"
                || active.get_attribute("contenteditable").as_deref() == Some("true")
            {
                return;
            }
        }

        // First check for a selected wire
        if let Some(selected_wire) = doc
            .query_selector(".wire-overlay path.wire-selected")
            .unwrap()
        {
            ke.prevent_default();
            delete_wire(&doc, &selected_wire);
            return;
        }

        // Then check for selected container(s) — support multi-select
        let selected = doc
            .query_selector_all(".canvas-container-node.selected")
            .unwrap();
        if selected.length() > 0 {
            ke.prevent_default();
            let mut count = 0u32;
            for i in 0..selected.length() {
                let node = selected.get(i).unwrap();
                let el: Element = node.dyn_into().unwrap();
                delete_container(&doc, &el);
                count += 1;
            }
            if count > 1 {
                show_tool_notification(
                    &doc,
                    "delete-multi",
                    &format!("{} containers deleted", count),
                );
            }
        }
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Wire up container dragging via header mousedown.
pub fn wire_container_dragging(document: &Document) {
    let headers = document.query_selector_all(".container-header").unwrap();
    for i in 0..headers.length() {
        let header = headers.get(i).unwrap();
        let header_el: Element = header.dyn_into().unwrap();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();

            let doc = web_sys::window().unwrap().document().unwrap();
            let target = me.current_target().unwrap();
            let header: Element = target.dyn_into().unwrap();
            let parent = header.parent_element().unwrap();

            parent.class_list().add_1("dragging").unwrap();

            let start_mx = me.client_x() as f32;
            let start_my = me.client_y() as f32;
            let style = parent.get_attribute("style").unwrap_or_default();
            let (orig_x, orig_y) = parse_position(&style);

            let parent_clone = parent.clone();
            let on_move = Closure::wrap(Box::new(move |ev: Event| {
                let mev: MouseEvent = ev.dyn_into().unwrap();
                let dx = mev.client_x() as f32 - start_mx;
                let dy = mev.client_y() as f32 - start_my;
                let new_x = snap_to_grid(orig_x + dx);
                let new_y = snap_to_grid(orig_y + dy);
                let new_style = update_position(&style, new_x, new_y);
                parent_clone.set_attribute("style", &new_style).unwrap();
            }) as Box<dyn FnMut(Event)>);

            let parent_clone2 = parent.clone();
            let on_up = Closure::wrap(Box::new(move |_ev: Event| {
                parent_clone2.class_list().remove_1("dragging").unwrap();
                // Push a history frame after drag completes.
                super::history::push_current_frame("drag container");
            }) as Box<dyn FnMut(Event)>);

            doc.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
                .unwrap();
            doc.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
                .unwrap();
            on_move.forget();
            on_up.forget();
        }) as Box<dyn FnMut(Event)>);

        header_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Wire up canvas pan (drag on empty canvas) and zoom (wheel).
pub fn wire_canvas_pan_zoom(document: &Document) {
    let canvas = match document.get_element_by_id("manifold-canvas") {
        Some(c) => c,
        None => return,
    };

    // Pan
    let canvas_clone = canvas.clone();
    let pan_closure = Closure::wrap(Box::new(move |e: Event| {
        let me: MouseEvent = e.dyn_into().unwrap();
        let target = me.target().unwrap();
        let el: Element = target.dyn_into().unwrap();
        let tag = el.tag_name().to_lowercase();
        if tag != "div" {
            return;
        }
        let cls = el.class_list();
        if !cls.contains("canvas-viewport-container") && !cls.contains("canvas-grid-svg") {
            return;
        }

        let start_mx = me.client_x() as f32;
        let start_my = me.client_y() as f32;
        let canvas_ref = canvas_clone.clone();

        let on_move = Closure::wrap(Box::new(move |ev: Event| {
            let mev: MouseEvent = ev.dyn_into().unwrap();
            let dx = mev.client_x() as f32 - start_mx;
            let dy = mev.client_y() as f32 - start_my;
            canvas_ref.set_scroll_left(((canvas_ref.scroll_left() as f32 - dx) as i32).into());
            canvas_ref.set_scroll_top(((canvas_ref.scroll_top() as f32 - dy) as i32).into());
        }) as Box<dyn FnMut(Event)>);

        let doc = web_sys::window().unwrap().document().unwrap();
        doc.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
            .unwrap();
        on_move.forget();

        let on_up = Closure::wrap(Box::new(move |_ev: Event| {
            // Listeners are forgotten; in a production app we'd track and remove them.
            // For the UX preview this is acceptable.
        }) as Box<dyn FnMut(Event)>);

        doc.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
            .unwrap();
        on_up.forget();
    }) as Box<dyn FnMut(Event)>);

    canvas
        .add_event_listener_with_callback("mousedown", pan_closure.as_ref().unchecked_ref())
        .unwrap();
    pan_closure.forget();

    // Zoom (wheel) — apply CSS transform scale to the canvas content layer
    let canvas_clone2 = canvas.clone();
    let zoom_closure = Closure::wrap(Box::new(move |e: Event| {
        let we: web_sys::WheelEvent = e.dyn_into().unwrap();
        we.prevent_default();
        // Zoom out when scrolling down, in when scrolling up
        let factor = if we.delta_y() > 0.0 { 0.9 } else { 1.1 };
        let canvas_el: Element = canvas_clone2.clone().dyn_into().unwrap();
        let cur = canvas_el
            .get_attribute("data-zoom")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
        let new_zoom = (cur * factor).max(0.3).min(3.0);
        canvas_el
            .set_attribute("data-zoom", &new_zoom.to_string())
            .unwrap();

        // Apply transform to the canvas content (the inner layer with containers)
        if let Some(content) = canvas_el.query_selector(".canvas-content-layer").unwrap() {
            let content_el: web_sys::HtmlElement = content.dyn_into().unwrap();
            content_el
                .style()
                .set_property("transform", &format!("scale({})", new_zoom))
                .unwrap();
            content_el
                .style()
                .set_property("transform-origin", "0 0")
                .unwrap();
        }

        // Update zoom indicator if present
        if let Some(indicator) = canvas_el.query_selector(".canvas-zoom-indicator").unwrap() {
            indicator.set_text_content(Some(&format!("{:.0}%", new_zoom * 100.0)));
        }
    }) as Box<dyn FnMut(Event)>);

    canvas
        .add_event_listener_with_callback("wheel", zoom_closure.as_ref().unchecked_ref())
        .unwrap();
    zoom_closure.forget();
}

/// Wire up toolbox dock button selection + flyout panel, and family
/// header collapse/expand.
pub fn wire_toolbox_dock(document: &Document) {
    // Wire family header collapse/expand
    let headers = document.query_selector_all(".dock-family-header").unwrap();
    for i in 0..headers.length() {
        let header = headers.get(i).unwrap();
        let header_el: Element = header.dyn_into().unwrap();
        let family_id = header_el.get_attribute("data-family").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            // Find the children container for this family
            if let Some(section) = doc
                .query_selector(&format!(
                    ".dock-family-section[data-family=\"{}\"]",
                    family_id
                ))
                .unwrap()
            {
                if let Some(children) = section.query_selector(".dock-family-children").unwrap() {
                    if children.class_list().contains("expanded") {
                        children.class_list().remove_1("expanded").unwrap();
                    } else {
                        children.class_list().add_1("expanded").unwrap();
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        header_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire toolbox button selection + flyout
    let buttons = document.query_selector_all(".toolbox-dock-btn").unwrap();
    for i in 0..buttons.length() {
        let btn = buttons.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let toolbox_id = btn_el.get_attribute("data-toolbox").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();

            // Toggle active state on all dock buttons
            let all = doc.query_selector_all(".toolbox-dock-btn").unwrap();
            let mut clicked_is_active = false;
            for j in 0..all.length() {
                let n = all.get(j).unwrap();
                let ne: Element = n.dyn_into().unwrap();
                if ne.get_attribute("data-toolbox").as_deref() == Some(&toolbox_id) {
                    clicked_is_active = ne.class_list().contains("active");
                    ne.class_list().remove_1("active").unwrap();
                } else {
                    ne.class_list().remove_1("active").unwrap();
                }
            }

            if clicked_is_active {
                // Was active — hide flyout
                super::docks::hide_flyout(&doc);
            } else {
                // Was not active — activate and show flyout
                let all2 = doc.query_selector_all(".toolbox-dock-btn").unwrap();
                for j in 0..all2.length() {
                    let n = all2.get(j).unwrap();
                    let ne: Element = n.dyn_into().unwrap();
                    if ne.get_attribute("data-toolbox").as_deref() == Some(&toolbox_id) {
                        ne.class_list().add_1("active").unwrap();
                        break;
                    }
                }
                super::docks::show_flyout(&doc, &toolbox_id);
                // Wire tool button clicks in the flyout
                wire_flyout_tools(&doc);
            }
        }) as Box<dyn FnMut(Event)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Wire tool button clicks and tool-chain selection/drag inside the flyout panel.
/// PlaceContainer tools place a container on the canvas; other tools
/// show an honest "present — awaiting backend wiring" notification.
/// Tool-chain labels are clickable (activate on focused surface) and draggable
/// (drag onto a container to activate there).
fn wire_flyout_tools(document: &Document) {
    // Wire tool-chain label clicks (select/activate chain)
    let chain_labels = document.query_selector_all(".toolchain-label").unwrap();
    for i in 0..chain_labels.length() {
        let label = chain_labels.get(i).unwrap();
        let label_el: Element = label.dyn_into().unwrap();
        let chain_id = label_el.get_attribute("data-chain-id").unwrap_or_default();
        let chain_id_for_drag = chain_id.clone();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Toggle selected state on this chain label
            let all_labels = doc.query_selector_all(".toolchain-label").unwrap();
            let mut was_selected = false;
            for j in 0..all_labels.length() {
                let l = all_labels.get(j).unwrap();
                let le: Element = l.dyn_into().unwrap();
                if le.get_attribute("data-chain-id").as_deref() == Some(&chain_id) {
                    was_selected = le.class_list().contains("selected");
                    le.class_list().remove_1("selected").unwrap();
                } else {
                    le.class_list().remove_1("selected").unwrap();
                }
            }
            if !was_selected {
                // Activate this chain
                for j in 0..all_labels.length() {
                    let l = all_labels.get(j).unwrap();
                    let le: Element = l.dyn_into().unwrap();
                    if le.get_attribute("data-chain-id").as_deref() == Some(&chain_id) {
                        le.class_list().add_1("selected").unwrap();
                        break;
                    }
                }
                // Show the chain's tools on the contextual instrument panel
                super::instrument_panel::activate_chain(&doc, &chain_id);
            } else {
                // Deactivate — clear chain from instrument panel
                super::instrument_panel::deactivate_chain(&doc);
            }
        }) as Box<dyn FnMut(Event)>);

        label_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        // Wire drag start
        let drag_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.stop_propagation();
            if let Some(dt) = de.data_transfer() {
                dt.set_data("application/x-toolchain-id", &chain_id_for_drag)
                    .unwrap();
                dt.set_effect_allowed("copy");
            }
        }) as Box<dyn FnMut(Event)>);

        label_el
            .add_event_listener_with_callback("dragstart", drag_closure.as_ref().unchecked_ref())
            .unwrap();
        drag_closure.forget();
    }

    // Wire container drop zones for tool-chain drag
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let container = containers.get(i).unwrap();
        let container_el: Element = container.dyn_into().unwrap();
        let container_el_for_drop = container_el.clone();
        let container_el_for_over = container_el.clone();

        let drop_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.prevent_default();
            if let Some(dt) = de.data_transfer() {
                if let Ok(chain_id) = dt.get_data("application/x-toolchain-id") {
                    if !chain_id.is_empty() {
                        // Activate this chain on this container
                        let doc = web_sys::window().unwrap().document().unwrap();
                        super::instrument_panel::activate_chain_on_container(&doc, &chain_id);
                        // Select the container to show the instrument panel
                        let all = doc.query_selector_all(".canvas-container-node").unwrap();
                        for j in 0..all.length() {
                            let n = all.get(j).unwrap();
                            let ne: Element = n.dyn_into().unwrap();
                            ne.class_list().remove_1("selected").unwrap();
                        }
                        container_el_for_drop
                            .class_list()
                            .add_1("selected")
                            .unwrap();
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        container_el
            .add_event_listener_with_callback("drop", drop_closure.as_ref().unchecked_ref())
            .unwrap();
        drop_closure.forget();

        let dragover_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.prevent_default();
            if let Some(dt) = de.data_transfer() {
                dt.set_drop_effect("copy");
            }
        }) as Box<dyn FnMut(Event)>);

        container_el_for_over
            .add_event_listener_with_callback("dragover", dragover_closure.as_ref().unchecked_ref())
            .unwrap();
        dragover_closure.forget();
    }

    // Wire tool button clicks
    let tools = document.query_selector_all(".tool-btn").unwrap();
    for i in 0..tools.length() {
        let tool = tools.get(i).unwrap();
        let tool_el: Element = tool.dyn_into().unwrap();
        let tool_id = tool_el.get_attribute("data-tool-id").unwrap_or_default();
        let label = tool_el
            .query_selector(".tool-btn-label")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();
        let kind_badge = tool_el
            .query_selector(".tool-btn-kind")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if kind_badge == "place" {
                // PlaceContainer tool — extract container type from tool_id
                // tool_id format: "toolbox:place_<type>" e.g. "office:place_doc"
                let container_type = tool_id.split("place_").nth(1).unwrap_or("doc").to_string();
                place_container_on_canvas(&doc, &container_type, &label);
            } else {
                show_tool_notification(&doc, &tool_id, &label);
            }
        }) as Box<dyn FnMut(Event)>);

        tool_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Place a new container on the canvas at a default position.
/// The container is placed at a slight offset from the top-left to
/// avoid overlapping existing containers.
/// Place a container on the canvas via a menu action (public, for topbar).
pub fn place_container_via_menu(document: &Document, container_type: &str, label: &str) {
    place_container_on_canvas(document, container_type, label);
}

fn place_container_on_canvas(document: &Document, container_type: &str, label: &str) {
    use crate::tool_chest::core::registry::SeedContainer;

    // Count existing containers to offset new ones
    let existing = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    let count = existing.length() as f32;
    let x = 80.0 + (count % 5.0) * 40.0;
    let y = 60.0 + (count % 5.0) * 40.0;

    let container = SeedContainer {
        container_type: container_type.into(),
        title: label.trim_start_matches("+ ").to_string(),
        x,
        y,
        width: 400.0,
        height: 300.0,
        z: 100.0 + count,
        honesty: "missing".into(),
        ..Default::default()
    };

    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::containers::build_container(document, &container);

        // Append to the content layer if it exists, otherwise to the canvas
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&el).unwrap();
        } else {
            canvas.append_child(&el).unwrap();
        }

        // Re-wire interactions for the new container
        wire_container_selection(document);
        wire_container_dragging(document);
        wire_container_resize(document);
        wire_container_deletion(document);
        wire_port_dragging(document);

        // Push a history frame
        super::history::push_current_frame("place container");
    }
}

/// Show a transient notification when a tool is clicked.
/// Honest label: "present" — UI exists, engine not yet wired.
pub fn show_tool_notification(document: &Document, tool_id: &str, label: &str) {
    // Remove any existing notification
    if let Some(existing) = document.query_selector(".tool-notification").unwrap() {
        existing.remove();
    }

    let notif = document.create_element("div").unwrap();
    notif.set_class_name("tool-notification");
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 500; max-width: 320px; \
         display: flex; flex-direction: column; gap: 4px;",
    );

    let title = document.create_element("div").unwrap();
    title
        .set_attribute("style", "font-weight: 600; font-size: 11px;")
        .unwrap();
    title.set_text_content(Some(&format!("\u{1F4A1} {} ({})", label, tool_id)));
    notif.append_child(&title).unwrap();

    let status = document.create_element("div").unwrap();
    status
        .set_attribute("style", "color: var(--text-muted); font-size: 10px;")
        .unwrap();
    status.set_text_content(Some(
        "present \u{00B7} UI exists, engine wiring pending backend integration",
    ));
    notif.append_child(&status).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }

    // Auto-remove after 3 seconds via window.setTimeout
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    set_timeout(timeout.as_ref().unchecked_ref(), 3000);
    timeout.forget();
}

/// Wire up tray toggle buttons (dimension, time span, epistemic radio items).
/// These are dynamically created when a pod drop-tray opens, so this function
/// should be called after tray population. It uses event delegation on the
/// document to handle dynamically created buttons.
pub fn wire_selector_buttons(document: &Document) {
    // Wire tray toggle buttons via event delegation
    let closure = Closure::wrap(Box::new(move |e: Event| {
        let target: Element = match e.target() {
            Some(t) => t.dyn_into().unwrap(),
            None => return,
        };
        if !target.class_list().contains("tray-toggle-btn") {
            return;
        }

        // Find sibling buttons in the same row and deactivate them
        if let Some(parent) = target.parent_element() {
            let siblings = parent.query_selector_all(".tray-toggle-btn").unwrap();
            for j in 0..siblings.length() {
                let s = siblings.get(j).unwrap();
                let se: Element = s.dyn_into().unwrap();
                se.class_list().remove_1("active").unwrap();
            }
        }
        target.class_list().add_1("active").unwrap();
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire epistemic radio items
    let closure2 = Closure::wrap(Box::new(move |e: Event| {
        let target: Element = match e.target() {
            Some(t) => t.dyn_into().unwrap(),
            None => return,
        };
        if !target.class_list().contains("tray-radio-item") {
            return;
        }

        // Deactivate all radio items in the same tray
        let doc = web_sys::window().unwrap().document().unwrap();
        let all = doc.query_selector_all(".tray-radio-item").unwrap();
        for j in 0..all.length() {
            let n = all.get(j).unwrap();
            let ne: Element = n.dyn_into().unwrap();
            ne.class_list().remove_1("active").unwrap();
        }
        target.class_list().add_1("active").unwrap();
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("click", closure2.as_ref().unchecked_ref())
        .unwrap();
    closure2.forget();
}

/// Wire up container resize handles (bottom-right grip).
pub fn wire_container_resize(document: &Document) {
    let handles = document.query_selector_all(".resize-handle").unwrap();
    for i in 0..handles.length() {
        let handle = handles.get(i).unwrap();
        let handle_el: Element = handle.dyn_into().unwrap();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();

            let target = me.current_target().unwrap();
            let handle: Element = target.dyn_into().unwrap();
            let parent = handle.parent_element().unwrap();

            let start_mx = me.client_x() as f32;
            let start_my = me.client_y() as f32;
            let style = parent.get_attribute("style").unwrap_or_default();
            let (orig_w, orig_h) = parse_size(&style);

            let parent_clone = parent.clone();
            let on_move = Closure::wrap(Box::new(move |ev: Event| {
                let mev: MouseEvent = ev.dyn_into().unwrap();
                let dx = mev.client_x() as f32 - start_mx;
                let dy = mev.client_y() as f32 - start_my;
                let new_w = snap_to_grid((orig_w + dx).max(280.0));
                let new_h = snap_to_grid((orig_h + dy).max(180.0));
                let new_style = update_size(&style, new_w, new_h);
                parent_clone.set_attribute("style", &new_style).unwrap();
            }) as Box<dyn FnMut(Event)>);

            let doc = web_sys::window().unwrap().document().unwrap();
            doc.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
                .unwrap();
            on_move.forget();

            let on_up = Closure::wrap(Box::new(move |_ev: Event| {
                super::history::push_current_frame("resize container");
            }) as Box<dyn FnMut(Event)>);
            doc.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
                .unwrap();
            on_up.forget();
        }) as Box<dyn FnMut(Event)>);

        handle_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_position(style: &str) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("left: ") {
            x = val.trim_end_matches("px").parse().unwrap_or(0.0);
        } else if let Some(val) = part.strip_prefix("top: ") {
            y = val.trim_end_matches("px").parse().unwrap_or(0.0);
        }
    }
    (x, y)
}

fn update_z_index(style: &str, z: u32) -> String {
    let mut found = false;
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("z-index:") || part.starts_with("z-index: ") {
            result.push_str(&format!("z-index: {}; ", z));
            found = true;
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !found {
        result.push_str(&format!("z-index: {}; ", z));
    }
    result
}

fn update_position(style: &str, new_x: f32, new_y: f32) -> String {
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("left: ") {
            result.push_str(&format!("left: {}px; ", new_x as u32));
        } else if part.starts_with("top: ") {
            result.push_str(&format!("top: {}px; ", new_y as u32));
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !result.contains("left:") {
        result.push_str(&format!("left: {}px; ", new_x as u32));
    }
    if !result.contains("top:") {
        result.push_str(&format!("top: {}px; ", new_y as u32));
    }
    result
}

fn parse_size(style: &str) -> (f32, f32) {
    let mut w = 400.0;
    let mut h = 300.0;
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("width: ") {
            w = val.trim_end_matches("px").parse().unwrap_or(400.0);
        } else if let Some(val) = part.strip_prefix("height: ") {
            h = val.trim_end_matches("px").parse().unwrap_or(300.0);
        }
    }
    (w, h)
}

fn update_size(style: &str, new_w: f32, new_h: f32) -> String {
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("width: ") {
            result.push_str(&format!("width: {}px; ", new_w as u32));
        } else if part.starts_with("height: ") {
            result.push_str(&format!("height: {}px; ", new_h as u32));
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !result.contains("width:") {
        result.push_str(&format!("width: {}px; ", new_w as u32));
    }
    if !result.contains("height:") {
        result.push_str(&format!("height: {}px; ", new_h as u32));
    }
    result
}

// ---------------------------------------------------------------------------
// Grid snapping
// ---------------------------------------------------------------------------

/// Default grid size in pixels.
const GRID_SIZE: f32 = 16.0;

/// Snap a value to the nearest grid point.
pub fn snap_to_grid(value: f32) -> f32 {
    (value / GRID_SIZE).round() * GRID_SIZE
}

/// Snap a (x, y) pair to the grid.
pub fn snap_point(x: f32, y: f32) -> (f32, f32) {
    (snap_to_grid(x), snap_to_grid(y))
}

/// Clamp a value between min and max.
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Snap and clamp a position within canvas bounds.
pub fn snap_clamp_position(
    x: f32,
    y: f32,
    canvas_w: f32,
    canvas_h: f32,
    elem_w: f32,
    elem_h: f32,
) -> (f32, f32) {
    let sx = snap_to_grid(x);
    let sy = snap_to_grid(y);
    let cx = clamp(sx, 0.0, (canvas_w - elem_w).max(0.0));
    let cy = clamp(sy, 0.0, (canvas_h - elem_h).max(0.0));
    (cx, cy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_grid() {
        assert_eq!(snap_to_grid(0.0), 0.0);
        assert_eq!(snap_to_grid(15.0), 16.0);
        assert_eq!(snap_to_grid(17.0), 16.0);
        assert_eq!(snap_to_grid(24.0), 32.0);
        assert_eq!(snap_to_grid(-3.0), 0.0);
        assert_eq!(snap_to_grid(48.0), 48.0);
    }

    #[test]
    fn test_snap_point() {
        let (x, y) = snap_point(15.0, 33.0);
        assert_eq!(x, 16.0);
        assert_eq!(y, 32.0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_snap_clamp_position() {
        let (x, y) = snap_clamp_position(15.0, 33.0, 800.0, 600.0, 200.0, 150.0);
        assert_eq!(x, 16.0);
        assert_eq!(y, 32.0);

        let (x2, _) = snap_clamp_position(700.0, 0.0, 800.0, 600.0, 200.0, 150.0);
        assert_eq!(x2, 600.0);
    }
}
