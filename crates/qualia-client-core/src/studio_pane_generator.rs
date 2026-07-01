//! Keyword- and domain-driven pane layout planner for the studio prompt bar.
//!
//! Shared between the settings portal (`POST /generate_pane`) and any native
//! callers. The wasm studio fetches this API on desktop; the web demo falls back
//! to an in-crate copy in `webizen-studio::pane_generator`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum PresentationMode {
    #[default]
    GridBound,
    NodeRelational,
    Spatial,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PanePlacement {
    pub component_id: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    #[serde(default)]
    pub data_bindings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaneGenerationPlan {
    pub panes: Vec<PanePlacement>,
    pub presentation: PresentationMode,
    pub summary: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneratePaneRequest {
    pub prompt: String,
    #[serde(default)]
    pub palette_ids: Vec<String>,
    /// When set, prefer the ontology-domain preset (legal, health, commons, semantics).
    #[serde(default)]
    pub ontology_domain: Option<String>,
}

fn pane(id: &str, x: u16, y: u16, w: u16, h: u16, bindings: &[&str]) -> PanePlacement {
    PanePlacement {
        component_id: id.to_string(),
        x,
        y,
        w,
        h,
        data_bindings: bindings.iter().map(|s| s.to_string()).collect(),
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn has_palette_id(palette_ids: &[String], ids: &[&str]) -> bool {
    ids.iter()
        .any(|id| palette_ids.iter().any(|p| p == *id))
}

/// Domain presets aligned with `ontology_import_wizard::builtin_layout_suggestions`.
pub fn layout_from_ontology_domain(domain: &str) -> Option<PaneGenerationPlan> {
    let d = domain.to_ascii_lowercase();
    let plan = match d.as_str() {
        "legal" => PaneGenerationPlan {
            panes: vec![
                pane("contextual-workspace", 0, 0, 56, 62, &["n3:rules"]),
                pane("n3-logic-studio", 58, 0, 36, 30, &["n3:guardianship"]),
                pane("shacl-validator", 58, 32, 36, 30, &["shacl:shapes"]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Legal/guardianship preset: workspace, N3, SHACL.".to_string(),
        },
        "health" => PaneGenerationPlan {
            panes: vec![
                pane("health-monitor", 0, 0, 48, 40, &["fhir:Patient"]),
                pane("personal-ontology-builder", 50, 0, 44, 40, &[]),
                pane("llm-harness", 0, 42, 94, 20, &[]),
            ],
            presentation: PresentationMode::NodeRelational,
            summary: "Health/clinical preset: vitals, ontology builder, inference.".to_string(),
        },
        "commons" => PaneGenerationPlan {
            panes: vec![
                pane("nexus", 0, 0, 40, 36, &[]),
                pane("render-preview", 42, 0, 52, 62, &[]),
                pane("wal-inspector", 0, 38, 40, 24, &[]),
            ],
            presentation: PresentationMode::Spatial,
            summary: "Commons/spatial preset: nexus, render preview, WAL.".to_string(),
        },
        "semantics" => PaneGenerationPlan {
            panes: vec![
                pane("wordnet-demo", 0, 0, 46, 30, &[]),
                pane("sparql-explorer", 48, 0, 46, 30, &[]),
                pane("diffusion-visualizer", 0, 32, 94, 30, &[]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Research/semantics preset: WordNet, SPARQL, diffusion.".to_string(),
        },
        _ => return None,
    };
    Some(plan)
}

/// Map a natural-language prompt (and optional palette/domain hints) to a bounded layout.
pub fn generate_panes_from_request(req: &GeneratePaneRequest) -> PaneGenerationPlan {
    if let Some(domain) = req.ontology_domain.as_deref() {
        if let Some(plan) = layout_from_ontology_domain(domain) {
            return plan;
        }
    }

    let p = req.prompt.to_ascii_lowercase();
    let palette = &req.palette_ids;

    if contains_any(&p, &["health", "clinical", "vital", "fhir", "dicom", "patient"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("health-monitor", 0, 0, 48, 40, &["fhir:Patient"]),
                pane("sparql-explorer", 50, 0, 44, 40, &["sparql:clinical"]),
                pane("llm-harness", 0, 42, 94, 20, &[]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Health/clinical layout: vitals, SPARQL, inference harness.".to_string(),
        };
    }

    if contains_any(&p, &["legal", "guardian", "deontic", "rights", "shacl", "contract"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("contextual-workspace", 0, 0, 56, 62, &["n3:rules"]),
                pane("n3-logic-studio", 58, 0, 36, 30, &["n3:guardianship"]),
                pane("shacl-validator", 58, 32, 36, 30, &["shacl:shapes"]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Legal/guardianship layout: workspace, N3, SHACL.".to_string(),
        };
    }

    if contains_any(&p, &["spatial", "manifold", "10d", "portal", "volume", "render", "commons"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("nexus", 0, 0, 40, 36, &[]),
                pane("render-preview", 42, 0, 52, 62, &[]),
                pane("wal-inspector", 0, 38, 40, 24, &[]),
            ],
            presentation: PresentationMode::Spatial,
            summary: "Spatial commons layout: nexus, render preview, WAL.".to_string(),
        };
    }

    if contains_any(&p, &["graph", "node", "relation", "binding"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("provenance-graph", 0, 0, 46, 30, &[]),
                pane("sparql-explorer", 48, 0, 46, 30, &[]),
                pane("neuro-symbolic-chat", 0, 32, 94, 30, &[]),
            ],
            presentation: PresentationMode::NodeRelational,
            summary: "Node-relational layout: provenance graph, SPARQL, chat.".to_string(),
        };
    }

    if contains_any(&p, &["chat", "llm", "infer", "model", "agent"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("neuro-symbolic-chat", 0, 0, 62, 62, &[]),
                pane("inference-monitor", 64, 0, 30, 30, &[]),
                pane("lora-manager", 64, 32, 30, 30, &[]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Intelligence layout: chat, inference monitor, LoRA.".to_string(),
        };
    }

    if contains_any(&p, &["sparql", "rdf", "ontology", "triple", "knowledge", "wordnet", "semantics"]) {
        return PaneGenerationPlan {
            panes: vec![
                pane("sparql-explorer", 0, 0, 62, 62, &[]),
                pane("personal-ontology-builder", 64, 0, 30, 30, &[]),
                pane("n3-logic-studio", 64, 32, 30, 30, &[]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Knowledge layout: SPARQL, ontology builder, N3.".to_string(),
        };
    }

    if contains_any(&p, &["chart", "metric", "dashboard", "monitor", "track"]) {
        let chart = if has_palette_id(palette, &["time-series-chart"]) {
            "time-series-chart"
        } else {
            "card-view"
        };
        return PaneGenerationPlan {
            panes: vec![
                pane(chart, 0, 0, 56, 36, &[]),
                pane("data-ingest-form", 0, 38, 56, 24, &[]),
                pane("details-view", 58, 0, 36, 62, &[]),
            ],
            presentation: PresentationMode::GridBound,
            summary: "Dashboard layout: chart/metric card, ingest form, details.".to_string(),
        };
    }

    let primary = palette.first().map(|s| s.as_str()).unwrap_or("card-view");
    let secondary = palette.get(1).map(|s| s.as_str()).unwrap_or("details-view");
    PaneGenerationPlan {
        panes: vec![
            pane(primary, 0, 0, 56, 40, &[]),
            pane(secondary, 58, 0, 36, 40, &[]),
        ],
        presentation: PresentationMode::GridBound,
        summary: format!("Starter layout: {primary} + {secondary}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_prompt_selects_clinical_panes() {
        let plan = generate_panes_from_request(&GeneratePaneRequest {
            prompt: "Health tracker with vitals chart".to_string(),
            palette_ids: vec![],
            ontology_domain: None,
        });
        assert!(plan
            .panes
            .iter()
            .any(|p| p.component_id == "health-monitor"));
    }

    #[test]
    fn spatial_prompt_sets_spatial_mode() {
        let plan = generate_panes_from_request(&GeneratePaneRequest {
            prompt: "10D manifold spatial portal".to_string(),
            palette_ids: vec![],
            ontology_domain: None,
        });
        assert_eq!(plan.presentation, PresentationMode::Spatial);
    }

    #[test]
    fn ontology_domain_overrides_prompt() {
        let plan = generate_panes_from_request(&GeneratePaneRequest {
            prompt: "anything".to_string(),
            palette_ids: vec![],
            ontology_domain: Some("commons".to_string()),
        });
        assert_eq!(plan.presentation, PresentationMode::Spatial);
        assert!(plan.panes.iter().any(|p| p.component_id == "nexus"));
    }
}