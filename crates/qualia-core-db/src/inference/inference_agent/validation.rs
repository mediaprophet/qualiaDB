// Pre-flight intent validation for `LocalLlmAgent` — the Webizen Rights-Ontology
// rule checks that gate every inference call. Moved verbatim from the monolith.

use crate::modalities::logic::n3_compiler::AgentIntentFrame;
use crate::{q_hash, NQuin};

use super::local_agent::LocalLlmAgent;
use super::types::{
    AgentRuntime, WebizenVerdict, LLM_RULE_INTENT_FRAME_MISMATCH, LLM_RULE_NO_ADVERSARIAL_CONDUCT,
    LLM_RULE_NO_OUTBOUND_TELEMETRY, LLM_RULE_NO_SANCTUARY_ACCESS, SANCTUARY_SCOPE_WEBIZEN,
};

impl LocalLlmAgent {
    /// Zero-allocation pre-flight path for Core 1 (no `active_profile` heap lookup).
    pub fn validate_intent_frame(&self, frame: &AgentIntentFrame) -> WebizenVerdict {
        Self::evaluate_intent_frame(self, frame)
    }

    // NOTE: `pub(super)` widening (was a private associated fn) so the
    // `AgentRuntime` impl in `runtime.rs` can call it via `Self::` across the
    // new submodule boundary. Visibility stays crate-internal.
    pub(super) fn evaluate_intent_frame(agent: &LocalLlmAgent, frame: &AgentIntentFrame) -> WebizenVerdict {
        // Rule 1: No outbound network calls allowed from a Local backend.
        if frame.requires_network {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_OUTBOUND_TELEMETRY,
                reason: "Local backend: outbound network access violates Rights Ontology.",
                conduct_record: None,
            };
        }
        // Rule 2: Intent must not request access to Sanctuary-flagged graph scopes.
        let sanctuary_hit = (0..frame.scope_count as usize)
            .any(|i| frame.graph_scope[i] == SANCTUARY_SCOPE_WEBIZEN);
        if sanctuary_hit {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_SANCTUARY_ACCESS,
                reason: "Access to Sanctuary-flagged scope blocked.",
                conduct_record: None,
            };
        }

        // Rule 5: Cooperative Projects Directive — No adversarial, manipulative, or dishonest conduct.
        // Also tracks anti-human rights and discriminatory behavior for court auditing and liability.
        let is_adversarial = frame.intent_predicate == q_hash("llm:AdversarialOperation");
        let is_dishonest = frame.intent_predicate == q_hash("llm:DishonestOperation");
        let is_discriminatory = frame.intent_predicate == q_hash("llm:DiscriminatoryOperation");
        let is_anti_human_rights = frame.intent_predicate == q_hash("llm:AntiHumanRightsOperation");

        if is_adversarial || is_dishonest || is_discriminatory || is_anti_human_rights {
            let liability_weight: u64 = if is_anti_human_rights {
                100
            } else if is_discriminatory {
                80
            } else {
                50
            };
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let mut conduct_quin = NQuin {
                subject: q_hash(agent.agent_did()),
                predicate: q_hash("q42:conductViolation"),
                // Inline tag integer (0b001 << 60)
                object: liability_weight | (0b001u64 << 60),
                context: frame.principal_did_hash,
                // Pack time and flags into metadata
                metadata: (now_ms & 0xFFFFFFFF)
                    | ((is_anti_human_rights as u64) << 32)
                    | ((is_discriminatory as u64) << 33),
                parity: 0,
            };

            // Calculate parity fold (XOR fold)
            conduct_quin.parity = conduct_quin.subject
                ^ conduct_quin.predicate
                ^ conduct_quin.object
                ^ conduct_quin.context;

            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_ADVERSARIAL_CONDUCT,
                reason: "Cooperative Projects Directive Violation: Discriminatory, anti-human rights, or adversarial conduct detected.",
                conduct_record: Some(conduct_quin),
            };
        }

        // Rule 6: The Intent Predicate must align with the MCP Intent Frame.
        if frame.intent_predicate != frame.mcp_intent_frame_hash
            && frame.mcp_intent_frame_hash != crate::q_hash("purpose:General")
        {
            return WebizenVerdict::DenyWithExplanation {
                rule_violated: LLM_RULE_INTENT_FRAME_MISMATCH,
                reason: "Intent Frame Violation".into(),
                explanation: "The LLM attempted an operation outside the bounds of the active MCP Intent Frame.".into(),
            };
        }

        // Rule 8: Classified clearance — LLM cannot request above session ceiling.
        if frame.clearance_ceiling > 2 {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_SANCTUARY_ACCESS,
                reason: "Classified clearance requests require explicit Principal consent.",
                conduct_record: None,
            };
        }

        WebizenVerdict::Permit
    }
}
