//! Chat-lowering bridge for verified model-precision contracts.
//!
//! The bridge runs while assembling a request, before tokenisation and outside
//! the decode hot path. It never invents a contract: both core registries must
//! agree on profile ID, version, and specification identity.

use qualia_core_db::inference::conditioning::{
    select_prioritized_parts, RequestPart, RequestPartKind,
};
use qualia_core_db::inference::conditioning_opt::{
    resolve_global_active_contract, CompressionStrategy, ModelPrecisionContract,
    PrefixConfiguration,
};

/// Receipt returned when a verified precision contract changes a chat request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedPrecision {
    pub profile_id: String,
    pub profile_version: u64,
    pub spec_identity: u64,
    pub input_budget_bytes: u32,
    pub output_budget_tokens: u32,
    pub prefix: PrefixConfiguration,
    pub compression: CompressionStrategy,
    pub original_prompt_bytes: usize,
    pub lowered_prompt_bytes: usize,
    pub context_truncated: bool,
}

/// Resolve and apply the model's verified runtime contract. A missing or stale
/// contract is deliberately a no-op so ordinary chats retain their established
/// behaviour until calibration is explicitly promoted.
pub fn apply_active_model_precision(
    model_id: &str,
    augmented_prompt: &mut String,
    user_prompt: &str,
) -> Result<Option<AppliedPrecision>, String> {
    let Some(contract) = resolve_global_active_contract(model_id) else {
        return Ok(None);
    };
    apply_contract_to_prompt(augmented_prompt, user_prompt, &contract).map(Some)
}

/// Apply one already-verified contract. Public for deterministic tests and
/// alternate client surfaces; normal chat should use
/// [`apply_active_model_precision`].
pub fn apply_contract_to_prompt(
    augmented_prompt: &mut String,
    user_prompt: &str,
    contract: &ModelPrecisionContract,
) -> Result<AppliedPrecision, String> {
    let original_prompt_bytes = augmented_prompt.len();
    let user_tail = format!("{user_prompt}\n---");
    if !augmented_prompt.ends_with(&user_tail) {
        return Err(
            "conditioning lowerer received a prompt with an unexpected user suffix".to_string(),
        );
    }
    let user_content_start = augmented_prompt.len() - user_tail.len();
    let user_start = augmented_prompt[..user_content_start]
        .rfind("\n---\nUser")
        .ok_or("conditioning lowerer could not find the final user boundary")?;
    let user_suffix = &augmented_prompt[user_start..];
    let invariant_header = format!(
        "[qualia:conditioning profile={} version={} strategy={} prefix={}]\n",
        contract.profile_id,
        contract.profile_version,
        contract.compression.as_str(),
        contract.prefix.as_str(),
    );
    let cap = contract.budget.max_bytes as usize;
    let required_bytes = invariant_header.len().saturating_add(user_suffix.len());
    if required_bytes > cap {
        return Err("conditioning input budget cannot retain the final user request".to_string());
    }

    let prefix = &augmented_prompt[..user_start];

    // Priority-driven validation using typed request parts
    let header_part = RequestPart::required_instruction(&invariant_header);
    let user_part = RequestPart::user_prompt(user_suffix);
    let prefix_part = RequestPart::evidence("context", prefix, None);

    let parts = [header_part, user_part, prefix_part];
    let mut selected_parts = [RequestPart::required_instruction(""); 3];
    let selected_count = select_prioritized_parts(&parts, cap, &mut selected_parts)
        .map_err(|_| "conditioning input budget cannot retain the final user request".to_string())?;

    let had_evidence = selected_parts[..selected_count]
        .iter()
        .any(|p| p.kind == RequestPartKind::Evidence);

    let retained_prefix = if had_evidence {
        prefix
    } else {
        let required_bytes = invariant_header.len().saturating_add(user_suffix.len());
        let remaining_prefix_bytes = cap.saturating_sub(required_bytes);
        utf8_prefix(prefix, remaining_prefix_bytes)
    };
    let context_truncated = (selected_count < parts.len()) || (retained_prefix.len() < prefix.len());

    let mut lowered = String::with_capacity(
        invariant_header
            .len()
            .saturating_add(retained_prefix.len())
            .saturating_add(user_suffix.len()),
    );
    lowered.push_str(&invariant_header);
    lowered.push_str(retained_prefix);
    lowered.push_str(user_suffix);

    let receipt = AppliedPrecision {
        profile_id: contract.profile_id.clone(),
        profile_version: contract.profile_version,
        spec_identity: contract.spec_identity,
        input_budget_bytes: contract.budget.max_bytes,
        output_budget_tokens: contract.budget.output_tokens,
        prefix: contract.prefix,
        compression: contract.compression,
        original_prompt_bytes,
        lowered_prompt_bytes: lowered.len(),
        context_truncated,
    };
    *augmented_prompt = lowered;
    Ok(receipt)
}

/// Apply verified model-precision contract across explicitly structured request parts.
pub fn apply_contract_to_parts<'a>(
    parts: &[RequestPart<'a>],
    contract: &ModelPrecisionContract,
) -> Result<(String, AppliedPrecision), String> {
    let invariant_header = format!(
        "[qualia:conditioning profile={} version={} strategy={} prefix={}]\n",
        contract.profile_id,
        contract.profile_version,
        contract.compression.as_str(),
        contract.prefix.as_str(),
    );
    let cap = contract.budget.max_bytes as usize;
    let header_part = RequestPart::required_instruction(&invariant_header);

    let mut all_parts = Vec::with_capacity(parts.len() + 1);
    all_parts.push(header_part);
    all_parts.extend_from_slice(parts);

    let mut selected_parts = vec![RequestPart::required_instruction(""); all_parts.len()];
    let selected_count = select_prioritized_parts(&all_parts, cap, &mut selected_parts)
        .map_err(|e| format!("conditioning input budget cannot retain mandatory request parts: {e:?}"))?;

    let mut total_rendered_bytes = 0;
    for part in &selected_parts[..selected_count] {
        total_rendered_bytes += part.byte_len();
    }

    let mut rendered = String::with_capacity(total_rendered_bytes);
    let mut original_bytes = 0;
    for part in parts {
        original_bytes += part.byte_len();
    }
    for part in &selected_parts[..selected_count] {
        rendered.push_str(part.content);
    }

    let receipt = AppliedPrecision {
        profile_id: contract.profile_id.clone(),
        profile_version: contract.profile_version,
        spec_identity: contract.spec_identity,
        input_budget_bytes: contract.budget.max_bytes,
        output_budget_tokens: contract.budget.output_tokens,
        prefix: contract.prefix,
        compression: contract.compression,
        original_prompt_bytes: original_bytes,
        lowered_prompt_bytes: rendered.len(),
        context_truncated: selected_count < all_parts.len(),
    };

    Ok((rendered, receipt))
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::inference::conditioning::ConditioningBudget;

    fn contract(max_bytes: u32) -> ModelPrecisionContract {
        ModelPrecisionContract::new(
            "urn:qualia:profile:model:test",
            3,
            0xabc,
            ConditioningBudget {
                input_tokens: 1024,
                output_tokens: 96,
                tool_rounds: 1,
                max_bytes,
            },
            PrefixConfiguration::CanonicalCacheAligned,
            CompressionStrategy::CompactContext,
        )
        .unwrap()
    }

    #[test]
    fn lowerer_preserves_user_request_and_applies_budgets() {
        let mut prompt = format!(
            "capability:{}\n---\nUser: retain this exact request\n---",
            "α".repeat(80)
        );
        let receipt =
            apply_contract_to_prompt(&mut prompt, "retain this exact request", &contract(220))
                .unwrap();

        assert!(prompt.starts_with("[qualia:conditioning profile=urn:qualia:profile:model:test"));
        assert!(prompt.ends_with("---\nUser: retain this exact request\n---"));
        assert!(receipt.context_truncated);
        assert!(receipt.lowered_prompt_bytes <= 220);
        assert_eq!(receipt.output_budget_tokens, 96);
        assert_eq!(receipt.prefix, PrefixConfiguration::CanonicalCacheAligned);
    }

    #[test]
    fn lowerer_rejects_a_cap_that_cannot_retain_user_request() {
        let mut prompt =
            "context\n---\nUser: a request that must not be truncated\n---".to_string();
        let err = apply_contract_to_prompt(
            &mut prompt,
            "a request that must not be truncated",
            &contract(20),
        )
        .unwrap_err();

        assert!(err.contains("cannot retain"));
        assert!(prompt.starts_with("context"));
    }

    #[test]
    fn test_apply_contract_to_parts_prioritizes_and_enforces_budget() {
        let instr = RequestPart::required_instruction("Instructions: compute answer accurately.");
        let user = RequestPart::user_prompt("User query: what is 2+2?");
        let evidence = RequestPart::evidence("doc1", "Additional background information that is optional.", None);

        let parts = [instr, user, evidence];
        // Large enough budget: all fit
        let (rendered, receipt) = apply_contract_to_parts(&parts, &contract(500)).unwrap();
        assert!(rendered.contains("Instructions: compute answer accurately."));
        assert!(rendered.contains("User query: what is 2+2?"));
        assert!(rendered.contains("Additional background information"));
        assert!(!receipt.context_truncated);

        // Tight budget: drops optional evidence, retains mandatory parts (172 mandatory bytes <= 200 < 223 total)
        let (rendered_tight, receipt_tight) = apply_contract_to_parts(&parts, &contract(200)).unwrap();
        assert!(rendered_tight.contains("Instructions: compute answer accurately."));
        assert!(rendered_tight.contains("User query: what is 2+2?"));
        assert!(!rendered_tight.contains("Additional background information"));
        assert!(receipt_tight.context_truncated);

        // Impossible budget: cannot fit mandatory parts -> Err
        assert!(apply_contract_to_parts(&parts, &contract(100)).is_err());
    }
}
