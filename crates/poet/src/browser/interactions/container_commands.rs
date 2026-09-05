//! Container selection, deletion, duplication, and keyboard commands.

use super::*;

/// Wire up container selection — clicking a container selects it and
/// brings it to the top z-index (dynamic layering).
pub fn wire_container_selection(document: &Document) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&el, "selection") {
            continue;
        }
        let el_clone = el.clone();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();

            let doc = web_sys::window().unwrap().document().unwrap();

            let target_id = el_clone.get_attribute("data-id").unwrap_or_default();
            let pending_source = PENDING_WIRE_SOURCE.with(|pending| pending.borrow().clone());
            if let Some(source_id) = pending_source {
                if source_id != target_id {
                    if let Ok(Some(source)) = doc.query_selector(&format!(
                        ".canvas-container-node[data-id=\"{}\"]",
                        source_id
                    )) {
                        create_wire(&doc, &source, &el_clone);
                    }
                    clear_pending_wire(&doc);
                    return;
                }
            }

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
            super::super::instrument_panel::show_for_container(&doc, &el_clone);
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
        .query_selector_all(".container-action-btn.delete-btn")
        .unwrap();
    for i in 0..close_btns.length() {
        let btn = close_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&btn_el, "delete") {
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
    let container_id = container.get_attribute("data-id").unwrap_or_default();
    if !container_id.is_empty() {
        if let Ok(paths) = document.query_selector_all(".wire-overlay path") {
            for i in (0..paths.length()).rev() {
                let Some(node) = paths.get(i) else { continue };
                let Ok(path) = node.dyn_into::<Element>() else {
                    continue;
                };
                let attached = path.get_attribute("data-source-id").as_deref()
                    == Some(&container_id)
                    || path.get_attribute("data-target-id").as_deref() == Some(&container_id);
                if attached {
                    remove_wire_and_label(document, &path);
                }
            }
        }
    }
    container.remove();
    // Hide instrument panel since the container it was showing for is gone
    super::super::instrument_panel::hide(document);
    // Push a history frame
    super::super::history::push_current_frame("delete container");
    // Show a brief notification
    show_tool_notification(document, "delete-container", "Container deleted");
}

/// Delete a wire (SVG path) from the wire overlay, along with its label.
/// Pushes a history frame and hides the wire inspector.
fn delete_wire(document: &Document, wire_path: &Element) {
    remove_wire_and_label(document, wire_path);
    // Hide wire inspector
    super::super::wire_inspector::hide();
    // Push a history frame
    super::super::history::push_current_frame("delete wire");
    // Show notification
    show_tool_notification(document, "delete-wire", "Wire deleted");
}

fn remove_wire_and_label(_document: &Document, wire_path: &Element) {
    let wire_id = wire_path.get_attribute("data-id").unwrap_or_default();
    if let Some(parent) = wire_path.parent_element() {
        if let Ok(Some(label)) =
            parent.query_selector(&format!(".wire-label-text[data-wire-id=\"{}\"]", wire_id))
        {
            label.remove();
        }
    }
    wire_path.remove();
}

/// Public wrapper to delete a wire element.
pub fn delete_wire_element(document: &Document, wire_path: &Element) {
    delete_wire(document, wire_path);
}

pub fn delete_container_by_id(document: &Document, container_id: &str) {
    if let Ok(Some(container)) = document.query_selector(&format!(
        ".canvas-container-node[data-id=\"{}\"]",
        container_id
    )) {
        delete_container(document, &container);
    }
}

pub fn duplicate_container_by_id(document: &Document, container_id: &str) {
    if let Ok(Some(container)) = document.query_selector(&format!(
        ".canvas-container-node[data-id=\"{}\"]",
        container_id
    )) {
        duplicate_container(document, &container);
        wire_container_selection(document);
        wire_container_dragging(document);
        wire_container_resize(document);
        wire_container_deletion(document);
        wire_port_dragging(document);
        super::super::history::push_current_frame("duplicate container");
    }
}

pub fn begin_wire_connection(document: &Document, container_id: &str) {
    clear_pending_wire(document);
    if let Ok(Some(source)) = document.query_selector(&format!(
        ".canvas-container-node[data-id=\"{}\"]",
        container_id
    )) {
        let _ = source.class_list().add_1("wire-source-active");
        PENDING_WIRE_SOURCE.with(|pending| *pending.borrow_mut() = Some(container_id.to_string()));
        show_tool_notification(
            document,
            "wire-mode",
            "Connection mode: select a target container (Esc cancels)",
        );
    }
}

fn clear_pending_wire(document: &Document) {
    PENDING_WIRE_SOURCE.with(|pending| pending.borrow_mut().take());
    if let Ok(Some(source)) = document.query_selector(".canvas-container-node.wire-source-active") {
        let _ = source.class_list().remove_1("wire-source-active");
    }
}

pub fn cancel_wire_connection(document: &Document) {
    clear_pending_wire(document);
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
        super::super::history::push_current_frame("duplicate container");

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
        super::super::wire_inspector::wire_wire_inspector(&doc);
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Programmatic duplication of selected containers (for top menubar and palette actions).
pub fn duplicate_selected_containers(document: &Document) {
    let selected = document
        .query_selector_all(".canvas-container-node.selected")
        .unwrap();
    if selected.length() == 0 {
        show_tool_notification(document, "dup", "No container selected to duplicate");
        return;
    }

    let mut count = 0u32;
    for i in 0..selected.length() {
        let node = selected.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        duplicate_container(document, &el);
        count += 1;
    }

    super::super::history::push_current_frame("duplicate container");
    show_tool_notification(
        document,
        "dup",
        &format!("{} container(s) duplicated", count),
    );

    wire_container_selection(document);
    wire_container_dragging(document);
    wire_container_resize(document);
    wire_container_deletion(document);
    super::super::wire_inspector::wire_wire_inspector(document);
}

/// Apply zoom delta or absolute zoom scale to the canvas viewport.
pub fn apply_canvas_zoom(document: &Document, zoom_delta: f32, absolute: bool) {
    if let Some(canvas_el) = document.get_element_by_id("manifold-canvas") {
        let current_zoom = super::super::canvas_extent::zoom_of(&canvas_el);
        let (pan_x, pan_y) = super::super::canvas_extent::pan_of(&canvas_el);
        let new_zoom = if absolute {
            zoom_delta.clamp(0.15, 4.0)
        } else {
            (current_zoom + zoom_delta).clamp(0.15, 4.0)
        };
        super::super::canvas_extent::set_view(&canvas_el, pan_x, pan_y, new_zoom);
    }
}

/// Auto-arrange all containers on the canvas into a clean, responsive grid layout.
pub fn auto_arrange_containers(document: &Document) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    let count = containers.length();
    if count == 0 {
        return;
    }

    let margin_x = 40.0;
    let margin_y = 40.0;
    let gap = 24.0;
    let max_cols = if count > 6 { 3 } else { 2 };

    let mut current_col = 0;
    let mut col_y = vec![margin_y; max_cols];

    for i in 0..count {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();

        let style = el.get_attribute("style").unwrap_or_default();
        let (w, h) = parse_size(&style);

        let x = margin_x + (current_col as f32) * (w + gap);
        let y = col_y[current_col];

        let new_style = format!(
            "left: {}; top: {}; width: {}; height: {}; z-index: {}; transition: left 0.3s ease, top 0.3s ease;",
            px(x),
            px(y),
            px(w),
            px(h),
            i + 1
        );
        el.set_attribute("style", &new_style).unwrap();

        col_y[current_col] += h + gap;
        current_col = (current_col + 1) % max_cols;
    }

    super::super::history::push_current_frame("auto-arrange containers");
    show_tool_notification(document, "tidy", "Containers auto-arranged in clean grid");
}

/// Duplicate a single container — clone it with a 30px offset.
fn duplicate_container(document: &Document, container: &Element) {
    let mut model = super::super::canvas_state::container_from_element(container);
    model.id = super::super::canvas_state::next_container_id(&model.container_type);

    // Deselect the original
    container.class_list().remove_1("selected").unwrap();

    // Offset position by 30px
    let style = container.get_attribute("style").unwrap_or_default();
    let (x, y) = parse_position(&style);
    let new_x = x + 30.0;
    let new_y = y + 30.0;
    model.x = new_x;
    model.y = new_y;
    model.z = (HIGHEST_Z.fetch_add(1, Ordering::SeqCst) + 1) as f32;
    model.title = format!("{} copy", model.title);
    let clone_el = super::super::containers::build_container(document, &model);

    // Select the clone
    clone_el.class_list().add_1("selected").unwrap();

    // Append to the canvas content layer (or canvas as fallback)
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&clone_el).unwrap();
        } else {
            canvas.append_child(&clone_el).unwrap();
        }
        super::super::canvas_extent::pan_to_show(document, new_x, new_y, model.width, model.height);
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
