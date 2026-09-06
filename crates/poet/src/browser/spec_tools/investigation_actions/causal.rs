//! Local causal reasoning without live graph daemon capabilities.

use web_sys::Element;

pub(super) fn run(_document: &web_sys::Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:intervene" => Some(intervene(container)),
        "investigation:counterfactual" => Some(counterfactual(container)),
        "investigation:validate-causal-model" => Some(validate_causal_model(container)),
        _ => None,
    }
}

fn intervene(container: &Element) -> Result<(), String> {
    let model = container
        .get_attribute("data-causal-model")
        .ok_or_else(|| {
            "No local causal model on this surface. Annotate variables with data-causal-model first.".to_string()
        })?;
    container
        .set_attribute("data-causal-intervention", &format!("do(X)|model:{model}"))
        .map_err(|_| "Failed to record causal intervention.".to_string())
}

fn counterfactual(container: &Element) -> Result<(), String> {
    let model = container
        .get_attribute("data-causal-model")
        .ok_or_else(|| "No local causal model for counterfactual query.".to_string())?;
    container
        .set_attribute("data-causal-counterfactual", &format!("what_if|model:{model}"))
        .map_err(|_| "Failed to record counterfactual query.".to_string())
}

fn validate_causal_model(container: &Element) -> Result<(), String> {
    let model = container.get_attribute("data-causal-model");
    let evidence = container.get_attribute("data-evidence-item");
    let status = match (model, evidence) {
        (Some(m), Some(_)) => format!("validated:model_and_evidence_present|model:{m}"),
        (Some(m), None) => format!("partial:model_only|model:{m}"),
        _ => "invalid:no_causal_model".to_string(),
    };
    container
        .set_attribute("data-causal-validation", &status)
        .map_err(|_| "Failed to validate causal model.".to_string())
}

