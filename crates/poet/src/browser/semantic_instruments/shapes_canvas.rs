//! Visual SHACL constraints as extra SVG nodes under the instrument pipeline.
//!
//! Not `owl:Thing`. Not `Host.*`. Nodes come from
//! [`super::shapes::input_constraints`] / [`output_constraints`].
//! Centres are clamped via [`super::drag::clamp_center`] to 520×160.

use super::a11y::announce_select;
use super::drag::{clamp_center, CANVAS_H, CANVAS_W, NODE_H, NODE_W};
use super::graph_keys::GraphFocus;
use super::shapes::{input_constraints, living_safe_guard, output_constraints, ShapeConstraint};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const INPUT_Y: f32 = 40.0;
const OUTPUT_Y: f32 = 110.0;
const PAD: f32 = 28.0;

fn is_owl_thing(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower == "owl:thing" || lower.ends_with("#thing") || lower.contains("owl:thing")
}

fn keep_constraint(c: &ShapeConstraint) -> bool {
    !is_owl_thing(c.path)
        && !c.path.contains("Host.")
        && !c.message.contains("Host.")
        && !is_owl_thing(c.message)
}

fn living_safe_row(constraints: &'static [ShapeConstraint]) -> Vec<&'static ShapeConstraint> {
    constraints.iter().filter(|c| keep_constraint(c)).collect()
}

/// Living-safe constraint paths for an entry point (input then output).
/// `Host.*` and unknown entries yield an empty lens.
pub fn canvas_nodes(entry: &str) -> Vec<&'static str> {
    living_safe_row(input_constraints(entry))
        .into_iter()
        .chain(living_safe_row(output_constraints(entry)))
        .map(|c| c.path)
        .collect()
}

fn row_centres(n: usize, y: f32) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let usable = (CANVAS_W - PAD * 2.0).max(NODE_W);
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.5
            } else {
                i as f32 / (n - 1) as f32
            };
            let x = PAD + NODE_W * 0.5 + t * (usable - NODE_W);
            clamp_center(x, y)
        })
        .collect()
}

fn svg_el(document: &web_sys::Document, name: &str) -> Element {
    document.create_element_ns(Some(SVG_NS), name).unwrap()
}

fn append_node(
    document: &web_sys::Document,
    svg: &Element,
    root: &Element,
    c: &ShapeConstraint,
    x: f32,
    y: f32,
) {
    let g = svg_el(document, "g");
    g.set_attribute("tabindex", "0").ok();
    g.set_attribute("role", "button").ok();
    g.set_attribute("aria-label", c.path).ok();
    g.set_attribute("data-shape-path", c.path).ok();
    g.set_attribute("data-shape-message", c.message).ok();

    let rect = svg_el(document, "rect");
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

    let text = svg_el(document, "text");
    text.set_attribute("x", &format!("{x:.1}")).ok();
    text.set_attribute("y", &format!("{:.1}", y + 4.0)).ok();
    text.set_attribute("text-anchor", "middle").ok();
    text.set_attribute("font-size", "11").ok();
    text.set_text_content(Some(c.path));
    g.append_child(&text).unwrap();

    svg.append_child(&g).unwrap();
    wire_click(root, &g, c.message);
}

fn append_row(
    document: &web_sys::Document,
    svg: &Element,
    root: &Element,
    constraints: &[&ShapeConstraint],
    y: f32,
) {
    let pts = row_centres(constraints.len(), y);
    for (c, (x, y)) in constraints.iter().zip(pts.iter()) {
        append_node(document, svg, root, c, *x, *y);
    }
}

fn wire_click(root: &Element, node: &Element, message: &'static str) {
    let root_c = root.clone();
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        if let Some(status) = root_c.query_selector("[role=status]").ok().flatten() {
            status.set_text_content(Some(message));
        }
    }) as Box<dyn FnMut(_)>);
    node.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

fn set_focus_status(root: &Element, i: usize, paths: &[&str]) {
    let Some(path) = paths.get(i) else {
        return;
    };
    if path.contains("Host.") {
        return;
    }
    root.set_attribute("data-shapes-focus", &format!("{i}")).ok();
    if let Some(status) = root.query_selector("[role=status]").ok().flatten() {
        status.set_text_content(Some(&announce_select(path)));
    }
}

fn wire_keys(root: &Element) {
    let paths = canvas_nodes("assess");
    let len = paths.len();
    let root_c = root.clone();
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let Ok(ke) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        let key = ke.key();
        if key != "ArrowLeft" && key != "ArrowRight" {
            return;
        }
        let mut focus = GraphFocus::new(len);
        if let Some(cur) = root_c.get_attribute("data-shapes-focus") {
            if let Ok(i) = cur.parse::<usize>() {
                focus.index = i.min(len.saturating_sub(1));
            }
        }
        focus.handle(&key);
        set_focus_status(&root_c, focus.index, &paths);
    }) as Box<dyn FnMut(_)>);
    root.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

/// Extra SVG shape-constraint canvas under the pipeline. Client-side chrome.
pub fn build_shapes_canvas(document: &web_sys::Document) -> web_sys::Element {
    let inputs = living_safe_row(input_constraints("assess"));
    let outputs = living_safe_row(output_constraints("assess"));

    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-shapes-canvas");
    root.set_attribute("data-shapes-canvas", "1").ok();
    root.set_attribute("role", "application").ok();
    root.set_attribute("tabindex", "0").ok();
    root.set_attribute("aria-label", "instrument shape canvas")
        .ok();

    let svg = svg_el(document, "svg");
    svg.set_attribute("width", &format!("{CANVAS_W}")).ok();
    svg.set_attribute("height", &format!("{CANVAS_H}")).ok();
    svg.set_attribute("viewBox", &format!("0 0 {CANVAS_W} {CANVAS_H}"))
        .ok();
    svg.set_attribute("data-shapes-canvas-svg", "1").ok();

    append_row(document, &svg, &root, &inputs, INPUT_Y);
    append_row(document, &svg, &root, &outputs, OUTPUT_Y);
    root.append_child(&svg).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-shapes-canvas-status", "1").ok();
    status.set_class_name("instrument-shapes-canvas-status");
    status.set_text_content(Some(living_safe_guard()));
    root.append_child(&status).unwrap();

    wire_keys(&root);
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assess_canvas_count_matches_input_plus_output_lens() {
        let nodes = canvas_nodes("assess");
        assert_eq!(
            nodes.len(),
            input_constraints("assess").len() + output_constraints("assess").len()
        );
        assert_eq!(nodes, canvas_nodes("recognise"));
        assert_eq!(nodes[0], "si:inputShape");
        assert!(nodes.contains(&"si:resultKind"));
    }

    #[test]
    fn host_entries_are_empty() {
        assert!(canvas_nodes("Host.").is_empty());
        assert!(canvas_nodes("Host.assess").is_empty());
        assert!(canvas_nodes("Host.recognise").is_empty());
        assert!(canvas_nodes("Host.Instrument.assess").is_empty());
        assert!(canvas_nodes("").is_empty());
        assert!(canvas_nodes("run").is_empty());
    }

    #[test]
    fn living_safe_paths_are_not_owl_thing_or_host() {
        for entry in ["assess", "recognise"] {
            for path in canvas_nodes(entry) {
                assert!(!is_owl_thing(path));
                assert!(!path.contains("Host."));
                assert!(!path.contains("owl:Thing"));
            }
        }
        assert!(is_owl_thing("owl:Thing"));
        assert!(!keep_constraint(&ShapeConstraint {
            path: "owl:Thing",
            min_count: 1,
            message: "no",
        }));
    }

    #[test]
    fn input_row_is_y_forty_output_is_y_one_ten() {
        let ins = row_centres(2, INPUT_Y);
        let outs = row_centres(2, OUTPUT_Y);
        assert_eq!(ins.len(), 2);
        assert_eq!(outs.len(), 2);
        assert_eq!(ins[0].1, INPUT_Y);
        assert_eq!(ins[1].1, INPUT_Y);
        assert_eq!(outs[0].1, OUTPUT_Y);
        assert_eq!(outs[1].1, OUTPUT_Y);
        assert!(ins[0].0 < ins[1].0);
        assert!(outs[0].0 < outs[1].0);
        assert_eq!(row_centres(0, INPUT_Y).len(), 0);
    }

    #[test]
    fn assess_arrows_stay_on_canvas_paths() {
        let paths = canvas_nodes("assess");
        let mut focus = GraphFocus::new(paths.len());
        focus.handle("ArrowLeft");
        assert_eq!(focus.index, 0);
        assert_eq!(announce_select(paths[0]), format!("selected {}", paths[0]));
        for _ in 0..paths.len() + 2 {
            focus.handle("ArrowRight");
        }
        assert_eq!(focus.index, paths.len().saturating_sub(1));
        let path = paths[focus.index];
        assert!(!path.contains("Host."));
        assert_eq!(announce_select(path), format!("selected {path}"));
    }
}
