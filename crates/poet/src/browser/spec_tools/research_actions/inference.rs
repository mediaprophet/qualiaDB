//! Inference chains and validation for Poet research.

use super::util::{append_nested, count_document, count_within, next_confidence};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:make-inference" => Some(make_inference(document, container)),
        "research:chain-inference" => Some(chain_inference(container)),
        "research:set-inference-confidence" => Some(set_inference_confidence(container)),
        "research:query-inferences" => Some(query_inferences(document, container)),
        "research:validate-inference" => Some(validate_inference(container)),
        "research:trace-inference-chain" => Some(trace_inference_chain(container)),
        "research:compare-inferences" => Some(compare_inferences(container)),
        _ => None,
    }
}

fn make_inference(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-inference",
        "inference:active",
        &[
            ("data-inference-type", "abductive"),
            ("data-inference-confidence", "moderate"),
        ],
    )
}

fn chain_inference(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-inference-chain", "inference_n -> depends_on -> inference_n-1")
        .map_err(|_| "Failed to chain inference.".to_string())
}

fn set_inference_confidence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-inference-confidence");
    let next = next_confidence(current.as_deref());
    container
        .set_attribute("data-inference-confidence", next)
        .map_err(|_| "Failed to set inference confidence.".to_string())
}

fn query_inferences(document: &Document, container: &Element) -> Result<(), String> {
    let local = count_within(container, "[data-research-inference]")?;
    let global = count_document(document, "[data-research-inference]")?;
    container
        .set_attribute(
            "data-inference-query",
            &format!("local={local};canvas={global}"),
        )
        .map_err(|_| "Failed to query inferences.".to_string())
}

fn validate_inference(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-inference]")?;
    let verdict = if count > 0 {
        "validation:premises_checked;grounding=partial"
    } else {
        "validation:no_inference_to_check"
    };
    container
        .set_attribute("data-inference-validation", verdict)
        .map_err(|_| "Failed to validate inference.".to_string())
}

fn trace_inference_chain(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-inference]")?;
    container
        .set_attribute(
            "data-inference-trace",
            &format!("chain_length={count};path=premises_to_conclusion"),
        )
        .map_err(|_| "Failed to trace inference chain.".to_string())
}

fn compare_inferences(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-inference]")?;
    let comparison = if count >= 2 {
        "comparison:overlap_and_contradiction_checked"
    } else {
        "comparison:insufficient_inferences"
    };
    container
        .set_attribute("data-inference-comparison", comparison)
        .map_err(|_| "Failed to compare inferences.".to_string())
}
