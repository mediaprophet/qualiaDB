//! Semantic wire creation, rendering, and refresh operations.

use super::*;

/// Wire up port-to-port wire drawing. Drag from an output port to an
/// input port to create a connection wire between two containers.
pub fn wire_port_dragging(document: &Document) {
    init_global_pointer_listeners(document);

    let out_ports = document.query_selector_all(".port-out").unwrap();
    for i in 0..out_ports.length() {
        let port = out_ports.get(i).unwrap();
        let port_el: Element = port.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&port_el, "port") {
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
    let wire_id = super::super::canvas_state::next_wire_id();
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
    super::super::wire_inspector::wire_wire_inspector(document);

    // Immediately open wire inspector so user can define semantics
    super::super::wire_inspector::show_inspector(document, &wire_id);

    super::super::canvas_extent::ensure_manifold_extent(document);
    super::super::history::push_current_frame("draw wire");

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
