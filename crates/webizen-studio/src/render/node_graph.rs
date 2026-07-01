//! Node-relational presentation — SVG graph of panes and their bindings.

use dioxus::prelude::*;
use crate::canvas_model::{Page, PanePlacement};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GraphEdge {
    from_idx: usize,
    to_idx: usize,
    label: String,
}

fn pane_rect(pane: &PanePlacement) -> (f64, f64, f64, f64) {
    let w = (pane.w.max(20) as f64) * POINT_SCALE;
    let h = (pane.h.max(10) as f64) * POINT_SCALE;
    let x = pane.x as f64 * POINT_SCALE;
    let y = pane.y as f64 * POINT_SCALE;
    (x, y, w, h)
}

/// Collect directed edges from anchor links and shared data_bindings.
fn derive_graph_edges(panes: &[PanePlacement]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let index_by_id: std::collections::HashMap<&str, usize> = panes
        .iter()
        .enumerate()
        .map(|(i, p)| (p.component_id.as_str(), i))
        .collect();

    for (to_idx, pane) in panes.iter().enumerate() {
        if let Some(anchor) = pane.anchor.as_deref() {
            if let Some(&from_idx) = index_by_id.get(anchor) {
                let key = (from_idx, to_idx, "anchor".to_string());
                if seen.insert(key.clone()) {
                    edges.push(GraphEdge {
                        from_idx,
                        to_idx,
                        label: "anchor".to_string(),
                    });
                }
            }
        }

        for binding in &pane.data_bindings {
            for (from_idx, other) in panes.iter().enumerate() {
                if from_idx == to_idx {
                    continue;
                }
                if other.data_bindings.iter().any(|b| b == binding) {
                    let key = (from_idx.min(to_idx), from_idx.max(to_idx), binding.clone());
                    if seen.insert(key) {
                        edges.push(GraphEdge {
                            from_idx,
                            to_idx,
                            label: binding.clone(),
                        });
                    }
                }
            }
        }
    }

    edges
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
                            d: "{cubic_path(from.cx, from.cy, to.cx, to.cy)}",
                            fill: "none",
                            stroke: "var(--qualia-accent, #f59e0b)",
                            stroke_width: "1.5",
                            opacity: "0.55",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_model::LayerBehavior;

    fn pane(cid: &str, anchor: Option<&str>, bindings: &[&str]) -> PanePlacement {
        PanePlacement {
            component_id: cid.to_string(),
            x: 0,
            y: 0,
            w: 10,
            h: 8,
            data_bindings: bindings.iter().map(|s| s.to_string()).collect(),
            binds_rpc: None,
            requires_capability: vec![],
            ui_mode: None,
            layer: LayerBehavior::Docked,
            anchor: anchor.map(str::to_string),
            min_w_points: 0,
            min_h_points: 0,
            supported_presentations: vec![],
            theme: Default::default(),
        }
    }

    #[test]
    fn anchor_creates_directed_edge() {
        let panes = vec![
            pane("chat", None, &[]),
            pane("overlay", Some("chat"), &[]),
        ];
        let edges = derive_graph_edges(&panes);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_idx, 0);
        assert_eq!(edges[0].to_idx, 1);
    }

    #[test]
    fn shared_binding_creates_edge() {
        let panes = vec![
            pane("a", None, &["did:q42:user#chat"]),
            pane("b", None, &["did:q42:user#chat"]),
        ];
        let edges = derive_graph_edges(&panes);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].label, "did:q42:user#chat");
    }
}