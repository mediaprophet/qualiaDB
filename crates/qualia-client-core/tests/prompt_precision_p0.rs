//! Prompt-precision Wave 0 fixtures (PP-002…PP-004).
//! Non-network observation of current product behaviour — not behaviour fixes.

use qualia_client_core::agent_registry::{
    AgentBackendSpec, AgentDefinition, AgentSemanticFacet, AgentSemanticProfile, AgentSemanticTag,
};

/// Reconstruct the documented named-local augmentation order from CODEBASE-MAP /
/// `build_augmented_packet` (capability → semantic briefing → … → user). Does **not**
/// insert `AgentDefinition.system_prompt`.
fn observe_named_local_augmented_surface(
    capability_briefing: &str,
    semantic_briefing: &str,
    routing_brief: &str,
    user_prompt: &str,
) -> String {
    format!(
        "{}\n\n{}\n\n{}\n\n\n\n\n\n\n\n---\nUser: {}\n---",
        capability_briefing, semantic_briefing, routing_brief, user_prompt
    )
}

/// Documented MCP flatten — delegates to the real production lowering adapter.
fn observe_mcp_flat_prompt(system: Option<&str>, prompt: &str) -> String {
    let (args, _receipt) = qualia_client_core::conditioning::lower_mcp_tool_arguments(
        system, prompt, false,
    );
    args["prompt"].as_str().unwrap().to_string()
}

/// Documented remote-turn success JSON shape from `api::agents::run_remote_agent_turn`
/// (no network — static contract fixture). Unknown usage is reported as
/// `null` + `"unknown"`, never as measured zeros; the session append and the
/// graph commit are reported separately.
fn observe_remote_success_envelope(text: &str, model: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "text": text,
        "committed": true,
        "session_appended": true,
        "wal_committed": false,
        "block_reason": serde_json::Value::Null,
        "agent_backend": "remote",
        "model_id": model,
        "provenance_hashes": [],
        "citations": [],
        "tokens_generated": serde_json::Value::Null,
        "token_measurement": "unknown",
        "inference_duration_ms": 42,
        "latency_measurement": "measured",
        "conditioning": serde_json::Value::Null,
        "mcp_lowering": {
            "mode": "flattened",
            "role_degraded": false,
            "degradation_reason": serde_json::Value::Null
        },
    })
}

#[test]
fn pp002_named_local_role_surface_orders_instruction_then_user() {
    let surface = observe_named_local_augmented_surface(
        "CAPABILITY",
        "SEMANTIC",
        "ROUTING",
        "please summarise",
    );
    let user_idx = surface.find("---\nUser:").expect("user delimiter");
    let cap_idx = surface.find("CAPABILITY").expect("capability");
    let sem_idx = surface.find("SEMANTIC").expect("semantic");
    assert!(cap_idx < sem_idx && sem_idx < user_idx);
    // Roles are still concatenated text — fixture records that boundary today.
    assert!(!surface.contains("<|im_start|>system"));
}

#[test]
fn pp003_named_local_omits_roster_system_prompt_uses_semantic_briefing() {
    let mut agent = AgentDefinition::new(
        "pp-local",
        "PP Local",
        "fixture agent",
        AgentBackendSpec::LocalEngine { model_id: None },
        "ROSTER_SYSTEM_PROMPT_MUST_NOT_APPEAR",
    );
    agent.semantic_profile = AgentSemanticProfile {
        tags: vec![AgentSemanticTag {
            iri: "urn:qualia:tag:fixture".into(),
            label: "FixtureClass".into(),
            facet: AgentSemanticFacet::Classification,
            broader_iri: None,
        }],
    };
    let briefing = agent.semantic_profile.briefing();
    assert!(
        briefing.contains("FixtureClass") || briefing.contains("urn:qualia:tag:fixture"),
        "semantic briefing should surface profile tags; got: {briefing:?}"
    );

    let surface = observe_named_local_augmented_surface(
        "capability-brief",
        &briefing,
        "routing-brief",
        "user task",
    );
    assert!(
        !surface.contains("ROSTER_SYSTEM_PROMPT_MUST_NOT_APPEAR"),
        "named-local augmentation must omit AgentDefinition.system_prompt (observed gap)"
    );
    assert!(surface.contains(&briefing) || surface.contains("FixtureClass"));

    // Contrast: remote MCP flatten *does* carry system text when present.
    let flat = observe_mcp_flat_prompt(Some(&agent.system_prompt), "user task");
    assert!(flat.starts_with("ROSTER_SYSTEM_PROMPT_MUST_NOT_APPEAR"));
    assert!(flat.ends_with("user task"));
}

#[test]
fn pp003_run_chat_inference_for_agent_passes_semantic_not_system_prompt() {
    // Source-level observation: the binding call site selects semantic_profile only.
    // Full end-to-end needs APP_STATE; this fixture locks the API contract in tree.
    let src = include_str!("../src/chat_inference.rs");
    assert!(
        src.contains("semantic_profile: Some(agent.semantic_profile)"),
        "named-local agent turn must pass semantic_profile"
    );
    let agent_fn = src
        .split("pub fn run_chat_inference_for_agent")
        .nth(1)
        .expect("run_chat_inference_for_agent");
    let agent_fn = agent_fn.split("pub fn run_chat_inference_full").next().unwrap();
    assert!(
        !agent_fn.contains("system_prompt"),
        "run_chat_inference_for_agent must not reference system_prompt (omission fixture)"
    );
}

#[test]
fn pp004_mcp_flattens_system_and_user_into_single_prompt_argument() {
    // Production request building goes through the shared lowering adapter.
    let src = include_str!("../src/remote_mcp.rs");
    assert!(
        src.contains("lower_mcp_tool_arguments"),
        "remote MCP must lower via the shared adapter with a degradation receipt"
    );

    // Unknown/absent system-role capability → flattened single argument.
    let flat = observe_mcp_flat_prompt(Some("Be terse."), "hi");
    assert_eq!(flat, "Be terse.\n\nhi");
    let wire = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "llm_infer",
            "arguments": { "prompt": flat }
        }
    });
    assert_eq!(wire["method"], "tools/call");
    assert!(wire["params"]["arguments"].get("system").is_none());
    assert_eq!(wire["params"]["arguments"]["prompt"], "Be terse.\n\nhi");

    // Declared system-role capability → structured arguments, no degradation.
    let (args, receipt) = qualia_client_core::conditioning::lower_mcp_tool_arguments(
        Some("Be terse."), "hi", true,
    );
    assert_eq!(args["system"], "Be terse.");
    assert_eq!(args["prompt"], "hi");
    assert_eq!(
        receipt.mode,
        qualia_client_core::conditioning::McpLoweringMode::Structured
    );
    assert!(!receipt.role_degraded);
}

#[test]
fn pp004_remote_result_reports_unknown_usage_and_real_latency() {
    // Static fixture of the remote-turn envelope: unknown usage stays unknown
    // (null + "unknown"), measured latency is real, and session append is
    // reported separately from any graph commit.
    let env = observe_remote_success_envelope("hello", Some("phi-3"));
    assert_eq!(env["committed"], true);
    assert_eq!(env["wal_committed"], false);
    assert!(env["citations"].as_array().unwrap().is_empty());
    assert_eq!(env["tokens_generated"], serde_json::Value::Null);
    assert_eq!(env["token_measurement"], "unknown");
    assert_eq!(env["latency_measurement"], "measured");
    assert_eq!(env["inference_duration_ms"], 42);

    // Production source must not fabricate zero usage or blanket commit.
    let src = include_str!("../src/api/agents.rs");
    let turn_fn = src
        .split("pub fn run_remote_agent_turn")
        .nth(1)
        .expect("run_remote_agent_turn");
    assert!(
        !turn_fn.contains("\"tokens_generated\": 0"),
        "remote turn must not report unmeasured token usage as zero"
    );
    assert!(turn_fn.contains("\"token_measurement\": \"unknown\""));
    assert!(turn_fn.contains("\"mcp_lowering\""));
    assert!(turn_fn.contains("prepare_active_semantic_request"));
}
