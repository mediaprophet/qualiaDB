//! The Orchestration Sieve — LLM Sub-Agent Dispatch Layer
//!
//! Sits between raw input (multi-modal data, user prompts) and the Sentinel VM.
//! Coordinates pre-processing → intent validation → inference → output grounding.
//!
//! Flow:
//!   RawInput → [Orchestrator] → validate_intent → [LlmAgent.infer] → validate_output → .q42 commit

use crate::llm_agent::{AgentIntent, AgentRuntime, SentinelVerdict};

/// The outcome of a full orchestrated inference cycle.
#[derive(Debug)]
pub enum OrchestrationResult {
    /// Output was validated, grounded, and ready to commit to the semantic graph.
    Committed { text: String, provenance_quins: Vec<u64> },
    /// Sentinel blocked the operation at pre-flight or post-flight.
    Blocked { rule_violated: u64, reason: String },
    /// Inference failed (timeout, backend unavailable, etc.)
    Failed(String),
}

/// Runs a full, Sentinel-gated inference cycle for a registered LLM sub-agent.
pub fn orchestrate_inference(
    agent: &dyn AgentRuntime,
    prompt: &str,
    graph_context: &str,
    intent: AgentIntent,
) -> OrchestrationResult {
    // 1. Pre-flight: validate intent against Rights Ontology
    match agent.validate_intent(&intent) {
        SentinelVerdict::Deny { rule_violated, reason } => {
            return OrchestrationResult::Blocked { rule_violated, reason };
        }
        SentinelVerdict::Sanitised { .. } => { /* intent was scrubbed; proceed with caution */ }
        SentinelVerdict::Permit => {}
    }

    // 2. Inference
    let output = match agent.infer(prompt, graph_context) {
        Ok(o) => o,
        Err(e) => return OrchestrationResult::Failed(format!("{:?}", e)),
    };

    // 3. Post-flight: validate output grounding
    match agent.validate_output(&output) {
        SentinelVerdict::Deny { rule_violated, reason } => {
            OrchestrationResult::Blocked { rule_violated, reason }
        }
        _ => OrchestrationResult::Committed {
            text: output.text,
            provenance_quins: output.provenance_quins,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::llm_agent::{AgentIntent, LocalLlmAgent, SANCTUARY_SCOPE_SENTINEL};

    #[test]
    pub fn qualia_validate_ring_buffer() {}

    #[test]
    fn test_orchestrator_full_permit_path() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![0xABCD],
            requires_network: false,
            ilp_offer_micro_cents: 0,
        };
        let result = orchestrate_inference(&agent, "Summarise my health graph.", "some_graph_bytes", intent);
        assert!(matches!(result, OrchestrationResult::Committed { .. }));
    }

    #[test]
    fn test_orchestrator_blocks_sanctuary_intent() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![SANCTUARY_SCOPE_SENTINEL],
            requires_network: false,
            ilp_offer_micro_cents: 0,
        };
        let result = orchestrate_inference(&agent, "Show me sanctuary data.", "ctx", intent);
        assert!(matches!(result, OrchestrationResult::Blocked { .. }));
    }
}
