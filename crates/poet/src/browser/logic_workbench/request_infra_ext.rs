//! Validated requests for P2 infrastructure-extension panels.

use super::helpers::field_value;
use super::request_parse::{
    assignment, optional_bool, optional_u64, required_assignment, required_f64, required_f64_list,
    required_string_list,
};
use web_sys::Document;

pub(super) fn infra_ext_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let args = match mode {
        "crdt-sync" => {
            let source = field_value(document, "crdt-sync-input");
            serde_json::json!({
                "mode": "crdt",
                "local_clock": optional_u64(&source, "local_clock")?.unwrap_or(0),
                "remote_clock": optional_u64(&source, "remote_clock")?.unwrap_or(0),
                "local_object": optional_u64(&source, "local_object")?.unwrap_or(1),
                "remote_object": optional_u64(&source, "remote_object")?.unwrap_or(2),
                "selfhood": optional_bool(&source, "selfhood")?.unwrap_or(false),
                "principal": assignment(&source, "principal"),
                "delegate": assignment(&source, "delegate"),
                "context": assignment(&source, "context"),
                "expiry": optional_u64(&source, "expiry")?.unwrap_or(0),
                "now": optional_u64(&source, "now")?.unwrap_or(0)
            })
        }
        "agency-merkle" => {
            let source = field_value(document, "agency-merkle-input");
            serde_json::json!({
                "mode": "agency",
                "author": field_value(document, "agency-merkle-did"),
                "claims": required_string_list(&source, "claims")?
            })
        }
        "key-vault" => {
            let source = field_value(document, "key-vault-input");
            serde_json::json!({
                "mode": "key_vault",
                "operation": assignment(&source, "operation").unwrap_or("list"),
                "key_id": assignment(&source, "key_id"),
                "created_at": optional_u64(&source, "created_at")?.unwrap_or(0),
                "expires_at": optional_u64(&source, "expires_at")?,
                "now": optional_u64(&source, "now")?.unwrap_or(0)
            })
        }
        "policy-evaluator" => {
            let source = field_value(document, "policy-evaluator-input");
            serde_json::json!({
                "mode": "policy",
                "subject": assignment(&source, "subject"),
                "resource": assignment(&source, "resource"),
                "clearance": assignment(&source, "clearance").unwrap_or("public"),
                "sensitivity": assignment(&source, "sensitivity").unwrap_or("public"),
                "epistemic": assignment(&source, "epistemic").unwrap_or("active")
            })
        }
        "consent-manager" => {
            let source = field_value(document, "consent-input");
            serde_json::json!({
                "mode": "consent",
                "operation": field_value(document, "consent-op"),
                "scope": field_value(document, "consent-scope"),
                "expiry": optional_u64(&source, "expiry")?.unwrap_or(0),
                "now": optional_u64(&source, "now")?.unwrap_or(0),
                "revoked": optional_bool(&source, "revoked")?.unwrap_or(false)
            })
        }
        "carrier" => {
            let source = field_value(document, "carrier-input");
            serde_json::json!({
                "mode": "carrier",
                "payload": required_assignment(&source, "payload")?,
                "bound_tag": optional_u64(&source, "bound_tag")?
            })
        }
        "control-feedback" => {
            let source = field_value(document, "control-feedback-input");
            serde_json::json!({
                "mode": "control_feedback",
                "setpoint": required_f64(&source, "setpoint")?,
                "measured": required_f64(&source, "measured")?,
                "t": optional_u64(&source, "t")?.unwrap_or(1)
            })
        }
        "likeliness" => {
            let source = field_value(document, "likeliness-input");
            serde_json::json!({
                "mode": "likeliness",
                "premises": required_f64_list(&source, "premises")?
            })
        }
        "qubo" => {
            let source = field_value(document, "qubo-input");
            serde_json::json!({
                "mode": "qubo",
                "edges": required_string_list(&source, "edges")?
            })
        }
        "owl-converter" => {
            let source = field_value(document, "owl-input");
            serde_json::json!({
                "mode": "owl",
                "operation": field_value(document, "owl-op"),
                "triples": required_string_list(&source, "triples")?
            })
        }
        _ => {
            return Err(format!(
                "Unknown infrastructure-extension request `{mode}`."
            ));
        }
    };
    Ok(("InfraExtLogic.compute", args))
}
