//! Spatial canvas: grid, pan/zoom, wires, draggable/resizable nodes.

use super::containers::NodeBody;
use super::kinds::CanvasNode;
use super::store::Workbench;
use dioxus::prelude::*;

#[derive(Clone, Debug)]
enum Gesture {
    Idle,
    Pan {
        sx: f64,
        sy: f64,
        px: f64,
        py: f64,
    },
    Move {
        id: String,
        sx: f64,
        sy: f64,
        nx: f64,
        ny: f64,
    },
    Resize {
        id: String,
        sx: f64,
        sy: f64,
        nw: f64,
        nh: f64,
    },
}

#[component]
pub fn CanvasStage(wb: Signal<Workbench>) -> Element {
    let w = wb();
    let mode = w.dim.css();
    let xf = w.stage_transform();
    let mut gesture = use_signal(|| Gesture::Idle);
    let zoom = w.zoom;
    let dragging = !matches!(gesture(), Gesture::Idle | Gesture::Pan { .. });
    rsx! {
        div {
            class: "canvas-viewport-container {mode}",
            id: "canvas-viewport",
            onmousedown: move |e| {
                let c = e.data().client_coordinates();
                let s = wb();
                gesture.set(Gesture::Pan {
                    sx: c.x,
                    sy: c.y,
                    px: s.pan_x,
                    py: s.pan_y,
                });
            },
            onmousemove: move |e| apply_move(wb, gesture(), e.data().client_coordinates().x, e.data().client_coordinates().y, zoom),
            onmouseup: move |_| gesture.set(Gesture::Idle),
            onmouseleave: move |_| gesture.set(Gesture::Idle),
            onwheel: move |e| {
                let delta = match e.data().delta() {
                    dioxus::html::geometry::WheelDelta::Pixels(p) => p.y,
                    dioxus::html::geometry::WheelDelta::Lines(l) => l.y * 80.0,
                    dioxus::html::geometry::WheelDelta::Pages(p) => p.y * 400.0,
                };
                let mut s = wb();
                let factor = if delta > 0.0 { 0.9 } else { 1.1 };
                s.zoom = (s.zoom * factor).clamp(0.3, 3.0);
                wb.set(s);
            },
            svg { class: "canvas-grid-svg",
                defs {
                    pattern { id: "grid-pattern", width: "40", height: "40", pattern_units: "userSpaceOnUse",
                        path { d: "M 40 0 L 0 0 0 40", fill: "none", stroke: "rgba(255,255,255,0.04)", "stroke-width": "1" }
                    }
                }
                rect { width: "100%", height: "100%", fill: "url(#grid-pattern)" }
            }
            div {
                class: if dragging { "canvas-stage live" } else { "canvas-stage" },
                id: "canvas-stage",
                style: "transform:{xf};",
                svg { class: "wires-svg-layer", id: "wires-layer",
                    for wire in w.wires.iter() {
                        if let (Some(a), Some(b)) = (w.node(&wire.from), w.node(&wire.to)) {
                            WirePath { a: a.clone(), b: b.clone(), kind: wire.kind.clone(), label: wire.label.clone() }
                        }
                    }
                }
                for node in w.nodes.iter() {
                    ContainerNode {
                        wb,
                        gesture,
                        node: node.clone(),
                        selected: w.selected.as_deref() == Some(node.id.as_str()),
                        dimmed: w.dimmed(node),
                        dragging: match gesture() {
                            Gesture::Move { ref id, .. } | Gesture::Resize { ref id, .. } => id == &node.id,
                            _ => false,
                        },
                    }
                }
            }
            div { class: "canvas-hud",
                button { class: "hud-btn", onclick: move |_| zoom_by(wb, 0.8), "-" }
                span { class: "zoom-level-text", "{(w.zoom * 100.0) as i32}%" }
                button { class: "hud-btn", onclick: move |_| zoom_by(wb, 1.2), "+" }
                button { class: "hud-btn", style: "font-size:11px;margin-left:4px;",
                    onclick: move |_| { let mut s = wb(); s.zoom = 0.9; s.pan_x = 70.0; s.pan_y = 40.0; wb.set(s); },
                    "Recenter"
                }
            }
        }
    }
}

fn zoom_by(mut wb: Signal<Workbench>, factor: f64) {
    let mut s = wb();
    s.zoom = (s.zoom * factor).clamp(0.3, 3.0);
    wb.set(s);
}

fn apply_move(mut wb: Signal<Workbench>, g: Gesture, x: f64, y: f64, zoom: f64) {
    let z = zoom.max(0.05);
    match g {
        Gesture::Idle => {}
        Gesture::Pan { sx, sy, px, py } => {
            let mut s = wb();
            s.pan_x = px + (x - sx);
            s.pan_y = py + (y - sy);
            wb.set(s);
        }
        Gesture::Move {
            id,
            sx,
            sy,
            nx,
            ny,
        } => {
            let mut s = wb();
            s.move_node(&id, nx + (x - sx) / z, ny + (y - sy) / z);
            wb.set(s);
        }
        Gesture::Resize {
            id,
            sx,
            sy,
            nw,
            nh,
        } => {
            let mut s = wb();
            s.resize_node(&id, nw + (x - sx) / z, nh + (y - sy) / z);
            wb.set(s);
        }
    }
}

#[component]
fn WirePath(a: CanvasNode, b: CanvasNode, kind: String, label: String) -> Element {
    let x1 = a.x + a.width;
    let y1 = a.y + a.height / 2.0;
    let x2 = b.x;
    let y2 = b.y + b.height / 2.0;
    let mx = (x1 + x2) / 2.0;
    let d = format!("M {x1} {y1} C {mx} {y1}, {mx} {y2}, {x2} {y2}");
    let class = format!("connection-wire wire-{kind}");
    rsx! {
        path { class: "{class}", d: "{d}" }
        text { class: "wire-label-text", x: "{mx}", y: "{(y1 + y2) / 2.0 - 6.0}", "{label}" }
    }
}

#[component]
fn ContainerNode(
    wb: Signal<Workbench>,
    gesture: Signal<Gesture>,
    node: CanvasNode,
    selected: bool,
    dimmed: bool,
    dragging: bool,
) -> Element {
    let id = node.id.clone();
    let id_move = node.id.clone();
    let id_resize = node.id.clone();
    let id_close = node.id.clone();
    let nx = node.x;
    let ny = node.y;
    let nw = node.width;
    let nh = node.height;
    let mut class = "canvas-container-node".to_string();
    if selected {
        class.push_str(" selected");
    }
    if dimmed {
        class.push_str(" strata-dimmed");
    }
    if dragging {
        class.push_str(" dragging");
    }
    let z_style = if node.z != 0.0 {
        format!(
            "transform:translateZ({}px) scale({});",
            node.z.min(80.0) / 20.0,
            node.d
        )
    } else {
        String::new()
    };
    rsx! {
        div {
            class: "{class}",
            id: "{node.id}",
            style: "left:{node.x}px;top:{node.y}px;width:{node.width}px;height:{node.height}px;{z_style}",
            onmousedown: move |e| {
                e.stop_propagation();
                let mut s = wb();
                s.selected = Some(id.clone());
                wb.set(s);
            },
            div {
                class: "container-header",
                onmousedown: move |e| {
                    e.stop_propagation();
                    let c = e.data().client_coordinates();
                    gesture.set(Gesture::Move {
                        id: id_move.clone(),
                        sx: c.x,
                        sy: c.y,
                        nx,
                        ny,
                    });
                },
                div { class: "container-title-group",
                    span { class: "container-type-tag tag-{node.kind.id()}", "{node.kind.id()}" }
                    span { class: "strata-badge {node.strata.css()}", "{node.strata.id()}" }
                    span { class: "modality-badge {node.epistemic.css()}", "{node.epistemic.icon()} {node.epistemic.id()}" }
                    span { class: "container-title", "{node.title}" }
                    span { class: "container-xyzd-badge", "[z:{node.z as i32}, d:{node.d}]" }
                }
                div { class: "container-actions",
                    button { class: "container-action-btn",
                        onclick: move |e| {
                            e.stop_propagation();
                            let mut s = wb();
                            s.close(&id_close);
                            wb.set(s);
                        },
                        "×"
                    }
                }
            }
            div { class: "container-body",
                NodeBody { kind: node.kind }
            }
            div { class: "container-port port-in" }
            div { class: "container-port port-out" }
            div {
                class: "container-resizer",
                title: "Resize",
                onmousedown: move |e| {
                    e.stop_propagation();
                    let c = e.data().client_coordinates();
                    gesture.set(Gesture::Resize {
                        id: id_resize.clone(),
                        sx: c.x,
                        sy: c.y,
                        nw,
                        nh,
                    });
                },
            }
        }
    }
}
