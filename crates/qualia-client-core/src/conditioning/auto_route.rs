//! Chat-lowering bridge for verified model-precision contracts.
//!
//! The bridge runs while assembling a request, before tokenisation and outside
//! the decode hot path. It never invents a contract: both core registries must
//! agree on profile ID, version, and specification identity.

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
    let remaining_prefix_bytes = cap - required_bytes;
    let retained_prefix = utf8_prefix(prefix, remaining_prefix_bytes);
    let context_truncated = retained_prefix.len() < prefix.len();

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
}
