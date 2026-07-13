// The `AgentRuntime` implementation for `LocalLlmAgent`: the object-safe entry
// points (backend / agent_did / validate_intent / infer / validate_output /
// memory_budget_remaining). Moved verbatim from the monolith.

use std::time::{Duration, Instant};

use crate::modalities::logic::n3_compiler::N3OutputMode;

use super::config::{effective_inference_timeout_ms, LLM_MEMORY_BUDGET_BYTES, MAX_OUTPUT_TOKENS};
use super::local_agent::LocalLlmAgent;
use super::types::{
    AgentBackend, AgentError, AgentIntent, AgentOutput, AgentRuntime, WebizenVerdict,
    LLM_RULE_PROFILE_VIOLATION, LLM_RULE_PROVENANCE_REQUIRED, LLM_RULE_TOKEN_BUDGET,
};

impl AgentRuntime for LocalLlmAgent {
    fn backend(&self) -> &AgentBackend {
        &self.backend
    }
    fn agent_did(&self) -> &str {
        &self.agent_did
    }

    fn validate_intent(&self, intent: &AgentIntent) -> WebizenVerdict {
        let sieve_on = matches!(
            intent.output_mode,
            N3OutputMode::GraphMutation | N3OutputMode::N3Assertions
        );
        self.use_sieve_output
            .store(sieve_on, std::sync::atomic::Ordering::Relaxed);
        if sieve_on {
            let mut spec = crate::neuro_symbolic_sieve::SieveLexSpec::graph_mutation_default();
            for &scope_hash in &intent.requested_graph_scope {
                if scope_hash != 0 {
                    spec.push_predicate(scope_hash);
                }
            }
            for &namespace_hash in &intent.context_namespaces {
                if namespace_hash != 0 {
                    spec.push_predicate(namespace_hash);
                }
            }
            *self.sieve_spec.lock().unwrap_or_else(|e| e.into_inner()) = spec;
        }

        let frame = intent.to_frame();
        let base = Self::evaluate_intent_frame(self, &frame);
        if !matches!(base, WebizenVerdict::Permit) {
            return base;
        }

        // Rule 7: Profile Constraints (Intent frames and Engine masking)
        if let Some(profile) = &intent.active_profile {
            if !profile.allows_intent(intent.intent_predicate) {
                return WebizenVerdict::DenyWithExplanation {
                    rule_violated: LLM_RULE_PROFILE_VIOLATION,
                    reason: "Profile Violation".into(),
                    explanation: "This capability profile explicitly blocks this intent frame."
                        .into(),
                };
            }
        }

        WebizenVerdict::Permit
    }

    fn infer(&self, prompt: &str, graph_context: &str) -> Result<AgentOutput, AgentError> {
        let t0 = Instant::now();

        // Memory guard
        let current = self
            .memory_used_bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        if current > LLM_MEMORY_BUDGET_BYTES {
            return Err(AgentError::MemoryBudgetExceeded);
        }

        // Timeout guard (production: run in a separate thread with channel)
        let deadline = Duration::from_millis(effective_inference_timeout_ms());
        let (text, provenance, tokens, semantic_quin) =
            self.infer_local_model(prompt, graph_context);
        if t0.elapsed() > deadline {
            return Err(AgentError::Timeout);
        }
        if text == "[sieve-misaligned]" && semantic_quin.is_none() {
            return Err(AgentError::SieveMisaligned);
        }

        Ok(AgentOutput {
            text,
            semantic_quin,
            provenance_quins: provenance,
            tokens_generated: tokens,
            inference_duration_ms: t0.elapsed().as_millis() as u64,
            peak_memory_bytes: current,
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> WebizenVerdict {
        // Rule 3: All outputs MUST be grounded with at least one provenance citation.
        if output.provenance_quins.is_empty() {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_PROVENANCE_REQUIRED,
                reason: "Output has no provenance citations. Cannot commit ungrounded content to the semantic graph.",
                conduct_record: None,
            };
        }
        // Rule 4: Output must not exceed token budget (prevents runaway generation).
        if output.tokens_generated > MAX_OUTPUT_TOKENS {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_TOKEN_BUDGET,
                reason: "Token budget exceeded.",
                conduct_record: None,
            };
        }
        WebizenVerdict::Permit
    }

    fn memory_budget_remaining(&self) -> u64 {
        let used = self
            .memory_used_bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        LLM_MEMORY_BUDGET_BYTES.saturating_sub(used)
    }
}
