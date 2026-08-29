//! Requests for temporal, answer-set, paraconsistent, and symbolic reasoning panels.

use super::helpers::field_value;
use super::request_parse::{assignment, optional_f64, required_assignment, required_string_list};
use web_sys::Document;

pub(super) fn reasoning_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    match mode {
        "ltl" | "ltl-safety" => ltl_request(document, mode == "ltl-safety"),
        "asp" | "asp-optimal" => {
            let source = field_value(document, "asp-editor");
            if source.trim().is_empty() {
                return Err("Enter a bounded ASP program before evaluation.".into());
            }
            Ok((
                "SymbolicAndDefeasibleLogic.asp",
                serde_json::json!({
                    "source": source,
                    "operation": if mode == "asp-optimal" { "optimal" } else { "enumerate" }
                }),
            ))
        }
        "paraconsistent-saturation" => {
            let source = field_value(document, "paraconsistent-editor");
            Ok((
                "ParaconsistentLogic.route",
                serde_json::json!({
                    "threshold": optional_f64(&source, "threshold")?.unwrap_or(0.5)
                }),
            ))
        }
        "inference" | "inference-explain" => {
            let source = field_value(document, "infer-kb");
            if source.trim().is_empty() {
                return Err("Enter an N3 knowledge base before inference.".into());
            }
            Ok((
                "N3Logic.evaluate",
                serde_json::json!({
                    "source": source,
                    "mode": "infer",
                    "explain": mode == "inference-explain",
                    "context": "urn:poet:symbolic-inference"
                }),
            ))
        }
        _ => Err(format!("Unknown reasoning request `{mode}`.")),
    }
}

fn ltl_request(
    document: &Document,
    safety: bool,
) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "ltl-editor");
    let operator = if safety {
        "G".to_string()
    } else {
        required_assignment(&source, "operator")?.to_ascii_uppercase()
    };
    if !matches!(operator.as_str(), "G" | "F" | "X" | "U" | "R") {
        return Err("`operator` must be G, F, X, U, or R.".into());
    }
    let trace = required_string_list(&source, "trace")?;
    let (predicate, left, right) = if matches!(operator.as_str(), "G" | "F" | "X") {
        let key = if safety && assignment(&source, "invariant").is_some() {
            "invariant"
        } else {
            "predicate"
        };
        (Some(required_assignment(&source, key)?), None, None)
    } else {
        (
            None,
            Some(required_assignment(&source, "left")?),
            Some(required_assignment(&source, "right")?),
        )
    };
    Ok((
        "TemporalAndDescriptionLogic.ltl.evaluate",
        serde_json::json!({
            "operator": operator,
            "predicate": predicate,
            "left": left,
            "right": right,
            "trace": trace,
            "safety": safety
        }),
    ))
}
