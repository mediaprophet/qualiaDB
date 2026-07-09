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

// ─── Sticky 1-thread infer pool (native) ────────────────────────────────────
// Every `infer` previously did `thread::spawn` + `QTensorEngine::new()` which
// rebuilds wgpu pipelines (~seconds) even when the mmap is already resident.
// A size-1 rayon pool keeps a dedicated OS thread whose `thread_local` engine
// survives across jobs; same-path multi-turn / multi-prompt reuses the engine.
#[cfg(not(target_arch = "wasm32"))]
mod sticky_infer {
    use crate::gguf_bridge::QTensorEngine;
    use std::cell::RefCell;
    use std::sync::OnceLock;

    pub struct StickyEngine {
        pub path: String,
        pub engine: QTensorEngine,
    }

    thread_local! {
        static ENGINE: RefCell<Option<StickyEngine>> = const { RefCell::new(None) };
    }

    pub fn pool() -> &'static rayon::ThreadPool {
        static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
        POOL.get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .thread_name(|i| format!("qualia-infer-{i}"))
                .build()
                .expect("qualia sticky infer pool")
        })
    }

    /// Borrow-or-reload the sticky engine for `path`, then run `f`.
    pub fn with_engine<R>(path: &str, mut load: impl FnMut(&mut QTensorEngine), f: impl FnOnce(&mut QTensorEngine) -> R) -> R {
        ENGINE.with(|cell| {
            let mut slot = cell.borrow_mut();
            let reload = match slot.as_ref() {
                Some(s) => s.path != path,
                None => true,
            };
            if reload {
                let mut engine = QTensorEngine::new();
                load(&mut engine);
                *slot = Some(StickyEngine {
                    path: path.to_string(),
                    engine,
                });
            }
            f(&mut slot.as_mut().expect("sticky engine just loaded").engine)
        })
    }
}

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
/// MC2b harness iteration: CPU SDPA decode is very slow in wasm; trim budget until Option B.
#[cfg(all(not(test), target_arch = "wasm32"))]
const DECODE_TOKEN_BUDGET: u32 = 32;
// Codex P0: default per-turn decode cap. Was MAX_OUTPUT_TOKENS (2048) → at ~3 tok/s a no-EOS reply
// ran ~11 min and the app looked frozen. 256 keeps a turn bounded; MAX_OUTPUT_TOKENS stays the
// absolute ceiling and the cooperative deadline (INFERENCE_TIMEOUT_MS, checked INSIDE the decode
// loop) bounds wall-clock time independently.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
const DECODE_TOKEN_BUDGET: u32 = 256;

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
    engine: &crate::gguf_bridge::QTensorEngine,
    idx: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
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

/// Decode fallback when full hidden-state projection is unavailable: real GGUF
/// embedding lookup when mmap/index exist, otherwise deterministic pseudo logits.
fn embedding_fallback_logits(
    engine: &crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::gguf_sharder::GgufTensorIndex>,
    lora_adapter: Option<&crate::lora::LoRAAdapter>,
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    if let (Some(idx), Some(mmap)) = (tensor_idx, engine.gguf_mmap.as_deref()) {
        if let Some(adapter) = lora_adapter {
            lora_embedding_forward(engine, idx, mmap, token_id, emb_dim, emb_buf, wt, adapter)
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                cpu_embedding_forward(engine, idx, mmap, token_id, emb_dim, emb_buf, wt)
            }
            #[cfg(target_arch = "wasm32")]
            {
                let n =
                    idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
                if n > 0 {
                    engine.dispatch_fused_transformer_block(wt, &emb_buf[..n])
                } else {
                    pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
                }
            }
        }
    } else {
        pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
    }
}

fn push_decode_stream_delta(
    tok: &crate::gguf_sharder::GgufTokenizer,
    out_ids: &[u32],
    streamed_len: &mut usize,
    stream_tx: Option<&std::sync::mpsc::SyncSender<String>>,
) {
    let Some(tx) = stream_tx else {
        return;
    };
    let full = tok.decode(out_ids);
    if full.len() <= *streamed_len {
        return;
    }
    let delta = full[*streamed_len..].to_string();
    *streamed_len = full.len();
    let _ = tx.send(delta);
}

/// Load sibling `.q42.cbor-ld` (if present) and merge its stop-token set into `tok`.
/// Also tries preferring a sibling `.p64` path's helper when `model_path` is a GGUF
/// that has already been converted beside it.
fn apply_model_helper_stops(model_path: &str, tok: &mut crate::gguf_sharder::GgufTokenizer) {
    let path = std::path::Path::new(model_path);
    // Direct: path is already .p64 (or any path with a sibling helper).
    if let Ok(Some(h)) = crate::model_helper::ModelHelper::load_beside_p64(path) {
        h.apply_stops_to_tokenizer(tok);
        return;
    }
    // Prefer converted sibling: foo.gguf → foo.p64 + foo.q42.cbor-ld
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
    {
        let p64 = path.with_extension("p64");
        if let Ok(Some(h)) = crate::model_helper::ModelHelper::load_beside_p64(&p64) {
            h.apply_stops_to_tokenizer(tok);
        }
    }
}

/// Outcome of one topology-draft accept attempt (B3.1d).
enum TopologyDraftStep {
    NoDraft,
    Denied,
    AcceptedFull,
    Fallthrough,
    Stop {
        sieve_failed: bool,
        semantic_quin: Option<NQuin>,
    },
}

fn drain_tensor_context_inject() {
    while let Some(inject) = crate::compute_universe::pop_tensor_context() {
        if let Some(tensor) = crate::tensor::resident_substrate::global_resident_substrate()
            .tensor_at(inject.tensor_index)
        {
            crate::compute_universe::publish_query_tensor(tensor, inject.subject_hash);
        }
    }
}

fn try_accept_topology_draft(
    engine: &mut crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::gguf_sharder::GgufTensorIndex>,
    draft_mapper: &crate::topology_draft::TopologyDraftMapper<'_>,
    ctx: &mut Vec<u32>,
    emb_dim: usize,
    emb_buf: &mut [f32],
    scratch_a: &mut [f32],
    scratch_b: &mut [f32],
    out_ids: &mut Vec<u32>,
    sieve: &mut Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    prov_hash: u64,
    eos: u32,
    tok: &crate::gguf_sharder::GgufTokenizer,
    streamed_len: &mut usize,
    stream_tx: Option<&std::sync::mpsc::SyncSender<String>>,
    mut on_token: Option<&mut dyn FnMut(String)>,
) -> TopologyDraftStep {
    let idx = match tensor_idx {
        Some(i) => i,
        None => return TopologyDraftStep::NoDraft,
    };
    let draft = match crate::compute_universe::pop_topology_draft() {
        Some(d) => d,
        None => return TopologyDraftStep::NoDraft,
    };

    let mut mapped = draft;
    for i in 0..mapped.draft_len as usize {
        mapped.draft_ids[i] = draft_mapper.concept_to_token_id(mapped.concept_hashes[i]);
    }

    if !crate::compute_universe::sentinel_allows_topology_draft(&mapped) {
        return TopologyDraftStep::Denied;
    }

    let accepted = engine.verify_topology_draft_batch(
        idx,
        ctx,
        &mapped,
        emb_dim,
        &mut emb_buf[..emb_dim],
        scratch_a,
        scratch_b,
        TEST_TRANSFORMER_LAYER_CAP,
        TEST_VOCAB_CHUNK_CAP,
    );
    crate::gpu_context::record_draft_acceptance(accepted, mapped.draft_len as u32);

    if accepted == 0 {
        return TopologyDraftStep::Fallthrough;
    }

    let mut sieve_failed = false;
    let mut semantic_quin = None;
    for i in 0..accepted as usize {
        let id = mapped.draft_ids[i];
        out_ids.push(id);
        if let Some(ref mut s) = sieve {
            if s.apply_token(id).is_err() {
                sieve_failed = true;
                break;
            }
            if s.is_complete() {
                semantic_quin = Some(s.assemble_quin(prov_hash));
                break;
            }
        } else if let Some(tx) = stream_tx {
            push_decode_stream_delta(tok, out_ids, streamed_len, Some(tx));
        } else if let Some(ref mut cb) = on_token {
            let full = tok.decode(out_ids);
            if full.len() > *streamed_len {
                let delta = full[*streamed_len..].to_string();
                *streamed_len = full.len();
                cb(delta);
            }
        }
        if tok.is_stop_token(id) || *ctx.last().unwrap_or(&eos) == eos {
            break;
        }
    }

    if sieve_failed || semantic_quin.is_some() {
        return TopologyDraftStep::Stop {
            sieve_failed,
            semantic_quin,
        };
    }
    if accepted == mapped.draft_len as u32 {
        TopologyDraftStep::AcceptedFull
    } else {
        TopologyDraftStep::Fallthrough
    }
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
        if crate::q42_volume::is_unified_volume(p).unwrap_or(false) {
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

static PREFIX_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<u64, Box<[f32]>>>,
> = std::sync::OnceLock::new();

fn get_prefix_cache() -> &'static std::sync::Mutex<std::collections::HashMap<u64, Box<[f32]>>> {
    PREFIX_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
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
        n_in: usize,
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
        guard
            .as_ref()
            .and_then(|m| m.active())
            .map(|a| a.context_type)
    }

    /// Wire the `.q42.lex` sidecar used to populate FSM sieve masks at inference time.
    pub fn configure_sieve_lex(&self, path: impl Into<String>) {
        *self
            .sieve_lex_path
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(path.into());
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
    pub fn infer_local_model_streaming<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, on_token)
    }

    fn infer_local_model(
        &self,
        prompt: &str,
        graph_context: &str,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner::<fn(String)>(prompt, graph_context, None)
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables, unused_mut))]
    fn infer_local_model_inner<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        mut on_token: Option<F>,
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
            self.sieve_lex_path
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            None
        };

        // ── Native GPU path ─────────────────────────────────────────────────
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::gguf_bridge::{QTensor, QTensorEngine};
            use crate::gguf_sharder::GgufTokenizer;
            use rtrb::RingBuffer;

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
                    let (ctx, conf, _switched) =
                        mgr.auto_switch(&prompt_owned, mgr.detector.confidence_threshold);
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

            // Sticky pool thread owns the engine (thread_local); caller runs Sentinel.
            let (done_tx, done_rx) =
                std::sync::mpsc::sync_channel::<(String, u32, Option<NQuin>, bool)>(1);

            // ── LLM engine job (sticky 1-thread pool) ────────────────────────
            sticky_infer::pool().spawn(move || {
                let result = sticky_infer::with_engine(
                    model_path.as_str(),
                    |engine| {
                        // Load only on cache miss / path change.
                        if let Some(mmap) =
                            crate::resident_model::resident_mmap_for_path(model_path.as_str())
                        {
                            let is_p64 = crate::p64_weight::has_p64_magic(&mmap[..]);
                            let adopted = if is_p64 {
                                engine.adopt_resident_p64_mmap(mmap).is_ok()
                            } else {
                                engine.adopt_resident_mmap(mmap).is_ok()
                            };
                            if !adopted {
                                engine.load_model(&model_path);
                            }
                        } else {
                            engine.load_model(&model_path);
                        }
                    },
                    |engine: &mut QTensorEngine| {
                // Initialize Tokio runtime for the sticky thread (once per job; cheap if already warm).
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap_or_else(|e| {
                        panic!("Failed to create Tokio runtime for LLM thread: {}", e)
                    });
                let _rt_guard = rt.enter();

                // A0 phase timing (D17/D22): once-per-phase, off the per-token hot path.
                let t_phase = std::time::Instant::now();

                // Thermal-eviction WAL is a native-only file mmap (`memmap2`); on
                // wasm there is no mmap'd-file WAL, so the telemetry is simply absent.
                #[cfg(not(target_arch = "wasm32"))]
                let mut thermal_wal_opt = {
                    let wal_path = std::env::var("QUALIA_DATA_DIR")
                        .map(|p| std::path::PathBuf::from(p).join("thermal_eviction.wal"))
                        .unwrap_or_else(|_| std::env::temp_dir().join("thermal_eviction.wal"));
                    crate::inference::thermal_wal::ThermalWal::open(&wal_path, 1024).ok()
                };

                let lora_adapter = lora_for_thread;
                let sieve_spec = sieve_spec;
                let sieve_lex_path = sieve_lex_path;

                // Tokenizer + tensor index come from the matching on-disk
                // format. P64 carries a Q42T tokenizer section and a manifest;
                // GGUF carries both in its own metadata.
                let is_p64_mmap = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::p64_weight::has_p64_magic(&m[..]))
                    .unwrap_or(false);
                // Prefer engine-cached index from adopt (zero re-CRC); else parse once.
                let p64_index = engine.p64_index.clone().or_else(|| {
                    if is_p64_mmap {
                        engine
                            .gguf_mmap
                            .as_ref()
                            .and_then(|m| crate::p64_weight::P64TensorIndex::from_p64(m).ok())
                    } else {
                        None
                    }
                });
                let mut tok = if let (Some(qi), Some(m)) =
                    (p64_index.as_ref(), engine.gguf_mmap.as_ref())
                {
                    GgufTokenizer::from_p64_section(qi.tokenizer_bytes(m)).unwrap_or_default()
                } else {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .map(|m| GgufTokenizer::from_gguf(m))
                        .unwrap_or_default()
                };
                // Sibling `.q42.cbor-ld` helper (convert-time stop set / chat metadata).
                apply_model_helper_stops(&model_path, &mut tok);

                let tensor_idx = engine.tensor_index_cache.clone().or_else(|| {
                    if let Some(qi) = p64_index {
                        Some(qi.to_gguf_index())
                    } else {
                        engine
                            .gguf_mmap
                            .as_ref()
                            .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m))
                    }
                });

                let mut ctx = tok.encode_chat_prompt(&prompt_owned);
                // Keep `eos` for draft/topology APIs that still take a single id; decode
                // termination uses the full stop set (eos + chat end-of-turn specials).
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
                let mut prefix_cached = false;
                if prov_hash != 0 {
                    if let Ok(cache) = get_prefix_cache().lock() {
                        if let Some(cached_kv) = cache.get(&prov_hash) {
                            engine.set_kv_cache_cpu(cached_kv);
                            prefix_cached = true;
                        }
                    }
                }

                if !prefix_cached {
                    engine.reset_kv_cache();
                }

                // Phase boundary: load (mmap/adopt + tokenizer + tensor index + setup) done.
                crate::llm_bench::record_load_ns(t_phase.elapsed().as_nanos() as u64);
                let t_prefill = std::time::Instant::now();

                // Chunked prefill: populate KV for prompt tokens [0, prompt_len-1).
                let prompt_len = ctx.len();
                crate::tensor::kv_provenance::rebuild_prompt_provenance(
                    prompt_len as u32,
                    crate::tensor::resident_substrate::global_resident_substrate().node_count(),
                    0,
                );
                let draft_mapper = crate::topology_draft::TopologyDraftMapper::new(&tok);
                if !prefix_cached {
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
                                if !engine.dispatch_prefill_chunk(
                                    idx,
                                    &mut prefill_chunk[..batch_elems],
                                    emb_dim,
                                    n as u32,
                                    pos as u32,
                                    &mut scratch_a,
                                    &mut scratch_b,
                                    TEST_TRANSFORMER_LAYER_CAP,
                                ) {
                                    crate::gguf_bridge::wlog(&format!(
                                        "[llm] PREFILL chunk FAILED pos={pos} n={n}"
                                    ));
                                }
                                pos += n;
                            }
                        }
                    }

                    if prov_hash != 0 {
                        if let Some(cpu_kv) = engine.get_kv_cache_cpu() {
                            if let Ok(mut cache) = get_prefix_cache().lock() {
                                cache.insert(prov_hash, cpu_kv.into());
                            }
                        }
                    }
                }

                // Phase boundary: prefill done. Decode phase begins below.
                crate::llm_bench::record_prefill(
                    t_prefill.elapsed().as_nanos() as u64,
                    prompt_len.saturating_sub(1) as u64,
                );

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
                    // Benchmark override (A0): a fixed decode count for stable tok/s; 0 = default.
                    let ov = crate::llm_bench::decode_budget_override();
                    if ov > 0 {
                        ov as usize
                    } else {
                        DECODE_TOKEN_BUDGET as usize
                    }
                };

                #[cfg(not(target_arch = "wasm32"))]
                crate::compute_universe::start_tensor_search_producer();
                crate::compute_universe::publish_query_tensor(
                    crate::tensor::Tensor10D::default(),
                    0,
                );

                let t_decode = std::time::Instant::now();
                // A1a: GPU top-1 decode path toggle (default-on; QUALIA_LLM_GPU_TOPK / set_gpu_topk).
                let gpu_topk_enabled = crate::llm_bench::gpu_topk_enabled();
                // W2: exact CPU sampler. `None` ⇒ greedy argmax (pre-W2 byte-identical path). When
                // active, decode uses the legacy forward (leaves the normed hidden in `emb_buf`),
                // reads back the FULL logit vector, and runs the penalty/temp/top-k/top-p chain.
                #[cfg(not(target_arch = "wasm32"))]
                let mut sampler =
                    crate::llm_bench::sampler_config().map(crate::sampler::SamplerState::new);
                #[cfg(target_arch = "wasm32")]
                let mut sampler: Option<crate::sampler::SamplerState> = None;
                let mut sampler_logits: Vec<f32> = Vec::new();
                // Decode-profiler (gated): one-shot empty submit→wait baseline on the SAME device, so
                // the bench can separate per-token fence latency from real kernel compute time.
                #[cfg(not(target_arch = "wasm32"))]
                if std::env::var("QUALIA_LLM_PROFILE_DECODE").is_ok() {
                    let n = 64u32;
                    crate::llm_bench::record_empty_rt(
                        engine.bench_empty_submit_roundtrip(n),
                        n as u64,
                    );
                }

                // Phase 6: Initialize Semantic Chunking State
                let mut current_page_id = 100u64; // Starting mock page ID
                let chunk_policy = crate::q42::q42_kvp::Q42ChunkPolicy {
                    max_tokens: 128,
                    semantic_shift_threshold: 0.0,
                    discourse_boundary_weight: 0.0,
                    attention_phase_weight: 0.0,
                    max_entropy_drop: -2.0, // A threshold that will trigger when top1 and top2 are close
                    thermal_pressure_bias: 0.0,
                    reserved: [0; 40],
                };

                for step in 0..gen_budget {
                    crate::gpu_context::record_llm_decode_step();

                    // Codex P0 — cooperative deadline: break BEFORE the wall-clock timeout instead of
                    // the old post-hoc check in infer() that let a no-EOS run continue for minutes.
                    // t_decode starts at decode entry (post-prefill); INFERENCE_TIMEOUT_MS bounds the
                    // generation phase so the call never appears frozen.
                    if t_decode.elapsed().as_millis() as u64 >= INFERENCE_TIMEOUT_MS {
                        break;
                    }

                    // W7 — periodic GPU thermal check during sustained decode: recommends a TDP cap,
                    // and (when the auto-cap user option is on + a real NVML governor is present)
                    // applies it under sustained Critical, restoring on cool-down. Cheap NVML read
                    // every 32 tokens; no-op without the `nvml` feature or an NVIDIA card.
                    #[cfg(not(target_arch = "wasm32"))]
                    if step > 0 && step % 32 == 0 {
                        crate::inference::thermal_telemetry::thermal_tick();
                    }

                    // W6a — prompt-lookup speculative decode (default OFF, exact-output). Draft the
                    // next few tokens by n-gram lookup, verify them in ONE batched forward, and emit
                    // the longest greedily-agreeing prefix + the model's own correction token. Only
                    // when no sieve/sampler/route is active and the full model runs (unit-test layer
                    // cap keeps per-layer semantics). Bit-identical to greedy (a6a). Falls through to
                    // the normal path on any ineligibility.
                    #[cfg(not(target_arch = "wasm32"))]
                    if crate::llm_bench::spec_decode_enabled()
                        && sieve.is_none()
                        && sampler.is_none()
                        && TEST_TRANSFORMER_LAYER_CAP == 0
                    {
                        if let Some(idx) = tensor_idx.as_ref() {
                            let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                            let draft = crate::prompt_lookup::propose(
                                &ctx,
                                crate::prompt_lookup::MAX_DRAFT,
                            );
                            if draft.len > 0 {
                                // inputs = [cur, d0..d_{m-1}] at positions [pos, pos+m], pos = ctx.len()-1.
                                let mut inputs = Vec::with_capacity(draft.len + 1);
                                inputs.push(cur);
                                inputs.extend_from_slice(draft.as_slice());
                                let pos = ctx.len().saturating_sub(1) as u32;
                                let mut amax: Vec<u32> = Vec::new();
                                let mut alog: Vec<f32> = Vec::new();
                                if engine
                                    .verify_draft_batch(idx, &inputs, pos, &mut amax, &mut alog)
                                    .is_some()
                                    && amax.len() == inputs.len()
                                {
                                    // Accept the longest prefix where argmax[i] == draft[i]; then emit
                                    // d0..d_{k-1} (accepted) + argmax[k] (correction/bonus) = k+1 tokens.
                                    let m = draft.len;
                                    let mut k = 0usize;
                                    while k < m && amax[k] == draft.tokens[k] {
                                        k += 1;
                                    }
                                    crate::llm_bench::record_spec_step(m as u64, k as u64);
                                    let mut stop = false;
                                    for i in 0..=k {
                                        let (tokn, logv) = if i < k {
                                            (draft.tokens[i], alog[i])
                                        } else {
                                            (amax[k], alog[k])
                                        };
                                        // Sentinel: anomaly flag from the top logit's IEEE-754 bytes.
                                        let anomaly = if logv.to_le_bytes()[0] == 0x99 {
                                            0x99u8
                                        } else {
                                            0x01u8
                                        };
                                        let _ = lp.push(LlmMsg::Logit(LogitSummary {
                                            _top_id: tokn,
                                            anomaly,
                                        }));
                                        let next = tokn % vlen;
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
                                        if tok.is_stop_token(next) || out_ids.len() >= gen_budget {
                                            stop = true;
                                            break;
                                        }
                                    }
                                    if stop {
                                        break;
                                    }
                                    continue; // skip the normal single-token path this step
                                }
                            }
                        }
                    }

                    let draft_step = try_accept_topology_draft(
                        engine,
                        tensor_idx.as_ref(),
                        &draft_mapper,
                        &mut ctx,
                        emb_dim,
                        &mut emb_buf,
                        &mut scratch_a,
                        &mut scratch_b,
                        &mut out_ids,
                        &mut sieve,
                        prov_hash,
                        eos,
                        &tok,
                        &mut streamed_len,
                        stream_tx_thread.as_ref(),
                        None,
                    );
                    match draft_step {
                        TopologyDraftStep::AcceptedFull => continue,
                        TopologyDraftStep::Stop {
                            sieve_failed: sf,
                            semantic_quin: sq,
                        } => {
                            sieve_failed = sf;
                            semantic_quin = sq;
                            break;
                        }
                        _ => {}
                    }

                    drain_tensor_context_inject();
                    let _attention_mask = crate::compute_universe::attention_route_mask();
                    // Check ControlStream for a DenyRollback injected in the previous step.
                    let mut rollback = cc.pop().is_ok();
                    if matches!(draft_step, TopologyDraftStep::Denied) {
                        rollback = true;
                    }

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                    crate::compute_universe::publish_decode_hint(cur, step as u32);

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
                            let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                            // Resident-token fast path: the WHOLE forward (32 layers + output norm
                            // + logits top-1) in ONE submit with ONE fence. `Some` means the token
                            // was produced and the KV cache was written; `None` falls through to
                            // the legacy per-layer path unchanged (non-sieve, full-depth only —
                            // the unit-test 2-layer cap keeps its per-layer semantics).
                            #[cfg(not(target_arch = "wasm32"))]
                            let resident_hit = if gpu_topk_enabled
                                && sieve_mask.is_none()
                                && sampler.is_none()
                                && TEST_TRANSFORMER_LAYER_CAP == 0
                            {
                                let t_res = std::time::Instant::now();
                                let hit = engine.dispatch_token_forward_resident(
                                    idx,
                                    &emb_buf[..emb_dim],
                                    token_idx,
                                );
                                if hit.is_some() {
                                    crate::llm_bench::add_decode_forward_ns(
                                        t_res.elapsed().as_nanos() as u64,
                                    );
                                    crate::llm_bench::record_resident_hit();
                                } else {
                                    crate::llm_bench::record_resident_fallback();
                                }
                                hit
                            } else {
                                None
                            };
                            #[cfg(target_arch = "wasm32")]
                            let resident_hit: Option<
                                crate::gguf_bridge::StreamingArgmaxResult,
                            > = None;

                            if resident_hit.is_none() {
                                // Decode-profiler: time the 32-layer forward (legacy path).
                                let t_fwd = std::time::Instant::now();
                                let _layers = engine.dispatch_transformer_forward(
                                    idx,
                                    &mut emb_buf[..emb_dim],
                                    emb_dim,
                                    &mut scratch_a,
                                    &mut scratch_b,
                                    token_idx,
                                    TEST_TRANSFORMER_LAYER_CAP,
                                );
                                // Final output_norm before the vocab projection — REQUIRED on all
                                // targets.
                                let _ = engine.apply_output_norm_inplace(
                                    idx,
                                    &mut emb_buf[..emb_dim],
                                    emb_dim,
                                );
                                crate::llm_bench::add_decode_forward_ns(
                                    t_fwd.elapsed().as_nanos() as u64
                                );
                            }
                            // Decode-profiler: time the output projection (argmax / top-k).
                            let t_out = std::time::Instant::now();
                            // W2: exact sampling — read back the FULL logit vector for this token and
                            // run the CPU chain. Only when a non-greedy sampler is installed; on any
                            // readback failure, `sampled` stays None and the greedy paths below run
                            // (never a silent hang). The legacy forward above left the normed hidden
                            // in `emb_buf`, so the projection input is correct.
                            #[cfg(not(target_arch = "wasm32"))]
                            let sampled: Option<(usize, f32)> = if let Some(s) = sampler.as_mut() {
                                let vocab = idx
                                    .logits_projection_info()
                                    .map(|i| QTensorEngine::matmul_dims(i).1)
                                    .unwrap_or(0);
                                if vocab > 0 {
                                    if sampler_logits.len() < vocab {
                                        sampler_logits.resize(vocab, 0.0);
                                    }
                                    // Existing chunked projection; `written == vocab` iff it produced
                                    // REAL logits (else it degraded to copying hidden → not sampleable,
                                    // so we fall through to the greedy paths rather than sample garbage).
                                    let written = engine.dispatch_output_logits_into(
                                        idx,
                                        &emb_buf[..emb_dim],
                                        emb_dim,
                                        &mut sampler_logits[..vocab],
                                    );
                                    if written == vocab {
                                        let tok = s.sample(&mut sampler_logits[..vocab], &ctx);
                                        Some((tok as usize, sampler_logits[tok as usize]))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            #[cfg(target_arch = "wasm32")]
                            let sampled: Option<(usize, f32)> = None;

                            // A1a: GPU top-1 path (additive, default-on; non-sieve only in v1).
                            // Returns the argmax token via the on-GPU block reduction; falls
                            // through to the existing argmax path if disabled or on any failure — the
                            // working path is never bypassed. Skipped when sampling produced a token.
                            let topk_hit = if sampled.is_some() {
                                None
                            } else if resident_hit.is_some() {
                                resident_hit
                            } else if gpu_topk_enabled && sieve_mask.is_none() {
                                engine.dispatch_output_top1_chunked(
                                    idx,
                                    &emb_buf[..emb_dim],
                                    emb_dim,
                                )
                            } else {
                                None
                            };
                            let out_sel = if let Some(sel) = sampled {
                                crate::llm_bench::record_sampled_token();
                                sel
                            } else if let Some(item) = topk_hit {
                                crate::llm_bench::record_topk_hit();
                                (item.best_token_id as usize, item.max_logit)
                            } else if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                                idx,
                                &emb_buf[..emb_dim],
                                emb_dim,
                                &mut scratch_a[..],
                                TEST_VOCAB_CHUNK_CAP,
                                sieve_mask,
                            ) {
                                crate::llm_bench::record_argmax_fallback();
                                if argmax.max_logit > f32::NEG_INFINITY {
                                    (argmax.best_token_id as usize, argmax.max_logit)
                                } else {
                                    sieve_failed = true;
                                    (0usize, f32::NEG_INFINITY)
                                }
                            } else {
                                let mut top1_v = f32::NEG_INFINITY;
                                let mut top1_i = 0usize;
                                let mut top2_v = f32::NEG_INFINITY;
                                for (i, &v) in emb_buf[..emb_dim].iter().enumerate() {
                                    if v > top1_v {
                                        top2_v = top1_v;
                                        top1_v = v;
                                        top1_i = i;
                                    } else if v > top2_v {
                                        top2_v = v;
                                    }
                                }

                                // Phase 6: Semantic Chunking Entropy Calculation
                                let fast_entropy = -(top1_v - top2_v);
                                if fast_entropy > chunk_policy.max_entropy_drop {
                                    current_page_id += 1;

                                    #[cfg(not(target_arch = "wasm32"))]
                                    if let Some(ref mut wal) = thermal_wal_opt {
                                        let record =
                                            crate::inference::thermal_wal::ThermalEvictionRecord {
                                                timestamp_ms: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_millis()
                                                    as u64,
                                                page_id: (current_page_id - 1) as u32,
                                                fast_entropy,
                                                top1_v,
                                                top2_v,
                                                reserved: [0; 8],
                                            };
                                        wal.append(record);
                                    }

                                    if std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
                                        eprintln!(
                                            "[NEW CHUNK] page_id={} entropy={:.3}",
                                            current_page_id, fast_entropy
                                        );
                                        eprintln!(
                                            "[THERMAL EVICT] Hard eviction logged for page_id={}",
                                            current_page_id - 1
                                        );
                                    }
                                }

                                // Update KV provenance for this new token
                                let token_idx = ctx.len() as u32;
                                crate::tensor::kv_provenance::record_kv_provenance(
                                    token_idx,
                                    token_idx,
                                    current_page_id,
                                );

                                (top1_i, top1_v)
                            };
                            crate::llm_bench::add_decode_output_ns(
                                t_out.elapsed().as_nanos() as u64
                            );
                            out_sel
                        } else {
                            (0usize, 0.0)
                        }
                    } else {
                        let wt = QTensor::new(vec![emb_dim, emb_dim], 0, true);
                        let logits = embedding_fallback_logits(
                            &engine,
                            tensor_idx.as_ref(),
                            lora_adapter.as_ref(),
                            cur,
                            emb_dim,
                            &mut emb_buf[..],
                            &wt,
                        );
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

                    // #48 diagnostic: reveal eos vs argmax for the first step (gated, native).
                    if step == 0 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
                        eprintln!(
                            "[decode-dbg] step0 eos={} vlen={} prompt_last={} top_i={} top_v={} decoded={:?}",
                            eos,
                            vlen,
                            cur,
                            top_i,
                            top_v,
                            tok.decode(&[top_i as u32])
                        );
                        if let Some(idx) = tensor_idx.as_ref() {
                            if let Some(top5) = engine.dispatch_output_topk_chunked(
                                idx,
                                &emb_buf[..emb_dim],
                                emb_dim,
                                5,
                            ) {
                                for it in &top5 {
                                    eprintln!(
                                        "[top5] id={} logit={:.3} dec={:?}",
                                        it.token_id,
                                        it.logit,
                                        tok.decode(&[it.token_id])
                                    );
                                }
                            }
                        }
                    }

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
                        // Stop on eos AND chat end-of-turn (e.g. <|eot_id|>, <|im_end|>).
                        if tok.is_stop_token(next) {
                            break;
                        }
                    }
                }

                // Phase boundary: decode loop complete.
                crate::llm_bench::record_decode(
                    t_decode.elapsed().as_nanos() as u64,
                    out_ids.len() as u64,
                );

                let _ = lp.push(LlmMsg::Eos);
                let text = if semantic_quin.is_some() {
                    String::new()
                } else if sieve_failed {
                    String::from("[sieve-misaligned]")
                } else {
                    tok.decode(&out_ids)
                };
                (text, out_ids.len() as u32, semantic_quin, sieve_failed)
                    }, // sticky_infer::with_engine f
                ); // sticky_infer::with_engine
                let _ = done_tx.send(result);
            }); // sticky pool spawn

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
                done_rx.recv().unwrap_or_else(|_| (String::new(), 0, None, false));
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
            use crate::gguf_bridge::QTensor;
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

            // ── WASM Extension Bus Offloading ────────────────────────────────
            if crate::extension_bus::wasm_bus::is_connected() {
                if let Some(cb) = on_token {
                    let _ = crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, cb);
                } else {
                    let _ =
                        crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, |_| {});
                }
                return (String::new(), vec![prov_hash], 0, None);
            }
            let prompt_owned = prompt.to_string();

            // ── LoRA context detection (before thread spawn) ─────────────────
            // Detect the prompt domain and pre-load the matching LoRA adapter.
            // The pre-computed delta vectors are cloned into the inference thread
            // as fixed-size heap data — one allocation per infer call, not per token.
            #[allow(unused_variables)]
            let lora_active_adapter: Option<crate::lora::LoRAAdapter> = {
                let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut mgr) = *guard {
                    let (ctx, conf, _switched) =
                        mgr.auto_switch(&prompt_owned, mgr.detector.confidence_threshold);
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
                    let engine_guard =
                        crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| g.borrow_mut().take());
                    engine_guard.expect(
                        "WASM WebGPU engine not initialized. Call initialize_webgpu_engine first.",
                    )
                };

                let is_p64_mmap = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::p64_weight::has_p64_magic(&m[..]))
                    .unwrap_or(false);
                let p64_index = if is_p64_mmap {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .and_then(|m| crate::p64_weight::P64TensorIndex::from_p64(m).ok())
                } else {
                    None
                };
                let mut tok = if let (Some(qi), Some(m)) =
                    (p64_index.as_ref(), engine.gguf_mmap.as_ref())
                {
                    GgufTokenizer::from_p64_section(qi.tokenizer_bytes(m)).unwrap_or_default()
                } else {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .map(|m| GgufTokenizer::from_gguf(m))
                        .unwrap_or_default()
                };
                apply_model_helper_stops(&model_path, &mut tok);

                let tensor_idx = if let Some(qi) = p64_index {
                    Some(qi.to_gguf_index())
                } else {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m))
                };

                let mut ctx = tok.encode_chat_prompt(&prompt_owned);
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
                crate::tensor::kv_provenance::rebuild_prompt_provenance(
                    prompt_len as u32,
                    crate::tensor::resident_substrate::global_resident_substrate().node_count(),
                    0,
                );
                let draft_mapper = crate::topology_draft::TopologyDraftMapper::new(&tok);
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
                            if !engine.dispatch_prefill_chunk(
                                idx,
                                &mut prefill_chunk[..batch_elems],
                                emb_dim,
                                n as u32,
                                pos as u32,
                                &mut scratch_a,
                                &mut scratch_b,
                                TEST_TRANSFORMER_LAYER_CAP,
                            ) {
                                crate::gguf_bridge::wlog(&format!(
                                    "[llm] PREFILL chunk FAILED pos={pos} n={n}"
                                ));
                            }
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

                crate::compute_universe::start_tensor_search_producer();
                crate::compute_universe::publish_query_tensor(
                    crate::tensor::Tensor10D::default(),
                    0,
                );

                for step in 0..gen_budget {
                    crate::gpu_context::record_llm_decode_step();

                    let on_token_sink = on_token.as_mut().map(|cb| cb as &mut dyn FnMut(String));
                    let draft_step = try_accept_topology_draft(
                        engine,
                        tensor_idx.as_ref(),
                        &draft_mapper,
                        &mut ctx,
                        emb_dim,
                        &mut emb_buf,
                        &mut scratch_a,
                        &mut scratch_b,
                        &mut out_ids,
                        &mut sieve,
                        prov_hash,
                        eos,
                        &tok,
                        &mut streamed_len,
                        None,
                        on_token_sink,
                    );
                    match draft_step {
                        TopologyDraftStep::AcceptedFull => continue,
                        TopologyDraftStep::Stop {
                            sieve_failed: sf,
                            semantic_quin: sq,
                        } => {
                            sieve_failed = sf;
                            semantic_quin = sq;
                            break;
                        }
                        _ => {}
                    }

                    drain_tensor_context_inject();
                    let _attention_mask = crate::compute_universe::attention_route_mask();

                    let rollback_val = rollback;
                    rollback = false;
                    let mut rollback = rollback_val;
                    if matches!(draft_step, TopologyDraftStep::Denied) {
                        rollback = true;
                    }

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                    crate::compute_universe::publish_decode_hint(cur, step as u32);

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
                            // Final output_norm before the vocab projection — REQUIRED on all targets.
                            let _ = engine.apply_output_norm_inplace(
                                idx,
                                &mut emb_buf[..emb_dim],
                                emb_dim,
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
                        let logits = embedding_fallback_logits(
                            &engine,
                            tensor_idx.as_ref(),
                            lora_adapter.as_ref(),
                            cur,
                            emb_dim,
                            &mut emb_buf[..],
                            &wt,
                        );
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
                        if tok.is_stop_token(next) {
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
