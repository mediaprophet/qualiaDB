//! Request-boundary lowering for Prompt Precision local-model calls.

use serde::Deserialize;

use super::poet_llm_api::PoetLlmRequest;

const MAX_PROMPT_BYTES: usize = 32 * 1024;
const MAX_CONDITIONING_REQUIREMENTS: usize = 64;
const MAX_CONDITIONING_EVIDENCE: usize = 64;
const MAX_CONDITIONING_DOMAINS: usize = 16;

/// An owned, cold-boundary profile. It is compiled into caller-buffered plan
/// data before inference starts; no profile changes the original request.
///
/// Public so the same profile document can be compiled for the desktop chat,
/// Ollama and remote-MCP routes — not only the POET HTTP surface.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptConditioning {
    #[serde(default = "default_schema_version")]
    schema_version: u16,
    #[serde(default)]
    profile_id: String,
    #[serde(default)]
    objective: String,
    #[serde(default)]
    requirements: Vec<PromptRequirement>,
    #[serde(default)]
    evidence: Vec<PromptEvidence>,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    allowed_graph_scopes: Vec<u64>,
    #[serde(default)]
    disclosure_ceiling: u8,
    #[serde(default = "default_tools_allowed")]
    tools_allowed: bool,
    #[serde(default)]
    budget: Option<PromptConditioningBudget>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptRequirement {
    id: String,
    #[serde(default = "default_requirement_class")]
    class: String,
    rule: String,
    #[serde(default)]
    validator: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    priority: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptEvidence {
    source_id: String,
    #[serde(default)]
    scope: u64,
    #[serde(default)]
    sensitivity: u8,
    content: String,
    #[serde(default)]
    qualifier: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptConditioningBudget {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    tool_rounds: u16,
    #[serde(default)]
    max_bytes: u32,
}

/// The compiled+rendered result of one profile against one request.
/// `plan_id`/`profile_id` are `None` for the no-conditioning legacy path.
#[derive(Debug, Clone)]
pub struct PreparedPrompt {
    pub text: String,
    pub plan_id: Option<u64>,
    pub profile_id: Option<String>,
    pub selected_evidence: usize,
    pub token_budget: u32,
}

fn default_schema_version() -> u16 {
    1
}
fn default_requirement_class() -> String {
    "guidance".into()
}
fn default_tools_allowed() -> bool {
    true
}

/// Compile and render the semantic profile into the precise prompt consumed by
/// local inference. The returned receipt makes the applied plan inspectable.
/// Thin adapter over [`prepare_conditioned_prompt`] — identical semantics to
/// the shared route-agnostic entry point.
pub fn prepare_prompt(request: &PoetLlmRequest) -> Result<PreparedPrompt, String> {
    let Some(raw) = request.conditioning.as_ref() else {
        return Ok(PreparedPrompt {
            text: request.prompt.clone(),
            plan_id: None,
            profile_id: None,
            selected_evidence: 0,
            token_budget: request.max_tokens,
        });
    };
    prepare_conditioned_prompt(
        raw,
        &request.prompt,
        &request.principal_did,
        &request.graph_context,
        request.max_tokens,
    )
}

/// Route-agnostic preparation: compile the same profile document for any
/// inference route (HTTP, desktop chat, Ollama, remote MCP). `prompt` is the
/// raw user request — used as the objective when the profile omits one —
/// and `graph_context` supplies the fallback allowed-scope hash.
pub fn prepare_conditioned_prompt(
    raw: &serde_json::Value,
    prompt: &str,
    principal_did: &str,
    graph_context: &str,
    max_tokens: u32,
) -> Result<PreparedPrompt, String> {
    let profile: PromptConditioning = serde_json::from_value(raw.clone())
        .map_err(|error| format!("Invalid conditioning profile: {error}"))?;
    if profile.requirements.len() > MAX_CONDITIONING_REQUIREMENTS
        || profile.evidence.len() > MAX_CONDITIONING_EVIDENCE
        || profile.domains.len() > MAX_CONDITIONING_DOMAINS
        || profile.allowed_graph_scopes.len() > MAX_CONDITIONING_DOMAINS
    {
        return Err("Conditioning profile exceeds its bounded request limits".into());
    }
    let profile_id = if profile.profile_id.trim().is_empty() {
        "urn:qualia:profile:request".to_string()
    } else {
        profile.profile_id.trim().to_string()
    };
    let objective = if profile.objective.trim().is_empty() {
        prompt
    } else {
        profile.objective.trim()
    };
    let input_budget = profile.budget.unwrap_or(PromptConditioningBudget {
        input_tokens: 4096,
        output_tokens: max_tokens,
        tool_rounds: 0,
        max_bytes: MAX_PROMPT_BYTES as u32,
    });
    let budget = crate::inference::conditioning::ConditioningBudget {
        input_tokens: input_budget.input_tokens.max(1),
        output_tokens: if input_budget.output_tokens == 0 {
            max_tokens
        } else {
            input_budget.output_tokens
        },
        tool_rounds: input_budget.tool_rounds,
        max_bytes: if input_budget.max_bytes == 0 {
            MAX_PROMPT_BYTES as u32
        } else {
            input_budget.max_bytes.min(MAX_PROMPT_BYTES as u32)
        },
        thinking_token_budget: None,
    };
    let mut requirements = Vec::with_capacity(profile.requirements.len());
    for requirement in &profile.requirements {
        let class = match requirement.class.as_str() {
            "enforced" => crate::inference::conditioning::RequirementClass::Enforced,
            "evidence_obligation" | "evidence" => {
                crate::inference::conditioning::RequirementClass::EvidenceObligation
            }
            "guidance" => crate::inference::conditioning::RequirementClass::Guidance,
            other => return Err(format!("Unknown conditioning requirement class: {other}")),
        };
        requirements.push(crate::inference::conditioning::RequirementRef {
            id: requirement.id.trim(),
            class,
            rule: requirement.rule.trim(),
            validator: requirement.validator.as_deref(),
            required: requirement.required,
            priority: requirement.priority,
        });
    }
    let domain_refs: Vec<u64> = profile
        .domains
        .iter()
        .map(|domain| crate::q_hash(domain))
        .collect();
    let evidence: Vec<crate::inference::conditioning::EvidencePart<'_>> = profile
        .evidence
        .iter()
        .map(|item| crate::inference::conditioning::EvidencePart {
            source_id: item.source_id.trim(),
            scope: item.scope,
            sensitivity: item.sensitivity,
            content: item.content.trim(),
            qualifier: item.qualifier.as_deref(),
        })
        .collect();
    let fallback_scope = crate::q_hash(graph_context);
    let allowed_scopes = if profile.allowed_graph_scopes.is_empty() && !graph_context.is_empty() {
        std::slice::from_ref(&fallback_scope)
    } else {
        profile.allowed_graph_scopes.as_slice()
    };
    let authority = crate::inference::conditioning::AuthorityView {
        principal_did_hash: crate::q_hash(principal_did),
        disclosure_ceiling: profile.disclosure_ceiling,
        allowed_graph_scopes: allowed_scopes,
        tools_allowed: profile.tools_allowed,
    };
    let spec = crate::inference::conditioning::ConditioningSpec {
        schema_version: profile.schema_version,
        profile_id: &profile_id,
        objective,
        requirements: &requirements,
        domain_refs: &domain_refs,
        output_contract: crate::inference::conditioning::OutputContractRef {
            schema_hint: None,
            min_citations: 0,
        },
        budget,
    };
    let mut outcomes = [crate::inference::conditioning::RequirementOutcome::applied("");
        MAX_CONDITIONING_REQUIREMENTS];
    let mut selected = [crate::inference::conditioning::EvidencePart {
        source_id: "",
        scope: 0,
        sensitivity: 0,
        content: "",
        qualifier: None,
    }; MAX_CONDITIONING_EVIDENCE];
    let mut buffers = crate::inference::conditioning::CompileBuffers {
        outcomes: &mut outcomes[..requirements.len()],
        selected_evidence: &mut selected[..evidence.len()],
    };
    let plan = crate::inference::conditioning::compile_into(
        &spec,
        &authority,
        &crate::inference::conditioning::BackendCapabilities::native_gguf_baseline(),
        &evidence,
        &mut buffers,
    )
    .map_err(|error| format!("Conditioning profile cannot be compiled: {error:?}"))?;
    let mut rendered = vec![0u8; budget.max_bytes as usize];
    let rendered_summary = crate::inference::conditioning::render_into(
        &plan,
        &spec,
        &selected[..plan.evidence_count],
        crate::inference::conditioning::RenderTarget::NativePlain,
        &mut rendered,
    )
    .map_err(|error| format!("Conditioning profile cannot be rendered: {error:?}"))?;
    let text = std::str::from_utf8(&rendered[..rendered_summary.bytes_written])
        .map_err(|_| "Conditioning renderer returned invalid UTF-8".to_string())?
        .to_string();
    Ok(PreparedPrompt {
        text,
        plan_id: Some(plan.plan_id),
        profile_id: Some(profile_id.clone()),
        selected_evidence: plan.evidence_count,
        token_budget: max_tokens.min(budget.output_tokens),
    })
}
