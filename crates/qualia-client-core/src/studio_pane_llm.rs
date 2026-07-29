//! Optional local-LLM layout generation for `/generate_pane`.
//!
//! When a model is active, asks the orchestrated chat path for a JSON layout plan.
//! Falls back to the keyword planner in `studio_pane_generator` on any failure.

use crate::chat_inference;
use crate::model_lifecycle;
use crate::studio_pane_generator::{
    generate_panes_from_request, GeneratePaneRequest, PaneGenerationPlan, PanePlacement,
    PresentationMode,
};
use qualia_core_db::orchestrator::ModelLifecycle;

const LAYOUT_SESSION: &str = "studio-pane-layout";

/// Generate a pane layout, preferring local LLM output when available.
pub fn generate_panes_with_llm_or_fallback(req: &GeneratePaneRequest) -> PaneGenerationPlan {
    if req.use_llm.unwrap_or(true) {
        if let Some(plan) = try_llm_layout(req) {
            return plan;
        }
    }
    generate_panes_from_request(req)
}

fn try_llm_layout(req: &GeneratePaneRequest) -> Option<PaneGenerationPlan> {
    if model_lifecycle::get_model_lifecycle_state() != ModelLifecycle::Active {
        return None;
    }

    let palette_hint = if req.palette_ids.is_empty() {
        "health-monitor, sparql-explorer, n3-logic-studio, neuro-symbolic-chat, card-view"
            .to_string()
    } else {
        req.palette_ids.join(", ")
    };

    let prompt = format!(
        "You are a Qualia Webizen studio layout planner. Return ONLY valid JSON (no markdown) with this shape:\n\
         {{\"presentation\":\"GridBound|NodeRelational|Spatial\",\"summary\":\"...\",\"panes\":[{{\"component_id\":\"...\",\"x\":0,\"y\":0,\"w\":40,\"h\":30,\"data_bindings\":[\"ns:term\"]}}]}}\n\
         Grid is 96x64 points. Use component_id values from this palette: {palette_hint}.\n\
         User request: {}",
        req.prompt
    );

    let result = chat_inference::run_chat_inference_with_options(LAYOUT_SESSION, &prompt, None);
    if !result.committed || result.block_reason.is_some() {
        return None;
    }

    parse_llm_plan_json(&result.text).map(|mut plan| {
        if plan.summary.is_empty() {
            plan.summary = format!("LLM layout for: {}", req.prompt);
        }
        plan
    })
}

/// Parse model JSON into a layout plan; tolerates fenced code blocks.
pub fn parse_llm_plan_json(text: &str) -> Option<PaneGenerationPlan> {
    let trimmed = text.trim();
    let json_body = extract_json_object(trimmed)?;
    let raw: LlmPlanJson = serde_json::from_str(json_body).ok()?;
    if raw.panes.is_empty() {
        return None;
    }
    let presentation = match raw.presentation.to_ascii_lowercase().as_str() {
        "spatial" => PresentationMode::Spatial,
        "noderelational" | "node_relational" | "nodes" => PresentationMode::NodeRelational,
        _ => PresentationMode::GridBound,
    };
    Some(PaneGenerationPlan {
        panes: raw
            .panes
            .into_iter()
            .map(|p| PanePlacement {
                component_id: p.component_id,
                x: p.x,
                y: p.y,
                w: p.w,
                h: p.h,
                data_bindings: p.data_bindings,
            })
            .collect(),
        presentation,
        summary: raw
            .summary
            .unwrap_or_else(|| "LLM-generated layout".to_string()),
    })
}

#[derive(serde::Deserialize)]
struct LlmPlanJson {
    #[serde(default)]
    presentation: String,
    #[serde(default)]
    summary: Option<String>,
    panes: Vec<LlmPaneJson>,
}

#[derive(serde::Deserialize)]
struct LlmPaneJson {
    component_id: String,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    #[serde(default)]
    data_bindings: Vec<String>,
}

fn extract_json_object(text: &str) -> Option<&str> {
    if text.starts_with('{') {
        return Some(text);
    }
    if let Some(start) = text.find("```json") {
        let rest = &text[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim());
        }
    }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Some(&text[start..=end]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_llm_json_layout() {
        let json = r#"{"presentation":"Spatial","summary":"test","panes":[{"component_id":"nexus","x":0,"y":0,"w":40,"h":36,"data_bindings":[]}]}"#;
        let plan = parse_llm_plan_json(json).unwrap();
        assert_eq!(plan.presentation, PresentationMode::Spatial);
        assert_eq!(plan.panes[0].component_id, "nexus");
    }

    #[test]
    fn fallback_when_llm_inactive() {
        let plan = generate_panes_with_llm_or_fallback(&GeneratePaneRequest {
            prompt: "health vitals".to_string(),
            palette_ids: vec![],
            ontology_domain: None,
            use_llm: Some(true),
        });
        assert!(!plan.panes.is_empty());
    }
}
