// Data types, the `AgentRuntime` trait, and governance rule IDs for the
// inference agent. Pure declarations — no decode logic.

use crate::NQuin;
use serde::{Deserialize, Serialize};

use crate::modalities::logic::n3_compiler::{
    AgentIntentFrame, N3OutputMode, MAX_CONTEXT_NAMESPACE_SLOTS, MAX_INTENT_SCOPE_SLOTS,
};

// ─── AgentBackend ────────────────────────────────────────────────────────────
/// Describes where inference actually runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentBackend {
    /// Quantized local model (llama.cpp WASM / ONNX Runtime / WebLLM + WebGPU).
    /// This is the PREFERRED backend — no outbound traffic.
    Local {
        model_path: String,   // e.g. "~/.qualia/models/phi3-mini-4bit.gguf"
        context_window: u32,  // tokens; typically 4096 for Phi-3-mini
        quantization: String, // "Q4_K_M", "Q8_0", etc.
        /// Path to mmproj / vision projector GGUF when `modality` is multimodal.
        #[serde(default)]
        vision_projector_path: Option<String>,
        /// `text` or `multimodal`
        #[serde(default = "default_local_modality")]
        modality: String,
        /// Architecture hint: `llava`, `qwen2vl`, `smolvlm`, `gemma3`, etc.
        #[serde(default)]
        architecture: Option<String>,
    },
    /// Remote model call. REQUIRES:
    ///   - Explicit Principal consent (signed VC)
    ///   - Nym mixnet routing (no raw IP correlation)
    ///   - ILP micropayment for every call
    ///   - Full audit trail written to .q42
    Remote {
        endpoint_did: String, // did:git of the approved remote provider
        nym_gateway: String,  // Nym gateway address
        ilp_budget_micro_cents: u64,
    },
    /// Local first; falls back to Remote only with Principal consent.
    Hybrid {
        local_model_path: String,
        remote_endpoint_did: String,
        consent_required: bool, // Always true in production
    },
}

// NOTE: `pub(super)` widening (was a private module-level fn) so `local_agent.rs`
// can call it from the constructor across the new submodule boundary.
pub(super) fn default_local_modality() -> String {
    "text".to_string()
}

/// Structured intent message from LLM → Webizen. Every call must declare
/// what it intends to do — the Webizen validates this against the Rights Ontology
/// BEFORE the LLM ever sees the user's semantic graph.
///
/// Cold-path session struct (serde/MCP). For zero-allocation pre-flight use
/// [`AgentIntentFrame`] via [`AgentIntent::to_frame`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIntent {
    /// N3Logic predicate hash declaring the class of operation.
    /// e.g. q_hash("llm:ReadGraph"), q_hash("llm:WriteGraph"), q_hash("llm:ExternalCall")
    pub intent_predicate: u64,
    /// The sub-graph slice the agent is requesting access to (Quin hash ranges).
    pub requested_graph_scope: Vec<u64>,
    /// Routed ontology and predicate namespaces relevant to this turn.
    #[serde(default)]
    pub context_namespaces: Vec<u64>,
    /// Whether this intent requires outbound network access.
    pub requires_network: bool,
    /// Optional ILP payment offer for the operation (0 for fully local ops).
    pub ilp_offer_micro_cents: u64,
    /// The DID hash of the natural person who commanded or instantiated this session.
    pub principal_did_hash: u64,
    /// The persistent Intent Frame Hash established by the MCP session.
    pub mcp_intent_frame_hash: u64,
    /// How the orchestrator should treat model output on the symbolic path.
    #[serde(default)]
    pub output_mode: N3OutputMode,
    /// Maximum sensitivity clearance for this session (bits `[56..63]`).
    #[serde(default)]
    pub clearance_ceiling: u8,
    /// Maximum Sentinel VM depth before `SentinelError::MemoryOverflow`.
    #[serde(default = "default_max_sentinel_depth")]
    pub max_sentinel_depth: u8,
    /// The active capability profile, if one is bound to this session.
    #[serde(skip)]
    pub active_profile: Option<crate::profiles::CapabilityProfile>,
}

fn default_max_sentinel_depth() -> u8 {
    32
}

impl AgentIntent {
    /// Determines whether this intent is critical enough to proceed during a thermal event.
    pub fn is_critical(&self) -> bool {
        // Mock constant for a critical operation (e.g. q_hash("llm:EmergencyIntake"))
        self.intent_predicate == 0xC12171CA1
    }

    /// Copy into a stack-allocated frame for Core-1 pre-flight validation.
    pub fn to_frame(&self) -> AgentIntentFrame {
        let mut graph_scope = [0u64; MAX_INTENT_SCOPE_SLOTS];
        let mut context_namespaces = [0u64; MAX_CONTEXT_NAMESPACE_SLOTS];
        let scope_count = self.requested_graph_scope.len().min(MAX_INTENT_SCOPE_SLOTS) as u8;
        let context_namespace_count = self
            .context_namespaces
            .len()
            .min(MAX_CONTEXT_NAMESPACE_SLOTS) as u8;
        for (i, hash) in self
            .requested_graph_scope
            .iter()
            .take(MAX_INTENT_SCOPE_SLOTS)
            .enumerate()
        {
            graph_scope[i] = *hash;
        }
        for (i, hash) in self
            .context_namespaces
            .iter()
            .take(MAX_CONTEXT_NAMESPACE_SLOTS)
            .enumerate()
        {
            context_namespaces[i] = *hash;
        }
        AgentIntentFrame {
            intent_predicate: self.intent_predicate,
            principal_did_hash: self.principal_did_hash,
            mcp_intent_frame_hash: self.mcp_intent_frame_hash,
            ilp_offer_micro_cents: self.ilp_offer_micro_cents,
            scope_count,
            context_namespace_count,
            requires_network: self.requires_network,
            output_mode: self.output_mode,
            clearance_ceiling: self.clearance_ceiling,
            max_sentinel_depth: self.max_sentinel_depth,
            graph_scope,
            context_namespaces,
        }
    }
}

impl AgentIntentFrame {
    /// Build a hot-path frame without heap allocation beyond the source intent's scope vec.
    pub fn from_intent(intent: &AgentIntent) -> Self {
        intent.to_frame()
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
    Deny {
        rule_violated: u64,
        reason: &'static str,
        conduct_record: Option<NQuin>,
    },
    /// Block with a detailed explanation for the user, usually tied to an Intent Frame violation.
    DenyWithExplanation {
        rule_violated: u64,
        reason: String,
        explanation: String,
    },
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
    /// Structured graph emission when the neuro-symbolic sieve completes (no heap parse).
    #[serde(default)]
    pub semantic_quin: Option<NQuin>,
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
    /// Sieve mask rejected all logits — model output unaligned with graph grammar.
    SieveMisaligned,
}

// ─── N3Logic Rule IDs (FNV-1a hashes of the rule URIs) ───────────────────────
// These match the corresponding rules in `docs/llm-governance-rules.n3`
pub const LLM_RULE_NO_OUTBOUND_TELEMETRY: u64 = 0xA1B2C3D4E5F60001;
pub const LLM_RULE_NO_SANCTUARY_ACCESS: u64 = 0xA1B2C3D4E5F60002;
pub const LLM_RULE_PROVENANCE_REQUIRED: u64 = 0xA1B2C3D4E5F60003;
pub const LLM_RULE_TOKEN_BUDGET: u64 = 0xA1B2C3D4E5F60004;
pub const LLM_RULE_REMOTE_CONSENT: u64 = 0xA1B2C3D4E5F60005;
pub const LLM_RULE_NO_ADVERSARIAL_CONDUCT: u64 = 0xA1B2C3D4E5F60006;
pub const LLM_RULE_INTENT_FRAME_MISMATCH: u64 = 0xA1B2C3D4E5F60007;
pub const LLM_RULE_PROFILE_VIOLATION: u64 = 0xA1B2C3D4E5F60008;

/// Special webizen hash marking a Sanctuary-flagged graph scope.
pub const SANCTUARY_SCOPE_WEBIZEN: u64 = 0xDEAD_BABE_CAFE_0042;
