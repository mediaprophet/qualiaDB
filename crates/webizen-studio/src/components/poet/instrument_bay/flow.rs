//! Studio SVG instrument flow (SI-09/10). Duplicate of Poet 4-node graph.
//! In-memory node drag. Studio does not depend on poet. No Host IDs.

use super::undo::{classify, LayoutKey, LayoutUndo};
use dioxus::prelude::*;
use webizen_studio::semantic_instruments::host_id_refused;

const HELD_HOST: &str = "unknown entry point (not a Host ID)";
const GRAPH_LABEL: &str = "instrument node graph";
const LAYOUT_ATTR: &str = "data-si-flow-layout";
const RAIL: [(f64, f64); 4] = [(50.0, 56.0), (170.0, 56.0), (290.0, 56.0), (410.0, 56.0)];
const X_MIN: f64 = 48.0;
const X_MAX: f64 = 472.0;
const Y_MIN: f64 = 20.0;
const Y_MAX: f64 = 100.0;

fn clamp_pos(x: f64, y: f64) -> (f64, f64) {
    (x.clamp(X_MIN, X_MAX), y.clamp(Y_MIN, Y_MAX))
}

/// Encode centres as `"x,y;x,y;..."` at two decimal places. No Host IDs.
fn encode_positions(pts: &[(f64, f64)]) -> String {
    let mut out = String::new();
    for (i, &(x, y)) in pts.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(&format!("{x:.2},{y:.2}"));
    }
    out
}

/// Decode centres. Skip malformed pairs and any token containing `Host`.
fn decode_positions(s: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for token in s.split(';') {
        let token = token.trim();
        if token.is_empty() || token.contains("Host") {
            continue;
        }
        let Some((xs, ys)) = token.split_once(',') else {
            continue;
        };
        if ys.contains(',') {
            continue;
        }
        let Ok(x) = xs.trim().parse::<f64>() else {
            continue;
        };
        let Ok(y) = ys.trim().parse::<f64>() else {
            continue;
        };
        if x.is_finite() && y.is_finite() {
            out.push((x, y));
        }
    }
    out
}

/// Restore a saved layout only when the decoded count matches `n`.
fn layout_for_count(encoded: &str, n: usize) -> Option<Vec<(f64, f64)>> {
    if encoded.contains("Host") {
        return None;
    }
    let decoded = decode_positions(encoded);
    if decoded.len() != n {
        return None;
    }
    Some(decoded.into_iter().map(|(x, y)| clamp_pos(x, y)).collect())
}

fn load_saved_layout(n: usize) -> Option<Vec<(f64, f64)>> {
    #[cfg(target_arch = "wasm32")]
    {
        let saved = web_sys::window()?
            .document()?
            .document_element()?
            .get_attribute(LAYOUT_ATTR)?;
        layout_for_count(&saved, n)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = n;
        None
    }
}

fn save_layout(pts: &[(f64, f64)]) {
    let encoded = encode_positions(pts);
    if encoded.is_empty() || encoded.contains("Host") {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(html) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = html.set_attribute(LAYOUT_ATTR, &encoded);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = encoded;
}

fn move_held(
    held: Signal<Option<usize>>,
    grab: Signal<(f64, f64)>,
    mut pos: Signal<Vec<(f64, f64)>>,
    client_x: f64,
    client_y: f64,
) {
    let Some(i) = held() else {
        return;
    };
    let (gx, gy) = grab();
    if let Some(slot) = pos.write().get_mut(i) {
        *slot = clamp_pos(client_x - gx, client_y - gy);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlowKind {
    Entry,
    InputShape,
    Logic,
    OutputShape,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FlowNode {
    kind: FlowKind,
    label: &'static str,
}

const ASSESS: &[FlowNode] = &[
    FlowNode { kind: FlowKind::Entry, label: "assess" },
    FlowNode { kind: FlowKind::InputShape, label: "input shape" },
    FlowNode { kind: FlowKind::Logic, label: "N3/CML logic" },
    FlowNode { kind: FlowKind::OutputShape, label: "output shape" },
];

fn nodes_for(entry: &str) -> &'static [FlowNode] {
    if host_id_refused(entry) {
        &[]
    } else {
        ASSESS
    }
}

#[component]
pub fn InstrumentFlow(entry: String) -> Element {
    let nodes = nodes_for(&entry);
    let mut focus = use_signal(|| 0usize);
    let mut status = use_signal(|| {
        nodes
            .first()
            .map(|n| n.label.to_string())
            .unwrap_or_else(|| HELD_HOST.into())
    });
    let mut pos = use_signal(|| load_saved_layout(RAIL.len()).unwrap_or_else(|| RAIL.to_vec()));
    let mut history = use_signal(|| {
        let mut u = LayoutUndo::new(32);
        u.push(encode_positions(
            &load_saved_layout(RAIL.len()).unwrap_or_else(|| RAIL.to_vec()),
        ));
        u
    });
    let mut held = use_signal(|| None::<usize>);
    let mut grab = use_signal(|| (0.0, 0.0));

    if nodes.is_empty() {
        return rsx! {
            div {
                role: "region",
                "aria-label": GRAPH_LABEL,
                class: "lexicon-held-gate",
                "data-node-count": "0",
                div { role: "status", "{HELD_HOST}" }
            }
        };
    }

    let pts = pos();
    rsx! {
        div {
            class: "instrument-graph",
            role: "application",
            "aria-label": GRAPH_LABEL,
            "data-node-count": "{nodes.len()}",
            "data-layout": "{encode_positions(&pts)}",
            tabindex: "0",
            onkeydown: move |e| {
                let key = e.key().to_string();
                let mods = e.data().modifiers();
                let ctrl = mods.ctrl() || mods.meta();
                match classify(&key, ctrl) {
                    LayoutKey::Undo => {
                        if !history.read().can_undo() {
                            return;
                        }
                        e.prevent_default();
                        e.stop_propagation();
                        history.write().undo();
                        if let Some(prev) = history.read().current().map(str::to_string) {
                            if let Some(pts) = layout_for_count(&prev, nodes.len()) {
                                pos.set(pts.clone());
                                save_layout(&pts);
                            }
                        }
                    }
                    LayoutKey::Redo => {
                        if !history.read().can_redo() {
                            return;
                        }
                        e.prevent_default();
                        e.stop_propagation();
                        if let Some(next) = history.write().redo() {
                            if let Some(pts) = layout_for_count(&next, nodes.len()) {
                                pos.set(pts.clone());
                                save_layout(&pts);
                            }
                        }
                    }
                    LayoutKey::Ignore => {
                        let mut i = focus();
                        match key.as_str() {
                            "ArrowRight" | "Right" if i + 1 < nodes.len() => i += 1,
                            "ArrowLeft" | "Left" if i > 0 => i -= 1,
                            _ => {}
                        }
                        focus.set(i);
                        if let Some(n) = nodes.get(i) {
                            status.set(n.label.to_string());
                        }
                    }
                }
            },
            onmousemove: move |e| {
                let c = e.data().client_coordinates();
                move_held(held, grab, pos, c.x, c.y);
            },
            onmouseup: move |_| {
                if held().is_some() {
                    let pts = pos();
                    save_layout(&pts);
                    history.write().push(encode_positions(&pts));
                }
                held.set(None);
            },
            onmouseleave: move |_| {
                if held().is_some() {
                    let pts = pos();
                    save_layout(&pts);
                    history.write().push(encode_positions(&pts));
                }
                held.set(None);
            },
            svg {
                width: "520",
                height: "120",
                view_box: "0 0 520 120",
                for i in 0..nodes.len().saturating_sub(1) {
                    line {
                        x1: "{pts[i].0 + 40.0}",
                        y1: "{pts[i].1}",
                        x2: "{pts[i + 1].0 - 40.0}",
                        y2: "{pts[i + 1].1}",
                        stroke: "currentColor",
                        stroke_width: "2",
                    }
                }
                for (i, node) in nodes.iter().copied().enumerate() {
                    g {
                        "data-flow-node": "{i}",
                        onmousedown: move |e| {
                            e.stop_propagation();
                            let c = e.data().client_coordinates();
                            let (nx, ny) = pos().get(i).copied().unwrap_or(RAIL[0]);
                            grab.set((c.x - nx, c.y - ny));
                            held.set(Some(i));
                            focus.set(i);
                            status.set(node.label.to_string());
                        },
                        onmousemove: move |e| {
                            let c = e.data().client_coordinates();
                            move_held(held, grab, pos, c.x, c.y);
                        },
                        rect {
                            x: "{pts[i].0 - 48.0}",
                            y: "{pts[i].1 - 18.0}",
                            width: "96",
                            height: "36",
                            rx: "6",
                            fill: if focus() == i { "rgba(125,211,252,0.28)" } else { "rgba(125,211,252,0.12)" },
                            stroke: "currentColor",
                        }
                        text {
                            x: "{pts[i].0}",
                            y: "{pts[i].1 + 4.0}",
                            text_anchor: "middle",
                            font_size: "11",
                            fill: "currentColor",
                            "{node.label}"
                        }
                    }
                }
            }
            div { role: "status", "{status}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_refused_empty() {
        assert!(nodes_for("Host.ClinicalRisk.framingham").is_empty());
        assert!(nodes_for("Host.Anything").is_empty());
        assert!(host_id_refused("Host.assess"));
        assert_eq!(HELD_HOST, "unknown entry point (not a Host ID)");
    }

    #[test]
    fn assess_has_four_nodes() {
        let n = nodes_for("assess");
        assert_eq!(n.len(), 4);
        assert!(matches!(n[0].kind, FlowKind::Entry));
        assert!(matches!(n[1].kind, FlowKind::InputShape));
        assert!(matches!(n[2].kind, FlowKind::Logic));
        assert!(matches!(n[3].kind, FlowKind::OutputShape));
    }

    #[test]
    fn labels_have_no_host() {
        for node in nodes_for("assess") {
            assert!(!node.label.is_empty());
            assert!(!node.label.contains("Host."));
        }
        assert!(!GRAPH_LABEL.contains("Host."));
    }

    #[test]
    fn clamp_pos_keeps_in_bounds() {
        assert_eq!(clamp_pos(50.0, 56.0), (50.0, 56.0));
        assert_eq!(clamp_pos(10.0, 0.0), (48.0, 20.0));
        assert_eq!(clamp_pos(500.0, 200.0), (472.0, 100.0));
        assert_eq!(clamp_pos(48.0, 20.0), (48.0, 20.0));
        assert_eq!(clamp_pos(472.0, 100.0), (472.0, 100.0));
        for &(x, y) in &RAIL {
            assert_eq!(clamp_pos(x, y), (x, y));
        }
    }

    #[test]
    fn flow_layout_round_trips_and_rejects_host() {
        let encoded = encode_positions(&RAIL);
        assert_eq!(encoded, "50.00,56.00;170.00,56.00;290.00,56.00;410.00,56.00");
        let back = layout_for_count(&encoded, 4).expect("count match");
        assert_eq!(back, RAIL);
        assert!(layout_for_count(&encoded, 3).is_none());
        assert!(layout_for_count("10.00,20.00;Host.1,2.00", 2).is_none());
        assert!(decode_positions("10.00,20.00;Host.1,2.00;30.00,40.00")
            == vec![(10.00, 20.00), (30.00, 40.00)]);
        assert!(!LAYOUT_ATTR.contains("Host"));
        let clamped = layout_for_count("10.00,0.00;170.00,56.00;290.00,56.00;500.00,200.00", 4)
            .expect("count match");
        assert_eq!(clamped[0], (48.0, 20.0));
        assert_eq!(clamped[3], (472.0, 100.0));
    }
}
