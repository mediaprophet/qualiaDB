// The concrete `LocalLlmAgent` struct, its constructors, LoRA adapter
// management, sieve-lexicon wiring, and DID hashing. The Phase-8 decode path
// lives in `decode.rs`; intent validation in `validation.rs`; the
// `AgentRuntime` impl in `runtime.rs` — all separate impl blocks on this type.

use crate::q_hash;

use super::types::{default_local_modality, AgentBackend};

// ─── LocalLlmAgent ───────────────────────────────────────────────────────────
/// The concrete local inference agent. Uses a mock inference path for now;
/// swap `infer_local_model` for an actual llama.cpp FFI call.
pub struct LocalLlmAgent {
    pub agent_did: String,
    pub backend: AgentBackend,
    pub memory_used_bytes: std::sync::atomic::AtomicU64,
    // NOTE: the four fields below were private in the monolith. They are now
    // `pub(super)` widenings so the decode (`decode.rs`) and runtime
    // (`runtime.rs`) impl blocks can read/write them across the new submodule
    // boundary. Visibility stays crate-internal — the external API is unchanged.
    /// Set by `validate_intent` when `output_mode` requires graph-structured emission.
    pub(super) use_sieve_output: std::sync::atomic::AtomicBool,
    /// Memory-mapped `.q42.lex` sidecar for dynamic sieve masks.
    pub(super) sieve_lex_path: std::sync::Mutex<Option<String>>,
    /// IRI hashes to resolve through the lexicon for Subject / Predicate / Object slots.
    pub(super) sieve_spec: std::sync::Mutex<crate::neuro_symbolic_sieve::SieveLexSpec>,
    /// Optional LoRA adapter manager for zero-copy context-driven neural adaptation.
    /// When set, the prompt is classified into a domain (Medical / Legal / Chemical / …)
    /// and the matching adapter's delta is applied to the embedding hidden state before
    /// the autoregressive decode loop.
    pub(super) lora_manager: std::sync::Mutex<Option<crate::lora::LoRAAdapterManager>>,
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
}
