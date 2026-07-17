//! Principal-gated MCP tool loop (U3-A / U3-B).
//!
//! Talk and agent UIs never invoke local MCP tools without an explicit
//! principal Permit. Deny never reaches the MCP surface.
//!
//! Flow: **propose → Permit / Deny → execute (if Permit + allowlist) → result**.
//!
//! Allowlist source of truth: [`crate::agent_registry::AgentDefinition::allowed_mcp_tools`].
//! - Empty list → deny-all for tools
//! - `"*"` → all tools (use sparingly; prefer explicit names)
//! - otherwise exact tool name match via [`AgentDefinition::has_tool`]
//!
//! In-process dispatch uses
//! [`qualia_core_db::mcp::mcp_server::handle_jsonrpc_message`] — no second
//! LLM HTTP API, no external agent SDK.

use serde::{Deserialize, Serialize};

use crate::agent_registry::AgentDefinition;
use qualia_core_db::mcp::mcp_server::handle_jsonrpc_message;

/// Safe golden tools for dogfood (empty args or `{"op":"list"}`).
pub const SAFE_SEED_TOOLS: &[&str] = &["list_capabilities", "computer_vision"];

/// One entry from the local MCP `tools/list` surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
}

/// Outcome of the principal + allowlist gate (no MCP call).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// Principal did not Permit — never call MCP.
    DenyPrincipal,
    /// Tool is not on the agent's allowlist (and no `*`).
    DenyAllowlist,
    /// Both principal and allowlist permit — safe to dispatch.
    Allow,
}

/// Pure gate: principal flag + allowlist membership. Unit-testable without
/// APP_STATE or MCP.
pub fn evaluate_tool_gate(principal_permitted: bool, agent_has_tool: bool) -> GateDecision {
    if !principal_permitted {
        return GateDecision::DenyPrincipal;
    }
    if !agent_has_tool {
        return GateDecision::DenyAllowlist;
    }
    GateDecision::Allow
}

fn gate_error(decision: GateDecision) -> Option<&'static str> {
    match decision {
        GateDecision::DenyPrincipal => Some("denied by principal"),
        GateDecision::DenyAllowlist => Some("not on allowlist"),
        GateDecision::Allow => None,
    }
}

/// List local in-process MCP tools via JSON-RPC `tools/list`.
pub fn mcp_list_local_tools() -> Result<Vec<McpToolInfo>, String> {
    let req = r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list"}"#;
    let resp = handle_jsonrpc_message(req, false, false)
        .ok_or_else(|| "MCP tools/list returned no response".to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(&resp).map_err(|e| format!("tools/list parse: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!(
            "tools/list error: {}",
            err.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
        ));
    }
    let tools = v
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| "tools/list missing result.tools".to_string())?;
    let mut out = Vec::with_capacity(tools.len());
    for t in tools {
        let name = t
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = t
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        out.push(McpToolInfo { name, description });
    }
    Ok(out)
}

/// Dispatch one MCP `tools/call` in-process. **Caller must already have
/// passed the principal + allowlist gate.** Prefer
/// [`mcp_call_tool_gated_for_agent`].
pub fn dispatch_mcp_tool_call(tool_name: &str, arguments_json: &str) -> Result<String, String> {
    if tool_name.trim().is_empty() {
        return Err("tool name is required".to_string());
    }
    let args_val: serde_json::Value = if arguments_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(arguments_json)
            .map_err(|e| format!("arguments_json is not valid JSON: {e}"))?
    };
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "gated-call",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": args_val,
        }
    });
    let req_str =
        serde_json::to_string(&request).map_err(|e| format!("serialize tools/call: {e}"))?;
    // Local tools: no QPU/LLM side-channel for the gated Talk path (fail-closed extras off).
    let resp = handle_jsonrpc_message(&req_str, false, false)
        .ok_or_else(|| "MCP tools/call returned no response".to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(&resp).map_err(|e| format!("tools/call parse: {e}"))?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("MCP error");
        return Err(msg.to_string());
    }
    // Prefer text content array (MCP standard); fall back to whole result.
    if let Some(content) = v
        .pointer("/result/content")
        .and_then(|c| c.as_array())
    {
        let mut texts = Vec::new();
        for item in content {
            if let Some(t) = item.get("text").and_then(|x| x.as_str()) {
                texts.push(t.to_string());
            }
        }
        if !texts.is_empty() {
            return Ok(texts.join("\n"));
        }
    }
    if let Some(result) = v.get("result") {
        return Ok(result.to_string());
    }
    Ok(resp)
}

/// Gate + optional dispatch for a concrete [`AgentDefinition`] (testable without storage).
///
/// Errors (fail-closed, no MCP on deny paths):
/// - `"denied by principal"` when `principal_permitted` is false
/// - `"not on allowlist"` when the tool is not permitted for this agent
pub fn mcp_call_tool_gated_for_agent(
    agent: &AgentDefinition,
    tool_name: &str,
    arguments_json: &str,
    principal_permitted: bool,
) -> Result<String, String> {
    let decision = evaluate_tool_gate(principal_permitted, agent.has_tool(tool_name));
    if let Some(msg) = gate_error(decision) {
        return Err(msg.to_string());
    }
    dispatch_mcp_tool_call(tool_name, arguments_json)
}

/// Load agent from roster, gate, then dispatch. Empty slug resolves to `"local"`.
pub fn mcp_call_tool_gated(
    storage_root: &std::path::Path,
    agent_slug: &str,
    tool_name: &str,
    arguments_json: &str,
    principal_permitted: bool,
) -> Result<String, String> {
    let slug = if agent_slug.trim().is_empty() {
        "local"
    } else {
        agent_slug.trim()
    };
    let agent = crate::agent_registry::get_agent(storage_root, slug)
        .ok_or_else(|| format!("no agent '{slug}' in roster"))?;
    if !agent.enabled {
        return Err(format!("agent '{slug}' is disabled"));
    }
    mcp_call_tool_gated_for_agent(&agent, tool_name, arguments_json, principal_permitted)
}

/// Set `allowed_mcp_tools` on an existing roster agent and persist.
pub fn agent_set_allowed_mcp_tools(
    storage_root: &std::path::Path,
    slug: &str,
    tools: Vec<String>,
) -> Result<(), String> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err("agent slug is required".to_string());
    }
    let mut agent = crate::agent_registry::get_agent(storage_root, slug)
        .ok_or_else(|| format!("no agent '{slug}' in roster"))?;
    agent.allowed_mcp_tools = tools
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    crate::agent_registry::upsert_agent(storage_root, agent)
}

/// If the agent's allowlist is empty, seed the safe golden tools and persist.
/// Returns the (possibly updated) allowlist. Does **not** auto-Permit any call.
pub fn ensure_safe_tool_allowlist(
    storage_root: &std::path::Path,
    slug: &str,
) -> Result<Vec<String>, String> {
    let slug = if slug.trim().is_empty() {
        "local"
    } else {
        slug.trim()
    };
    let mut agent = crate::agent_registry::get_agent(storage_root, slug)
        .ok_or_else(|| format!("no agent '{slug}' in roster"))?;
    if agent.allowed_mcp_tools.is_empty() {
        agent.allowed_mcp_tools = SAFE_SEED_TOOLS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        crate::agent_registry::upsert_agent(storage_root, agent.clone())?;
    }
    Ok(agent.allowed_mcp_tools)
}

// ── Tests: gate deny / allowlist reject never dispatch ───────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_registry::{AgentBackendSpec, AgentDefinition};
    use tempfile::tempdir;

    fn agent_with_tools(tools: Vec<&str>) -> AgentDefinition {
        let mut a = AgentDefinition::new(
            "local",
            "Local",
            "test",
            AgentBackendSpec::LocalEngine { model_id: None },
            "persona",
        );
        a.allowed_mcp_tools = tools.into_iter().map(|s| s.to_string()).collect();
        a
    }

    #[test]
    fn gate_deny_principal_without_dispatch() {
        assert_eq!(
            evaluate_tool_gate(false, true),
            GateDecision::DenyPrincipal
        );
        let agent = agent_with_tools(vec!["list_capabilities"]);
        let err = mcp_call_tool_gated_for_agent(&agent, "list_capabilities", "{}", false)
            .expect_err("must deny");
        assert_eq!(err, "denied by principal");
    }

    #[test]
    fn gate_deny_allowlist_without_dispatch() {
        assert_eq!(
            evaluate_tool_gate(true, false),
            GateDecision::DenyAllowlist
        );
        // Empty allowlist = deny-all
        let agent = agent_with_tools(vec![]);
        let err = mcp_call_tool_gated_for_agent(&agent, "list_capabilities", "{}", true)
            .expect_err("must reject allowlist");
        assert_eq!(err, "not on allowlist");

        // Explicit list without this tool
        let agent = agent_with_tools(vec!["computer_vision"]);
        let err = mcp_call_tool_gated_for_agent(&agent, "list_capabilities", "{}", true)
            .expect_err("must reject");
        assert_eq!(err, "not on allowlist");
    }

    #[test]
    fn gate_allow_when_principal_and_tool_listed() {
        assert_eq!(evaluate_tool_gate(true, true), GateDecision::Allow);
        let agent = agent_with_tools(vec!["list_capabilities"]);
        // Real in-process MCP dispatch for golden tool
        let out = mcp_call_tool_gated_for_agent(&agent, "list_capabilities", "{}", true)
            .expect("permit + allowlist should dispatch");
        assert!(!out.is_empty(), "list_capabilities should return text");
    }

    #[test]
    fn wildcard_allowlist_permits_any_named_tool_gate() {
        let agent = agent_with_tools(vec!["*"]);
        assert!(agent.has_tool("list_capabilities"));
        assert_eq!(
            evaluate_tool_gate(true, agent.has_tool("anything")),
            GateDecision::Allow
        );
    }

    #[test]
    fn list_local_tools_includes_safe_golden() {
        let tools = mcp_list_local_tools().expect("tools/list");
        assert!(
            tools.iter().any(|t| t.name == "list_capabilities"),
            "expected list_capabilities in catalogue"
        );
        assert!(
            tools.iter().any(|t| t.name == "computer_vision"),
            "expected computer_vision in catalogue"
        );
    }

    #[test]
    fn set_allowlist_and_ensure_seed_roundtrip() {
        let dir = tempdir().unwrap();
        // Materialise default local agent
        let a = crate::agent_registry::default_local_agent();
        crate::agent_registry::upsert_agent(dir.path(), a).unwrap();
        assert!(crate::agent_registry::get_agent(dir.path(), "local")
            .unwrap()
            .allowed_mcp_tools
            .is_empty());

        let seeded = ensure_safe_tool_allowlist(dir.path(), "local").unwrap();
        assert_eq!(seeded, vec!["list_capabilities", "computer_vision"]);

        agent_set_allowed_mcp_tools(
            dir.path(),
            "local",
            vec!["list_capabilities".into()],
        )
        .unwrap();
        let a = crate::agent_registry::get_agent(dir.path(), "local").unwrap();
        assert_eq!(a.allowed_mcp_tools, vec!["list_capabilities"]);

        // ensure does not re-expand a non-empty list
        let again = ensure_safe_tool_allowlist(dir.path(), "local").unwrap();
        assert_eq!(again, vec!["list_capabilities"]);
    }

    #[test]
    fn storage_gated_deny_principal() {
        let dir = tempdir().unwrap();
        let mut a = crate::agent_registry::default_local_agent();
        a.allowed_mcp_tools = vec!["list_capabilities".into()];
        crate::agent_registry::upsert_agent(dir.path(), a).unwrap();
        let err = mcp_call_tool_gated(dir.path(), "local", "list_capabilities", "{}", false)
            .expect_err("deny");
        assert_eq!(err, "denied by principal");
    }
}
