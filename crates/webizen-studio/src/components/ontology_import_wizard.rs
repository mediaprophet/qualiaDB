//! Gentle ontology import flow — suggests pane layouts from domain tags.

use dioxus::prelude::*;
use crate::canvas_model::{PanePlacement, PresentationMode};

#[derive(Clone, Debug)]
pub struct OntologyLayoutSuggestion {
    pub label: String,
    pub domain: String,
    pub description: String,
    pub panes: Vec<PanePlacement>,
    pub presentation: PresentationMode,
}

fn pane(id: &str, x: u16, y: u16, w: u16, h: u16) -> PanePlacement {
    PanePlacement {
        component_id: id.to_string(),
        x,
        y,
        w,
        h,
        data_bindings: Vec::new(),
        binds_rpc: None,
        requires_capability: Vec::new(),
        ui_mode: None,
        layer: Default::default(),
        anchor: None,
        min_w_points: 0,
        min_h_points: 0,
        supported_presentations: Vec::new(),
        theme: Default::default(),
    }
}

pub fn builtin_layout_suggestions() -> Vec<OntologyLayoutSuggestion> {
    vec![
        OntologyLayoutSuggestion {
            label: "Legal & guardianship".to_string(),
            domain: "legal".to_string(),
            description: "N3 rules, SHACL shapes, and contextual workspace for care contracts."
                .to_string(),
            presentation: PresentationMode::GridBound,
            panes: vec![
                pane("contextual-workspace", 0, 0, 56, 62),
                pane("n3-logic-studio", 58, 0, 36, 30),
                pane("sparql-explorer", 58, 32, 36, 30),
            ],
        },
        OntologyLayoutSuggestion {
            label: "Health & clinical".to_string(),
            domain: "health".to_string(),
            description: "Vitals monitor, ontology browser, and inference harness for FHIR-aligned graphs."
                .to_string(),
            presentation: PresentationMode::NodeRelational,
            panes: vec![
                pane("health-monitor", 0, 0, 48, 40),
                pane("personal-ontology-builder", 50, 0, 44, 40),
                pane("llm-harness", 0, 42, 94, 20),
            ],
        },
        OntologyLayoutSuggestion {
            label: "Commons & spatial".to_string(),
            domain: "commons".to_string(),
            description: "10D manifold portal with nexus dashboard for bilateral micro-commons intake."
                .to_string(),
            presentation: PresentationMode::Spatial,
            panes: vec![
                pane("nexus", 0, 0, 40, 36),
                pane("render-preview", 42, 0, 52, 62),
                pane("wal-inspector", 0, 38, 40, 24),
            ],
        },
        OntologyLayoutSuggestion {
            label: "Research & semantics".to_string(),
            domain: "semantics".to_string(),
            description: "WordNet demo, SPARQL explorer, and diffusion visualizer for lexical grounding."
                .to_string(),
            presentation: PresentationMode::GridBound,
            panes: vec![
                pane("wordnet-demo", 0, 0, 46, 30),
                pane("sparql-explorer", 48, 0, 46, 30),
                pane("diffusion-visualizer", 0, 32, 94, 30),
            ],
        },
    ]
}

#[component]
pub fn OntologyImportWizard(
    on_apply: EventHandler<OntologyLayoutSuggestion>,
) -> Element {
    let suggestions = builtin_layout_suggestions();
    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 0.65rem;",
            h3 {
                style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0;",
                "Ontology layouts"
            }
            p {
                style: "font-size: 0.72rem; color: var(--qualia-text-muted, #888); margin: 0; line-height: 1.45;",
                "Import an ontology, then pick a starter layout matched to its domain tags."
            }
            for suggestion in suggestions {
                div {
                    key: "{suggestion.domain}",
                    style: "padding: 0.65rem 0.75rem; border-radius: 8px; border: 1px solid var(--qualia-border, #333); background: var(--qualia-surface-elevated, #1a1a1a);",
                    div {
                        style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.25rem;",
                        "{suggestion.label}"
                    }
                    div {
                        style: "font-size: 0.68rem; color: var(--qualia-text-muted, #888); margin-bottom: 0.5rem; line-height: 1.4;",
                        "{suggestion.description}"
                    }
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; gap: 0.5rem;",
                        span {
                            style: "font-size: 0.62rem; color: var(--qualia-accent, #f59e0b); text-transform: uppercase; letter-spacing: 0.06em;",
                            "{suggestion.domain} · {suggestion.panes.len()} panes"
                        }
                        button {
                            style: "padding: 0.25rem 0.55rem; font-size: 0.65rem; border-radius: 6px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.1); color: var(--qualia-text); cursor: pointer;",
                            onclick: {
                                let s = suggestion.clone();
                                move |_| on_apply.call(s.clone())
                            },
                            "Apply layout"
                        }
                    }
                }
            }
        }
    }
}