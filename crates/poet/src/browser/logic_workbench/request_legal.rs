//! Validated requests for legal-logic panels.

use super::helpers::field_value;
use super::request_parse::{
    assignment, bool_assignment, optional_f64, optional_u64, required_assignment,
    required_string_list,
};
use web_sys::Document;

pub(super) fn legal_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let args = match mode {
        "jural" => serde_json::json!({
            "mode": "jural",
            "role": field_value(document, "jural-role")
        }),
        "stit" | "stit-joint" => {
            let source = field_value(document, "stit-context");
            serde_json::json!({
                "mode": "stit",
                "operation": if mode == "stit-joint" { "joint" } else { "evaluate" },
                "agent": required_field(document, "stit-agent", "agent DID")?,
                "action": required_field(document, "stit-action", "action")?,
                "brought_about": optional_bool(&source, "brought_about")?.unwrap_or(false),
                "could_do_otherwise": optional_bool(&source, "could_do_otherwise")?.unwrap_or(false),
                "members": required_string_list(&source, "members")?,
                "joint_acted": optional_bool(&source, "joint_acted")?.unwrap_or(false)
            })
        }
        "causal" | "causal-overdetermination" => {
            let source = field_value(document, "causal-editor");
            serde_json::json!({
                "mode": "causal",
                "operation": if mode.ends_with("overdetermination") { "overdetermination" } else { "trace" },
                "target": required_assignment(&source, "target")?,
                "edges": required_string_list(&source, "edges")?
            })
        }
        "responsibility" | "responsibility-vacuum" => {
            let source = field_value(document, "resp-editor");
            serde_json::json!({
                "mode": "responsibility",
                "operation": if mode.ends_with("vacuum") { "vacuum" } else { "adjudicate" },
                "confirmed": optional_bool(&source, "confirmed")?.unwrap_or(false),
                "dismissed": optional_bool(&source, "dismissed")?.unwrap_or(false),
                "harm_occurred": optional_bool(&source, "harm_occurred")?.unwrap_or(false),
                "accountable_person": optional_bool(&source, "accountable_person")?.unwrap_or(false)
            })
        }
        "capacity" => {
            let source = field_value(document, "capacity-context");
            serde_json::json!({
                "mode": "capacity",
                "query": field_value(document, "capacity-type"),
                "agent": required_field(document, "capacity-agent", "agent DID")?,
                "status": required_assignment(&source, "status")?,
                "dependent": assignment(&source, "dependent"),
                "guardianship": optional_bool(&source, "guardianship")?.unwrap_or(false),
                "deceased": optional_bool(&source, "deceased")?.unwrap_or(false),
                "representative": optional_bool(&source, "representative")?.unwrap_or(false),
                "imbalance": optional_f64(&source, "imbalance")?.unwrap_or(0.0),
                "explicit_threat": optional_bool(&source, "explicit_threat")?.unwrap_or(false),
                "duress_threshold": optional_f64(&source, "duress_threshold")?.unwrap_or(0.7)
            })
        }
        "delegation" | "delegation-revocation" => {
            let source = field_value(document, "deleg-editor");
            serde_json::json!({
                "mode": "delegation",
                "operation": if mode.ends_with("revocation") { "revocation" } else { "trace" },
                "parent_domains": required_string_list(&source, "parent_domains")?,
                "child_domains": required_string_list(&source, "child_domains")?,
                "revoked_domains": required_string_list(&source, "revoked_domains")?,
                "requested_domain": required_assignment(&source, "requested_domain")?
            })
        }
        "contract" => {
            let source = field_value(document, "contract-editor");
            serde_json::json!({
                "mode": "contract",
                "stipulated": optional_bool(&source, "stipulated")?.unwrap_or(false),
                "accepted": optional_bool(&source, "accepted")?.unwrap_or(false),
                "offeror_capacity": required_assignment(&source, "offeror_capacity")?,
                "acceptor_capacity": required_assignment(&source, "acceptor_capacity")?,
                "instrument": assignment(&source, "instrument")
            })
        }
        "consensus" => {
            let source = field_value(document, "consensus-context");
            let threshold = field_value(document, "consensus-threshold");
            let (votes, parties) = parse_threshold(&threshold)?;
            serde_json::json!({
                "mode": "consensus",
                "query": field_value(document, "consensus-mode"),
                "votes": votes,
                "parties": parties,
                "partitioned": optional_bool(&source, "partitioned")?.unwrap_or(false)
            })
        }
        "meta-deontic" | "meta-deontic-endorse" => {
            let source = field_value(document, "meta-deontic-editor");
            serde_json::json!({
                "mode": "meta_deontic",
                "actor": required_assignment(&source, "actor")?,
                "action": required_assignment(&source, "action")?,
                "instrument": required_assignment(&source, "instrument")?,
                "now": optional_u64(&source, "now")?.unwrap_or(0),
                "endorse": mode.ends_with("endorse")
            })
        }
        "grounded-extension"
        | "preferred-extension"
        | "stable-extension"
        | "complete-extension"
        | "argumentation-visualize" => {
            let source = field_value(document, "arg-editor");
            let semantics = if mode == "argumentation-visualize" {
                field_value(document, "arg-semantics")
            } else {
                mode.trim_end_matches("-extension").to_string()
            };
            serde_json::json!({
                "mode": "argumentation",
                "semantics": semantics,
                "arguments": required_string_list(&source, "arguments")?,
                "attacks": required_string_list(&source, "attacks")?,
                "visualize": mode == "argumentation-visualize"
            })
        }
        _ => return Err(format!("Unknown legal request `{mode}`.")),
    };
    Ok(("LegalLogic.compute", args))
}

fn optional_bool(source: &str, key: &str) -> Result<Option<bool>, String> {
    assignment(source, key)
        .map(|_| bool_assignment(source, key))
        .transpose()
}

fn required_field(document: &Document, id: &str, name: &str) -> Result<String, String> {
    let value = field_value(document, id);
    if value.trim().is_empty() {
        Err(format!("Enter the {name}."))
    } else {
        Ok(value)
    }
}

fn parse_threshold(value: &str) -> Result<(u64, u64), String> {
    let (votes, parties) = value
        .trim()
        .split_once("-of-")
        .ok_or_else(|| "Consensus threshold must use M-of-N, for example 3-of-5.".to_string())?;
    let votes = votes
        .parse::<u64>()
        .map_err(|_| "Consensus M must be an integer.".to_string())?;
    let parties = parties
        .parse::<u64>()
        .map_err(|_| "Consensus N must be an integer.".to_string())?;
    if parties == 0 || votes > parties {
        return Err("Consensus threshold must satisfy 0 <= M <= N and N > 0.".into());
    }
    Ok((votes, parties))
}

#[cfg(test)]
mod tests {
    use super::parse_threshold;

    #[test]
    fn consensus_threshold_is_validated() {
        assert_eq!(parse_threshold("3-of-5").unwrap(), (3, 5));
        assert!(parse_threshold("6-of-5").is_err());
        assert!(parse_threshold("three of five").is_err());
    }
}
