// qualia-llm-agent: LLM Sub-Agent Layer for the Intentional Computing Ecosystem
//
// This crate implements the AgentRuntime trait and the Webizen-gated message
// protocol that governs every LLM interaction under the Principal-Agent Duty of Care.
//
// Architecture:
//   Principal (Natural Person)
//     └── qualiaDB Webizen VM  ← GATEKEEPER (validates all I/O)
//           └── LlmAgent (AgentRuntime impl)
//                 ├── Backend::Local  (llama.cpp / WebLLM / ONNX)
//                 ├── Backend::Remote (Nym-tunnelled, ILP-metered, user-consented)
//                 └── Backend::Hybrid (local first, remote fallback with consent)
//
// CRITICAL CONSTRAINT: All paths enforce:
//   - Zero outbound telemetry
//   - All outputs must be cited to a QualiaQuin provenance chain
//   - Webizen validates I/O before touching the semantic graph
//   - Memory budget hard-capped; default 128MB within 512MB floor

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crate::{q_hash, QualiaQuin};

// ─── Constants ──────────────────────────────────────────────────────────────
/// Hard memory ceiling for the LLM runtime within the 512MB system floor.
/// Leaves the remaining 384MB for the Webizen VM, SLG Arena, and WASM stack.
pub const LLM_MEMORY_BUDGET_BYTES: u64 = 128 * 1024 * 1024; // 128 MB

/// Maximum tokens the agent may generate in a single turn. Enforces deterministic
/// compute cost — no runaway generation that blocks the edge device.
pub const MAX_OUTPUT_TOKENS: u32 = 2048;

/// Maximum milliseconds for a local inference call before timeout.
pub const INFERENCE_TIMEOUT_MS: u64 = 30_000;

// ─── AgentBackend ────────────────────────────────────────────────────────────
/// Describes where inference actually runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentBackend {
    /// Quantized local model (llama.cpp WASM / ONNX Runtime / WebLLM + WebGPU).
    /// This is the PREFERRED backend — no outbound traffic.
    Local {
        model_path: String,     // e.g. "~/.qualia/models/phi3-mini-4bit.gguf"
        context_window: u32,    // tokens; typically 4096 for Phi-3-mini
        quantization: String,   // "Q4_K_M", "Q8_0", etc.
    },
    /// Remote model call. REQUIRES:
    ///   - Explicit Principal consent (signed VC)
    ///   - Nym mixnet routing (no raw IP correlation)
    ///   - ILP micropayment for every call
    ///   - Full audit trail written to .q42
    Remote {
        endpoint_did: String,   // did:git of the approved remote provider
        nym_gateway: String,    // Nym gateway address
        ilp_budget_micro_cents: u64,
    },
    /// Local first; falls back to Remote only with Principal consent.
    Hybrid {
        local_model_path: String,
        remote_endpoint_did: String,
        consent_required: bool, // Always true in production
    },
}

// ─── AgentIntent ─────────────────────────────────────────────────────────────
/// Structured intent message from LLM → Webizen. Every call must declare
/// what it intends to do — the Webizen validates this against the Rights Ontology
/// BEFORE the LLM ever sees the user's semantic graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIntent {
    /// N3Logic predicate hash declaring the class of operation.
    /// e.g. q_hash("llm:ReadGraph"), q_hash("llm:WriteGraph"), q_hash("llm:ExternalCall")
    pub intent_predicate: u64,
    /// The sub-graph slice the agent is requesting access to (Quin hash ranges).
    pub requested_graph_scope: Vec<u64>,
    /// Whether this intent requires outbound network access.
    pub requires_network: bool,
    /// Optional ILP payment offer for the operation (0 for fully local ops).
    pub ilp_offer_micro_cents: u64,
    /// The DID hash of the natural person who commanded or instantiated this session.
    pub principal_did_hash: u64,
    /// The persistent Intent Frame Hash established by the MCP session.
    pub mcp_intent_frame_hash: u64,
    /// The active capability profile, if one is bound to this session.
    #[serde(skip)]
    pub active_profile: Option<crate::profiles::CapabilityProfile>,
}

impl AgentIntent {
    /// Determines whether this intent is critical enough to proceed during a thermal event.
    pub fn is_critical(&self) -> bool {
        // Mock constant for a critical operation (e.g. q_hash("llm:EmergencyIntake"))
        self.intent_predicate == 0xC12171CA1
    }
}

// ─── WebizenVerdict ─────────────────────────────────────────────────────────
/// The Webizen VM's ruling on an AgentIntent or AgentOutput.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WebizenVerdict {
    /// Proceed. The intent/output is compliant with the Rights Ontology.
    Permit,
    /// Block. Reason is an N3Logic rule hash that caused the rejection.
    /// Can optionally carry a 48-byte Quin to write immediately to the immutable ledger.
    Deny { rule_violated: u64, reason: &'static str, conduct_record: Option<QualiaQuin> },
    /// Block with a detailed explanation for the user, usually tied to an Intent Frame violation.
    DenyWithExplanation { rule_violated: u64, reason: String, explanation: String },
    /// The operation might be valid, but requires explicit reconfirmation from the Principal.
    RequireReconfirmation { reason: String },
    /// The output was modified (sanitised) by the Webizen before passing through.
    Sanitised { original_hash: u64 },
}

// ─── AgentOutput ─────────────────────────────────────────────────────────────
/// The raw output from the LLM, before Webizen validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// The text generated by the LLM.
    pub text: String,
    /// Provenance citations — hashes of QualiuQuins this output is grounded in.
    /// MUST be non-empty: uncited outputs are blocked by the Webizen.
    pub provenance_quins: Vec<u64>,
    /// Tokens consumed (for compute metering).
    pub tokens_generated: u32,
    /// Inference duration.
    pub inference_duration_ms: u64,
    /// Memory peak during inference.
    pub peak_memory_bytes: u64,
}

// ─── AgentRuntime trait ───────────────────────────────────────────────────────
/// The core abstraction. All LLM backends MUST implement this.
/// The trait is object-safe so it can be boxed and swapped at runtime.
pub trait AgentRuntime: Send + Sync {
    /// Returns the configured backend variant.
    fn backend(&self) -> &AgentBackend;

    /// Returns the name/DID of this agent instance for audit purposes.
    fn agent_did(&self) -> &str;

    /// Submits an intent to the Webizen for pre-flight validation.
    /// This MUST be called before `infer`. Callers must not proceed if
    /// the verdict is `Deny`.
    fn validate_intent(&self, intent: &AgentIntent) -> WebizenVerdict;

    /// Runs inference on the given prompt and graph context.
    /// `graph_context` is a serialised sub-graph slice provided by the Webizen.
    fn infer(&self, prompt: &str, graph_context: &str) -> Result<AgentOutput, AgentError>;

    /// Submits the LLM output to the Webizen for post-flight grounding check.
    /// The Webizen verifies provenance citations exist in the live graph.
    fn validate_output(&self, output: &AgentOutput) -> WebizenVerdict;

    /// Returns remaining memory budget in bytes.
    fn memory_budget_remaining(&self) -> u64;
}

// ─── AgentError ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentError {
    /// Webizen blocked the intent before inference started.
    WebizenDenied { rule_violated: u64, reason: String },
    /// Inference timed out (> INFERENCE_TIMEOUT_MS).
    Timeout,
    /// LLM output had no provenance citations — rejected as ungrounded.
    UngroundedOutput,
    /// Memory budget exceeded.
    MemoryBudgetExceeded,
    /// Backend not available (model file missing, remote unreachable, etc.)
    BackendUnavailable(String),
}

// ─── LocalLlmAgent ───────────────────────────────────────────────────────────
/// The concrete local inference agent. Uses a mock inference path for now;
/// swap `infer_local_model` for an actual llama.cpp FFI call.
pub struct LocalLlmAgent {
    pub agent_did: String,
    pub backend: AgentBackend,
    pub memory_used_bytes: std::sync::atomic::AtomicU64,
}

impl LocalLlmAgent {
    pub fn new(agent_did: impl Into<String>, model_path: impl Into<String>) -> Self {
        Self {
            agent_did: agent_did.into(),
            backend: AgentBackend::Local {
                model_path: model_path.into(),
                context_window: 4096,
                quantization: "Q4_K_M".into(),
            },
            memory_used_bytes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Phase 8: Bifurcated Compute - SPSC Wait-Free Intercept
    /// Uses `rtrb` (Real-Time Ring Buffer) to establish a true zero-allocation,
    /// wait-free communication bridge between the LLM Engine and the Webizen Sentinel.
    fn infer_local_model(&self, prompt: &str, graph_context: &str) -> (String, Vec<u64>, u32) {
        use rtrb::RingBuffer;
        use std::thread;
        use std::time::Duration;

        #[derive(Clone, Debug)]
        enum VectorOp {
            TokenBytes([u8; 16]), // Simulated 128-bit vector embedding
            EndOfStream,
        }

        #[derive(Clone, Debug)]
        enum WebizenOp {
            DenyRollback,
        }

        // 1. Establish the Dual SPSC Wait-Free Ring Buffers
        // Logit Stream: LLM -> Sentinel (Vector topology)
        let (mut logit_p, mut logit_c) = RingBuffer::<VectorOp>::new(1024);
        
        // Control Stream: Sentinel -> LLM (Rollback commands)
        let (mut control_p, mut control_c) = RingBuffer::<WebizenOp>::new(16);

        // 2. Isolate B: LLM Engine Thread (Generates tokens)
        let llm_handle = thread::spawn(move || {
            let mut final_text = String::new();
            let output_text = "The rapid development of modern infrastructure... Wait, the internet did not exist in 1930.";
            let words: Vec<&str> = output_text.split_whitespace().collect();
            let mut tokens_generated = 0;
            
            for word in words {
                // Check Control Stream for wait-free intercepts from the Sentinel
                if let Ok(WebizenOp::DenyRollback) = control_c.pop() {
                    // LLM Engine handles the rollback immediately without OS locks
                    thread::sleep(Duration::from_millis(10));
                    final_text.push_str("[recalculated deterministic tensor] ");
                    tokens_generated += 3; // approximation for recalculation
                    continue;
                }

                // Generate Logit (Mocking specific words as anomalous signatures)
                let mut vector = [0u8; 16];
                if word.contains("internet") || word.contains("modern") {
                    vector[0] = 0x99; // Anomaly signature
                } else {
                    vector[0] = 0x01; // Safe signature
                }

                // Push vector down the Logit Stream
                let _ = logit_p.push(VectorOp::TokenBytes(vector));
                
                final_text.push_str(word);
                final_text.push_str(" ");
                tokens_generated += 1;
                thread::sleep(Duration::from_millis(5)); // Simulating inference latency
            }
            
            let _ = logit_p.push(VectorOp::EndOfStream);
            (final_text, tokens_generated)
        });

        // 3. Isolate A: Webizen Sentinel Thread (Audits the vector stream natively on main thread)
        loop {
            // Wait-free read attempt
            if let Ok(vector_op) = logit_c.pop() {
                match vector_op {
                    VectorOp::EndOfStream => break,
                    VectorOp::TokenBytes(bytes) => {
                        // Phase 8: Sentinel detects a mathematical/temporal anomaly natively in the bytes!
                        // 0x99 is our mocked "anachronistic token" signature.
                        if bytes[0] == 0x99 {
                            // Inject zero-allocation wait-free rollback signal instantly!
                            let _ = control_p.push(WebizenOp::DenyRollback);
                        }
                    }
                }
            }
        }

        let (output_text, tokens) = llm_handle.join().unwrap_or((String::new(), 0));
        let prov_hash = graph_context.bytes().take(8).fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        (output_text, vec![prov_hash], tokens)
    }
}

impl AgentRuntime for LocalLlmAgent {
    fn backend(&self) -> &AgentBackend { &self.backend }
    fn agent_did(&self) -> &str { &self.agent_did }

    fn validate_intent(&self, intent: &AgentIntent) -> WebizenVerdict {
        // Rule 1: No outbound network calls allowed from a Local backend.
        if intent.requires_network {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_OUTBOUND_TELEMETRY,
                reason: "Local backend: outbound network access violates Rights Ontology.",
                conduct_record: None,
            };
        }
        // Rule 2: Intent must not request access to Sanctuary-flagged graph scopes.
        // (A real check would query the SLG Arena for SANCTUARY metadata bits.)
        if intent.requested_graph_scope.iter().any(|&h| h == SANCTUARY_SCOPE_WEBIZEN) {
            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_SANCTUARY_ACCESS,
                reason: "Access to Sanctuary-flagged scope blocked.",
                conduct_record: None,
            };
        }
        
        // Rule 5: Cooperative Projects Directive — No adversarial, manipulative, or dishonest conduct.
        // Also tracks anti-human rights and discriminatory behavior for court auditing and liability.
        let is_adversarial = intent.intent_predicate == q_hash("llm:AdversarialOperation");
        let is_dishonest = intent.intent_predicate == q_hash("llm:DishonestOperation");
        let is_discriminatory = intent.intent_predicate == q_hash("llm:DiscriminatoryOperation");
        let is_anti_human_rights = intent.intent_predicate == q_hash("llm:AntiHumanRightsOperation");

        if is_adversarial || is_dishonest || is_discriminatory || is_anti_human_rights {
            let liability_weight: u64 = if is_anti_human_rights { 100 } else if is_discriminatory { 80 } else { 50 };
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            
            let mut conduct_quin = QualiaQuin {
                subject: q_hash(self.agent_did()),
                predicate: q_hash("q42:conductViolation"),
                // Inline tag integer (0b001 << 60)
                object: liability_weight | (0b001u64 << 60),
                context: intent.principal_did_hash,
                // Pack time and flags into metadata
                metadata: (now_ms & 0xFFFFFFFF) | ((is_anti_human_rights as u64) << 32) | ((is_discriminatory as u64) << 33),
                parity: 0,
            };
            
            // Calculate parity fold (XOR fold)
            conduct_quin.parity = conduct_quin.subject ^ conduct_quin.predicate ^ conduct_quin.object ^ conduct_quin.context;

            return WebizenVerdict::Deny {
                rule_violated: LLM_RULE_NO_ADVERSARIAL_CONDUCT,
                reason: "Cooperative Projects Directive Violation: Discriminatory, anti-human rights, or adversarial conduct detected.",
                conduct_record: Some(conduct_quin),
            };
        }

        // Rule 6: The Intent Predicate must align with the MCP Intent Frame.
        if intent.intent_predicate != intent.mcp_intent_frame_hash && intent.mcp_intent_frame_hash != crate::q_hash("purpose:General") {
            return WebizenVerdict::DenyWithExplanation {
                rule_violated: LLM_RULE_INTENT_FRAME_MISMATCH,
                reason: "Intent Frame Violation".into(),
                explanation: "The LLM attempted an operation outside the bounds of the active MCP Intent Frame.".into(),
            };
        }

        // Rule 7: Profile Constraints (Intent frames and Engine masking)
        if let Some(profile) = &intent.active_profile {
            if !profile.allows_intent(intent.intent_predicate) {
                return WebizenVerdict::DenyWithExplanation {
                    rule_violated: LLM_RULE_PROFILE_VIOLATION,
                    reason: "Profile Violation".into(),
                    explanation: "This capability profile explicitly blocks this intent frame.".into(),
                };
            }
            // For actual engine operations, the Orchestrator/Webizen VM would call `allows_engine()` 
            // when mapping the intent to native opcodes.
        }

        WebizenVerdict::Permit
    }

    fn infer(&self, prompt: &str, graph_context: &str) -> Result<AgentOutput, AgentError> {
        let t0 = Instant::now();

        // Memory guard
        let current = self.memory_used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        if current > LLM_MEMORY_BUDGET_BYTES {
            return Err(AgentError::MemoryBudgetExceeded);
        }

        // Timeout guard (production: run in a separate thread with channel)
        let deadline = Duration::from_millis(INFERENCE_TIMEOUT_MS);
        let (text, provenance, tokens) = self.infer_local_model(prompt, graph_context);
        if t0.elapsed() > deadline {
            return Err(AgentError::Timeout);
        }

        Ok(AgentOutput {
            text,
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
        let used = self.memory_used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        LLM_MEMORY_BUDGET_BYTES.saturating_sub(used)
    }
}

// ─── N3Logic Rule IDs (FNV-1a hashes of the rule URIs) ───────────────────────
// These match the corresponding rules in `docs/llm-governance-rules.n3`
pub const LLM_RULE_NO_OUTBOUND_TELEMETRY: u64 = 0xA1B2C3D4E5F60001;
pub const LLM_RULE_NO_SANCTUARY_ACCESS:   u64 = 0xA1B2C3D4E5F60002;
pub const LLM_RULE_PROVENANCE_REQUIRED:   u64 = 0xA1B2C3D4E5F60003;
pub const LLM_RULE_TOKEN_BUDGET:          u64 = 0xA1B2C3D4E5F60004;
pub const LLM_RULE_REMOTE_CONSENT:        u64 = 0xA1B2C3D4E5F60005;
pub const LLM_RULE_NO_ADVERSARIAL_CONDUCT:u64 = 0xA1B2C3D4E5F60006;
pub const LLM_RULE_INTENT_FRAME_MISMATCH: u64 = 0xA1B2C3D4E5F60007;
pub const LLM_RULE_PROFILE_VIOLATION:     u64 = 0xA1B2C3D4E5F60008;

/// Special webizen hash marking a Sanctuary-flagged graph scope.
pub const SANCTUARY_SCOPE_WEBIZEN: u64 = 0xDEAD_BABE_CAFE_0042;

// ─── Tests ───────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> LocalLlmAgent {
        LocalLlmAgent::new("did:git:antigravity-llm-001", "~/.qualia/models/phi3-mini.gguf")
    }

    #[test]
    fn test_webizen_blocks_outbound_network() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![],
            requires_network: true,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            active_profile: None,
        };
        let verdict = agent.validate_intent(&intent);
        assert!(matches!(verdict, WebizenVerdict::Deny { .. }), "Webizen must block outbound calls from local backend");
    }

    #[test]
    fn test_webizen_blocks_sanctuary_scope() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![SANCTUARY_SCOPE_WEBIZEN],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            active_profile: None,
        };
        let verdict = agent.validate_intent(&intent);
        assert!(matches!(verdict, WebizenVerdict::Deny { .. }), "Webizen must block Sanctuary scope access");
    }

    #[test]
    fn test_webizen_permits_valid_local_intent() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![0xDEAD_BEEF],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            active_profile: None,
        };
        assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);
    }

    #[test]
    fn test_full_roundtrip_grounded_output() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![0x1234],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            active_profile: None,
        };
        assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);

        let output = agent.infer("What is my health status?", "graph_context_bytes_here").unwrap();
        assert!(!output.text.is_empty());

        let post_verdict = agent.validate_output(&output);
        assert_eq!(post_verdict, WebizenVerdict::Permit, "Grounded output should pass post-flight check");
    }

    #[test]
    fn test_webizen_blocks_ungrounded_output() {
        let agent = make_agent();
        let ungrounded = AgentOutput {
            text: "I made this up with no sources.".into(),
            provenance_quins: vec![], // <-- no citations
            tokens_generated: 10,
            inference_duration_ms: 5,
            peak_memory_bytes: 0,
        };
        let verdict = agent.validate_output(&ungrounded);
        assert!(matches!(verdict, WebizenVerdict::Deny { .. }), "Webizen must block ungrounded output");
    }

    #[test]
    fn test_zero_allocation_adversarial_conduct_denial() {
        let _profiler = dhat::Profiler::builder().testing().build();
        
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: crate::q_hash("llm:AdversarialOperation"),
            requested_graph_scope: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: crate::q_hash("did:q42:human-rights-test-subject"),
            mcp_intent_frame_hash: crate::q_hash("purpose:General"),
            active_profile: None,
        };
        
        // Warm up any internal system components that might allocate on first use
        let _ = std::time::SystemTime::now();
        
        let stats_before = dhat::HeapStats::get();
        
        // Execute the intent validation (hot path)
        let verdict = agent.validate_intent(&intent);
        
        let stats_after = dhat::HeapStats::get();
        
        // Verify we got the Deny verdict with the QualiaQuin
        if let WebizenVerdict::Deny { conduct_record, .. } = verdict {
            assert!(conduct_record.is_some(), "Conduct record Quin must be generated");
            let quin = conduct_record.unwrap();
            assert_eq!(quin.predicate, crate::q_hash("q42:conductViolation"));
        } else {
            panic!("Expected Deny verdict for adversarial operation");
        }
        
        // Assert ABSOLUTELY ZERO heap allocations occurred during validate_intent
        assert_eq!(
            stats_after.total_blocks - stats_before.total_blocks,
            0,
            "validate_intent must not allocate on the heap"
        );
        assert_eq!(
            stats_after.total_bytes - stats_before.total_bytes,
            0,
            "validate_intent must not allocate on the heap"
        );
    }
}
