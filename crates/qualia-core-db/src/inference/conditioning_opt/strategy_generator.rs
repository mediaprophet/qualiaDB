//! Candidate precision strategy generation for model-specific tuning (B0–B5).

use super::model_target::ModelPrecisionTarget;
use crate::inference::conditioning::{
    ConditioningBudget, OutputContractRef, RequirementClass, RequirementRef,
};

/// A candidate conditioning profile strategy generated for model evaluation.
#[derive(Debug, Clone)]
pub struct CandidateStrategy {
    pub label: String,
    pub description: String,
    pub spec_profile_id: String,
    pub budget: ConditioningBudget,
    pub requirements: Vec<CandidateRequirement>,
    pub output_contract: OutputContractRef<'static>,
}

/// A requirement template within a candidate strategy.
#[derive(Debug, Clone)]
pub struct CandidateRequirement {
    pub id: String,
    pub class: RequirementClass,
    pub rule: String,
    pub validator: Option<String>,
    pub required: bool,
    pub priority: u8,
}

impl CandidateRequirement {
    pub fn as_ref(&self) -> RequirementRef<'_> {
        RequirementRef {
            id: &self.id,
            class: self.class,
            rule: &self.rule,
            validator: self.validator.as_deref(),
            required: self.required,
            priority: self.priority,
        }
    }
}

/// Generator producing standard B0–B5 candidates tailored to a model's operational envelope.
pub struct StrategyGenerator;

impl StrategyGenerator {
    /// Generate standard B0–B5 candidate strategies for a target model.
    pub fn generate_candidates(
        target: &ModelPrecisionTarget,
        _objective: &str,
    ) -> Vec<CandidateStrategy> {
        let max_ctx = target.context_window;
        let base_input = (max_ctx / 4).min(4096) as u32;
        let base_output = (max_ctx / 8).min(2048).max(512) as u32;

        vec![
            // B0: Baseline minimal prompt (no strict requirements)
            CandidateStrategy {
                label: "B0_baseline".into(),
                description: "Minimal baseline instruction without graph conditioning".into(),
                spec_profile_id: format!("urn:qualia:opt:{}:b0", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input,
                    output_tokens: base_output,
                    tool_rounds: 1,
                    max_bytes: 4096,
                },
                requirements: vec![],
                output_contract: OutputContractRef {
                    schema_hint: None,
                    min_citations: 0,
                },
            },
            // B1: Explicit structured text rules
            CandidateStrategy {
                label: "B1_explicit_text".into(),
                description: "Explicit natural language rules and verification criteria".into(),
                spec_profile_id: format!("urn:qualia:opt:{}:b1", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input + 512,
                    output_tokens: base_output,
                    tool_rounds: 2,
                    max_bytes: 8192,
                },
                requirements: vec![
                    CandidateRequirement {
                        id: "req_accuracy".into(),
                        class: RequirementClass::Guidance,
                        rule: "Ensure factual correctness and precise technical execution".into(),
                        validator: None,
                        required: false,
                        priority: 10,
                    },
                    CandidateRequirement {
                        id: "req_citations".into(),
                        class: RequirementClass::Guidance,
                        rule: "Cite authoritative evidence identifiers when asserting facts".into(),
                        validator: None,
                        required: false,
                        priority: 5,
                    },
                ],
                output_contract: OutputContractRef {
                    schema_hint: None,
                    min_citations: 1,
                },
            },
            // B2: Canonical compiled ontology rules with enforced validation
            CandidateStrategy {
                label: "B2_compiled_graph".into(),
                description: "Ontology-backed requirements with strict validator constraints"
                    .into(),
                spec_profile_id: format!("urn:qualia:opt:{}:b2", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input + 1024,
                    output_tokens: base_output,
                    tool_rounds: 3,
                    max_bytes: 16384,
                },
                requirements: vec![
                    CandidateRequirement {
                        id: "req_schema_validity".into(),
                        class: RequirementClass::Enforced,
                        rule: "Output must strictly adhere to domain schema contract".into(),
                        validator: Some("urn:qualia:shacl:validator:strict".into()),
                        required: true,
                        priority: 20,
                    },
                    CandidateRequirement {
                        id: "req_traceability".into(),
                        class: RequirementClass::Enforced,
                        rule: "All assertions must link to verified graph entities".into(),
                        validator: Some("urn:qualia:shacl:validator:provenance".into()),
                        required: true,
                        priority: 15,
                    },
                ],
                output_contract: OutputContractRef {
                    schema_hint: Some("application/json"),
                    min_citations: 2,
                },
            },
            // B3: Compact token-budget compressed conditioning
            CandidateStrategy {
                label: "B3_compressed".into(),
                description: "Compact budget allocation prioritizing token throughput".into(),
                spec_profile_id: format!("urn:qualia:opt:{}:b3", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input / 2,
                    output_tokens: base_output / 2,
                    tool_rounds: 1,
                    max_bytes: 2048,
                },
                requirements: vec![CandidateRequirement {
                    id: "req_concise_precision".into(),
                    class: RequirementClass::Guidance,
                    rule: "Deliver concise, authoritative response without preamble".into(),
                    validator: None,
                    required: false,
                    priority: 10,
                }],
                output_contract: OutputContractRef {
                    schema_hint: None,
                    min_citations: 1,
                },
            },
            // B4: Exact-prefix cache aligned strategy
            CandidateStrategy {
                label: "B4_prefix_aligned".into(),
                description: "Deterministic prefix alignment maximizing KV-cache reuse".into(),
                spec_profile_id: format!("urn:qualia:opt:{}:b4", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input,
                    output_tokens: base_output,
                    tool_rounds: 2,
                    max_bytes: 8192,
                },
                requirements: vec![CandidateRequirement {
                    id: "req_canonical_prefix".into(),
                    class: RequirementClass::Guidance,
                    rule: "Adhere to invariant prompt prefix layout for cache hit rate".into(),
                    validator: None,
                    required: false,
                    priority: 10,
                }],
                output_contract: OutputContractRef {
                    schema_hint: None,
                    min_citations: 1,
                },
            },
            // B5: Optimized hybrid candidate tailored to model's family
            CandidateStrategy {
                label: "B5_optimized_hybrid".into(),
                description: format!(
                    "Model-tailored precision strategy optimized for {} family",
                    target.family.as_str()
                ),
                spec_profile_id: format!("urn:qualia:opt:{}:b5", target.model_id),
                budget: ConditioningBudget {
                    input_tokens: base_input + 512,
                    output_tokens: base_output + 256,
                    tool_rounds: 2,
                    max_bytes: 8192,
                },
                requirements: vec![
                    CandidateRequirement {
                        id: "req_model_optimized".into(),
                        class: RequirementClass::Enforced,
                        rule: format!(
                            "Satisfy {} domain invariants and cite verified evidence",
                            target.target_domain
                        ),
                        validator: Some("urn:qualia:shacl:validator:domain".into()),
                        required: true,
                        priority: 25,
                    },
                    CandidateRequirement {
                        id: "req_role_separation".into(),
                        class: RequirementClass::Guidance,
                        rule: "Preserve strict isolation between instructions and evidence".into(),
                        validator: None,
                        required: false,
                        priority: 15,
                    },
                ],
                output_contract: OutputContractRef {
                    schema_hint: Some("application/json"),
                    min_citations: 1,
                },
            },
        ]
    }
}
