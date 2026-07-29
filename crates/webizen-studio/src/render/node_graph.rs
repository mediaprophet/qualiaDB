//! Node-relational presentation — SVG graph of panes and their bindings.

use crate::canvas_graph::{derive_graph_edges, edge_visual};
use crate::canvas_model::{Page, PanePlacement};
use dioxus::prelude::*;

/// Pixels per layout point (matches grid and spatial canvases).
const POINT_SCALE: f64 = 6.0;

#[derive(Clone, Debug, PartialEq)]
struct NodeLayout {
    idx: usize,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
}

fn pane_rect(pane: &PanePlacement) -> (f64, f64, f64, f64) {
    let w = (pane.w.max(20) as f64) * POINT_SCALE;
    let h = (pane.h.max(10) as f64) * POINT_SCALE;
    let x = pane.x as f64 * POINT_SCALE;
    let y = pane.y as f64 * POINT_SCALE;
    (x, y, w, h)
}

fn cubic_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    let dx = (x2 - x1).abs().max(40.0);
    format!(
        "M {x1:.1} {y1:.1} C {cx1:.1} {y1:.1}, {cx2:.1} {y2:.1}, {x2:.1} {y2:.1}",
        cx1 = x1 + dx * 0.4,
        cx2 = x2 - dx * 0.4,
    )
}

#[component]
pub fn NodeGraphCanvas(page: Page) -> Element {
    let layouts: Vec<NodeLayout> = page
        .panes
        .iter()
        .enumerate()
        .map(|(idx, pane)| {
            let (x, y, w, h) = pane_rect(pane);
            NodeLayout {
                idx,
                cx: x + w * 0.5,
                cy: y + h * 0.5,
                w,
                h,
            }
        })
        .collect();

    let edges = derive_graph_edges(&page.panes);

    let (svg_w, svg_h) = layouts.iter().fold((640.0_f64, 480.0_f64), |(mw, mh), n| {
        (
            mw.max(n.cx + n.w * 0.5 + 24.0),
            mh.max(n.cy + n.h * 0.5 + 24.0),
        )
    });

    rsx! {
        style {
            r#"
            @keyframes node-edge-flow {{
                from {{ stroke-dashoffset: 24; }}
                to {{ stroke-dashoffset: 0; }}
            }}
            .node-graph-edge-strong {{
                animation: node-edge-flow 2.4s linear infinite;
            }}
            "#
        }
        div {
            style: "position: relative; width: 100%; height: 100%; min-height: 500px; background: var(--qualia-bg, #0a0a0a); border: 1px solid var(--qualia-border, #333); border-radius: 12px; overflow: auto;",

            svg {
                width: "{svg_w}",
                height: "{svg_h}",
                style: "position: absolute; inset: 0; pointer-events: none; z-index: 0;",
                defs {
                    marker {
                        id: "node-graph-arrow",
                        marker_width: "8",
                        marker_height: "8",
                        ref_x: "6",
                        ref_y: "4",
                        orient: "auto",
                        path {
                            d: "M0,0 L8,4 L0,8 Z",
                            fill: "var(--qualia-accent, #f59e0b)",
                            opacity: "0.7",
                        }
                    }
                }
                for edge in edges.iter() {
                    if let (Some(from), Some(to)) = (
                        layouts.iter().find(|n| n.idx == edge.from_idx),
                        layouts.iter().find(|n| n.idx == edge.to_idx),
                    ) {
                        path {
                            key: "{edge.from_idx}-{edge.to_idx}-{edge.label}",
                            class: if edge.strength >= 2 { "node-graph-edge node-graph-edge-strong" } else { "node-graph-edge" },
                            d: "{cubic_path(from.cx, from.cy, to.cx, to.cy)}",
                            fill: "none",
                            stroke: "var(--qualia-accent, #f59e0b)",
                            stroke_width: "{edge_visual(edge.strength).0}",
                            opacity: "{edge_visual(edge.strength).1}",
                            stroke_dasharray: if edge.strength >= 2 { "8 4" } else { "none" },
                            style: "{edge_visual(edge.strength).2}",
                            "marker-end": "url(#node-graph-arrow)",
                        }
                    }
                }
            }

            for (idx, pane) in page.panes.iter().enumerate() {
                div {
                    key: "{idx}",
                    style: "position: absolute; left: {pane.x as f64 * POINT_SCALE}px; top: {pane.y as f64 * POINT_SCALE}px; width: {(pane.w.max(20) as f64) * POINT_SCALE}px; min-height: {(pane.h.max(10) as f64) * POINT_SCALE}px; background: var(--qualia-surface, #181818); border: 1px solid var(--qualia-accent, #0ff); border-radius: 8px; padding: 0.5rem; display: flex; flex-direction: column; box-shadow: 0 8px 16px rgba(0,0,0,0.3); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); z-index: 1;",
                    div {
                        style: "font-size: 0.75rem; font-weight: bold; color: var(--qualia-text, #fff); margin-bottom: 0.4rem; border-bottom: 1px solid var(--qualia-border, #333); padding-bottom: 0.2rem;",
                        "{pane.component_id}"
                    }
                    if !pane.data_bindings.is_empty() {
                        div {
                            style: "font-size: 0.65rem; color: var(--qualia-text-muted, #888); line-height: 1.4;",
                            for binding in pane.data_bindings.iter() {
                                div { "{binding}" }
                            }
                        }
                    }
                    if let Some(anchor) = pane.anchor.as_ref() {
                        div {
                            style: "font-size: 0.6rem; color: var(--qualia-accent); margin-top: 0.25rem;",
                            "↳ {anchor}"
                        }
                    }
                }
            }
        }
    }
}
