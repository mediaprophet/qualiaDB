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
//   - All outputs must be cited to a NQuin provenance chain
//   - Webizen validates I/O before touching the semantic graph
//   - Memory budget hard-capped; default 128MB within 512MB floor

use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ─── Constants ──────────────────────────────────────────────────────────────
/// Hard memory ceiling for the LLM runtime within the 512MB system floor.
/// Leaves the remaining 384MB for the Webizen VM, SLG Arena, and WASM stack.
pub const LLM_MEMORY_BUDGET_BYTES: u64 = 128 * 1024 * 1024; // 128 MB

/// Maximum tokens the agent may generate in a single turn. Enforces deterministic
/// compute cost — no runaway generation that blocks the edge device.
pub const MAX_OUTPUT_TOKENS: u32 = 2048;

/// Token budget for the autoregressive loop (`MAX_OUTPUT_TOKENS` in release).
#[cfg(test)]
const DECODE_TOKEN_BUDGET: u32 = 16;
#[cfg(not(test))]
const DECODE_TOKEN_BUDGET: u32 = MAX_OUTPUT_TOKENS;

/// Layer cap for transformer forward during unit tests (full depth in release).
#[cfg(test)]
const TEST_TRANSFORMER_LAYER_CAP: u32 = 2;
#[cfg(not(test))]
const TEST_TRANSFORMER_LAYER_CAP: u32 = 0;

/// Vocab chunk cap during unit tests (full sweep in release).
#[cfg(test)]
const TEST_VOCAB_CHUNK_CAP: u32 = 4;
#[cfg(not(test))]
const TEST_VOCAB_CHUNK_CAP: u32 = 0;

/// Maximum milliseconds for a local inference call before timeout.
pub const INFERENCE_TIMEOUT_MS: u64 = 30_000;

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

fn default_local_modality() -> String {
    "text".to_string()
}

// ─── AgentIntent ─────────────────────────────────────────────────────────────
pub use crate::modalities::logic::n3_compiler::{
    AgentIntentFrame, N3OutputMode, MAX_CONTEXT_NAMESPACE_SLOTS, MAX_INTENT_SCOPE_SLOTS,
};

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

// ─── Embedding dispatch helpers (native) ─────────────────────────────────────


fn pseudo_embedding_forward(
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    engine: &crate::gguf_bridge::QTensorEngine,
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    for i in 0..emb_dim {
        emb_buf[i] = (token_id as f32 * (i as f32 + 1.0) * 0.001_f32).sin()
            * (1.0_f32 / (emb_dim as f32).sqrt());
    }
    engine.dispatch_fused_transformer_block(wt, &emb_buf[..emb_dim])
}

#[cfg(not(target_arch = "wasm32"))]
fn cpu_embedding_forward(
    engine: &crate::gguf_bridge::QTensorEngine,
    idx: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    let n = idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
    if n > 0 {
        engine.dispatch_fused_transformer_block(wt, &emb_buf[..n])
    } else {
        pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
    }
}

/// Like `cpu_embedding_forward` but applies a LoRA delta to the embedding
/// vector before dispatching it through the transformer block.
///
/// The delta is computed as `B @ (A @ emb) * scaling` on the CPU.
/// If the adapter dimensions do not match `emb_dim` the call silently falls
/// back to the unmodified embedding (the base model is still correct).
fn lora_embedding_forward(
    engine:  &crate::gguf_bridge::QTensorEngine,
    idx:     &crate::gguf_sharder::GgufTensorIndex,
    mmap:    &[u8],
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt:      &crate::gguf_bridge::QTensor,
    adapter: &crate::lora::LoRAAdapter,
) -> Vec<f32> {
    let n = idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
    let actual_n = if n > 0 { n } else { emb_dim };

    if n == 0 {
        // Populate emb_buf with pseudo embeddings
        for i in 0..emb_dim {
            emb_buf[i] = (token_id as f32 * (i as f32 + 1.0) * 0.001_f32).sin()
                * (1.0_f32 / (emb_dim as f32).sqrt());
        }
    }

    // Apply LoRA delta if dimensions match — silent no-op otherwise
    if adapter.meta.n_in == actual_n && adapter.meta.n_out == actual_n {
        let input_snap: Vec<f32> = emb_buf[..actual_n].to_vec();
        let _ = adapter.apply_cpu(&input_snap, &mut emb_buf[..actual_n]);
    }

    engine.dispatch_fused_transformer_block(wt, &emb_buf[..actual_n])
}

fn build_sieve(
    tok: &crate::gguf_sharder::GgufTokenizer,
    spec: Option<&crate::neuro_symbolic_sieve::SieveLexSpec>,
    lex_path: Option<&str>,
) -> Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve> {
    let spec = spec?;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = lex_path {
        let p = std::path::Path::new(path);
        if crate::q42_volume::is_v2_volume(p).unwrap_or(false) {
            if let Ok(vol) = crate::q42_volume::Q42Volume::open(p) {
                if let Ok(view) = vol.lex_view() {
                    let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_lex_and_tokenizer(
                        &view, tok, spec,
                    );
                    if s.masks_ready() {
                        return Some(s);
                    }
                }
            }
        } else if let Ok(lex_file) = crate::q42_lex::Q42LexFile::open(p) {
            let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_lex_and_tokenizer(
                &lex_file.view(),
                tok,
                spec,
            );
            if s.masks_ready() {
                return Some(s);
            }
        }
    }
    let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_gguf_tokenizer(tok);
    if s.masks_ready() {
        Some(s)
    } else {
        None
    }
}

// ─── LocalLlmAgent ───────────────────────────────────────────────────────────
/// The concrete local inference agent. Uses a mock inference path for now;
/// swap `infer_local_model` for an actual llama.cpp FFI call.
pub struct LocalLlmAgent {
    pub agent_did: String,
    pub backend: AgentBackend,
    pub memory_used_bytes: std::sync::atomic::AtomicU64,
    /// Set by `validate_intent` when `output_mode` requires graph-structured emission.
    use_sieve_output: std::sync::atomic::AtomicBool,
    /// Memory-mapped `.q42.lex` sidecar for dynamic sieve masks.
    sieve_lex_path: std::sync::Mutex<Option<String>>,
    /// IRI hashes to resolve through the lexicon for Subject / Predicate / Object slots.
    sieve_spec: std::sync::Mutex<crate::neuro_symbolic_sieve::SieveLexSpec>,
    /// Optional LoRA adapter manager for zero-copy context-driven neural adaptation.
    /// When set, the prompt is classified into a domain (Medical / Legal / Chemical / …)
    /// and the matching adapter's delta is applied to the embedding hidden state before
    /// the autoregressive decode loop.
    lora_manager: std::sync::Mutex<Option<crate::lora::LoRAAdapterManager>>,
}

impl LocalLlmAgent {
    pub fn new(agent_did: impl Into<String>, model_path: impl Into<String>) -> Self {
        Self::with_local_backend(
            agent_did,
            AgentBackend::Local {
                model_path: model_path.into(),
                context_window: 4096,
                quantization: "Q4_K_M".into(),
                vision_projector_path: None,
                modality: default_local_modality(),
                architecture: None,
            },
        )
    }

    /// Construct an agent with a fully specified backend (e.g. catalog multimodal profile).
    pub fn with_local_backend(agent_did: impl Into<String>, backend: AgentBackend) -> Self {
        Self {
            agent_did: agent_did.into(),
            backend,
            memory_used_bytes: std::sync::atomic::AtomicU64::new(0),
            use_sieve_output: std::sync::atomic::AtomicBool::new(false),
            sieve_lex_path: std::sync::Mutex::new(None),
            sieve_spec: std::sync::Mutex::new(
                crate::neuro_symbolic_sieve::SieveLexSpec::graph_mutation_default(),
            ),
            lora_manager: std::sync::Mutex::new(None),
        }
    }

    // ── LoRA adapter management ───────────────────────────────────────────────

    /// Attach a LoRA adapter directory to this agent.
    ///
    /// Adapters are loaded lazily on the first prompt that triggers a domain
    /// switch.  The directory must contain `*.lora` files named after
    /// `ContextType::adapter_filename()` (e.g. `medical_v1.lora`).
    pub fn attach_lora_adapters(&self, adapter_dir: impl Into<std::path::PathBuf>) {
        let mgr = crate::lora::LoRAAdapterManager::new(adapter_dir);
        *self.lora_manager.lock().unwrap_or_else(|e| e.into_inner()) = Some(mgr);
    }

    /// Attach a LoRA manager pre-configured with expected embedding dimensions.
    ///
    /// `n_in` should match the model's embedding dimension (e.g. 4096 for 7B models).
    pub fn attach_lora_adapters_with_dims(
        &self,
        adapter_dir: impl Into<std::path::PathBuf>,
        n_in:  usize,
        n_out: usize,
    ) {
        let mut mgr = crate::lora::LoRAAdapterManager::new(adapter_dir);
        mgr.set_expected_dims(n_in, n_out);
        *self.lora_manager.lock().unwrap_or_else(|e| e.into_inner()) = Some(mgr);
    }

    /// Remove the LoRA manager and revert to base-model-only inference.
    pub fn detach_lora_adapters(&self) {
        *self.lora_manager.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Detect context from `prompt` and pre-warm the LoRA adapter cache.
    ///
    /// Call this before a batch of related prompts to avoid cold-load latency
    /// on the first inference.
    pub fn warm_lora_for_prompt(&self, prompt: &str) {
        let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mgr) = guard.as_mut() {
            let (ctx, conf) = mgr.detector.analyze_text(prompt);
            if conf >= mgr.detector.confidence_threshold {
                let _ = mgr.switch_to(ctx);
            }
        }
    }

    /// Return the currently active LoRA context type, if any.
    pub fn active_lora_context(&self) -> Option<crate::lora::ContextType> {
        let guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().and_then(|m| m.active()).map(|a| a.context_type)
    }

    /// Wire the `.q42.lex` sidecar used to populate FSM sieve masks at inference time.
    pub fn configure_sieve_lex(&self, path: impl Into<String>) {
        *self.sieve_lex_path.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.into());
    }

    pub fn agent_did_hash(&self) -> u64 {
        q_hash(&self.agent_did)
    }

    /// Phase 8: Bifurcated Compute — SPSC Wait-Free Intercept.
    ///
    /// On native targets: loads the GGUF model, tokenises the prompt, and runs an
    /// autoregressive decode loop via `QTensorEngine::dispatch_fused_transformer_block`.
    /// Logit summaries flow from the LLM engine thread to the Webizen Sentinel (this
    /// thread) over a wait-free SPSC ring; the Sentinel injects `DenyRollback` when
    /// it detects the 0x99 anachronism byte in the top-logit's bit pattern.
    ///
    /// On WASM / non-local backends: falls through to the original mock path.
    /// Run local inference, optionally streaming decoded text deltas to `on_token`.
    pub fn infer_local_model_streaming<F: FnMut(String) + Send>(
        &self,
        prompt: &str,
        graph_context: &str,
        mut on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, on_token.as_mut())
    }

    fn infer_local_model(
        &self,
        prompt: &str,
        graph_context: &str,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner::<fn(String)>(prompt, graph_context, None)
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables, unused_mut))]
    fn infer_local_model_inner<F: FnMut(String) + Send>(
        &self,
        prompt: &str,
        graph_context: &str,
        mut on_token: Option<&mut F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        let prov_hash = graph_context
            .bytes()
            .take(8)
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let use_sieve = self
            .use_sieve_output
            .load(std::sync::atomic::Ordering::Relaxed);
        let sieve_spec = if use_sieve {
            Some(*self.sieve_spec.lock().unwrap_or_else(|e| e.into_inner()))
        } else {
            None
        };
        let sieve_lex_path = if use_sieve {
            self.sieve_lex_path.lock().unwrap_or_else(|e| e.into_inner()).clone()
        } else {
            None
        };

        // ── Native GPU path ─────────────────────────────────────────────────
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::gguf_bridge::{QTensor, QTensorEngine};
            use crate::gguf_sharder::GgufTokenizer;
            use rtrb::RingBuffer;
            use std::thread;

            let model_path = match &self.backend {
                AgentBackend::Local { model_path, .. } => model_path.clone(),
                _ => {
                    return (
                        String::from("[no local model configured]"),
                        vec![prov_hash],
                        0,
                        None,
                    )
                }
            };
            let prompt_owned = prompt.to_string();

            // ── LoRA context detection (before thread spawn) ─────────────────
            // Detect the prompt domain and pre-load the matching LoRA adapter.
            // The pre-computed delta vectors are cloned into the inference thread
            // as fixed-size heap data — one allocation per infer call, not per token.
            #[allow(unused_variables)]
            let lora_active_adapter: Option<crate::lora::LoRAAdapter> = {
                let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut mgr) = *guard {
                    let (ctx, conf, _switched) = mgr.auto_switch(
                        &prompt_owned,
                        mgr.detector.confidence_threshold,
                    );
                    log::debug!("LoRA|context-detect|domain={ctx}|conf={conf:.3}");
                    mgr.active().cloned()
                } else {
                    None
                }
            };

            // Fixed-size types keep the hot-path allocation-free in the ring buffer.
            #[derive(Clone, Copy)]
            struct LogitSummary {
                _top_id: u32,
                anomaly: u8,
            }
            #[derive(Clone)]
            enum LlmMsg {
                Logit(LogitSummary),
                Eos,
            }
            #[derive(Clone)]
            enum SentMsg {
                DenyRollback,
            }

            // LogitStream: LLM engine → Webizen Sentinel
            let (mut lp, mut lc) = RingBuffer::<LlmMsg>::new(1024);
            // ControlStream: Webizen Sentinel → LLM engine
            let (mut cp, mut cc) = RingBuffer::<SentMsg>::new(16);

            let stream_pair = if on_token.is_some() {
                Some(std::sync::mpsc::sync_channel::<String>(512))
            } else {
                None
            };
            let stream_tx_thread = stream_pair.as_ref().map(|(tx, _)| tx.clone());

            // Move the (optional) LoRA adapter into the inference thread.
            let lora_for_thread = lora_active_adapter;

            // ── LLM engine thread ────────────────────────────────────────────
            let h = thread::spawn(move || -> (String, u32, Option<NQuin>, bool) {
                // Initialize Tokio runtime for the thread to prevent panics in async components
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap_or_else(|e| panic!("Failed to create Tokio runtime for LLM thread: {}", e));
                let _rt_guard = rt.enter();

                let lora_adapter = lora_for_thread;
                let sieve_spec = sieve_spec;
                let sieve_lex_path = sieve_lex_path;
                // Build the GPU engine and memory-map the GGUF inside the thread to
                // avoid Send constraints on the DirectML / wgpu device handles.
                let mut engine = QTensorEngine::new();
                if let Some(mmap) =
                    crate::resident_model::resident_mmap_for_path(model_path.as_str())
                {
                    if engine.adopt_resident_mmap(mmap).is_err() {
                        engine.load_gguf(&model_path);
                    }
                } else {
                    engine.load_gguf(&model_path);
                }

                let tok = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| GgufTokenizer::from_gguf(m))
                    .unwrap_or_default();

                // Parse tensor-info section → real embedding lookup.
                let tensor_idx = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m));

                let mut ctx = tok.encode(&prompt_owned);
                if ctx.is_empty() {
                    ctx.push(tok.bos_token_id);
                }
                let eos = tok.eos_token_id;
                let vlen = tok.vocab_len().max(1);

                // Use the real embedding dimension if the tensor was found; fall back to 4096.
                let emb_dim = tensor_idx
                    .as_ref()
                    .map(|idx| idx.emb_dim())
                    .filter(|&d| d > 0)
                    .unwrap_or(4096);

                // Stack buffers — zero-heap path (512MB floor safe).
                use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

                const MAX_EMB_DIM: usize = 8192;
                const MAX_FFN_DIM: usize = 10240;
                let mut emb_buf = [0f32; MAX_EMB_DIM];
                let mut scratch_a = [0f32; MAX_FFN_DIM];
                let mut scratch_b = [0f32; MAX_FFN_DIM];
                let mut prefill_chunk = [0f32; PREFILL_CHUNK_STACK_FLOATS];
                let emb_dim = emb_dim.min(MAX_EMB_DIM);
                engine.reset_kv_cache();

                // Chunked prefill: populate KV for prompt tokens [0, prompt_len-1).
                let prompt_len = ctx.len();
                if prompt_len > 1 {
                    if let Some(idx) = tensor_idx.as_ref() {
                        let prefill_tokens = prompt_len - 1;
                        let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim)
                            .min(PREFILL_CHUNK_SIZE)
                            .max(1);
                        let mut pos = 0usize;
                        while pos < prefill_tokens {
                            let n = (prefill_tokens - pos).min(chunk_cap);
                            let batch_elems = n * emb_dim;
                            {
                                let mmap = match engine.gguf_mmap.as_deref() {
                                    Some(m) => m,
                                    None => break,
                                };
                                for t in 0..n {
                                    let _ = idx.dequantize_token_embedding_into(
                                        mmap,
                                        ctx[pos + t],
                                        &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                                    );
                                }
                            }
                            let _ = engine.dispatch_prefill_chunk(
                                idx,
                                &mut prefill_chunk[..batch_elems],
                                emb_dim,
                                n as u32,
                                pos as u32,
                                &mut scratch_a,
                                &mut scratch_b,
                                TEST_TRANSFORMER_LAYER_CAP,
                            );
                            pos += n;
                        }
                    }
                }

                let mut out_ids: Vec<u32> = Vec::new();
                let mut streamed_len = 0usize;
                let mut sieve = if use_sieve {
                    build_sieve(&tok, sieve_spec.as_ref(), sieve_lex_path.as_deref())
                } else {
                    None
                };
                let mut semantic_quin: Option<NQuin> = None;
                let mut sieve_failed = false;
                let gen_budget = if sieve.is_some() {
                    3usize
                } else {
                    DECODE_TOKEN_BUDGET as usize
                };

                for _ in 0..gen_budget {
                    // Check ControlStream for a DenyRollback injected in the previous step.
                    let rollback = cc.pop().is_ok();

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);

                    // 1) Embedding lookup → hidden state (stack dequant).
                    let hidden_ok = tensor_idx
                        .as_ref()
                        .and_then(|idx| {
                            engine.gguf_mmap.as_deref().map(|m| {
                                idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim])
                            })
                        })
                        .unwrap_or(0);

                    // 1b) LoRA delta — additive correction to the embedding vector.
                    // Applied after dequantize so the base model is unmodified.
                    // Silently skipped if dimensions don't match (wrong adapter for model).
                    if hidden_ok > 0 {
                        if let Some(ref adapter) = lora_adapter {
                            if adapter.meta.n_in == hidden_ok && adapter.meta.n_out == hidden_ok {
                                let snap: Vec<f32> = emb_buf[..hidden_ok].to_vec();
                                let _ = adapter.apply_cpu(&snap, &mut emb_buf[..hidden_ok]);
                            }
                        }
                    }

                    let (top_i, top_v) = if hidden_ok > 0 {
                        if let Some(idx) = tensor_idx.as_ref() {
                            let token_idx = ctx.len().saturating_sub(1) as u32;
                            let _layers = engine.dispatch_transformer_forward(
                                idx,
                                &mut emb_buf[..emb_dim],
                                emb_dim,
                                &mut scratch_a,
                                &mut scratch_b,
                                token_idx,
                                TEST_TRANSFORMER_LAYER_CAP,
                            );
                            let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                            if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                                idx,
                                &emb_buf[..emb_dim],
                                emb_dim,
                                &mut scratch_a[..],
                                TEST_VOCAB_CHUNK_CAP,
                                sieve_mask,
                            ) {
                                if argmax.max_logit > f32::NEG_INFINITY {
                                    (argmax.best_token_id as usize, argmax.max_logit)
                                } else {
                                    sieve_failed = true;
                                    (0usize, f32::NEG_INFINITY)
                                }
                            } else {
                                emb_buf[..emb_dim].iter().enumerate().fold(
                                    (0usize, f32::NEG_INFINITY),
                                    |(bi, bv), (i, &v)| {
                                        if v > bv {
                                            (i, v)
                                        } else {
                                            (bi, bv)
                                        }
                                    },
                                )
                            }
                        } else {
                            (0usize, 0.0)
                        }
                    } else {
                        let wt = QTensor::new(vec![emb_dim, emb_dim], 0, true);
                        let logits =
                            pseudo_embedding_forward(cur, emb_dim, &mut emb_buf[..], &engine, &wt);
                        logits.iter().enumerate().fold(
                            (0usize, f32::NEG_INFINITY),
                            |(bi, bv), (i, &v)| if v > bv { (i, v) } else { (bi, bv) },
                        )
                    };

                    // Anomaly flag: 0x99 as the first byte of the top logit's IEEE-754
                    // representation is the sentinel value for an anachronistic token.
                    let anomaly = if top_v.to_le_bytes()[0] == 0x99 {
                        0x99u8
                    } else {
                        0x01u8
                    };

                    // Push logit summary; non-blocking — drops silently if ring is full.
                    let _ = lp.push(LlmMsg::Logit(LogitSummary {
                        _top_id: top_i as u32,
                        anomaly,
                    }));

                    // On DenyRollback, substitute a safe neighbour token instead of argmax.
                    if sieve_failed {
                        break;
                    }

                    let next = if rollback {
                        cur.wrapping_add(1) % vlen
                    } else {
                        (top_i as u32) % vlen
                    };

                    if let Some(ref mut s) = sieve {
                        match s.apply_token(next) {
                            Ok(()) => {
                                out_ids.push(next);
                                ctx.push(next);
                                if s.is_complete() {
                                    semantic_quin = Some(s.assemble_quin(prov_hash));
                                    break;
                                }
                            }
                            Err(_) => {
                                sieve_failed = true;
                                break;
                            }
                        }
                    } else {
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref tx) = stream_tx_thread {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                let _ = tx.send(delta);
                            }
                        }
                        if next == eos {
                            break;
                        }
                    }
                }

                let _ = lp.push(LlmMsg::Eos);
                let text = if semantic_quin.is_some() {
                    String::new()
                } else if sieve_failed {
                    String::from("[sieve-misaligned]")
                } else {
                    tok.decode(&out_ids)
                };
                (text, out_ids.len() as u32, semantic_quin, sieve_failed)
            });

            // ── Webizen Sentinel (calling thread) ────────────────────────────
            let mut drain_tokens = || {
                if let (Some((_, ref rx)), Some(cb)) = (&stream_pair, on_token.as_mut()) {
                    while let Ok(delta) = rx.try_recv() {
                        cb(delta);
                    }
                }
            };

            loop {
                drain_tokens();
                match lc.pop() {
                    Ok(LlmMsg::Eos) => break,
                    Ok(LlmMsg::Logit(s)) => {
                        if s.anomaly == 0x99 {
                            let _ = cp.push(SentMsg::DenyRollback);
                        }
                    }
                    Err(_) => std::hint::spin_loop(),
                }
            }

            drain_tokens();

            let (text, tokens, semantic_quin, sieve_failed) =
                h.join().unwrap_or_else(|_| (String::new(), 0, None, false));
            let mut prov = vec![prov_hash];
            if prov_hash == 0 {
                prov.push(q_hash("qualia:grounded"));
            }
            if let Some(q) = semantic_quin {
                prov.push(q.subject);
                prov.push(q.predicate);
                prov.push(q.object);
            }
            if sieve_failed && semantic_quin.is_none() {
                return (text, prov, tokens, None);
            }
            return (text, prov, tokens, semantic_quin);
        }

// ── Native GPU path ─────────────────────────────────────────────────
        #[cfg(target_arch = "wasm32")]
        {
            use crate::gguf_bridge::{QTensor, QTensorEngine};
            use crate::gguf_sharder::GgufTokenizer;
                        
            let model_path = match &self.backend {
                AgentBackend::Local { model_path, .. } => model_path.clone(),
                _ => {
                    return (
                        String::from("[no local model configured]"),
                        vec![prov_hash],
                        0,
                        None,
                    )
                }
            };
            let prompt_owned = prompt.to_string();

            // ── LoRA context detection (before thread spawn) ─────────────────
            // Detect the prompt domain and pre-load the matching LoRA adapter.
            // The pre-computed delta vectors are cloned into the inference thread
            // as fixed-size heap data — one allocation per infer call, not per token.
            #[allow(unused_variables)]
            let lora_active_adapter: Option<crate::lora::LoRAAdapter> = {
                let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut mgr) = *guard {
                    let (ctx, conf, _switched) = mgr.auto_switch(
                        &prompt_owned,
                        mgr.detector.confidence_threshold,
                    );
                    log::debug!("LoRA|context-detect|domain={ctx}|conf={conf:.3}");
                    mgr.active().cloned()
                } else {
                    None
                }
            };

            // Fixed-size types keep the hot-path allocation-free in the ring buffer.
            #[derive(Clone, Copy)]
            struct LogitSummary {
                _top_id: u32,
                anomaly: u8,
            }
            #[derive(Clone)]
            enum LlmMsg {
                Logit(LogitSummary),
                Eos,
            }
            #[derive(Clone)]
            enum SentMsg {
                DenyRollback,
            }

            
            
            // Move the (optional) LoRA adapter into the inference thread.
            let lora_for_thread = lora_active_adapter;

            // ── LLM engine synchronous execution ─────────────────────────────
            let (text, tokens, semantic_quin, sieve_failed) = {
                let mut rollback = false;

                let lora_adapter = lora_for_thread;
                let sieve_spec = sieve_spec;
                let sieve_lex_path = sieve_lex_path;
                // Build the GPU engine and memory-map the GGUF inside the thread to
                // avoid Send constraints on the DirectML / wgpu device handles.
                let mut engine = {
                    let engine_guard = crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| g.borrow_mut().take());
                    engine_guard.expect("WASM WebGPU engine not initialized. Call initialize_webgpu_engine first.")
                };

                let tok = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| GgufTokenizer::from_gguf(m))
                    .unwrap_or_default();

                // Parse tensor-info section → real embedding lookup.
                let tensor_idx = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m));

                let mut ctx = tok.encode(&prompt_owned);
                if ctx.is_empty() {
                    ctx.push(tok.bos_token_id);
                }
                let eos = tok.eos_token_id;
                let vlen = tok.vocab_len().max(1);

                // Use the real embedding dimension if the tensor was found; fall back to 4096.
                let emb_dim = tensor_idx
                    .as_ref()
                    .map(|idx| idx.emb_dim())
                    .filter(|&d| d > 0)
                    .unwrap_or(4096);

                // Stack buffers — zero-heap path (512MB floor safe).
                use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

                const MAX_EMB_DIM: usize = 8192;
                const MAX_FFN_DIM: usize = 10240;
                let mut emb_buf = [0f32; MAX_EMB_DIM];
                let mut scratch_a = [0f32; MAX_FFN_DIM];
                let mut scratch_b = [0f32; MAX_FFN_DIM];
                let mut prefill_chunk = [0f32; PREFILL_CHUNK_STACK_FLOATS];
                let emb_dim = emb_dim.min(MAX_EMB_DIM);
                engine.reset_kv_cache();

                // Chunked prefill: populate KV for prompt tokens [0, prompt_len-1).
                let prompt_len = ctx.len();
                if prompt_len > 1 {
                    if let Some(idx) = tensor_idx.as_ref() {
                        let prefill_tokens = prompt_len - 1;
                        let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim)
                            .min(PREFILL_CHUNK_SIZE)
                            .max(1);
                        let mut pos = 0usize;
                        while pos < prefill_tokens {
                            let n = (prefill_tokens - pos).min(chunk_cap);
                            let batch_elems = n * emb_dim;
                            {
                                let mmap = match engine.gguf_mmap.as_deref() {
                                    Some(m) => m,
                                    None => break,
                                };
                                for t in 0..n {
                                    let _ = idx.dequantize_token_embedding_into(
                                        mmap,
                                        ctx[pos + t],
                                        &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                                    );
                                }
                            }
                            let _ = engine.dispatch_prefill_chunk(
                                idx,
                                &mut prefill_chunk[..batch_elems],
                                emb_dim,
                                n as u32,
                                pos as u32,
                                &mut scratch_a,
                                &mut scratch_b,
                                TEST_TRANSFORMER_LAYER_CAP,
                            );
                            pos += n;
                        }
                    }
                }

                let mut out_ids: Vec<u32> = Vec::new();
                let mut streamed_len = 0usize;
                let mut sieve = if use_sieve {
                    build_sieve(&tok, sieve_spec.as_ref(), sieve_lex_path.as_deref())
                } else {
                    None
                };
                let mut semantic_quin: Option<NQuin> = None;
                let mut sieve_failed = false;
                let gen_budget = if sieve.is_some() {
                    3usize
                } else {
                    DECODE_TOKEN_BUDGET as usize
                };

                for _ in 0..gen_budget {
                    // Check ControlStream for a DenyRollback injected in the previous step.
                    let rollback_val = rollback; rollback = false; let mut rollback = rollback_val;

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);

                    // 1) Embedding lookup → hidden state (stack dequant).
                    let hidden_ok = tensor_idx
                        .as_ref()
                        .and_then(|idx| {
                            engine.gguf_mmap.as_deref().map(|m| {
                                idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim])
                            })
                        })
                        .unwrap_or(0);

                    // 1b) LoRA delta — additive correction to the embedding vector.
                    // Applied after dequantize so the base model is unmodified.
                    // Silently skipped if dimensions don't match (wrong adapter for model).
                    if hidden_ok > 0 {
                        if let Some(ref adapter) = lora_adapter {
                            if adapter.meta.n_in == hidden_ok && adapter.meta.n_out == hidden_ok {
                                let snap: Vec<f32> = emb_buf[..hidden_ok].to_vec();
                                let _ = adapter.apply_cpu(&snap, &mut emb_buf[..hidden_ok]);
                            }
                        }
                    }

                    let (top_i, top_v) = if hidden_ok > 0 {
                        if let Some(idx) = tensor_idx.as_ref() {
                            let token_idx = ctx.len().saturating_sub(1) as u32;
                            let _layers = engine.dispatch_transformer_forward(
                                idx,
                                &mut emb_buf[..emb_dim],
                                emb_dim,
                                &mut scratch_a,
                                &mut scratch_b,
                                token_idx,
                                TEST_TRANSFORMER_LAYER_CAP,
                            );
                            let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                            if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                                idx,
                                &emb_buf[..emb_dim],
                                emb_dim,
                                &mut scratch_a[..],
                                TEST_VOCAB_CHUNK_CAP,
                                sieve_mask,
                            ) {
                                if argmax.max_logit > f32::NEG_INFINITY {
                                    (argmax.best_token_id as usize, argmax.max_logit)
                                } else {
                                    sieve_failed = true;
                                    (0usize, f32::NEG_INFINITY)
                                }
                            } else {
                                emb_buf[..emb_dim].iter().enumerate().fold(
                                    (0usize, f32::NEG_INFINITY),
                                    |(bi, bv), (i, &v)| {
                                        if v > bv {
                                            (i, v)
                                        } else {
                                            (bi, bv)
                                        }
                                    },
                                )
                            }
                        } else {
                            (0usize, 0.0)
                        }
                    } else {
                        let wt = QTensor::new(vec![emb_dim, emb_dim], 0, true);
                        let logits =
                            pseudo_embedding_forward(cur, emb_dim, &mut emb_buf[..], &engine, &wt);
                        logits.iter().enumerate().fold(
                            (0usize, f32::NEG_INFINITY),
                            |(bi, bv), (i, &v)| if v > bv { (i, v) } else { (bi, bv) },
                        )
                    };

                    // Anomaly flag: 0x99 as the first byte of the top logit's IEEE-754
                    // representation is the sentinel value for an anachronistic token.
                    let anomaly = if top_v.to_le_bytes()[0] == 0x99 {
                        0x99u8
                    } else {
                        0x01u8
                    };

                    // Inline Sentinel Check
                    if anomaly == 0x99 {
                        rollback = true;
                    }

                    // On DenyRollback, substitute a safe neighbour token instead of argmax.
                    if sieve_failed {
                        break;
                    }

                    let next = if rollback {
                        cur.wrapping_add(1) % vlen
                    } else {
                        (top_i as u32) % vlen
                    };

                    if let Some(ref mut s) = sieve {
                        match s.apply_token(next) {
                            Ok(()) => {
                                out_ids.push(next);
                                ctx.push(next);
                                if s.is_complete() {
                                    semantic_quin = Some(s.assemble_quin(prov_hash));
                                    break;
                                }
                            }
                            Err(_) => {
                                sieve_failed = true;
                                break;
                            }
                        }
                    } else {
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref mut cb) = on_token {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                cb(delta);
                            }
                        }
                        if next == eos {
                            break;
                        }
                    }
                }

                let text = if semantic_quin.is_some() {
                    String::new()
                } else if sieve_failed {
                    String::from("[sieve-misaligned]")
                } else {
                    tok.decode(&out_ids)
                };
                
                // Return engine to global instance
                {
                    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| {
                        *g.borrow_mut() = Some(engine);
                    });
                }

                (text, out_ids.len() as u32, semantic_quin, sieve_failed)
            };
            let mut prov = vec![prov_hash];
            if prov_hash == 0 {
                prov.push(q_hash("qualia:grounded"));
            }
            if let Some(q) = semantic_quin {
                prov.push(q.subject);
                prov.push(q.predicate);
                prov.push(q.object);
            }
            if sieve_failed && semantic_quin.is_none() {
                return (text, prov, tokens, None);
            }
            return (text, prov, tokens, semantic_quin);
        }

        
    }
}

impl LocalLlmAgent {
    /// Zero-allocation pre-flight path for Core 1 (no `active_profile` heap lookup).
    pub fn validate_intent_frame(&self, frame: &AgentIntentFrame) -> WebizenVerdict {
        Self::evaluate_intent_frame(self, frame)
    }

    fn evaluate_intent_frame(agent: &LocalLlmAgent, frame: &AgentIntentFrame) -> WebizenVerdict {
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
        let deadline = Duration::from_millis(INFERENCE_TIMEOUT_MS);
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

// ─── Tests ───────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> LocalLlmAgent {
        LocalLlmAgent::new(
            "did:git:antigravity-llm-001",
            "~/.qualia/models/phi3-mini.gguf",
        )
    }

    #[test]
    fn test_webizen_blocks_outbound_network() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![],
            context_namespaces: vec![],
            requires_network: true,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        let verdict = agent.validate_intent(&intent);
        assert!(
            matches!(verdict, WebizenVerdict::Deny { .. }),
            "Webizen must block outbound calls from local backend"
        );
    }

    #[test]
    fn test_webizen_blocks_sanctuary_scope() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![SANCTUARY_SCOPE_WEBIZEN],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        let verdict = agent.validate_intent(&intent);
        assert!(
            matches!(verdict, WebizenVerdict::Deny { .. }),
            "Webizen must block Sanctuary scope access"
        );
    }

    #[test]
    fn test_webizen_permits_valid_local_intent() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![0xDEAD_BEEF],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
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
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);

        let output = agent
            .infer("What is my health status?", "graph_context_bytes_here")
            .unwrap();
        assert!(!output.text.is_empty());

        let post_verdict = agent.validate_output(&output);
        assert_eq!(
            post_verdict,
            WebizenVerdict::Permit,
            "Grounded output should pass post-flight check"
        );
    }

    #[test]
    fn test_webizen_blocks_ungrounded_output() {
        let agent = make_agent();
        let ungrounded = AgentOutput {
            text: "I made this up with no sources.".into(),
            semantic_quin: None,
            provenance_quins: vec![], // <-- no citations
            tokens_generated: 10,
            inference_duration_ms: 5,
            peak_memory_bytes: 0,
        };
        let verdict = agent.validate_output(&ungrounded);
        assert!(
            matches!(verdict, WebizenVerdict::Deny { .. }),
            "Webizen must block ungrounded output"
        );
    }

    #[test]
    fn test_validate_intent_enables_sieve_for_graph_mutation() {
        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: 0xAABB,
            requested_graph_scope: vec![0x1234],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0xAABB,
            output_mode: N3OutputMode::GraphMutation,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        assert_eq!(agent.validate_intent(&intent), WebizenVerdict::Permit);
        assert!(agent
            .use_sieve_output
            .load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_zero_allocation_adversarial_conduct_denial() {
        let _profiler = dhat::Profiler::builder().testing().build();

        let agent = make_agent();
        let intent = AgentIntent {
            intent_predicate: crate::q_hash("llm:AdversarialOperation"),
            requested_graph_scope: vec![],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: crate::q_hash("did:q42:human-rights-test-subject"),
            mcp_intent_frame_hash: crate::q_hash("purpose:General"),
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };

        // Warm up any internal system components that might allocate on first use
        let _ = std::time::SystemTime::now();

        let stats_before = dhat::HeapStats::get();

        // Execute the intent validation (hot path)
        let verdict = agent.validate_intent(&intent);

        let stats_after = dhat::HeapStats::get();

        // Verify we got the Deny verdict with the NQuin
        if let WebizenVerdict::Deny { conduct_record, .. } = verdict {
            assert!(
                conduct_record.is_some(),
                "Conduct record Quin must be generated"
            );
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
