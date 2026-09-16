//! SVG node-graph chrome for the instrument flow (SI-09).
//! Keyboard model: [`super::graph_keys::GraphFocus`]. Pointer drag via
//! [`super::drag`]. Ctrl+Z/Y layout history via [`super::undo`]. No Host IDs.

use super::a11y::announce_select;
use super::drag::{apply_delta, clamp_center, hit_index, CANVAS_H, CANVAS_W, NODE_H, NODE_W};
use super::flow::{flow_for_entry, FlowNode};
use super::graph_keys::GraphFocus;
use super::persist;
use super::undo::LayoutUndo;
use super::undo_keys::{classify, LayoutKey};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, KeyboardEvent, MouseEvent, PointerEvent};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const HISTORY_MAX: usize = 32;

/// Decode an encoded layout of exactly `n` centres, clamped to the canvas.
pub fn centres_from_encoded(encoded: &str, n: usize) -> Option<Vec<(f32, f32)>> {
    if encoded.contains("Host") {
        return None;
    }
    let decoded = persist::decode_positions(encoded);
    if decoded.len() != n {
        return None;
    }
    Some(
        decoded
            .into_iter()
            .map(|(x, y)| clamp_center(x, y))
            .collect(),
    )
}

/// Evenly spaced (x, y) centres for `n` nodes along a horizontal rail.
pub fn layout_positions(n: usize) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let y = 56.0;
    let pad = 28.0;
    let usable = (CANVAS_W - pad * 2.0).max(NODE_W);
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.5
            } else {
                i as f32 / (n - 1) as f32
            };
            (pad + NODE_W * 0.5 + t * (usable - NODE_W), y)
        })
        .collect()
}

pub fn build_graph_view(document: &Document) -> Element {
    let nodes = flow_for_entry("assess");
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-graph");
    root.set_attribute("data-instrument-graph", "1").ok();
    root.set_attribute("role", "application").ok();
    root.set_attribute("aria-label", "instrument node graph").ok();
    root.set_attribute("tabindex", "0").ok();

    let svg = document.create_element_ns(Some(SVG_NS), "svg").unwrap();
    svg.set_attribute("width", &format!("{CANVAS_W}")).ok();
    svg.set_attribute("height", &format!("{CANVAS_H}")).ok();
    svg.set_attribute("viewBox", &format!("0 0 {CANVAS_W} {CANVAS_H}"))
        .ok();
    svg.set_attribute("data-instrument-graph-svg", "1").ok();

    let n = nodes.len();
    let mut positions = layout_positions(n);
    if let Some(saved) = document
        .document_element()
        .and_then(|e| e.get_attribute("data-si-graph-layout"))
    {
        if let Some(decoded) = centres_from_encoded(&saved, n) {
            positions = decoded;
        }
    }
    let origins = positions.clone();
    for (i, pair) in positions.windows(2).enumerate() {
        let (x1, y1) = pair[0];
        let (x2, y2) = pair[1];
        let line = document.create_element_ns(Some(SVG_NS), "line").unwrap();
        line.set_attribute("data-edge", &format!("{i}")).ok();
        line.set_attribute("x1", &format!("{:.1}", x1 + NODE_W * 0.45))
            .ok();
        line.set_attribute("y1", &format!("{y1:.1}")).ok();
        line.set_attribute("x2", &format!("{:.1}", x2 - NODE_W * 0.45))
            .ok();
        line.set_attribute("y2", &format!("{y2:.1}")).ok();
        line.set_attribute("stroke", "currentColor").ok();
        line.set_attribute("stroke-width", "2").ok();
        svg.append_child(&line).unwrap();
    }
    for (i, ((x, y), node)) in positions.iter().zip(nodes.iter()).enumerate() {
        let g = document.create_element_ns(Some(SVG_NS), "g").unwrap();
        g.set_attribute("tabindex", "0").ok();
        g.set_attribute("data-graph-index", &format!("{i}")).ok();
        g.set_attribute("data-cx", &format!("{x:.1}")).ok();
        g.set_attribute("data-cy", &format!("{y:.1}")).ok();
        g.set_attribute("aria-label", node.label).ok();
        g.set_attribute("role", "button").ok();
        let rect = document.create_element_ns(Some(SVG_NS), "rect").unwrap();
        rect.set_attribute("x", &format!("{:.1}", x - NODE_W * 0.5))
            .ok();
        rect.set_attribute("y", &format!("{:.1}", y - NODE_H * 0.5))
            .ok();
        rect.set_attribute("width", &format!("{NODE_W}")).ok();
        rect.set_attribute("height", &format!("{NODE_H}")).ok();
        rect.set_attribute("rx", "6").ok();
        rect.set_attribute("fill", "rgba(125,211,252,0.12)").ok();
        rect.set_attribute("stroke", "currentColor").ok();
        g.append_child(&rect).unwrap();
        let text = document.create_element_ns(Some(SVG_NS), "text").unwrap();
        text.set_attribute("x", &format!("{x:.1}")).ok();
        text.set_attribute("y", &format!("{:.1}", y + 4.0)).ok();
        text.set_attribute("text-anchor", "middle").ok();
        text.set_attribute("font-size", "11").ok();
        text.set_text_content(Some(node.label));
        g.append_child(&text).unwrap();
        svg.append_child(&g).unwrap();
    }
    root.append_child(&svg).unwrap();
    let centres = collect_centres(&svg, n);
    if centres.len() == n {
        write_layout(&root, &centres, false);
    }

    let history = Rc::new(RefCell::new(LayoutUndo::new(HISTORY_MAX)));
    if centres.len() == n {
        history
            .borrow_mut()
            .push(persist::encode_positions(&centres));
    }

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-graph-status", "1").ok();
    status.set_class_name("instrument-flow-status");
    if let Some(first) = nodes.first() {
        status.set_text_content(Some(first.label));
    }
    root.append_child(&status).unwrap();

    wire_keys(&root, &svg, nodes, &origins, history.clone());
    wire_pointer(&root, &svg, nodes, &origins, history);
    root
}

fn write_layout(root: &Element, pts: &[(f32, f32)], session: bool) {
    let encoded = persist::encode_positions(pts);
    root.set_attribute("data-layout", &encoded).ok();
    if !session {
        return;
    }
    if let Some(html) = root.owner_document().and_then(|d| d.document_element()) {
        html.set_attribute("data-si-graph-layout", &encoded).ok();
    }
}

fn attr_f32(el: &Element, name: &str) -> Option<f32> {
    el.get_attribute(name)?.parse().ok()
}

fn svg_local(svg: &Element, client_x: f32, client_y: f32) -> (f32, f32) {
    let r = svg.get_bounding_client_rect();
    (client_x - r.left() as f32, client_y - r.top() as f32)
}

fn client_xy(e: &web_sys::Event) -> Option<(f32, f32)> {
    if let Some(p) = e.dyn_ref::<PointerEvent>() {
        return Some((p.client_x() as f32, p.client_y() as f32));
    }
    let m = e.dyn_ref::<MouseEvent>()?;
    Some((m.client_x() as f32, m.client_y() as f32))
}

fn graph_group(svg: &Element, i: usize) -> Option<Element> {
    svg.query_selector(&format!("g[data-graph-index=\"{i}\"]"))
        .ok()
        .flatten()
}

fn collect_centres(svg: &Element, n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .filter_map(|i| {
            let g = graph_group(svg, i)?;
            Some((attr_f32(&g, "data-cx")?, attr_f32(&g, "data-cy")?))
        })
        .collect()
}

fn refresh_edges(svg: &Element, pts: &[(f32, f32)]) {
    for (i, pair) in pts.windows(2).enumerate() {
        let Some(line) = svg
            .query_selector(&format!("line[data-edge=\"{i}\"]"))
            .ok()
            .flatten()
        else {
            continue;
        };
        let (x1, y1) = pair[0];
        let (x2, y2) = pair[1];
        line.set_attribute("x1", &format!("{:.1}", x1 + NODE_W * 0.45))
            .ok();
        line.set_attribute("y1", &format!("{y1:.1}")).ok();
        line.set_attribute("x2", &format!("{:.1}", x2 - NODE_W * 0.45))
            .ok();
        line.set_attribute("y2", &format!("{y2:.1}")).ok();
    }
}

fn apply_centres(svg: &Element, origins: &[(f32, f32)], pts: &[(f32, f32)]) {
    for (i, &(x, y)) in pts.iter().enumerate() {
        let Some(g) = graph_group(svg, i) else {
            continue;
        };
        let Some(&origin) = origins.get(i) else {
            continue;
        };
        place_node(&g, origin, x, y);
    }
    refresh_edges(svg, pts);
}

fn restore_encoded(
    root: &Element,
    svg: &Element,
    origins: &[(f32, f32)],
    encoded: &str,
    n: usize,
) {
    let Some(pts) = centres_from_encoded(encoded, n) else {
        return;
    };
    apply_centres(svg, origins, &pts);
    write_layout(root, &pts, true);
}

fn commit_layout(
    root: &Element,
    svg: &Element,
    n: usize,
    history: &RefCell<LayoutUndo>,
) {
    let pts = collect_centres(svg, n);
    if pts.len() != n {
        return;
    }
    write_layout(root, &pts, true);
    history
        .borrow_mut()
        .push(persist::encode_positions(&pts));
}

fn set_select(root: &Element, i: usize, labels: &[String]) {
    let Some(label) = labels.get(i) else {
        return;
    };
    if label.contains("Host.") {
        return;
    }
    root.set_attribute("data-graph-focus", &format!("{i}")).ok();
    if let Some(status) = root.query_selector("[data-graph-status]").ok().flatten() {
        status.set_text_content(Some(&announce_select(label)));
    }
}

fn place_node(g: &Element, origin: (f32, f32), nx: f32, ny: f32) {
    g.set_attribute("data-cx", &format!("{nx:.1}")).ok();
    g.set_attribute("data-cy", &format!("{ny:.1}")).ok();
    let tx = nx - origin.0;
    let ty = ny - origin.1;
    g.set_attribute("transform", &format!("translate({tx:.1},{ty:.1})"))
        .ok();
}

fn labels_of(nodes: &[FlowNode]) -> Vec<String> {
    nodes.iter().map(|n| n.label.to_string()).collect()
}

fn wire_keys(
    root: &Element,
    svg: &Element,
    nodes: &[FlowNode],
    origins: &[(f32, f32)],
    history: Rc<RefCell<LayoutUndo>>,
) {
    let len = nodes.len();
    let labels = labels_of(nodes);
    let origins = origins.to_vec();
    let root_c = root.clone();
    let svg_c = svg.clone();
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let Ok(ke) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        match classify(&ke.key(), ke.ctrl_key(), ke.meta_key()) {
            LayoutKey::Undo => {
                if !history.borrow().can_undo() {
                    return;
                }
                ke.prevent_default();
                ke.stop_propagation();
                history.borrow_mut().undo();
                if let Some(prev) = history.borrow().current().map(str::to_string) {
                    restore_encoded(&root_c, &svg_c, &origins, &prev, len);
                }
            }
            LayoutKey::Redo => {
                if !history.borrow().can_redo() {
                    return;
                }
                ke.prevent_default();
                ke.stop_propagation();
                if let Some(next) = history.borrow_mut().redo() {
                    restore_encoded(&root_c, &svg_c, &origins, &next, len);
                }
            }
            LayoutKey::Ignore => {
                let mut focus = GraphFocus::new(len);
                if let Some(cur) = root_c.get_attribute("data-graph-focus") {
                    if let Ok(i) = cur.parse::<usize>() {
                        focus.index = i.min(len.saturating_sub(1));
                    }
                }
                focus.handle(&ke.key());
                set_select(&root_c, focus.index, &labels);
            }
        }
    }) as Box<dyn FnMut(_)>);
    root.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

fn wire_pointer(
    root: &Element,
    svg: &Element,
    nodes: &[FlowNode],
    origins: &[(f32, f32)],
    history: Rc<RefCell<LayoutUndo>>,
) {
    let n = nodes.len();
    let labels = labels_of(nodes);
    let origins = origins.to_vec();
    let last = Rc::new(Cell::new((0.0f32, 0.0f32)));

    {
        let svg_c = svg.clone();
        let root_c = root.clone();
        let labels_c = labels.clone();
        let last_c = last.clone();
        let down = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let Some((cx, cy)) = client_xy(&e) else {
                return;
            };
            let (px, py) = svg_local(&svg_c, cx, cy);
            last_c.set((px, py));
            let pts = collect_centres(&svg_c, n);
            if let Some(i) = hit_index(&pts, px, py) {
                svg_c.set_attribute("data-drag-index", &format!("{i}")).ok();
                set_select(&root_c, i, &labels_c);
            }
        }) as Box<dyn FnMut(_)>);
        svg.add_event_listener_with_callback("pointerdown", down.as_ref().unchecked_ref())
            .ok();
        down.forget();
    }

    {
        let svg_c = svg.clone();
        let origins_c = origins.clone();
        let last_c = last;
        let mv = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let Some(idx_s) = svg_c.get_attribute("data-drag-index") else {
                return;
            };
            let Ok(i) = idx_s.parse::<usize>() else {
                return;
            };
            let Some(g) = graph_group(&svg_c, i) else {
                return;
            };
            let Some((cx, cy)) = client_xy(&e) else {
                return;
            };
            let (px, py) = svg_local(&svg_c, cx, cy);
            let (lx, ly) = last_c.get();
            last_c.set((px, py));
            let cur_x = attr_f32(&g, "data-cx").unwrap_or(px);
            let cur_y = attr_f32(&g, "data-cy").unwrap_or(py);
            let (nx, ny) = apply_delta(cur_x, cur_y, px - lx, py - ly);
            let (nx, ny) = clamp_center(nx, ny);
            if let Some(&origin) = origins_c.get(i) {
                place_node(&g, origin, nx, ny);
            }
            let pts = collect_centres(&svg_c, n);
            if pts.len() == n {
                refresh_edges(&svg_c, &pts);
            }
        }) as Box<dyn FnMut(_)>);
        svg.add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref())
            .ok();
        mv.forget();
    }

    for ev in ["pointerup", "pointerleave"] {
        let svg_c = svg.clone();
        let root_c = root.clone();
        let history_c = history.clone();
        let up = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let was_drag = svg_c.get_attribute("data-drag-index").is_some();
            svg_c.remove_attribute("data-drag-index").ok();
            if was_drag {
                commit_layout(&root_c, &svg_c, n, &history_c);
            }
        }) as Box<dyn FnMut(_)>);
        svg.add_event_listener_with_callback(ev, up.as_ref().unchecked_ref())
            .ok();
        up.forget();
    }

    {
        let svg_c = svg.clone();
        let root_c = root.clone();
        let click = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let Some((cx, cy)) = client_xy(&e) else {
                return;
            };
            let (px, py) = svg_local(&svg_c, cx, cy);
            let pts = collect_centres(&svg_c, n);
            if let Some(i) = hit_index(&pts, px, py) {
                set_select(&root_c, i, &labels);
            }
        }) as Box<dyn FnMut(_)>);
        svg.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())
            .ok();
        click.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_four_nodes_increase_in_x() {
        let pts = layout_positions(4);
        assert_eq!(pts.len(), 4);
        assert!(pts[0].0 < pts[1].0 && pts[1].0 < pts[2].0 && pts[2].0 < pts[3].0);
        assert_eq!(pts[0].1, pts[3].1);
    }

    #[test]
    fn layout_empty_is_empty() {
        assert!(layout_positions(0).is_empty());
    }

    #[test]
    fn centres_from_encoded_round_trips_rail() {
        let pts = layout_positions(4);
        let encoded = persist::encode_positions(&pts);
        let back = centres_from_encoded(&encoded, 4).expect("count match");
        assert_eq!(back.len(), 4);
        for (a, b) in pts.iter().zip(back.iter()) {
            assert!((a.0 - b.0).abs() <= 0.02);
            assert!((a.1 - b.1).abs() <= 0.02);
        }
        assert!(centres_from_encoded(&encoded, 3).is_none());
        assert!(centres_from_encoded("Host.1,2.00;3.00,4.00", 2).is_none());
    }

    #[test]
    fn undo_current_is_previous_layout() {
        let mut u = LayoutUndo::new(8);
        let a = persist::encode_positions(&layout_positions(4));
        let mut moved = layout_positions(4);
        moved[0] = clamp_center(80.0, 40.0);
        let b = persist::encode_positions(&moved);
        u.push(a.clone());
        u.push(b.clone());
        assert!(u.can_undo());
        u.undo();
        assert_eq!(u.current(), Some(a.as_str()));
        let restored = centres_from_encoded(u.current().unwrap(), 4).unwrap();
        assert!((restored[0].0 - layout_positions(4)[0].0).abs() <= 0.02);
        assert_eq!(u.redo().as_deref(), Some(b.as_str()));
    }
}
