//! MCP tools for logic modalities, N3 rule evaluation, and symbolic inference.
//!
//! Exposes LTL, ASP, Deontic, Epistemic, Paraconsistent, and Defeasible reasoning.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::McpSystemError;

fn map_db_error(e: qualia_core_db::mcp::mcp_server::McpSystemError) -> McpSystemError {
    match e {
        qualia_core_db::mcp::mcp_server::McpSystemError::InvalidParameters => {
            McpSystemError::InvalidParameters
        }
        qualia_core_db::mcp::mcp_server::McpSystemError::ParseError => McpSystemError::ParseError,
        qualia_core_db::mcp::mcp_server::McpSystemError::SanctuaryGateTriggered => {
            McpSystemError::SanctuaryGateTriggered
        }
        _ => McpSystemError::ToolNotReady,
    }
}

/// `evaluate_modality` tool — evaluates LTL, ASP, deontic, epistemic, paraconsistent, and probabilistic modalities.
pub fn evaluate_modality(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::evaluate_modality(args).map_err(map_db_error)
}

/// `evaluate_logic_rules` tool — parses N3 rules and fires them in the Webizen VM.
pub fn evaluate_logic_rules(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::evaluate_logic_rules(args).map_err(map_db_error)
}

/// `symbolic_logic_infer` tool — defeasible forward-chaining and bounded SAT.
pub fn symbolic_logic_infer(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::symbolic_logic_infer(args).map_err(map_db_error)
}

/// `inject_test_quin` tool — injects a deterministic test Quin through the paraconsistent router.
pub fn inject_test_quin(args: &[u8]) -> Result<String, McpSystemError> {
    let v = crate::types::parse_tool_args(args)?;
    let subject = v.get("subject").and_then(serde_json::Value::as_u64).unwrap_or(42);
    let predicate = v.get("predicate").and_then(serde_json::Value::as_u64).unwrap_or_else(|| qualia_core_db::q_hash("q42:testClaim"));
    let object = v.get("object").and_then(serde_json::Value::as_u64).unwrap_or(1);
    let context = v.get("context").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let candidate = qualia_core_db::NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: subject ^ predicate ^ object ^ context,
    };

    let mut consistent = [qualia_core_db::NQuin::default(); 8];
    let mut isolated = [qualia_core_db::NQuin::default(); 8];
    let (c, i) = qualia_core_db::modalities::paraconsistent::route_paraconsistent(
        &[candidate],
        &mut consistent,
        &mut isolated,
    )
    .map_err(|_| McpSystemError::ParseError)?;

    let payload = serde_json::json!({
        "status": "success",
        "consistentCount": c,
        "isolatedCount": i,
        "total": c + i,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}
