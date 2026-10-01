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
    /// Dragging a wire out of a port: (sx, sy) client start, (ax, ay)
    /// anchor port in stage coords, (x, y) live preview end in stage coords.
    Wire {
        from: String,
        sx: f64,
        sy: f64,
        ax: f64,
        ay: f64,
        x: f64,
        y: f64,
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
    // Live wire-drag preview — stage coords, drawn in the transformed layer.
    let wire_preview = match gesture() {
        Gesture::Wire { from, x, y, .. } => w.node(&from).map(|a| {
            let x1 = a.x + a.width;
            let y1 = a.y + a.height / 2.0;
            let mx = (x1 + x) / 2.0;
            (format!("M {x1} {y1} C {mx} {y1}, {mx} {y}, {x} {y}"), x, y)
        }),
        _ => None,
    };
    rsx! {
        div {
            class: "canvas-viewport-container {mode}",
            id: "canvas-viewport",
            onmousedown: move |e| {
                if e.data().trigger_button()
                    != Some(dioxus::html::input_data::MouseButton::Primary)
                {
                    return;
                }
                let c = e.data().client_coordinates();
                let mut s = wb();
                // Click on empty canvas: deselect and cancel an armed wire.
                s.clear_selection();
                s.wire_source = None;
                let (px, py) = (s.pan_x, s.pan_y);
                wb.set(s);
                gesture.set(Gesture::Pan {
                    sx: c.x,
                    sy: c.y,
                    px,
                    py,
                });
            },
            onmousemove: move |e| {
                let c = e.data().client_coordinates();
                if let Gesture::Wire { from, sx, sy, ax, ay, .. } = gesture() {
                    let z = zoom.max(0.05);
                    gesture.set(Gesture::Wire {
                        from,
                        sx,
                        sy,
                        ax,
                        ay,
                        x: ax + (c.x - sx) / z,
                        y: ay + (c.y - sy) / z,
                    });
                } else {
                    apply_move(wb, gesture(), c.x, c.y, zoom);
                }
            },
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
                            WirePath {
                                wb,
                                id: wire.id.clone(),
                                a: a.clone(),
                                b: b.clone(),
                                kind: wire.kind.clone(),
                                label: wire.label.clone(),
                                selected: w.selected_wire.as_deref() == Some(wire.id.as_str()),
                            }
                        }
                    }
                    if let Some((d, ex, ey)) = wire_preview {
                        path {
                            class: "wire-preview",
                            d: "{d}",
                            style: "fill:none;stroke:var(--accent-cyan,#38bdf8);stroke-width:2;stroke-dasharray:6 4;",
                        }
                        circle { cx: "{ex}", cy: "{ey}", r: "4", fill: "var(--accent-cyan, #38bdf8)" }
                    }
                }
                for node in w.nodes.iter() {
                    ContainerNode {
                        wb,
                        gesture,
                        node: node.clone(),
                        selected: w.selected.as_deref() == Some(node.id.as_str()),
                        dimmed: w.dimmed(node),
                        armed: w.wire_source.as_deref() == Some(node.id.as_str()),
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
        Gesture::Move { id, sx, sy, nx, ny } => {
            let mut s = wb();
            s.move_node(&id, nx + (x - sx) / z, ny + (y - sy) / z);
            wb.set(s);
        }
        Gesture::Resize { id, sx, sy, nw, nh } => {
            let mut s = wb();
            s.resize_node(&id, nw + (x - sx) / z, nh + (y - sy) / z);
            wb.set(s);
        }
        // Wire drags update the preview point in the onmousemove closure —
        // the gesture carries no workbench state.
        Gesture::Wire { .. } => {}
    }
}

#[component]
fn WirePath(
    wb: Signal<Workbench>,
    id: String,
    a: CanvasNode,
    b: CanvasNode,
    kind: String,
    label: String,
    selected: bool,
) -> Element {
    let x1 = a.x + a.width;
    let y1 = a.y + a.height / 2.0;
    let x2 = b.x;
    let y2 = b.y + b.height / 2.0;
    let mx = (x1 + x2) / 2.0;
    let d = format!("M {x1} {y1} C {mx} {y1}, {mx} {y2}, {x2} {y2}");
    let class = if selected {
        format!("connection-wire wire-{kind} selected-wire")
    } else {
        format!("connection-wire wire-{kind}")
    };
    let label_sel = id.clone();
    let label_edit = id.clone();
    rsx! {
        g { class: "wire-group",
            // Fat invisible hit-target so wires are actually clickable.
            path {
                d: "{d}",
                fill: "none",
                stroke: "transparent",
                "stroke-width": "12",
                style: "cursor:pointer;",
                onclick: move |e| {
                    e.stop_propagation();
                    let mut s = wb();
                    s.select_wire(&id);
                    wb.set(s);
                },
            }
            path { class: "{class}", d: "{d}" }
            circle { cx: "{x1}", cy: "{y1}", r: "4", class: "wire-port-out", fill: "var(--accent-cyan, #38bdf8)" }
            circle { cx: "{x2}", cy: "{y2}", r: "4", class: "wire-port-in", fill: "var(--accent-emerald, #34d399)" }
            circle { cx: "{mx}", cy: "{(y1 + y2) / 2.0}", r: "3", class: "wire-pulse-particle", fill: "#f8fafc" }
            text {
                class: "wire-label-text",
                x: "{mx}",
                y: "{(y1 + y2) / 2.0 - 6.0}",
                style: "cursor:text;",
                onclick: move |e| {
                    e.stop_propagation();
                    let mut s = wb();
                    s.select_wire(&label_sel);
                    wb.set(s);
                },
                ondoubleclick: move |e| {
                    e.stop_propagation();
                    if let Some(label) =
                        super::host::prompt_text("Wire predicate (e.g. qualia:groundsObservation)")
                    {
                        let mut s = wb();
                        s.rename_wire(&label_edit, label);
                        wb.set(s);
                    }
                },
                "{label}"
            }
        }
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
    armed: bool,
) -> Element {
    let id = node.id.clone();
    let id_move = node.id.clone();
    let id_resize = node.id.clone();
    let id_close = node.id.clone();
    let id_wire = node.id.clone();
    let id_drop = node.id.clone();
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
    let armed_style = if armed {
        "box-shadow:0 0 0 2px var(--accent-gold, #F5A623);"
    } else {
        ""
    };
    rsx! {
        div {
            class: "{class}",
            id: "{node.id}",
            style: "left:{node.x}px;top:{node.y}px;width:{node.width}px;height:{node.height}px;{z_style}{armed_style}",
            onmousedown: move |e| {
                e.stop_propagation();
                let mut s = wb();
                match s.wire_source.clone() {
                    // Armed radial connect: clicking a different node completes it.
                    Some(src) if src != id => {
                        s.wire_source = None;
                        if s.connect_wire(&src, &id).is_some() {
                            s.note(format!("Wire connected → {id}"));
                        }
                    }
                    _ => {
                        s.wire_source = None;
                        s.select_node(&id);
                    }
                }
                wb.set(s);
            },
            onmouseup: move |e| {
                // Drop target for a port-dragged wire.
                if let Gesture::Wire { from, .. } = gesture() {
                    e.stop_propagation();
                    if from != id_drop {
                        let mut s = wb();
                        if s.connect_wire(&from, &id_drop).is_some() {
                            s.note(format!("Wire connected → {id_drop}"));
                        }
                        wb.set(s);
                    }
                    gesture.set(Gesture::Idle);
                }
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
            div {
                class: "container-port port-in",
                title: "Wire input — drop a connection here",
            }
            div {
                class: "container-port port-out",
                title: "Drag to connect a wire",
                style: "cursor:crosshair;",
                onmousedown: move |e| {
                    e.stop_propagation();
                    let c = e.data().client_coordinates();
                    gesture.set(Gesture::Wire {
                        from: id_wire.clone(),
                        sx: c.x,
                        sy: c.y,
                        ax: nx + nw,
                        ay: ny + nh / 2.0,
                        x: nx + nw,
                        y: ny + nh / 2.0,
                    });
                },
            }
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
