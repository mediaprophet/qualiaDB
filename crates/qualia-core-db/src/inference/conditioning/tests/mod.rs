//! Tests for portable conditioning compiler (P1C).

use super::*;

fn fixture_authority<'a>(scopes: &'a [u64]) -> AuthorityView<'a> {
    AuthorityView {
        principal_did_hash: 0x1234_5678,
        disclosure_ceiling: 2,
        allowed_graph_scopes: scopes,
        tools_allowed: true,
    }
}

fn fixture_spec<'a>(requirements: &'a [RequirementRef<'a>]) -> ConditioningSpec<'a> {
    ConditioningSpec {
        schema_version: 1,
        profile_id: "urn:qualia:conditioning:test-profile:v1",
        objective: "Perform test analysis with high precision",
        requirements,
        domain_refs: &[1, 2],
        output_contract: OutputContractRef {
            schema_hint: Some("json-v1"),
            min_citations: 1,
        },
        budget: ConditioningBudget {
            input_tokens: 1024,
            output_tokens: 256,
            tool_rounds: 2,
            max_bytes: 4096,
        },
    }
}

#[test]
fn test_conditioning_compilation_and_receipts() {
    let reqs = [
        RequirementRef {
            id: "R1",
            class: RequirementClass::Enforced,
            rule: "Must validate against schema",
            validator: Some("schema-validator-v1"),
            required: true,
            priority: 1,
        },
        RequirementRef {
            id: "R2",
            class: RequirementClass::EvidenceObligation,
            rule: "Must cite evidence fact",
            validator: None,
            required: false,
            priority: 2,
        },
        RequirementRef {
            id: "R3",
            class: RequirementClass::Guidance,
            rule: "Keep responses concise",
            validator: None,
            required: false,
            priority: 3,
        },
    ];

    let spec = fixture_spec(&reqs);
    let scopes = [100, 200];
    let authority = fixture_authority(&scopes);
    let caps = BackendCapabilities::native_gguf_baseline();

    let evidence = [
        EvidencePart {
            source_id: "doc-1",
            scope: 100,
            sensitivity: 1,
            content: "Evidence content alpha",
            qualifier: None,
        },
        EvidencePart {
            source_id: "doc-2",
            scope: 999, // disallowed scope
            sensitivity: 1,
            content: "Disallowed scope evidence",
            qualifier: None,
        },
        EvidencePart {
            source_id: "doc-3",
            scope: 100,
            sensitivity: 5, // exceeds disclosure ceiling 2
            content: "Top secret evidence",
            qualifier: None,
        },
    ];

    let mut outcomes_buf = [RequirementOutcome::applied(""); 8];
    let mut evidence_buf = [EvidencePart {
        source_id: "",
        scope: 0,
        sensitivity: 0,
        content: "",
        qualifier: None,
    }; 8];

    let mut buffers = CompileBuffers {
        outcomes: &mut outcomes_buf,
        selected_evidence: &mut evidence_buf,
    };

    let plan = compile_into(&spec, &authority, &caps, &evidence, &mut buffers)
        .expect("compilation should succeed");

    assert_eq!(plan.requirements_count, 3);
    assert_eq!(plan.outcomes_count, 3);
    // Only doc-1 should be selected (doc-2 disallowed scope, doc-3 exceeds sensitivity)
    assert_eq!(plan.evidence_count, 1);
    assert_eq!(buffers.selected_evidence[0].source_id, "doc-1");

    // Check requirement outcomes
    assert_eq!(buffers.outcomes[0].id, "R1");
    assert_eq!(
        buffers.outcomes[0].disposition,
        RequirementDisposition::Enforced
    );
    assert_eq!(buffers.outcomes[1].id, "R2");
    assert_eq!(
        buffers.outcomes[1].disposition,
        RequirementDisposition::Applied
    );
    assert_eq!(buffers.outcomes[2].id, "R3");
    assert_eq!(
        buffers.outcomes[2].disposition,
        RequirementDisposition::Applied
    );

    // Test rendering
    let mut render_buf = [0u8; 2048];
    let rendered = render_into(
        &plan,
        &spec,
        &buffers.selected_evidence[..plan.evidence_count],
        RenderTarget::NativeChatml,
        &mut render_buf,
    )
    .expect("rendering should succeed");

    let text = std::str::from_utf8(&render_buf[..rendered.bytes_written]).unwrap();
    assert!(text.contains("<|im_start|>system"));
    assert!(text.contains("Must validate against schema"));
    assert!(text.contains("Evidence content alpha"));
    assert!(!text.contains("Disallowed scope evidence"));
}

#[test]
fn test_conditioning_cbor_codec_roundtrip() {
    let plan = CompiledPlanSummary {
        plan_id: 0xdead_beef_cafe_babe,
        profile_id: "urn:qualia:conditioning:cbor-test",
        objective: "Test CBOR roundtrip serialization",
        requirements_count: 5,
        outcomes_count: 5,
        evidence_count: 2,
        budget: ConditioningBudget {
            input_tokens: 512,
            output_tokens: 128,
            tool_rounds: 1,
            max_bytes: 2048,
        },
    };

    let mut buf = [0u8; 512];
    let bytes_written = encode_plan_cbor(&plan, &mut buf).expect("encoding should succeed");
    assert!(bytes_written > 0);

    let decoded = decode_plan_cbor(&buf[..bytes_written]).expect("decoding should succeed");
    assert_eq!(decoded.plan_id, plan.plan_id);
    assert_eq!(decoded.profile_id, plan.profile_id);
    assert_eq!(decoded.objective, plan.objective);
    assert_eq!(decoded.requirements_count, plan.requirements_count);
    assert_eq!(decoded.outcomes_count, plan.outcomes_count);
    assert_eq!(decoded.evidence_count, plan.evidence_count);
}

#[test]
fn test_unauthorized_tools_disclosure_denied() {
    let reqs = [RequirementRef {
        id: "R1",
        class: RequirementClass::Guidance,
        rule: "Some rule",
        validator: None,
        required: true,
        priority: 1,
    }];

    let mut spec = fixture_spec(&reqs);
    spec.budget.tool_rounds = 3;

    let mut authority = fixture_authority(&[]);
    authority.tools_allowed = false; // principal denied tools

    assert_eq!(
        validate_spec(&spec, &authority),
        Err(ConditioningError::DisclosureDenied)
    );
}

#[test]
fn test_enforced_requirement_missing_validator_fails() {
    let reqs = [RequirementRef {
        id: "R1",
        class: RequirementClass::Enforced,
        rule: "Rule without validator",
        validator: None, // Missing validator on Enforced constraint!
        required: true,
        priority: 1,
    }];

    let spec = fixture_spec(&reqs);
    let authority = fixture_authority(&[]);

    assert_eq!(
        validate_spec(&spec, &authority),
        Err(ConditioningError::ValidationFailed)
    );
}
