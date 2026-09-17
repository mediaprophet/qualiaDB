//! Global pointer-event lifecycle and active interaction dispatch.

use super::*;

/// Initialize the single, shared global pointer event listeners on the document.
/// This prevents listener leaks where temporary move handlers stick to the mouse pointer.
pub fn init_global_pointer_listeners(document: &Document) {
    GLOBAL_LISTENERS_INITIALIZED.with(|init| {
        if *init.borrow() {
            return;
        }
        *init.borrow_mut() = true;

        // Global mousemove handler
        let on_move = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = match e.dyn_into() {
                Ok(m) => m,
                Err(_) => return,
            };
            let mx = me.client_x() as f32;
            let my = me.client_y() as f32;

            ACTIVE_INTERACTION.with(|ai| {
                if let Some(ref interaction) = *ai.borrow() {
                    match interaction {
                        ActivePointerInteraction::DraggingContainer {
                            container,
                            grab_dx,
                            grab_dy,
                        } => {
                            if let Some(win) = web_sys::window() {
                                if let Some(doc) = win.document() {
                                    if let Some(canvas) = doc.get_element_by_id("manifold-canvas") {
                                        super::super::canvas_extent::edge_pan_if_needed(
                                            &canvas, mx, my,
                                        );
                                        let (wx, wy) = super::super::canvas_extent::client_to_world(
                                            &canvas, mx, my,
                                        );
                                        let new_x = snap_to_grid(wx - grab_dx);
                                        let new_y = snap_to_grid(wy - grab_dy);
                                        let style =
                                            container.get_attribute("style").unwrap_or_default();
                                        let new_style = update_position(&style, new_x, new_y);
                                        let _ = container.set_attribute("style", &new_style);
                                        if let Ok(c_html) =
                                            container.clone().dyn_into::<HtmlElement>()
                                        {
                                            let _ = c_html.style().set_property("left", &px(new_x));
                                            let _ = c_html.style().set_property("top", &px(new_y));
                                        }
                                    }
                                    update_all_wires(&doc);
                                }
                            }
                        }
                        ActivePointerInteraction::ResizingContainer {
                            container,
                            start_mx,
                            start_my,
                            orig_w,
                            orig_h,
                            style,
                            zoom,
                        } => {
                            let dx = (mx - start_mx) / zoom;
                            let dy = (my - start_my) / zoom;
                            let new_w = snap_to_grid((orig_w + dx).max(280.0));
                            let new_h = snap_to_grid((orig_h + dy).max(180.0));
                            let new_style = update_size(style, new_w, new_h);
                            let _ = container.set_attribute("style", &new_style);
                            if let Ok(c_html) = container.clone().dyn_into::<HtmlElement>() {
                                let _ = c_html.style().set_property("width", &px(new_w));
                                let _ = c_html.style().set_property("height", &px(new_h));
                            }
                            if let Some(win) = web_sys::window() {
                                if let Some(doc) = win.document() {
                                    update_all_wires(&doc);
                                }
                            }
                        }
                        ActivePointerInteraction::PanningCanvas {
                            canvas,
                            start_mx,
                            start_my,
                            start_pan_x,
                            start_pan_y,
                        } => {
                            let zoom = super::super::canvas_extent::zoom_of(canvas);
                            super::super::canvas_extent::set_view(
                                canvas,
                                start_pan_x + (mx - start_mx),
                                start_pan_y + (my - start_my),
                                zoom,
                            );
                        }
                        ActivePointerInteraction::DraggingPort {
                            drag_path,
                            start_x,
                            start_y,
                            ..
                        } => {
                            let (rel_x, rel_y) = web_sys::window()
                                .and_then(|win| win.document())
                                .and_then(|doc| doc.get_element_by_id("manifold-canvas"))
                                .map(|canvas| {
                                    super::super::canvas_extent::client_to_world(&canvas, mx, my)
                                })
                                .unwrap_or((mx, my));
                            let dx = ((rel_x - start_x).abs()) * 0.5;
                            let path_d = format!(
                                "M {} {} C {} {}, {} {}, {} {}",
                                start_x,
                                start_y,
                                start_x + dx,
                                start_y,
                                rel_x - dx,
                                rel_y,
                                rel_x,
                                rel_y
                            );
                            let _ = drag_path.set_attribute("d", &path_d);
                        }
                    }
                }
            });
        }) as Box<dyn FnMut(Event)>);

        // Global mouseup handler
        let on_up = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = match e.dyn_into() {
                Ok(m) => m,
                Err(_) => return,
            };
            let mx = me.client_x() as f32;
            let my = me.client_y() as f32;

            let prev_interaction = ACTIVE_INTERACTION.with(|ai| ai.borrow_mut().take());
            if let Some(interaction) = prev_interaction {
                let doc = match web_sys::window().and_then(|w| w.document()) {
                    Some(d) => d,
                    None => return,
                };
                match interaction {
                    ActivePointerInteraction::DraggingContainer { container, .. } => {
                        let _ = container.class_list().remove_1("dragging");
                        update_all_wires(&doc);
                        super::super::canvas_extent::ensure_manifold_extent(&doc);
                        super::super::history::push_current_frame("drag container");
                    }
                    ActivePointerInteraction::ResizingContainer { .. } => {
                        update_all_wires(&doc);
                        super::super::canvas_extent::ensure_manifold_extent(&doc);
                        super::super::history::push_current_frame("resize container");
                    }
                    ActivePointerInteraction::PanningCanvas { .. } => {}
                    ActivePointerInteraction::DraggingPort {
                        source_container,
                        drag_svg,
                        ..
                    } => {
                        drag_svg.remove();
                        if let Some(el) = doc.element_from_point(mx, my) {
                            if let Ok(el) = el.dyn_into::<Element>() {
                                let is_port_in = el.class_list().contains("port-in");
                                let target_container_opt = if is_port_in {
                                    el.closest(".canvas-container-node").ok().flatten()
                                } else if el.class_list().contains("canvas-container-node") {
                                    Some(el)
                                } else {
                                    el.closest(".canvas-container-node").ok().flatten()
                                };

                                if let Some(target_container) = target_container_opt {
                                    if target_container != source_container {
                                        create_wire(&doc, &source_container, &target_container);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        let _ = document
            .add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        let _ =
            document.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_move.forget();
        on_up.forget();
    });
}
