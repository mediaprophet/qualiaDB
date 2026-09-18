use super::*;
use crate::inference::conditioning::{
    compile_into, AuthorityView, BackendCapabilities, CompileBuffers, ConditioningBudget,
    ConditioningSpec, OutputContractRef, RequirementClass, RequirementOutcome, RequirementRef,
};
use vibe::conditioning::{
    ConditioningBudgetDto, ConditioningProfileDto, RequirementClassDto, RequirementDto,
};

fn extract_dto_from_json(v: &Value) -> Result<ConditioningProfileDto, String> {
    let target = v.get("profile").or_else(|| v.get("spec")).unwrap_or(v);
    let obj = target.as_object().ok_or("profile must be an object")?;

    let schema_version = obj.get("schema_version").and_then(Value::as_u64).unwrap_or(1) as u16;
    let profile_id = obj
        .get("profile_id")
        .and_then(Value::as_str)
        .unwrap_or("urn:qualia:profile:default")
        .to_string();
    let objective = obj.get("objective").and_then(Value::as_str).unwrap_or("").to_string();

    let mut domains = Vec::new();
    if let Some(ds) = obj.get("domains").and_then(Value::as_array) {
        for d in ds {
            if let Some(s) = d.as_str() {
                domains.push(s.to_string());
            }
        }
    }

    let mut requirements = Vec::new();
    if let Some(rs) = obj.get("requirements").and_then(Value::as_array) {
        for r in rs {
            if let Some(rm) = r.as_object() {
                let id = rm.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                let class_str = rm.get("class").and_then(Value::as_str).unwrap_or("enforced");
                let class = match class_str {
                    "evidence_obligation" | "evidence" => RequirementClassDto::EvidenceObligation,
                    "guidance" => RequirementClassDto::Guidance,
                    _ => RequirementClassDto::Enforced,
                };
                let rule = rm.get("rule").and_then(Value::as_str).unwrap_or("").to_string();
                let validator = rm.get("validator").and_then(Value::as_str).map(String::from);
                let required = rm.get("required").and_then(Value::as_bool).unwrap_or(true);
                let priority = rm.get("priority").and_then(Value::as_u64).unwrap_or(100) as u8;

                requirements.push(RequirementDto {
                    id,
                    class,
                    rule,
                    validator,
                    required,
                    priority,
                });
            }
        }
    }

    let budget = if let Some(b) = obj.get("budget").and_then(Value::as_object) {
        ConditioningBudgetDto {
            input_tokens: b.get("input_tokens").and_then(Value::as_u64).unwrap_or(4096) as u32,
            output_tokens: b.get("output_tokens").and_then(Value::as_u64).unwrap_or(1024) as u32,
            tool_rounds: b.get("tool_rounds").and_then(Value::as_u64).unwrap_or(4) as u16,
        }
    } else {
        ConditioningBudgetDto {
            input_tokens: 4096,
            output_tokens: 1024,
            tool_rounds: 4,
        }
    };

    Ok(ConditioningProfileDto {
        schema_version,
        profile_id,
        objective,
        domains,
        requirements,
        budget,
    })
}

/// MCP tool: `conditioning_validate`
/// Validate a conditioning profile specification against schema and structural rules.
pub fn conditioning_validate(args: &[u8]) -> Result<String, McpSystemError> {
    let raw = parse_tool_args(args)?;
    let dto = match extract_dto_from_json(&raw) {
        Ok(dto) => dto,
        Err(e) => {
            return Ok(json!({
                "valid": false,
                "profile_id": "unknown",
                "violations": [e],
                "digest": 0,
                "schema_version": 0,
            })
            .to_string())
        }
    };

    match dto.validate() {
        Ok(()) => Ok(json!({
            "valid": true,
            "profile_id": dto.profile_id,
            "violations": [],
            "digest": dto.content_digest(),
            "schema_version": dto.schema_version,
        })
        .to_string()),
        Err(e) => Ok(json!({
            "valid": false,
            "profile_id": dto.profile_id,
            "violations": [e],
            "digest": 0,
            "schema_version": dto.schema_version,
        })
        .to_string()),
    }
}

/// MCP tool: `conditioning_compile`
/// Compile a conditioning profile into an execution contract and budget plan.
pub fn conditioning_compile(args: &[u8]) -> Result<String, McpSystemError> {
    let raw = parse_tool_args(args)?;
    let dto = extract_dto_from_json(&raw).map_err(|_| McpSystemError::InvalidParameters)?;
    dto.validate().map_err(|_| McpSystemError::InvalidParameters)?;

    let req_refs: Vec<RequirementRef> = dto
        .requirements
        .iter()
        .map(|r| RequirementRef {
            id: &r.id,
            class: match r.class {
                RequirementClassDto::Enforced => RequirementClass::Enforced,
                RequirementClassDto::EvidenceObligation => RequirementClass::EvidenceObligation,
                RequirementClassDto::Guidance => RequirementClass::Guidance,
            },
            rule: &r.rule,
            validator: r.validator.as_deref(),
            required: r.required,
            priority: r.priority,
        })
        .collect();

    let spec = ConditioningSpec {
        schema_version: dto.schema_version,
        profile_id: &dto.profile_id,
        objective: &dto.objective,
        requirements: &req_refs,
        domain_refs: &[],
        output_contract: OutputContractRef {
            schema_hint: None,
            min_citations: 0,
        },
        budget: ConditioningBudget {
            input_tokens: dto.budget.input_tokens,
            output_tokens: dto.budget.output_tokens,
            tool_rounds: dto.budget.tool_rounds,
            max_bytes: 65536,
        },
    };

    let authority = AuthorityView {
        principal_did_hash: 0,
        disclosure_ceiling: 0,
        allowed_graph_scopes: &[],
        tools_allowed: true,
    };

    let caps = BackendCapabilities::native_gguf_baseline();
    let mut outcome_buf = [RequirementOutcome::applied(""); 64];
    let mut evidence_buf = [crate::inference::conditioning::EvidencePart {
        source_id: "none",
        scope: 0,
        sensitivity: 0,
        content: "",
        qualifier: None,
    }; 64];

    let mut buffers = CompileBuffers {
        outcomes: &mut outcome_buf[..dto.requirements.len().min(64)],
        selected_evidence: &mut evidence_buf,
    };

    let summary = compile_into(&spec, &authority, &caps, &[], &mut buffers)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "plan_id": summary.plan_id,
        "profile_id": summary.profile_id,
        "requirements_count": summary.requirements_count,
        "evidence_count": summary.evidence_count,
        "input_token_budget": summary.budget.input_tokens,
        "output_token_budget": summary.budget.output_tokens,
    })
    .to_string())
}
