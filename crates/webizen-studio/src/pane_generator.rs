//! Keyword-driven pane layout planner for the studio prompt bar.

use crate::canvas_model::{LayerBehavior, PanePlacement, PresentationMode};
use crate::pane_registry::{builtin_pane_definitions, PaneDefinition};
use crate::theme_engine::ThemeBinding;

#[derive(Clone, Debug, PartialEq)]
pub struct PaneGenerationPlan {
    pub panes: Vec<PanePlacement>,
    pub presentation: PresentationMode,
    pub summary: String,
}

fn pane(
    id: &str,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    bindings: &[&str],
) -> PanePlacement {
    PanePlacement {
        component_id: id.to_string(),
        x,
        y,
        w,
        h,
        data_bindings: bindings.iter().map(|s| s.to_string()).collect(),
        binds_rpc: None,
        requires_capability: Vec::new(),
        ui_mode: None,
        layer: LayerBehavior::Docked,
        anchor: None,
        min_w_points: 0,
        min_h_points: 0,
        supported_presentations: Vec::new(),
        theme: ThemeBinding::default(),
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Map a natural-language prompt to a bounded pane layout using the built-in palette.
pub fn generate_panes_from_prompt(prompt: &str, palette: &[PaneDefinition]) -> PaneGenerationPlan {
    let p = prompt.to_ascii_lowercase();
    let has = |ids: &[&str]| ids.iter().any(|id| palette.iter().any(|d| d.component_id == *id));

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

    if contains_any(&p, &["spatial", "manifold", "10d", "portal", "volume", "render"]) {
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

    if contains_any(&p, &["sparql", "rdf", "ontology", "triple", "knowledge"]) {
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
        let chart = if has(&["time-series-chart"]) {
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

    // Default: two-pane starter from palette
    let primary = palette
        .first()
        .map(|d| d.component_id.as_str())
        .unwrap_or("card-view");
    let secondary = palette
        .get(1)
        .map(|d| d.component_id.as_str())
        .unwrap_or("details-view");
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
        let plan = generate_panes_from_prompt(
            "Health tracker with vitals chart",
            &builtin_pane_definitions(),
        );
        assert!(plan
            .panes
            .iter()
            .any(|p| p.component_id == "health-monitor"));
    }

    #[test]
    fn spatial_prompt_sets_spatial_mode() {
        let plan = generate_panes_from_prompt(
            "10D manifold spatial portal",
            &builtin_pane_definitions(),
        );
        assert_eq!(plan.presentation, PresentationMode::Spatial);
    }
}