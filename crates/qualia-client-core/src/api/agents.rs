//! Agent roster, MCP tool loop

#![allow(non_snake_case)]

use super::*;

use std::path::Path;

fn agent_roster_storage() -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("Application not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    Ok(storage)
}

pub fn agent_roster_list() -> Result<serde_json::Value, String> {
    let storage = agent_roster_storage()?;
    let roster = crate::agent_registry::load_roster(Path::new(&storage));
    serde_json::to_value(roster).map_err(|e| e.to_string())
}

pub fn agent_roster_get(slug: String) -> Result<serde_json::Value, String> {
    let storage = agent_roster_storage()?;
    match crate::agent_registry::get_agent(Path::new(&storage), &slug) {
        Some(a) => serde_json::to_value(a).map_err(|e| e.to_string()),
        None => Ok(serde_json::Value::Null),
    }
}

pub fn agent_roster_upsert(agent_json: String) -> Result<(), String> {
    let storage = agent_roster_storage()?;
    let agent: crate::agent_registry::AgentDefinition =
        serde_json::from_str(&agent_json).map_err(|e| format!("invalid agent JSON: {e}"))?;
    crate::agent_registry::upsert_agent(Path::new(&storage), agent)
}

pub fn agent_roster_remove(slug: String) -> Result<(), String> {
    let storage = agent_roster_storage()?;
    crate::agent_registry::remove_agent(Path::new(&storage), &slug)
}

// ── Principal-gated MCP tool loop (U3-A / U3-B) ────────────────────────────────
// Propose → Permit/Deny → execute. Deny never dispatches. Allowlist from roster.

/// List local in-process MCP tools (`tools/list`) for Talk / allowlist UI.
pub fn mcp_list_local_tools() -> Result<serde_json::Value, String> {
    let tools = crate::mcp_tool_loop::mcp_list_local_tools()?;
    serde_json::to_value(tools).map_err(|e| e.to_string())
}

/// Principal-gated MCP tool call. `principal_permitted = false` → Err without MCP.
/// Tool must be on the agent's `allowed_mcp_tools` (or `*`); empty allowlist denies all.
pub fn mcp_call_tool_gated(
    agent_slug: String,
    tool_name: String,
    arguments_json: String,
    principal_permitted: bool,
) -> Result<String, String> {
    let storage = agent_roster_storage()?;
    crate::mcp_tool_loop::mcp_call_tool_gated(
        Path::new(&storage),
        &agent_slug,
        &tool_name,
        &arguments_json,
        principal_permitted,
    )
}

/// Convenience: set `allowed_mcp_tools` on a roster agent (persist via upsert).
pub fn agent_set_allowed_mcp_tools(slug: String, tools: Vec<String>) -> Result<(), String> {
    let storage = agent_roster_storage()?;
    crate::mcp_tool_loop::agent_set_allowed_mcp_tools(Path::new(&storage), &slug, tools)
}

/// If allowlist is empty, seed `list_capabilities` + `computer_vision` (dogfood-safe).
/// Does not Permit any call — only widens the roster allowlist.
pub fn mcp_ensure_safe_tool_allowlist(slug: String) -> Result<serde_json::Value, String> {
    let storage = agent_roster_storage()?;
    let tools = crate::mcp_tool_loop::ensure_safe_tool_allowlist(Path::new(&storage), &slug)?;
    serde_json::to_value(tools).map_err(|e| e.to_string())
}

/// Convenience: create/update a REMOTE-MCP agent from primitives so the UI never hand-builds the
/// backend enum. `transport_kind` ∈ `"tcp"` | `"http"` | `"stdio"`; `endpoint` is `host:port` / a URL /
/// a command line respectively.
pub fn agent_roster_add_remote(
    slug: String,
    display_name: String,
    transport_kind: String,
    endpoint: String,
    infer_tool: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    use crate::agent_registry::{AgentBackendSpec, McpTransport};
    if slug.trim().is_empty() {
        return Err("agent slug is required".to_string());
    }
    let transport = match transport_kind.to_lowercase().as_str() {
        "tcp" => {
            let (host, port) = endpoint
                .rsplit_once(':')
                .ok_or_else(|| "TCP endpoint must be host:port".to_string())?;
            let port: u16 = port
                .trim()
                .parse()
                .map_err(|_| "invalid TCP port".to_string())?;
            McpTransport::Tcp {
                host: host.trim().to_string(),
                port,
            }
        }
        "http" => McpTransport::Http {
            url: endpoint.trim().to_string(),
        },
        "stdio" => {
            let mut parts = endpoint.split_whitespace().map(|s| s.to_string());
            let command = parts
                .next()
                .ok_or_else(|| "stdio endpoint needs a command".to_string())?;
            McpTransport::Stdio {
                command,
                args: parts.collect(),
            }
        }
        other => return Err(format!("unknown transport '{other}' (use tcp|http|stdio)")),
    };
    let backend = AgentBackendSpec::RemoteMcp {
        endpoint: endpoint.trim().to_string(),
        transport,
        infer_tool: infer_tool.filter(|s| !s.trim().is_empty()),
        model: model.filter(|s| !s.trim().is_empty()),
    };
    let mut agent = crate::agent_registry::AgentDefinition::new(
        slug,
        display_name,
        "Remote agent reached over MCP.".to_string(),
        backend,
        system_prompt.unwrap_or_default(),
    );
    agent.enabled = true;
    let storage = agent_roster_storage()?;
    crate::agent_registry::upsert_agent(Path::new(&storage), agent)
}

/// Backend kind of a roster agent: `"local"` | `"remote"` (unknown/empty slug → `"local"`).
pub fn agent_backend_kind(slug: Option<String>) -> Result<String, String> {
    let slug = match slug {
        Some(s) if !s.is_empty() => s,
        _ => return Ok("local".to_string()),
    };
    let storage = agent_roster_storage()?;
    match crate::agent_registry::get_agent(Path::new(&storage), &slug) {
        Some(a) => Ok(match a.backend {
            crate::agent_registry::AgentBackendSpec::LocalEngine { .. } => "local".to_string(),
            crate::agent_registry::AgentBackendSpec::RemoteMcp { .. } => "remote".to_string(),
        }),
        None => Ok("local".to_string()),
    }
}

/// Run one turn against a REMOTE-MCP agent from the roster (native-only). Privacy-gated via the job
/// router (Classified/sanctuary never leaves the device), then issues an MCP `tools/call` to the
/// provider and appends the reply as an agent message. Returns a ChatInferenceResult-shaped JSON.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_remote_agent_turn(
    session_id: String,
    slug: String,
    prompt: String,
) -> Result<serde_json::Value, String> {
    use crate::agent_registry::AgentBackendSpec;
    let storage = agent_roster_storage()?;
    let agent = crate::agent_registry::get_agent(Path::new(&storage), &slug)
        .ok_or_else(|| format!("no agent '{slug}' in roster"))?;
    if !agent.enabled {
        return Ok(remote_turn_blocked(&format!(
            "agent '{}' is disabled",
            agent.display_name
        )));
    }
    let (transport, infer_tool, model) = match &agent.backend {
        AgentBackendSpec::RemoteMcp {
            transport,
            infer_tool,
            model,
            ..
        } => (transport.clone(), infer_tool.clone(), model.clone()),
        AgentBackendSpec::LocalEngine { .. } => {
            return Err("agent is local — use the local inference path".to_string())
        }
    };

    // Privacy-first placement: a configured remote agent implies consent, but sanctuary/Classified
    // context must never leave the device.
    let local_active = crate::model_lifecycle::lifecycle_label(
        crate::model_lifecycle::get_model_lifecycle_state(),
    ) == "Active";
    let inputs = crate::job_router::RoutingInputs {
        sensitivity: wellfare_core::record::SensitivityClass::Restricted,
        local_available: local_active,
        external_consented: true,
        requires_capability: None,
        local_has_capability: false,
        estimated_cost_microcents: 0,
    };
    match crate::job_router::route_job(&inputs, &crate::job_router::RoutingPolicy::default()) {
        crate::job_router::RoutingDecision::Blocked { reason }
        | crate::job_router::RoutingDecision::NeedsConsent { reason } => {
            return Ok(remote_turn_blocked(&reason));
        }
        _ => {}
    }

    let system = if agent.system_prompt.trim().is_empty() {
        None
    } else {
        Some(agent.system_prompt.as_str())
    };
    let text = crate::remote_mcp::remote_mcp_infer(
        &transport,
        infer_tool.as_deref(),
        model.as_deref(),
        system,
        &prompt,
    )?;
    if !text.trim().is_empty() {
        let _ = append_chat_message(session_id, "agent".to_string(), text.clone());
    }
    Ok(serde_json::json!({
        "text": text,
        "committed": true,
        "block_reason": serde_json::Value::Null,
        "agent_backend": "remote",
        "model_id": model,
        "provenance_hashes": [],
        "citations": [],
        "tokens_generated": 0,
        "inference_duration_ms": 0,
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn remote_turn_blocked(reason: &str) -> serde_json::Value {
    serde_json::json!({
        "text": "",
        "committed": false,
        "block_reason": reason,
        "agent_backend": "remote",
    })
}

/// Store a chat turn's inline CML context (`#project:` / `#topic:` / `#task:` / `[[concept]]`) into the
/// person's inforg (their private library). No-op if the message has no tags. Returns concepts stored.
pub fn ingest_chat_cml(session_id: String, text: String) -> Result<usize, String> {
    let storage = agent_roster_storage()?;
    crate::cml_context::ingest_turn(Path::new(&storage), &session_id, &text).map(|v| v.len())
}
