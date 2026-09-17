//! Container drag, resize, and canvas pan/zoom behavior.

use super::*;

/// Wire up container dragging via header mousedown.
pub fn wire_container_dragging(document: &Document) {
    init_global_pointer_listeners(document);

    let headers = document.query_selector_all(".container-header").unwrap();
    for i in 0..headers.length() {
        let header = headers.get(i).unwrap();
        let header_el: Element = header.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&header_el, "drag") {
            continue;
        }

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = match e.dyn_into() {
                Ok(m) => m,
                Err(_) => return,
            };
            me.stop_propagation();
            me.prevent_default();

            let target = match me.current_target() {
                Some(t) => t,
                None => return,
            };
            let header: Element = match target.dyn_into() {
                Ok(h) => h,
                Err(_) => return,
            };
            let parent = match header.closest(".canvas-container-node").ok().flatten() {
                Some(p) => p,
                None => return,
            };

            let _ = parent.class_list().add_1("dragging");

            // Bring to front
            let next_z = HIGHEST_Z.fetch_add(1, Ordering::SeqCst) + 1;
            let style = parent.get_attribute("style").unwrap_or_default();
            let new_style = update_z_index(&style, next_z);
            let _ = parent.set_attribute("style", &new_style);

            let (orig_x, orig_y) = parse_position(&style);
            let start_mx = me.client_x() as f32;
            let start_my = me.client_y() as f32;
            let (grab_dx, grab_dy) = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.get_element_by_id("manifold-canvas"))
                .map(|canvas| {
                    let (wx, wy) =
                        super::super::canvas_extent::client_to_world(&canvas, start_mx, start_my);
                    (wx - orig_x, wy - orig_y)
                })
                .unwrap_or((0.0, 0.0));

            ACTIVE_INTERACTION.with(|ai| {
                *ai.borrow_mut() = Some(ActivePointerInteraction::DraggingContainer {
                    container: parent,
                    grab_dx,
                    grab_dy,
                });
            });
        }) as Box<dyn FnMut(Event)>);

        header_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Wire up canvas pan (drag on empty canvas) and zoom (wheel).
pub fn wire_canvas_pan_zoom(document: &Document) {
    init_global_pointer_listeners(document);

    let canvas = match document.get_element_by_id("manifold-canvas") {
        Some(c) => c,
        None => return,
    };
    if !super::super::dom_bindings::claim(&canvas, "pan-zoom") {
        return;
    }

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
        if !cls.contains("canvas-viewport-container")
            && !cls.contains("canvas-grid-svg")
            && !cls.contains("canvas-content-layer")
        {
            return;
        }

        let start_mx = me.client_x() as f32;
        let start_my = me.client_y() as f32;
        let (start_pan_x, start_pan_y) = super::super::canvas_extent::pan_of(&canvas_clone);

        ACTIVE_INTERACTION.with(|ai| {
            *ai.borrow_mut() = Some(ActivePointerInteraction::PanningCanvas {
                canvas: canvas_clone.clone(),
                start_mx,
                start_my,
                start_pan_x,
                start_pan_y,
            });
        });
    }) as Box<dyn FnMut(Event)>);

    canvas
        .add_event_listener_with_callback("mousedown", pan_closure.as_ref().unchecked_ref())
        .unwrap();
    pan_closure.forget();

    // Zoom (wheel) — scale around the cursor so the manifold extends under the pointer.
    let canvas_clone2 = canvas.clone();
    let zoom_closure = Closure::wrap(Box::new(move |e: Event| {
        let we: web_sys::WheelEvent = e.dyn_into().unwrap();
        we.prevent_default();
        let factor = if we.delta_y() > 0.0 { 0.9 } else { 1.1 };
        let canvas_el: Element = canvas_clone2.clone().dyn_into().unwrap();
        let cur = super::super::canvas_extent::zoom_of(&canvas_el);
        let new_zoom = (cur * factor).clamp(0.15, 4.0);
        let (wx, wy) = super::super::canvas_extent::client_to_world(
            &canvas_el,
            we.client_x() as f32,
            we.client_y() as f32,
        );
        let rect = canvas_el.get_bounding_client_rect();
        let pan_x = we.client_x() as f32 - rect.left() as f32 - wx * new_zoom;
        let pan_y = we.client_y() as f32 - rect.top() as f32 - wy * new_zoom;
        super::super::canvas_extent::set_view(&canvas_el, pan_x, pan_y, new_zoom);
    }) as Box<dyn FnMut(Event)>);

    canvas
        .add_event_listener_with_callback("wheel", zoom_closure.as_ref().unchecked_ref())
        .unwrap();
    zoom_closure.forget();
}
/// Wire up container resize handles (bottom-right grip).
pub fn wire_container_resize(document: &Document) {
    init_global_pointer_listeners(document);

    let handles = document
        .query_selector_all(".resize-handle, .container-resizer")
        .unwrap();
    for i in 0..handles.length() {
        let handle = handles.get(i).unwrap();
        let handle_el: Element = handle.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&handle_el, "resize") {
            continue;
        }

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = match e.dyn_into() {
                Ok(m) => m,
                Err(_) => return,
            };
            me.stop_propagation();
            me.prevent_default();

            let target = match me.current_target() {
                Some(t) => t,
                None => return,
            };
            let handle: Element = match target.dyn_into() {
                Ok(h) => h,
                Err(_) => return,
            };
            let parent = match handle.closest(".canvas-container-node").ok().flatten() {
                Some(p) => p,
                None => return,
            };

            let start_mx = me.client_x() as f32;
            let start_my = me.client_y() as f32;
            let style = parent.get_attribute("style").unwrap_or_default();
            let (orig_w, orig_h) = parse_size(&style);
            let zoom = current_canvas_zoom(&web_sys::window().unwrap().document().unwrap());

            ACTIVE_INTERACTION.with(|ai| {
                *ai.borrow_mut() = Some(ActivePointerInteraction::ResizingContainer {
                    container: parent,
                    start_mx,
                    start_my,
                    orig_w,
                    orig_h,
                    style,
                    zoom,
                });
            });
        }) as Box<dyn FnMut(Event)>);

        handle_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
