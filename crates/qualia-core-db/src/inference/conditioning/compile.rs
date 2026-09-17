//! Immutable plan assembly and requirement disposition compilation.

use super::authority::AuthorityView;
use super::budget::ConditioningBudget;
use super::capabilities::{BackendCapabilities, SupportLevel};
use super::identity::plan_identity;
use super::receipt::RequirementOutcome;
use super::requirement::RequirementClass;
use super::select::{select_evidence_into, EvidencePart};
use super::spec::{ConditioningError, ConditioningSpec};
use super::validate::validate_spec;

/// Fixed scratch buffers provided by caller for zero-heap compilation.
pub struct CompileBuffers<'a> {
    pub outcomes: &'a mut [RequirementOutcome<'a>],
    pub selected_evidence: &'a mut [EvidencePart<'a>],
}

/// Bounded summary of a compiled conditioning plan.
#[derive(Debug, Clone, Copy)]
pub struct CompiledPlanSummary<'a> {
    pub plan_id: u64,
    pub profile_id: &'a str,
    pub objective: &'a str,
    pub requirements_count: usize,
    pub outcomes_count: usize,
    pub evidence_count: usize,
    pub budget: ConditioningBudget,
}

/// Compile a conditioning spec and authority view into caller-supplied fixed buffers.
/// Adheres to Tier-1 zero allocation in hot paths.
pub fn compile_into<'a>(
    spec: &ConditioningSpec<'a>,
    authority: &AuthorityView<'a>,
    capabilities: &BackendCapabilities,
    available_evidence: &'a [EvidencePart<'a>],
    buffers: &mut CompileBuffers<'a>,
) -> Result<CompiledPlanSummary<'a>, ConditioningError> {
    // 1. Validate spec bounds, schema, references, and authority
    validate_spec(spec, authority)?;

    // 2. Select authorized evidence
    let evidence_count =
        select_evidence_into(available_evidence, authority, buffers.selected_evidence)?;

    // 3. Process requirements and record explicit disposition
    if buffers.outcomes.len() < spec.requirements.len() {
        return Err(ConditioningError::OutputBufferFull);
    }

    let mut outcomes_count = 0;
    for req in spec.requirements {
        let outcome = match req.class {
            RequirementClass::Enforced => {
                // If grammar or validation is unsupported
                if capabilities.output_grammar == SupportLevel::Unsupported {
                    if req.required {
                        return Err(ConditioningError::RequiredCapabilityMissing);
                    } else {
                        RequirementOutcome::degraded(req.id, "output_grammar unsupported")
                    }
                } else {
                    RequirementOutcome::enforced(req.id)
                }
            }
            RequirementClass::EvidenceObligation => {
                if evidence_count == 0 {
                    if req.required {
                        RequirementOutcome::rejected(req.id, "no authorized evidence selected")
                    } else {
                        RequirementOutcome::degraded(req.id, "evidence absent")
                    }
                } else {
                    RequirementOutcome::applied(req.id)
                }
            }
            RequirementClass::Guidance => {
                if capabilities.role_separation == SupportLevel::Unsupported {
                    RequirementOutcome::degraded(req.id, "role separation unsupported; flattened")
                } else {
                    RequirementOutcome::applied(req.id)
                }
            }
        };

        buffers.outcomes[outcomes_count] = outcome;
        outcomes_count += 1;
    }

    // 4. Compute canonical identity
    let plan_id = plan_identity(
        spec,
        &buffers.selected_evidence[..evidence_count],
        "native-target",
    );

    Ok(CompiledPlanSummary {
        plan_id,
        profile_id: spec.profile_id,
        objective: spec.objective,
        requirements_count: spec.requirements.len(),
        outcomes_count,
        evidence_count,
        budget: spec.budget,
    })
}
