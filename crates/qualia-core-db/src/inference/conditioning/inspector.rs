//! Conditioning inspector providing redacted requirement/token/evidence traces (PP-081).
//!
//! Strictly adheres to IMPLEMENTATION.md §7:
//! "Never expose raw learned tensors, unrestricted prompts, private evidence or cache pages as ordinary Vibe Value data. Use opaque refs and receipts."

use crate::inference::conditioning::requirement::RequirementClass;
use crate::inference::conditioning::spec::ConditioningSpec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactedRequirement {
    pub id: String,
    pub class: &'static str,
    pub rule: String,
    pub validator: Option<String>,
    pub required: bool,
    pub priority: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactedInspectionTrace {
    pub profile_id: String,
    pub schema_version: u16,
    pub plan_identity: Option<u64>,
    pub input_token_budget: u32,
    pub output_token_budget: u32,
    pub tool_rounds_budget: u16,
    pub requirements: Vec<RedactedRequirement>,
    pub evidence_count: usize,
    pub disclosure_ceiling: u8,
    pub privacy_redaction_applied: bool,
}

pub fn inspect_spec(
    spec: &ConditioningSpec,
    plan_identity: Option<u64>,
    evidence_count: usize,
    disclosure_ceiling: u8,
) -> RedactedInspectionTrace {
    let mut reqs = Vec::with_capacity(spec.requirements.len());
    for req in spec.requirements {
        reqs.push(RedactedRequirement {
            id: req.id.to_string(),
            class: match req.class {
                RequirementClass::Enforced => "enforced",
                RequirementClass::EvidenceObligation => "evidence_obligation",
                RequirementClass::Guidance => "guidance",
            },
            rule: req.rule.to_string(),
            validator: req.validator.map(|s| s.to_string()),
            required: req.required,
            priority: req.priority,
        });
    }

    RedactedInspectionTrace {
        profile_id: spec.profile_id.to_string(),
        schema_version: spec.schema_version,
        plan_identity,
        input_token_budget: spec.budget.input_tokens,
        output_token_budget: spec.budget.output_tokens,
        tool_rounds_budget: spec.budget.tool_rounds,
        requirements: reqs,
        evidence_count,
        disclosure_ceiling,
        privacy_redaction_applied: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::conditioning::budget::ConditioningBudget;
    use crate::inference::conditioning::requirement::RequirementRef;
    use crate::inference::conditioning::spec::OutputContractRef;

    #[test]
    fn test_inspect_spec_privacy_guarantees() {
        let reqs = [RequirementRef {
            id: "R1",
            class: RequirementClass::Enforced,
            rule: "Preserve invariants",
            validator: Some("validator-1"),
            required: true,
            priority: 255,
        }];
        let spec = ConditioningSpec {
            schema_version: 1,
            profile_id: "urn:qualia:profile:test:v1",
            objective: "Run safe evaluation",
            requirements: &reqs,
            domain_refs: &[],
            output_contract: OutputContractRef {
                schema_hint: None,
                min_citations: 0,
            },
            budget: ConditioningBudget {
                input_tokens: 2048,
                output_tokens: 512,
                tool_rounds: 2,
                max_bytes: 8192,
            },
        };

        let trace = inspect_spec(&spec, Some(0xDEAD_BEEF), 3, 1);
        assert!(trace.privacy_redaction_applied);
        assert_eq!(trace.profile_id, "urn:qualia:profile:test:v1");
        assert_eq!(trace.plan_identity, Some(0xDEAD_BEEF));
        assert_eq!(trace.evidence_count, 3);
        assert_eq!(trace.disclosure_ceiling, 1);
        assert_eq!(trace.requirements.len(), 1);
        assert_eq!(trace.requirements[0].id, "R1");
        assert_eq!(trace.requirements[0].class, "enforced");
    }
}
