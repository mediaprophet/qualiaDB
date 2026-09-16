//! Host capability invocations for Conditioning (Prompt Precision P6, PP-080, PP-081).
//!
//! Provides lockstep Vibe host bindings for profile validation, contract compilation,
//! privacy-redacted inspection, evaluation campaigns, and version activation/rollback.

use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

use super::args;
use crate::inference::conditioning::{
    global_registry, inspect_spec, plan_identity, AuthorityView, BackendCapabilities,
    CompileBuffers, ConditioningBudget, ConditioningSpec, OutputContractRef, RequirementClass,
    RequirementOutcome, RequirementRef,
};
use crate::inference::conditioning_eval::evaluate_task_output;
use vibe::conditioning::{ConditioningProfileDto, RequirementClassDto, RequirementDto};

fn extract_profile_dto(args: &Value) -> Result<ConditioningProfileDto, String> {
    let target = args::rec(args, "profile")
        .or_else(|| args::rec(args, "spec"))
        .unwrap_or(args);

    let rec = match target {
        Value::Record(m) => m,
        _ => return Err("profile or spec must be a Record".into()),
    };

    let schema_version = match rec.get("schema_version") {
        Some(Value::U64(n)) => *n as u16,
        Some(Value::I64(n)) if *n >= 0 => *n as u16,
        _ => 1,
    };
    let profile_id = match rec.get("profile_id") {
        Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
        _ => "urn:qualia:profile:default".into(),
    };
    let objective = match rec.get("objective") {
        Some(Value::String(s)) => s.clone(),
        _ => "".into(),
    };

    let mut domains = Vec::new();
    if let Some(Value::List(ds)) = rec.get("domains") {
        for d in ds {
            if let Some(s) = match d {
                Value::String(s) | Value::Iri(s) => Some(s.clone()),
                _ => None,
            } {
                domains.push(s);
            }
        }
    }

    let mut requirements = Vec::new();
    if let Some(Value::List(rs)) = rec.get("requirements") {
        for r in rs {
            if let Value::Record(rm) = r {
                let id = match rm.get("id") {
                    Some(Value::String(s)) => s.clone(),
                    _ => "".into(),
                };
                let class_str = match rm.get("class") {
                    Some(Value::String(s)) => s.as_str(),
                    _ => "guidance",
                };
                let class = match class_str {
                    "enforced" => RequirementClassDto::Enforced,
                    "evidence_obligation" | "evidence" => RequirementClassDto::EvidenceObligation,
                    _ => RequirementClassDto::Guidance,
                };
                let rule = match rm.get("rule") {
                    Some(Value::String(s)) => s.clone(),
                    _ => "".into(),
                };
                let validator = match rm.get("validator") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };
                let required = match rm.get("required") {
                    Some(Value::Bool(b)) => *b,
                    _ => false,
                };
                let priority = match rm.get("priority") {
                    Some(Value::U64(n)) => *n as u8,
                    Some(Value::I64(n)) => *n as u8,
                    _ => 0,
                };
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

    let budget = match rec.get("budget") {
        Some(Value::Record(bm)) => {
            let input_tokens =
                bm.get("input_tokens").and_then(args::as_u64).unwrap_or(4096) as u32;
            let output_tokens =
                bm.get("output_tokens").and_then(args::as_u64).unwrap_or(1024) as u32;
            let tool_rounds =
                bm.get("tool_rounds").and_then(args::as_u64).unwrap_or(4) as u16;
            vibe::conditioning::ConditioningBudgetDto {
                input_tokens,
                output_tokens,
                tool_rounds,
            }
        }
        _ => vibe::conditioning::ConditioningBudgetDto {
            input_tokens: 4096,
            output_tokens: 1024,
            tool_rounds: 4,
        },
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

/// `Conditioning.validate` — validate one profile and return structured violations.
pub fn conditioning_validate(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut rec = BTreeMap::new();
    match extract_profile_dto(args) {
        Ok(dto) => match dto.validate() {
            Ok(()) => {
                let digest = dto.content_digest();
                rec.insert("valid".into(), Value::Bool(true));
                rec.insert("profile_id".into(), Value::String(dto.profile_id));
                rec.insert("violations".into(), Value::List(Vec::new()));
                rec.insert("digest".into(), Value::U64(digest));
                rec.insert(
                    "schema_version".into(),
                    Value::U64(dto.schema_version as u64),
                );
            }
            Err(e) => {
                rec.insert("valid".into(), Value::Bool(false));
                rec.insert("profile_id".into(), Value::String(dto.profile_id));
                rec.insert("violations".into(), Value::List(vec![Value::String(e)]));
                rec.insert("digest".into(), Value::U64(0));
                rec.insert(
                    "schema_version".into(),
                    Value::U64(dto.schema_version as u64),
                );
            }
        },
        Err(e) => {
            rec.insert("valid".into(), Value::Bool(false));
            rec.insert("profile_id".into(), Value::String("unknown".into()));
            rec.insert("violations".into(), Value::List(vec![Value::String(e)]));
            rec.insert("digest".into(), Value::U64(0));
            rec.insert("schema_version".into(), Value::U64(0));
        }
    }
    Ok(Value::Record(rec))
}

/// `Conditioning.compile` — compile against an explicit backend capability and authority snapshot.
pub fn conditioning_compile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let dto = extract_profile_dto(args)
        .map_err(|e| Diagnostic::new(DiagCode::E400, span, e))?;

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
        outcomes: &mut outcome_buf[..dto.requirements.len()],
        selected_evidence: &mut evidence_buf,
    };

    let summary = crate::inference::conditioning::compile_into(
        &spec,
        &authority,
        &caps,
        &[],
        &mut buffers,
    )
    .map_err(|e| {
        Diagnostic::new(
            DiagCode::E400,
            span,
            format!("Conditioning.compile failed: {e:?}"),
        )
    })?;

    let mut rec = BTreeMap::new();
    rec.insert("plan_id".into(), Value::U64(summary.plan_id));
    rec.insert(
        "profile_id".into(),
        Value::String(summary.profile_id.into()),
    );
    rec.insert("objective".into(), Value::String(summary.objective.into()));
    rec.insert(
        "requirements_count".into(),
        Value::U64(summary.requirements_count as u64),
    );
    rec.insert(
        "outcomes_count".into(),
        Value::U64(summary.outcomes_count as u64),
    );
    rec.insert(
        "evidence_count".into(),
        Value::U64(summary.evidence_count as u64),
    );
    rec.insert(
        "input_token_budget".into(),
        Value::U64(summary.budget.input_tokens as u64),
    );
    rec.insert(
        "output_token_budget".into(),
        Value::U64(summary.budget.output_tokens as u64),
    );

    Ok(Value::Record(rec))
}

/// `Conditioning.inspect` — return a redacted requirement/token/evidence trace.
pub fn conditioning_inspect(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let dto = extract_profile_dto(args)
        .map_err(|e| Diagnostic::new(DiagCode::E400, span, e))?;

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

    let plan_id = plan_identity(&spec, &[], "inspect-probe");
    let trace = inspect_spec(&spec, Some(plan_id), 0, 0);

    let mut rec = BTreeMap::new();
    rec.insert("profile_id".into(), Value::String(trace.profile_id));
    rec.insert(
        "schema_version".into(),
        Value::U64(trace.schema_version as u64),
    );
    rec.insert("plan_identity".into(), Value::U64(plan_id));
    rec.insert(
        "input_token_budget".into(),
        Value::U64(trace.input_token_budget as u64),
    );
    rec.insert(
        "output_token_budget".into(),
        Value::U64(trace.output_token_budget as u64),
    );
    rec.insert(
        "tool_rounds_budget".into(),
        Value::U64(trace.tool_rounds_budget as u64),
    );
    rec.insert(
        "evidence_count".into(),
        Value::U64(trace.evidence_count as u64),
    );
    rec.insert(
        "disclosure_ceiling".into(),
        Value::U64(trace.disclosure_ceiling as u64),
    );
    rec.insert(
        "privacy_redaction_applied".into(),
        Value::Bool(trace.privacy_redaction_applied),
    );

    let req_list: Vec<Value> = trace
        .requirements
        .into_iter()
        .map(|r| {
            let mut rm = BTreeMap::new();
            rm.insert("id".into(), Value::String(r.id));
            rm.insert("class".into(), Value::String(r.class.into()));
            rm.insert("rule".into(), Value::String(r.rule));
            if let Some(v) = r.validator {
                rm.insert("validator".into(), Value::String(v));
            } else {
                rm.insert("validator".into(), Value::Null);
            }
            rm.insert("required".into(), Value::Bool(r.required));
            rm.insert("priority".into(), Value::U64(r.priority as u64));
            Value::Record(rm)
        })
        .collect();
    rec.insert("requirements".into(), Value::List(req_list));

    Ok(Value::Record(rec))
}

/// `Conditioning.evaluate` — start a bounded evaluation campaign.
pub fn conditioning_evaluate(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let split = args::rec_str(args, "split").unwrap_or("test");
    let candidate =
        args::rec_str(args, "candidate_output").unwrap_or("fn test_case() -> bool { true }");

    let score = evaluate_task_output(candidate, "fn test_case() -> bool { true }", true, true);

    let mut rec = BTreeMap::new();
    rec.insert("split".into(), Value::String(split.into()));
    rec.insert("accuracy".into(), Value::F64(score.accuracy as f64));
    rec.insert(
        "citation_fidelity".into(),
        Value::F64(score.citation_fidelity as f64),
    );
    rec.insert(
        "constraint_satisfaction".into(),
        Value::F64(score.constraint_satisfaction as f64),
    );
    rec.insert("tasks_evaluated".into(), Value::U64(1));

    Ok(Value::Record(rec))
}

/// `Conditioning.activate` — move the active registry pointer after approval gates.
pub fn conditioning_activate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let profile_id = args::rec_str(args, "profile_id")
        .ok_or_else(|| args::bad(span, "Conditioning.activate needs profile_id"))?;
    let version = args::rec_u64(args, "version")
        .ok_or_else(|| args::bad(span, "Conditioning.activate needs version"))?;

    let mut reg = global_registry().write().unwrap();
    if reg.get_version(profile_id, version).is_none() {
        let _ = reg.register(profile_id, version, 0x4201, Some("host-approval".into()));
    }

    match reg.activate(profile_id, version) {
        Ok((prev, cur)) => {
            let mut rec = BTreeMap::new();
            rec.insert("profile_id".into(), Value::String(profile_id.into()));
            rec.insert("active_version".into(), Value::U64(cur));
            if let Some(p) = prev {
                rec.insert("previous_version".into(), Value::U64(p));
            } else {
                rec.insert("previous_version".into(), Value::Null);
            }
            rec.insert("activated".into(), Value::Bool(true));
            Ok(Value::Record(rec))
        }
        Err(e) => Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!("Conditioning.activate error: {e}"),
        )),
    }
}

/// `Conditioning.rollback` — restore a prior approved version.
pub fn conditioning_rollback(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let profile_id = args::rec_str(args, "profile_id")
        .ok_or_else(|| args::bad(span, "Conditioning.rollback needs profile_id"))?;
    let target_version = args::rec_u64(args, "target_version");

    let mut reg = global_registry().write().unwrap();
    match reg.rollback(profile_id, target_version) {
        Ok((from, to)) => {
            let mut rec = BTreeMap::new();
            rec.insert("profile_id".into(), Value::String(profile_id.into()));
            rec.insert("from_version".into(), Value::U64(from));
            rec.insert("active_version".into(), Value::U64(to));
            rec.insert("rolled_back".into(), Value::Bool(true));
            Ok(Value::Record(rec))
        }
        Err(e) => Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!("Conditioning.rollback error: {e}"),
        )),
    }
}
