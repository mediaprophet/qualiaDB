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

/// Documented MCP flatten (mirrors `remote_mcp::build_infer_request`).
fn observe_mcp_flat_prompt(system: Option<&str>, prompt: &str) -> String {
    match system {
        Some(sys) if !sys.trim().is_empty() => format!("{sys}\n\n{prompt}"),
        _ => prompt.to_string(),
    }
}

/// Documented remote-turn success JSON shape from `api::agents::run_remote_agent_turn`
/// (no network — static contract fixture).
fn observe_remote_success_envelope(text: &str, model: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "text": text,
        "committed": true,
        "block_reason": serde_json::Value::Null,
        "agent_backend": "remote",
        "model_id": model,
        "provenance_hashes": [],
        "citations": [],
        "tokens_generated": 0,
        "inference_duration_ms": 0,
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
}

#[test]
fn pp004_remote_result_committed_true_with_zero_usage_is_not_measured_zero() {
    let env = observe_remote_success_envelope("hello", Some("phi-3"));
    assert_eq!(env["committed"], true);
    assert!(env["citations"].as_array().unwrap().is_empty());
    assert_eq!(env["tokens_generated"], 0);
    assert_eq!(env["inference_duration_ms"], 0);
    // Honesty: these zeros are unset/unknown coverage, not proven measured usage.
    const USAGE_COVERAGE_KNOWN: bool = false;
    assert!(
        !USAGE_COVERAGE_KNOWN,
        "remote path does not label usage coverage; treat 0 as unknown, not measured zero"
    );
}
