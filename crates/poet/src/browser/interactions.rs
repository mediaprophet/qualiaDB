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

/// Active drag, resize, pan, or wire drawing state across the canvas.
enum ActivePointerInteraction {
    DraggingContainer {
        container: Element,
        grab_dx: f32,
        grab_dy: f32,
    },
    ResizingContainer {
        container: Element,
        start_mx: f32,
        start_my: f32,
        orig_w: f32,
        orig_h: f32,
        style: String,
        zoom: f32,
    },
    PanningCanvas {
        canvas: Element,
        start_mx: f32,
        start_my: f32,
        start_pan_x: f32,
        start_pan_y: f32,
    },
    DraggingPort {
        source_container: Element,
        drag_svg: Element,
        drag_path: Element,
        start_x: f32,
        start_y: f32,
    },
}

// Thread-local for the currently active pointer interaction and selected container.
thread_local! {
    static ACTIVE_INTERACTION: RefCell<Option<ActivePointerInteraction>> = RefCell::new(None);
    static GLOBAL_LISTENERS_INITIALIZED: RefCell<bool> = RefCell::new(false);
    static SELECTED_CONTAINER: RefCell<Option<String>> = RefCell::new(None);
    static PENDING_WIRE_SOURCE: RefCell<Option<String>> = RefCell::new(None);
}

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
                                        super::canvas_extent::edge_pan_if_needed(&canvas, mx, my);
                                        let (wx, wy) =
                                            super::canvas_extent::client_to_world(&canvas, mx, my);
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
                            let zoom = super::canvas_extent::zoom_of(canvas);
                            super::canvas_extent::set_view(
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
                                    super::canvas_extent::client_to_world(&canvas, mx, my)
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
                        super::canvas_extent::ensure_manifold_extent(&doc);
                        super::history::push_current_frame("drag container");
                    }
                    ActivePointerInteraction::ResizingContainer { .. } => {
                        update_all_wires(&doc);
                        super::canvas_extent::ensure_manifold_extent(&doc);
                        super::history::push_current_frame("resize container");
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

/// Wire up port-to-port wire drawing. Drag from an output port to an
/// input port to create a connection wire between two containers.
pub fn wire_port_dragging(document: &Document) {
    init_global_pointer_listeners(document);

    let out_ports = document.query_selector_all(".port-out").unwrap();
    for i in 0..out_ports.length() {
        let port = out_ports.get(i).unwrap();
        let port_el: Element = port.dyn_into().unwrap();
        if !super::dom_bindings::claim(&port_el, "port") {
            continue;
        }

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

            if let Some(content) = canvas
                .query_selector(".canvas-content-layer")
                .ok()
                .flatten()
            {
                content.append_child(&drag_svg).unwrap();
            } else {
                canvas.append_child(&drag_svg).unwrap();
            }

            ACTIVE_INTERACTION.with(|ai| {
                *ai.borrow_mut() = Some(ActivePointerInteraction::DraggingPort {
                    source_container,
                    drag_svg,
                    drag_path,
                    start_x,
                    start_y,
                });
            });
        }) as Box<dyn FnMut(Event)>);

        port_el
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Create a permanent wire between two containers and prompt for semantic definition.
pub fn create_wire(document: &Document, source: &Element, target: &Element) {
    let canvas = match document.get_element_by_id("manifold-canvas") {
        Some(c) => c,
        None => return,
    };

    let src_id = source
        .get_attribute("data-id")
        .unwrap_or_else(|| "source".into());
    let tgt_id = target
        .get_attribute("data-id")
        .unwrap_or_else(|| "target".into());

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
            if let Some(content_layer) = canvas.query_selector(".canvas-content-layer").unwrap() {
                content_layer.append_child(&new_svg).unwrap();
            } else {
                canvas.append_child(&new_svg).unwrap();
            }
            new_svg
        }
    };

    // Generate a unique wire ID
    let wire_id = super::canvas_state::next_wire_id();
    let path = document.create_element_ns(Some(SVG_NS), "path").unwrap();
    path.set_attribute("d", &path_d).unwrap();
    path.set_attribute("class", "wire-active wire-selected")
        .unwrap();
    path.set_attribute("data-id", &wire_id).unwrap();
    path.set_attribute("data-source-id", &src_id).unwrap();
    path.set_attribute("data-target-id", &tgt_id).unwrap();
    path.set_attribute("data-predicate", "doc:references")
        .unwrap();
    path.set_attribute("data-modality", "active").unwrap();
    svg.append_child(&path).unwrap();

    // Midpoint label
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0 - 6.0;
    let text = document.create_element_ns(Some(SVG_NS), "text").unwrap();
    text.set_attribute("x", &mid_x.to_string()).unwrap();
    text.set_attribute("y", &mid_y.to_string()).unwrap();
    text.set_attribute("class", "wire-label-text").unwrap();
    text.set_attribute("data-wire-id", &wire_id).unwrap();
    text.set_text_content(Some("doc:references"));
    svg.append_child(&text).unwrap();

    // Re-wire wire inspector for the new wire
    super::wire_inspector::wire_wire_inspector(document);

    // Immediately open wire inspector so user can define semantics
    super::wire_inspector::show_inspector(document, &wire_id);

    super::canvas_extent::ensure_manifold_extent(document);
    super::history::push_current_frame("draw wire");

    // Show notification
    show_tool_notification(
        document,
        "wire-draw",
        &format!(
            "\u{26A1} Wire linked: [{}] \u{27F6} [{}] \u{00B7} Define Semantics",
            src_id, tgt_id
        ),
    );
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
        let src_id = &src.id;
        let tgt_id = &tgt.id;
        path.set_attribute("data-id", &conn.id).unwrap();
        path.set_attribute("data-source-id", &src_id).unwrap();
        path.set_attribute("data-target-id", &tgt_id).unwrap();
        path.set_attribute("data-predicate", &conn.label).unwrap();
        path.set_attribute("data-modality", &conn.wire_type)
            .unwrap();
        svg_el.append_child(&path).unwrap();

        // Midpoint label
        let mid_x = (start_x + end_x) / 2.0;
        let mid_y = (start_y + end_y) / 2.0 - 6.0;

        let text = document.create_element_ns(Some(SVG_NS), "text").unwrap();
        text.set_attribute("x", &mid_x.to_string()).unwrap();
        text.set_attribute("y", &mid_y.to_string()).unwrap();
        text.set_attribute("class", "wire-label-text").unwrap();
        text.set_attribute("data-wire-id", &conn.id).unwrap();
        text.set_text_content(Some(&conn.label));
        svg_el.append_child(&text).unwrap();
    }

    canvas.append_child(&svg).unwrap();
}

/// Dynamically recompute and update SVG paths and label positions for all wires on the canvas.
pub fn update_all_wires(document: &Document) {
    if let Ok(paths) = document.query_selector_all(".wire-overlay path") {
        for i in 0..paths.length() {
            if let Some(path) = paths.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                let src_id = path.get_attribute("data-source-id").unwrap_or_default();
                let tgt_id = path.get_attribute("data-target-id").unwrap_or_default();

                if src_id.is_empty() || tgt_id.is_empty() {
                    continue;
                }

                let src_opt = document
                    .query_selector(&format!(".canvas-container-node[data-id=\"{}\"]", src_id))
                    .ok()
                    .flatten();
                let tgt_opt = document
                    .query_selector(&format!(".canvas-container-node[data-id=\"{}\"]", tgt_id))
                    .ok()
                    .flatten();

                if let (Some(src), Some(tgt)) = (src_opt, tgt_opt) {
                    let src_style = src.get_attribute("style").unwrap_or_default();
                    let (src_x, src_y) = parse_position(&src_style);
                    let (src_w, src_h) = parse_size(&src_style);
                    let start_x = src_x + src_w;
                    let start_y = src_y + src_h / 2.0;

                    let tgt_style = tgt.get_attribute("style").unwrap_or_default();
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
                    let _ = path.set_attribute("d", &path_d);

                    // Update corresponding label text at midpoint
                    if let Some(svg) = path.parent_element() {
                        let wire_id = path.get_attribute("data-id").unwrap_or_default();
                        if let Ok(Some(label_el)) = svg.query_selector(&format!(
                            ".wire-label-text[data-wire-id=\"{}\"]",
                            wire_id
                        )) {
                            let mid_x = (start_x + end_x) / 2.0;
                            let mid_y = (start_y + end_y) / 2.0 - 6.0;
                            let _ = label_el.set_attribute("x", &mid_x.to_string());
                            let _ = label_el.set_attribute("y", &mid_y.to_string());
                        }
                    }
                }
            }
        }
    }
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
        if !super::dom_bindings::claim(&el, "selection") {
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
        .query_selector_all(".container-action-btn.delete-btn")
        .unwrap();
    for i in 0..close_btns.length() {
        let btn = close_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        if !super::dom_bindings::claim(&btn_el, "delete") {
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
    super::instrument_panel::hide(document);
    // Push a history frame
    super::history::push_current_frame("delete container");
    // Show a brief notification
    show_tool_notification(document, "delete-container", "Container deleted");
}

/// Delete a wire (SVG path) from the wire overlay, along with its label.
/// Pushes a history frame and hides the wire inspector.
fn delete_wire(document: &Document, wire_path: &Element) {
    remove_wire_and_label(document, wire_path);
    // Hide wire inspector
    super::wire_inspector::hide();
    // Push a history frame
    super::history::push_current_frame("delete wire");
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
        super::history::push_current_frame("duplicate container");
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

    super::history::push_current_frame("duplicate container");
    show_tool_notification(
        document,
        "dup",
        &format!("{} container(s) duplicated", count),
    );

    wire_container_selection(document);
    wire_container_dragging(document);
    wire_container_resize(document);
    wire_container_deletion(document);
    super::wire_inspector::wire_wire_inspector(document);
}

/// Apply zoom delta or absolute zoom scale to the canvas viewport.
pub fn apply_canvas_zoom(document: &Document, zoom_delta: f32, absolute: bool) {
    if let Some(canvas_el) = document.get_element_by_id("manifold-canvas") {
        let current_zoom = super::canvas_extent::zoom_of(&canvas_el);
        let (pan_x, pan_y) = super::canvas_extent::pan_of(&canvas_el);
        let new_zoom = if absolute {
            zoom_delta.clamp(0.15, 4.0)
        } else {
            (current_zoom + zoom_delta).clamp(0.15, 4.0)
        };
        super::canvas_extent::set_view(&canvas_el, pan_x, pan_y, new_zoom);
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

    super::history::push_current_frame("auto-arrange containers");
    show_tool_notification(document, "tidy", "Containers auto-arranged in clean grid");
}

/// Duplicate a single container — clone it with a 30px offset.
fn duplicate_container(document: &Document, container: &Element) {
    let mut model = super::canvas_state::container_from_element(container);
    model.id = super::canvas_state::next_container_id(&model.container_type);

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
    let clone_el = super::containers::build_container(document, &model);

    // Select the clone
    clone_el.class_list().add_1("selected").unwrap();

    // Append to the canvas content layer (or canvas as fallback)
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&clone_el).unwrap();
        } else {
            canvas.append_child(&clone_el).unwrap();
        }
        super::canvas_extent::pan_to_show(document, new_x, new_y, model.width, model.height);
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
    init_global_pointer_listeners(document);

    let headers = document.query_selector_all(".container-header").unwrap();
    for i in 0..headers.length() {
        let header = headers.get(i).unwrap();
        let header_el: Element = header.dyn_into().unwrap();
        if !super::dom_bindings::claim(&header_el, "drag") {
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
                        super::canvas_extent::client_to_world(&canvas, start_mx, start_my);
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
    if !super::dom_bindings::claim(&canvas, "pan-zoom") {
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
        let (start_pan_x, start_pan_y) = super::canvas_extent::pan_of(&canvas_clone);

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
        let cur = super::canvas_extent::zoom_of(&canvas_el);
        let new_zoom = (cur * factor).clamp(0.15, 4.0);
        let (wx, wy) = super::canvas_extent::client_to_world(
            &canvas_el,
            we.client_x() as f32,
            we.client_y() as f32,
        );
        let rect = canvas_el.get_bounding_client_rect();
        let pan_x = we.client_x() as f32 - rect.left() as f32 - wx * new_zoom;
        let pan_y = we.client_y() as f32 - rect.top() as f32 - wy * new_zoom;
        super::canvas_extent::set_view(&canvas_el, pan_x, pan_y, new_zoom);
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
        let header_for_toggle = header_el.clone();

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
                        let _ = header_for_toggle.set_attribute("aria-expanded", "false");
                        if let Ok(buttons) = children.query_selector_all(".toolbox-dock-btn") {
                            for index in 0..buttons.length() {
                                if let Some(node) = buttons.get(index) {
                                    if let Ok(button) = node.dyn_into::<Element>() {
                                        let _ = button.class_list().remove_1("active");
                                    }
                                }
                            }
                        }
                        super::docks::hide_flyout(&doc);
                    } else {
                        children.class_list().add_1("expanded").unwrap();
                        let _ = header_for_toggle.set_attribute("aria-expanded", "true");
                        // Open first toolbox in this family
                        if let Ok(Some(first_btn)) = children.query_selector(".toolbox-dock-btn") {
                            let tb_id = first_btn.get_attribute("data-toolbox").unwrap_or_default();
                            if !tb_id.is_empty() {
                                if let Ok(all) = doc.query_selector_all(".toolbox-dock-btn") {
                                    for index in 0..all.length() {
                                        if let Some(node) = all.get(index) {
                                            if let Ok(button) = node.dyn_into::<Element>() {
                                                let _ = button.class_list().remove_1("active");
                                            }
                                        }
                                    }
                                }
                                let _ = first_btn.class_list().add_1("active");
                                super::docks::show_flyout(&doc, &tb_id);
                                wire_flyout_tools(&doc);
                            }
                        }
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

/// Apply a real workspace grid layout for the selected toolbox anchor.
pub fn apply_toolbox_position(document: &Document, position: &str) {
    let position = match position {
        "top" | "right" | "bottom" => position,
        _ => "left",
    };
    if let Ok(Some(dock)) = document.query_selector(".toolbox-dock") {
        dock.set_class_name(&format!("toolbox-dock dock-pos-{}", position));
    }
    if let Ok(Some(workspace)) = document.query_selector(".main-workspace") {
        workspace.set_class_name(&format!("main-workspace dock-layout-{}", position));
    }
    if let Ok(Some(flyout)) = document.query_selector(".toolbox-flyout") {
        flyout.set_class_name(&format!("toolbox-flyout dock-{}", position));
    }
    if let Ok(buttons) = document.query_selector_all(".dock-pos-btn") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.get(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<HtmlElement>() else {
                continue;
            };
            let active = button.get_attribute("data-pos").as_deref() == Some(position);
            let _ = button.style().set_property(
                "color",
                if active {
                    "var(--accent-cyan)"
                } else {
                    "var(--text-muted)"
                },
            );
            let _ = button.style().set_property(
                "border-color",
                if active {
                    "var(--accent-cyan)"
                } else {
                    "transparent"
                },
            );
            let _ = button.set_attribute("aria-pressed", if active { "true" } else { "false" });
        }
    }
}

/// Wire tool button clicks and tool-chain selection/drag inside the flyout panel.
/// Placement is local; every other action is routed through the shared honest
/// action dispatcher.
/// Tool-chain labels are clickable (activate on focused surface) and draggable
/// (drag onto a container to activate there).
pub fn wire_flyout_tools(document: &Document) {
    // Wire tool-chain label clicks (select/activate chain)
    let chain_labels = document.query_selector_all(".toolchain-label").unwrap();
    for i in 0..chain_labels.length() {
        let label = chain_labels.get(i).unwrap();
        let label_el: Element = label.dyn_into().unwrap();
        if !super::dom_bindings::claim(&label_el, "flyout-chain") {
            continue;
        }
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
        if !super::dom_bindings::claim(&container_el, "chain-drop") {
            continue;
        }
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
        if !super::dom_bindings::claim(&tool_el, "flyout-tool") {
            continue;
        }
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
        let action = match tool_el.get_attribute("data-action").as_deref() {
            Some("query") => crate::tool_chest::core::intent_bus::ActionType::Query,
            Some("mutate") => crate::tool_chest::core::intent_bus::ActionType::Mutate,
            Some("publish") => crate::tool_chest::core::intent_bus::ActionType::Publish,
            Some("validate") => crate::tool_chest::core::intent_bus::ActionType::Validate,
            Some("navigate") => crate::tool_chest::core::intent_bus::ActionType::Navigate,
            Some("annotate") => crate::tool_chest::core::intent_bus::ActionType::Annotate,
            Some("cancel") => crate::tool_chest::core::intent_bus::ActionType::Cancel,
            _ => crate::tool_chest::core::intent_bus::ActionType::Invoke,
        };

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if kind_badge == "place" {
                // PlaceContainer tool — extract container type from tool_id
                // tool_id format: "toolbox:place_<type>" e.g. "office:place_doc"
                let container_type = tool_id.split("place_").nth(1).unwrap_or("doc").to_string();
                place_container_on_canvas(&doc, &container_type, &label);
            } else {
                super::tool_actions::dispatch(&doc, &tool_id, &label, action);
            }
        }) as Box<dyn FnMut(Event)>);

        tool_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Represents a container's 2D bounding box on the manifold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ContainerRect {
    pub fn overlaps(&self, other: &ContainerRect, margin: f32) -> bool {
        let left_a = self.x - margin;
        let right_a = self.x + self.w + margin;
        let top_a = self.y - margin;
        let bottom_a = self.y + self.h + margin;

        let left_b = other.x;
        let right_b = other.x + other.w;
        let top_b = other.y;
        let bottom_b = other.y + other.h;

        !(right_a <= left_b || left_a >= right_b || bottom_a <= top_b || top_a >= bottom_b)
    }
}

/// Parse all existing container rectangles from the DOM.
pub fn get_existing_container_rects(document: &Document) -> Vec<ContainerRect> {
    let mut rects = Vec::new();
    if let Ok(nodes) = document.query_selector_all(".canvas-container-node") {
        for i in 0..nodes.length() {
            if let Some(node) = nodes.get(i) {
                if let Ok(el) = node.dyn_into::<Element>() {
                    let style = el.get_attribute("style").unwrap_or_default();
                    let (x, y) = parse_position(&style);
                    let (w, h) = parse_size(&style);
                    rects.push(ContainerRect { x, y, w, h });
                }
            }
        }
    }
    rects
}

/// Find an optimal non-overlapping slot on the manifold for a new container.
/// The search is unbounded — new rows keep extending the lens instead of
/// clamping to a fixed 4×6 viewport grid.
pub fn find_smart_placement_slot(document: &Document, new_w: f32, new_h: f32) -> (f32, f32) {
    let existing_rects = get_existing_container_rects(document);

    let margin = 24.0;
    let cols = 4usize;
    let needed = existing_rects.len() + 1;
    let rows = (needed / cols + 2).max(6);
    for row in 0..rows {
        for col in 0..cols {
            let cand_x = 80.0 + (col as f32) * (new_w + 40.0);
            let cand_y = 60.0 + (row as f32) * (new_h + 40.0);
            let cand = ContainerRect {
                x: cand_x,
                y: cand_y,
                w: new_w,
                h: new_h,
            };

            let has_overlap = existing_rects.iter().any(|r| cand.overlaps(&r, margin));
            if !has_overlap {
                return (cand_x, cand_y);
            }
        }
    }

    let max_y = existing_rects
        .iter()
        .map(|r| r.y + r.h)
        .fold(0.0f32, f32::max);
    (80.0, max_y + 40.0)
}

/// Smoothly auto-arrange all containers on the current manifold into an organized non-overlapping grid.
pub fn auto_arrange_manifold(document: &Document) {
    reorganize_manifold_internal(document, false);
    super::history::push_current_frame("auto-arrange manifold");
    show_tool_notification(
        document,
        "auto-arrange",
        "Manifold containers auto-arranged \u{2728}",
    );
}

fn reorganize_manifold_internal(document: &Document, reserve_first_slot: bool) {
    let nodes = match document.query_selector_all(".canvas-container-node") {
        Ok(n) => n,
        Err(_) => return,
    };
    let total = nodes.length();
    if total == 0 {
        return;
    }

    let offset = if reserve_first_slot { 1 } else { 0 };
    for i in 0..total {
        if let Some(node) = nodes.get(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                // Apply smooth CSS transition class
                let _ = el.class_list().add_1("manifold-rearranging");

                let slot_idx = i + offset;
                let col = slot_idx % 3;
                let row = slot_idx / 3;
                let target_x = 80.0 + (col as f32) * 440.0;
                let target_y = 60.0 + (row as f32) * 340.0;

                let style = el.get_attribute("style").unwrap_or_default();
                let new_style = update_position(&style, target_x, target_y);
                let _ = el.set_attribute("style", &new_style);
            }
        }
    }

    super::canvas_extent::ensure_manifold_extent(document);

    // Schedule cleanup of transition class after 480ms
    let doc_clone = document.clone();
    let timeout = Closure::wrap(Box::new(move || {
        if let Ok(all) = doc_clone.query_selector_all(".canvas-container-node.manifold-rearranging")
        {
            for j in 0..all.length() {
                if let Some(n) = all.get(j) {
                    if let Ok(el) = n.dyn_into::<Element>() {
                        let _ = el.class_list().remove_1("manifold-rearranging");
                    }
                }
            }
        }
    }) as Box<dyn FnMut()>);
    set_timeout(timeout.as_ref().unchecked_ref(), 480);
    timeout.forget();
}

/// Place a new container on the canvas at an intelligent non-overlapping position.
/// Place a container on the canvas via a menu action (public, for topbar).
pub fn place_container_via_menu(document: &Document, container_type: &str, label: &str) {
    place_container_on_canvas(document, container_type, label);
}

fn place_container_on_canvas(document: &Document, container_type: &str, label: &str) {
    use crate::tool_chest::core::registry::SeedContainer;

    let width = 400.0;
    let height = 300.0;
    let (x, y) = find_smart_placement_slot(document, width, height);

    let next_z = HIGHEST_Z.fetch_add(1, Ordering::SeqCst) + 1;
    let container = SeedContainer {
        id: super::canvas_state::next_container_id(container_type),
        container_type: container_type.into(),
        title: label.trim_start_matches("+ ").to_string(),
        x,
        y,
        width,
        height,
        z: next_z as f32,
        honesty: "missing".into(),
        ..Default::default()
    };

    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::containers::build_container(document, &container);
        let _ = el.class_list().add_1("newly-placed");

        // Append to the content layer if it exists, otherwise to the canvas
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&el).unwrap();
        } else {
            canvas.append_child(&el).unwrap();
        }

        // Deselect all existing containers and select this newly placed one
        if let Ok(all) = document.query_selector_all(".canvas-container-node") {
            for j in 0..all.length() {
                if let Some(n) = all.get(j) {
                    if let Ok(ne) = n.dyn_into::<Element>() {
                        let _ = ne.class_list().remove_1("selected");
                    }
                }
            }
        }
        let _ = el.class_list().add_1("selected");

        // Re-wire interactions for the new container
        wire_container_selection(document);
        wire_container_dragging(document);
        wire_container_resize(document);
        wire_container_deletion(document);
        wire_port_dragging(document);

        super::canvas_extent::pan_to_show(document, x, y, width, height);
        super::history::push_current_frame("place container");
    }
}

/// Show a transient informational notification.
pub fn show_tool_notification(document: &Document, tool_id: &str, label: &str) {
    show_tool_status(document, label, tool_id, "info");
}

/// Show a transient action status without claiming unavailable work succeeded.
pub fn show_tool_status(document: &Document, title_text: &str, message: &str, status_kind: &str) {
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
    let glyph = match status_kind {
        "success" => "\u{2713}",
        "error" => "\u{26A0}",
        "running" => "\u{23F3}",
        "unavailable" => "\u{2298}",
        _ => "\u{1F4A1}",
    };
    title.set_text_content(Some(&format!("{} {}", glyph, title_text)));
    notif.append_child(&title).unwrap();

    let status = document.create_element("div").unwrap();
    status
        .set_attribute("style", "color: var(--text-muted); font-size: 10px;")
        .unwrap();
    status.set_text_content(Some(message));
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
    init_global_pointer_listeners(document);

    let handles = document
        .query_selector_all(".resize-handle, .container-resizer")
        .unwrap();
    for i in 0..handles.length() {
        let handle = handles.get(i).unwrap();
        let handle_el: Element = handle.dyn_into().unwrap();
        if !super::dom_bindings::claim(&handle_el, "resize") {
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

fn px(value: f32) -> String {
    format!("{}px", value.round() as i32)
}

fn update_position(style: &str, new_x: f32, new_y: f32) -> String {
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("left: ") {
            result.push_str(&format!("left: {}; ", px(new_x)));
        } else if part.starts_with("top: ") {
            result.push_str(&format!("top: {}; ", px(new_y)));
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !result.contains("left:") {
        result.push_str(&format!("left: {}; ", px(new_x)));
    }
    if !result.contains("top:") {
        result.push_str(&format!("top: {}; ", px(new_y)));
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
const GRID_SIZE: f32 = 8.0;

fn current_canvas_zoom(document: &Document) -> f32 {
    document
        .get_element_by_id("manifold-canvas")
        .and_then(|canvas| canvas.get_attribute("data-zoom"))
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0)
}

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

/// Snap a world position. The manifold is not a fixed box — coordinates
/// are not clamped to the current viewport.
pub fn snap_clamp_position(
    x: f32,
    y: f32,
    _canvas_w: f32,
    _canvas_h: f32,
    _elem_w: f32,
    _elem_h: f32,
) -> (f32, f32) {
    snap_point(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_grid() {
        assert_eq!(snap_to_grid(0.0), 0.0);
        assert_eq!(snap_to_grid(15.0), 16.0);
        assert_eq!(snap_to_grid(17.0), 16.0);
        assert_eq!(snap_to_grid(24.0), 24.0);
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

        let (x2, y2) = snap_clamp_position(-12.0, 700.0, 800.0, 600.0, 200.0, 150.0);
        assert_eq!(x2, -16.0);
        assert_eq!(y2, 704.0);
    }

    #[test]
    fn test_container_rect_overlaps() {
        let r1 = ContainerRect {
            x: 80.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        };
        let r2_overlapping = ContainerRect {
            x: 120.0,
            y: 100.0,
            w: 400.0,
            h: 300.0,
        };
        let r3_separate = ContainerRect {
            x: 520.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        };
        let r4_below = ContainerRect {
            x: 80.0,
            y: 400.0,
            w: 400.0,
            h: 300.0,
        };

        assert!(r1.overlaps(&r2_overlapping, 20.0));
        assert!(!r1.overlaps(&r3_separate, 20.0));
        assert!(!r1.overlaps(&r4_below, 20.0));
    }
}
