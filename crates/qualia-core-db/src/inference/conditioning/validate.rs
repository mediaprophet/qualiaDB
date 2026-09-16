//! Specification validation, bounds, reference, and conflict checking.

use super::authority::AuthorityView;
use super::requirement::RequirementClass;
use super::spec::{ConditioningError, ConditioningSpec};

pub const MAX_PROFILE_ID_LEN: usize = 256;
pub const MAX_OBJECTIVE_LEN: usize = 4096;
pub const MAX_REQUIREMENTS_COUNT: usize = 64;
pub const MAX_DOMAIN_REFS_COUNT: usize = 16;
pub const SUPPORTED_SCHEMA_VERSION: u16 = 1;

/// Validate a conditioning specification against schema constraints and authority view.
pub fn validate_spec<'a>(
    spec: &ConditioningSpec<'a>,
    authority: &AuthorityView<'a>,
) -> Result<(), ConditioningError> {
    if spec.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(ConditioningError::UnsupportedVersion);
    }

    if spec.profile_id.is_empty() || spec.profile_id.len() > MAX_PROFILE_ID_LEN {
        return Err(ConditioningError::ValidationFailed);
    }

    if spec.objective.is_empty() || spec.objective.len() > MAX_OBJECTIVE_LEN {
        return Err(ConditioningError::ValidationFailed);
    }

    if spec.requirements.len() > MAX_REQUIREMENTS_COUNT {
        return Err(ConditioningError::ContextBudgetExceeded);
    }

    if spec.domain_refs.len() > MAX_DOMAIN_REFS_COUNT {
        return Err(ConditioningError::ContextBudgetExceeded);
    }

    if spec.budget.max_bytes == 0 || spec.budget.input_tokens == 0 {
        return Err(ConditioningError::ValidationFailed);
    }

    // Check duplicate requirement IDs and enforce validator requirements
    for (i, req) in spec.requirements.iter().enumerate() {
        if req.id.is_empty() || req.rule.is_empty() {
            return Err(ConditioningError::ValidationFailed);
        }

        // An Enforced requirement MUST provide a non-empty validator reference
        if req.class == RequirementClass::Enforced {
            match req.validator {
                Some(v) if !v.trim().is_empty() => {}
                _ => return Err(ConditioningError::ValidationFailed),
            }
        }

        // Check duplicates
        for other in &spec.requirements[i + 1..] {
            if req.id == other.id {
                return Err(ConditioningError::DuplicateRequirement);
            }
        }
    }

    // If tools are required by budget (tool_rounds > 0) but authority disallows tools
    if spec.budget.tool_rounds > 0 && !authority.tools_allowed {
        return Err(ConditioningError::DisclosureDenied);
    }

    Ok(())
}
