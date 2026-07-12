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
//
// ─── Module layout (library-ized from the former `inference_agent.rs` monolith) ─
//   config          — constants + the effective-timeout helper
//   types           — AgentBackend / AgentIntent / WebizenVerdict / AgentOutput /
//                     AgentError / the AgentRuntime trait / rule-ID constants
//   sticky_infer    — the native sticky 1-thread infer pool (thread_local engine)
//   decode_helpers  — embedding dispatch, topology-draft accept, sieve, prefix KV
//   local_agent     — the LocalLlmAgent struct + constructors + LoRA / sieve wiring
//   decode          — Phase-8 bifurcated-compute decode path (impl LocalLlmAgent)
//   validation      — pre-flight intent validation (impl LocalLlmAgent)
//   runtime         — the AgentRuntime impl for LocalLlmAgent
// The full public surface is re-exported below, so every external path
// (`crate::inference::inference_agent::*` and the `llm_agent` alias) resolves
// exactly as before.

// ─── AgentIntent (re-exported hot-path frame types) ──────────────────────────
pub use crate::modalities::logic::n3_compiler::{
    AgentIntentFrame, N3OutputMode, MAX_CONTEXT_NAMESPACE_SLOTS, MAX_INTENT_SCOPE_SLOTS,
};

#[cfg(not(target_arch = "wasm32"))]
mod sticky_infer;

mod config;
mod decode;
mod decode_helpers;
mod local_agent;
mod runtime;
mod types;
mod validation;

pub use config::*;
pub use local_agent::*;
pub use types::*;

#[cfg(test)]
mod tests;
