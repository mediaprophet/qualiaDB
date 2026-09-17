//! Requests for CTL, defeasible, linear, and dialectical workbench panels.

use super::helpers::field_value;
use super::request_advanced::named_pairs;
use super::request_parse::{bool_assignment, optional_u64, required_assignment};
use web_sys::Document;

pub(super) fn formal_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let arguments = match mode {
        "ctl" => {
            let source = field_value(document, "ctl-editor");
            serde_json::json!({
                "mode": mode,
                "operator": required_assignment(&source, "operator")?,
                "start": required_assignment(&source, "start")?,
                "proposition": required_assignment(&source, "proposition")?,
                "phi": required_assignment(&source, "phi").unwrap_or_else(|_| "q42:true".into()),
                "transitions": named_pairs(&source, "transitions")?,
                "holds": named_pairs(&source, "holds")?
            })
        }
        "defeasible" => {
            let source = field_value(document, "defeasible-editor");
            serde_json::json!({
                "mode": mode,
                "literal": required_assignment(&source, "literal")?,
                "rule_a": required_assignment(&source, "rule_a")?,
                "kind_a": required_assignment(&source, "kind_a")?,
                "positive_a": bool_assignment(&source, "positive_a")?,
                "rule_b": required_assignment(&source, "rule_b")?,
                "kind_b": required_assignment(&source, "kind_b")?,
                "positive_b": bool_assignment(&source, "positive_b")?,
                "superior": required_assignment(&source, "superior")?,
                "ambiguity": required_assignment(&source, "ambiguity")?
            })
        }
        "linear" => {
            let source = field_value(document, "linear-editor");
            serde_json::json!({
                "mode": mode,
                "resource_a": required_assignment(&source, "resource_a")?,
                "resource_b": required_assignment(&source, "resource_b")?,
                "consumed_a": bool_assignment(&source, "consumed_a")?,
                "consumed_b": bool_assignment(&source, "consumed_b")?,
                "reusable_a": bool_assignment(&source, "reusable_a")?,
                "reusable_b": bool_assignment(&source, "reusable_b")?,
                "structural_rule": required_assignment(&source, "structural_rule")?
            })
        }
        "dialectical" => {
            let source = field_value(document, "dialectical-editor");
            serde_json::json!({
                "mode": mode,
                "subject": required_assignment(&source, "subject")?,
                "predicate": required_assignment(&source, "predicate")?,
                "thesis": required_assignment(&source, "thesis")?,
                "antithesis": required_assignment(&source, "antithesis")?,
                "supporting": optional_u64(&source, "supporting")?.unwrap_or(0),
                "objecting": optional_u64(&source, "objecting")?.unwrap_or(0)
            })
        }
        "dialectical-counterfactual" => {
            let source = field_value(document, "dialectical-editor");
            serde_json::json!({
                "mode": "dialectical_counterfactual",
                "causal_edges": named_pairs(&source, "causal_edges")?,
                "factual_outcome": required_assignment(&source, "factual_outcome")?,
                "intervention": required_assignment(&source, "intervention")?,
                "intervention_value": required_assignment(&source, "intervention_value")?,
                "target": required_assignment(&source, "target")?
            })
        }
        _ => return Err(format!("Unknown formal-logic panel `{mode}`.")),
    };
    Ok(("FormalLogic.compute", arguments))
}
