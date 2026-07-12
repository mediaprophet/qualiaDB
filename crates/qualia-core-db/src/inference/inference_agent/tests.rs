use super::*;

fn make_agent() -> LocalLlmAgent {
    LocalLlmAgent::new(
        "did:git:antigravity-llm-001",
        "~/.qualia/models/phi3-mini.gguf",
    )
}

#[test]
fn test_webizen_blocks_outbound_network() {
    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: 0xAABB,
        requested_graph_scope: vec![],
        context_namespaces: vec![],
        requires_network: true,
        ilp_offer_micro_cents: 0,
        principal_did_hash: 0,
        mcp_intent_frame_hash: 0xAABB,
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    let verdict = agent.validate_intent(&intent);
    assert!(
        matches!(verdict, WebizenVerdict::Deny { .. }),
        "Webizen must block outbound calls from local backend"
    );
}

#[test]
fn test_webizen_blocks_sanctuary_scope() {
    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: 0xAABB,
        requested_graph_scope: vec![SANCTUARY_SCOPE_WEBIZEN],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: 0,
        mcp_intent_frame_hash: 0xAABB,
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    let verdict = agent.validate_intent(&intent);
    assert!(
        matches!(verdict, WebizenVerdict::Deny { .. }),
        "Webizen must block Sanctuary scope access"
    );
}

#[test]
fn test_webizen_permits_valid_local_intent() {
    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: 0xAABB,
        requested_graph_scope: vec![0xDEAD_BEEF],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: 0,
        mcp_intent_frame_hash: 0xAABB,
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);
}

#[test]
fn test_full_roundtrip_grounded_output() {
    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: 0xAABB,
        requested_graph_scope: vec![0x1234],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: 0,
        mcp_intent_frame_hash: 0xAABB,
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);

    let output = agent
        .infer("What is my health status?", "graph_context_bytes_here")
        .unwrap();
    assert!(!output.text.is_empty());

    let post_verdict = agent.validate_output(&output);
    assert_eq!(
        post_verdict,
        WebizenVerdict::Permit,
        "Grounded output should pass post-flight check"
    );
}

#[test]
fn test_webizen_blocks_ungrounded_output() {
    let agent = make_agent();
    let ungrounded = AgentOutput {
        text: "I made this up with no sources.".into(),
        semantic_quin: None,
        provenance_quins: vec![], // <-- no citations
        tokens_generated: 10,
        inference_duration_ms: 5,
        peak_memory_bytes: 0,
    };
    let verdict = agent.validate_output(&ungrounded);
    assert!(
        matches!(verdict, WebizenVerdict::Deny { .. }),
        "Webizen must block ungrounded output"
    );
}

#[test]
fn test_validate_intent_enables_sieve_for_graph_mutation() {
    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: 0xAABB,
        requested_graph_scope: vec![0x1234],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: 0,
        mcp_intent_frame_hash: 0xAABB,
        output_mode: N3OutputMode::GraphMutation,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);
    assert!(agent
        .use_sieve_output
        .load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn test_zero_allocation_adversarial_conduct_denial() {
    let _profiler = dhat::Profiler::builder().testing().build();

    let agent = make_agent();
    let intent = AgentIntent {
        intent_predicate: crate::q_hash("llm:AdversarialOperation"),
        requested_graph_scope: vec![],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: crate::q_hash("did:q42:human-rights-test-subject"),
        mcp_intent_frame_hash: crate::q_hash("purpose:General"),
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };

    // Warm up any internal system components that might allocate on first use
    let _ = std::time::SystemTime::now();

    let stats_before = dhat::HeapStats::get();

    // Execute the intent validation (hot path)
    let verdict = agent.validate_intent(&intent);

    let stats_after = dhat::HeapStats::get();

    // Verify we got the Deny verdict with the NQuin
    if let WebizenVerdict::Deny { conduct_record, .. } = verdict {
        assert!(
            conduct_record.is_some(),
            "Conduct record Quin must be generated"
        );
        let quin = conduct_record.unwrap();
        assert_eq!(quin.predicate, crate::q_hash("q42:conductViolation"));
    } else {
        panic!("Expected Deny verdict for adversarial operation");
    }

    // Assert ABSOLUTELY ZERO heap allocations occurred during validate_intent
    assert_eq!(
        stats_after.total_blocks - stats_before.total_blocks,
        0,
        "validate_intent must not allocate on the heap"
    );
    assert_eq!(
        stats_after.total_bytes - stats_before.total_bytes,
        0,
        "validate_intent must not allocate on the heap"
    );
}
