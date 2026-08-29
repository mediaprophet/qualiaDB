//! Validated requests for P1 governance panels.

use super::helpers::field_value;
use super::request_parse::{
    assignment, optional_bool, optional_f64, optional_string_list, optional_u64,
    required_assignment, required_string_list, required_u64,
};
use web_sys::Document;

pub(super) fn governance_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let args = match mode {
        "value-flow" | "value-flow-royalty" => {
            let source = field_value(document, "value-flow-editor");
            if mode.ends_with("royalty") {
                serde_json::json!({
                    "mode": "value_flow",
                    "operation": "royalty",
                    "base": required_u64(&source, "base")?,
                    "agent_multiplier_percent": required_u64(&source, "agent_multiplier_percent")?,
                    "generations": optional_u64(&source, "generations")?.unwrap_or(0),
                    "share_percent": optional_u64(&source, "share_percent")?.unwrap_or(50)
                })
            } else {
                serde_json::json!({
                    "mode": "value_flow",
                    "operation": "flow",
                    "production_cost": required_u64(&source, "production_cost")?,
                    "roi_cap_percent": required_u64(&source, "roi_cap_percent")?,
                    "max_roi_percent": required_u64(&source, "max_roi_percent")?,
                    "pool": optional_u64(&source, "pool")?.unwrap_or(0),
                    "energy_returned": optional_u64(&source, "energy_returned")?.unwrap_or(0),
                    "energy_invested": optional_u64(&source, "energy_invested")?.unwrap_or(0),
                    "min_ratio": optional_f64(&source, "min_ratio")?.unwrap_or(1.0)
                })
            }
        }
        "interaction-governance" => {
            let source = field_value(document, "interaction-gov-context");
            serde_json::json!({
                "mode": "interaction",
                "requested_mode": field_value(document, "interaction-gov-mode"),
                "agent": field_value(document, "interaction-gov-agent"),
                "action": field_value(document, "interaction-gov-action"),
                "status": assignment(&source, "status"),
                "non_derogable": optional_bool(&source, "non_derogable")?.unwrap_or(false),
                "humanitarian": optional_bool(&source, "humanitarian")?.unwrap_or(false),
                "ambiguous": optional_bool(&source, "ambiguous")?.unwrap_or(false),
                "emergency": optional_bool(&source, "emergency")?.unwrap_or(false),
                "hard_core": optional_bool(&source, "hard_core")?.unwrap_or(false)
            })
        }
        "identity-fabric" | "identity-fabric-survive" => {
            let source = field_value(document, "identity-fabric-editor");
            serde_json::json!({
                "mode": "identity",
                "anchors": optional_string_list(&source, "anchors")?.unwrap_or_default(),
                "lost": optional_string_list(&source, "lost")?.unwrap_or_default(),
                "total_anchors": optional_u64(&source, "total_anchors")?,
                "lost_anchors": optional_u64(&source, "lost_anchors")?,
                "quorum": optional_u64(&source, "quorum")?.unwrap_or(1)
            })
        }
        "capability-gap" => {
            let source = field_value(document, "capability-gap-editor");
            serde_json::json!({
                "mode": "capability_gap",
                "required": required_string_list(&source, "required")?,
                "held": required_string_list(&source, "held")?,
                "equivalences": optional_string_list(&source, "equivalences")?.unwrap_or_default(),
                "goal": assignment(&source, "goal"),
                "edges": optional_string_list(&source, "edges")?.unwrap_or_default()
            })
        }
        "legal-compose" | "legal-compose-zk" => {
            let source = field_value(document, "legal-compose-editor");
            serde_json::json!({
                "mode": "legal_compose",
                "operation": if mode.ends_with("zk") { "zk" } else { "compose" },
                "all_claims": optional_string_list(&source, "all_claims")?.unwrap_or_default(),
                "reveal": optional_string_list(&source, "reveal")?.unwrap_or_default(),
                "machine_proposed": optional_bool(&source, "machine_proposed")?.unwrap_or(false),
                "human_attested": optional_bool(&source, "human_attested")?.unwrap_or(false),
                "translatable": optional_bool(&source, "translatable")?.unwrap_or(true),
                "instrument": assignment(&source, "instrument"),
                "proportionate": optional_bool(&source, "proportionate")?.unwrap_or(false),
                "proof_verified": optional_bool(&source, "proof_verified")?.unwrap_or(false)
            })
        }
        "deontic-compose" | "deontic-compose-mens" => {
            let source = field_value(document, "deontic-compose-editor");
            serde_json::json!({
                "mode": "deontic_compose",
                "agent": required_assignment(&source, "agent")?,
                "content": required_assignment(&source, "content")?,
                "opcode": required_assignment(&source, "opcode")?,
                "brought_about": optional_bool(&source, "brought_about")?.unwrap_or(false),
                "knows": optional_bool(&source, "knows")?.unwrap_or(false),
                "had_duty_to_know": optional_bool(&source, "had_duty_to_know")?.unwrap_or(false),
                "norm_jurisdiction": assignment(&source, "norm_jurisdiction"),
                "target_jurisdiction": assignment(&source, "target_jurisdiction"),
                "within": optional_string_list(&source, "within")?.unwrap_or_default(),
                "trust": optional_f64(&source, "trust")?.unwrap_or(1.0),
                "trust_threshold": optional_f64(&source, "trust_threshold")?.unwrap_or(0.5)
            })
        }
        _ => return Err(format!("Unknown governance request `{mode}`.")),
    };
    Ok(("GovernanceLogic.compute", args))
}
