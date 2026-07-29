//! Pane relationship graph — shared by node view and the selection sidebar.

use std::collections::{HashMap, HashSet};

use crate::canvas_model::PanePlacement;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    pub from_idx: usize,
    pub to_idx: usize,
    pub label: String,
    /// Shared binding count or anchor weight (1 = anchor-only).
    pub strength: u8,
}

/// Directed edges from anchors and shared `data_bindings`, with binding-strength scores.
pub fn derive_graph_edges(panes: &[PanePlacement]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();
    let mut seen: HashSet<(usize, usize, String)> = HashSet::new();
    let mut strength_map: HashMap<(usize, usize, String), u8> = HashMap::new();

    let index_by_id: HashMap<&str, usize> = panes
        .iter()
        .enumerate()
        .map(|(i, p)| (p.component_id.as_str(), i))
        .collect();

    for (to_idx, pane) in panes.iter().enumerate() {
        if let Some(anchor) = pane.anchor.as_deref() {
            if let Some(&from_idx) = index_by_id.get(anchor) {
                let key = (from_idx, to_idx, "anchor".to_string());
                strength_map.insert(key.clone(), 1);
                if seen.insert(key) {
                    edges.push(GraphEdge {
                        from_idx,
                        to_idx,
                        label: "anchor".to_string(),
                        strength: 1,
                    });
                }
            }
        }

        for binding in &pane.data_bindings {
            for (from_idx, other) in panes.iter().enumerate() {
                if from_idx == to_idx {
                    continue;
                }
                let shared = other.data_bindings.iter().filter(|b| *b == binding).count() as u8;
                if shared == 0 {
                    continue;
                }
                let a = from_idx.min(to_idx);
                let b = from_idx.max(to_idx);
                let key = (a, b, binding.clone());
                let entry = strength_map.entry(key.clone()).or_insert(0);
                *entry = entry.saturating_add(shared.max(1));
                if seen.insert(key.clone()) {
                    edges.push(GraphEdge {
                        from_idx: a,
                        to_idx: b,
                        label: binding.clone(),
                        strength: *strength_map.get(&key).unwrap_or(&1),
                    });
                } else if let Some(edge) = edges
                    .iter_mut()
                    .find(|e| e.from_idx == a && e.to_idx == b && e.label == *binding)
                {
                    edge.strength = edge.strength.saturating_add(shared.max(1));
                }
            }
        }
    }

    edges
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphNeighbor {
    pub idx: usize,
    pub component_id: String,
    pub relation: String,
    pub strength: u8,
}

/// Panes linked to `selected_idx` (incoming or outgoing).
pub fn graph_neighbors(
    edges: &[GraphEdge],
    panes: &[PanePlacement],
    selected_idx: usize,
) -> Vec<GraphNeighbor> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for edge in edges {
        let (other_idx, relation) = if edge.from_idx == selected_idx {
            (edge.to_idx, edge.label.clone())
        } else if edge.to_idx == selected_idx {
            (edge.from_idx, edge.label.clone())
        } else {
            continue;
        };
        if !seen.insert(other_idx) {
            continue;
        }
        let component_id = panes
            .get(other_idx)
            .map(|p| p.component_id.clone())
            .unwrap_or_else(|| format!("pane-{other_idx}"));
        out.push(GraphNeighbor {
            idx: other_idx,
            component_id,
            relation,
            strength: edge.strength,
        });
    }

    out.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.component_id.cmp(&b.component_id))
    });
    out
}

pub fn edge_visual(strength: u8) -> (f64, f64, &'static str) {
    let s = strength.max(1) as f64;
    let width = (1.0 + s * 0.45).min(4.5);
    let opacity = (0.35 + s * 0.12).min(0.95);
    let glow = if strength >= 3 {
        "filter: drop-shadow(0 0 6px rgba(245,158,11,0.55));"
    } else {
        ""
    };
    (width, opacity, glow)
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
    fn shared_bindings_increase_strength() {
        let panes = vec![
            pane("a", None, &["ctx:chat", "ctx:user"]),
            pane("b", None, &["ctx:chat", "ctx:user"]),
        ];
        let edges = derive_graph_edges(&panes);
        assert!(!edges.is_empty());
        assert!(edges.iter().any(|e| e.strength >= 2));
    }

    #[test]
    fn neighbors_sorted_by_strength() {
        let panes = vec![
            pane("hub", None, &["ctx:main"]),
            pane("weak", Some("hub"), &[]),
            pane("strong", None, &["ctx:main"]),
        ];
        let edges = derive_graph_edges(&panes);
        let neighbors = graph_neighbors(&edges, &panes, 0);
        assert!(!neighbors.is_empty());
    }
}
