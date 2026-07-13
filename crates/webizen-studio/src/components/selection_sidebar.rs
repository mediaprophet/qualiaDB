//! Graph-driven pane picker — neighbors and companion suggestions for the selected HUD pane.

use dioxus::prelude::*;

use std::collections::HashSet;

use crate::canvas_graph::{derive_graph_edges, graph_neighbors, GraphEdge};
use crate::canvas_model::{Page, PanePlacement};
use crate::pane_registry::{category_label, PaneCategory, PaneDefinition};

const MAX_SUGGESTIONS: usize = 6;

fn suggest_companion_panes(
    selected_component_id: &str,
    palette: &[PaneDefinition],
    on_canvas: &[PanePlacement],
) -> Vec<PaneDefinition> {
    let selected = palette.iter().find(|p| p.component_id == selected_component_id);
    let category = selected.map(|p| p.category.clone());
    let present: HashSet<&str> = on_canvas
        .iter()
        .map(|p| p.component_id.as_str())
        .collect();

    let mut scored: Vec<(i32, PaneDefinition)> = palette
        .iter()
        .filter(|p| !present.contains(p.component_id.as_str()))
        .map(|p| {
            let mut score = 0i32;
            if let Some(ref cat) = category {
                if p.category == *cat {
                    score += 3;
                }
                score += category_affinity(cat, &p.category);
            }
            if p.component_id.contains("sparql") || p.component_id.contains("ontology") {
                score += 1;
            }
            (score, p.clone())
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.display_name.cmp(&b.1.display_name)));
    scored
        .into_iter()
        .filter(|(s, _)| *s > 0)
        .take(MAX_SUGGESTIONS)
        .map(|(_, p)| p)
        .collect()
}

fn category_affinity(from: &PaneCategory, to: &PaneCategory) -> i32 {
    use PaneCategory::*;
    match (from, to) {
        (Knowledge, Governance) | (Governance, Knowledge) => 2,
        (Knowledge, Intelligence) | (Intelligence, Knowledge) => 2,
        (Intelligence, Computational) | (Computational, Intelligence) => 1,
        (Data, Knowledge) | (Knowledge, Data) => 2,
        (Network, Data) | (Data, Network) => 1,
        (Governance, Data) | (Data, Governance) => 1,
        _ => 0,
    }
}

#[component]
pub fn SelectionSidebar(
    page: Page,
    selected_idx: Option<usize>,
    palette: Vec<PaneDefinition>,
    on_select_pane: EventHandler<usize>,
    on_add_component: EventHandler<String>,
) -> Element {
    let edges: Vec<GraphEdge> = derive_graph_edges(&page.panes);

    let (selected_label, neighbors, companions) = match selected_idx {
        Some(idx) => {
            let label = page
                .panes
                .get(idx)
                .map(|p| p.component_id.clone())
                .unwrap_or_else(|| format!("pane-{idx}"));
            let neighbors = graph_neighbors(&edges, &page.panes, idx);
            let companions = page
                .panes
                .get(idx)
                .map(|p| suggest_companion_panes(&p.component_id, &palette, &page.panes))
                .unwrap_or_default();
            (Some(label), neighbors, companions)
        }
        None => (None, Vec::new(), Vec::new()),
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 0.65rem;",
            h3 {
                style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0;",
                "Graph selection"
            }

            if let Some(label) = selected_label {
                div {
                    style: "font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.4;",
                    "Selected: "
                    span { style: "color: var(--qualia-accent); font-weight: 600;", "{label}" }
                }
            } else {
                p {
                    style: "font-size: 0.72rem; color: var(--qualia-text-muted); margin: 0;",
                    "Select a pane on the canvas to see graph-linked neighbors and suggested companions."
                }
            }

            if !neighbors.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 0.35rem;",
                    div {
                        style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.06em;",
                        "Linked panes"
                    }
                    for n in neighbors {
                        button {
                            key: "{n.idx}",
                            style: "text-align: left; padding: 0.45rem 0.55rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: var(--qualia-surface-elevated, #1a1a1a); color: var(--qualia-text); cursor: pointer; font-size: 0.72rem;",
                            onclick: {
                                let idx = n.idx;
                                move |_| on_select_pane.call(idx)
                            },
                            div {
                                style: "font-weight: 600;",
                                "{n.component_id}"
                            }
                            div {
                                style: "font-size: 0.62rem; color: var(--qualia-text-muted); margin-top: 0.15rem;",
                                "{n.relation} · strength {n.strength}"
                            }
                        }
                    }
                }
            }

            if !companions.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 0.35rem;",
                    div {
                        style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.06em;",
                        "Suggested companions"
                    }
                    for item in companions {
                        button {
                            key: "{item.component_id}",
                            style: "text-align: left; padding: 0.4rem 0.55rem; border-radius: 6px; border: 1px dashed var(--qualia-border); background: transparent; color: var(--qualia-text); cursor: pointer; font-size: 0.7rem; display: flex; justify-content: space-between; align-items: center; gap: 0.4rem;",
                            onclick: {
                                let id = item.component_id.clone();
                                move |_| on_add_component.call(id.clone())
                            },
                            span { "{item.display_name}" }
                            span {
                                style: "font-size: 0.58rem; color: var(--qualia-accent); opacity: 0.85;",
                                "{category_label(&item.category)}"
                            }
                        }
                    }
                }
            }
        }
    }
}