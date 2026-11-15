// Phase 8 bifurcated-compute decode path for `LocalLlmAgent`.
//
// This is the LIVE LLM decode agent: it loads the GGUF/P64 model, tokenises,
// runs the autoregressive loop through `QTensorEngine`, and streams logit
// summaries from the LLM engine thread to the Webizen Sentinel over wait-free
// SPSC ring buffers. Moved verbatim from the `inference_agent.rs` monolith —
// no logic, control-flow, or signature changes.

#[cfg(not(target_arch = "wasm32"))]
use super::config::effective_inference_timeout_ms;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
use super::config::{DECODE_TOKEN_BUDGET, TEST_TRANSFORMER_LAYER_CAP, TEST_VOCAB_CHUNK_CAP};
use super::control::DecodeControl;
#[cfg(not(target_arch = "wasm32"))]
use super::decode_helpers::get_prefix_cache;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
#[allow(unused_imports)]
use super::decode_helpers::{
    apply_model_helper_stops, build_sieve, drain_tensor_context_inject, embedding_fallback_logits,
    try_accept_topology_draft, TopologyDraftStep,
};
use super::local_agent::LocalLlmAgent;
#[cfg(not(target_arch = "wasm32"))]
use super::sticky_infer;
#[allow(unused_imports)]
use super::types::AgentBackend;
#[cfg(not(target_arch = "wasm32"))]
use crate::inference::runtime::stage::{
    InferenceStageKind, StageStatus, StageTrace, ThinkingEvent, ThinkingTracker,
};
#[allow(unused_imports)]
use crate::{q_hash, NQuin};

impl LocalLlmAgent {
    /// Phase 8: Bifurcated Compute — SPSC Wait-Free Intercept.
    ///
    /// On native targets: loads the GGUF model, tokenises the prompt, and runs an
    /// autoregressive decode loop via `QTensorEngine::dispatch_fused_transformer_block`.
    /// Logit summaries flow from the LLM engine thread to the Webizen Sentinel (this
    /// thread) over a wait-free SPSC ring. The Sentinel may inject `DenyRollback`
    /// for real governance signals; the old IEEE-754 `0x99` mantissa check was
    /// removed (it fired randomly ~1/256 tokens and corrupted the stream).
    ///
    /// On WASM / non-local backends: falls through to the original mock path.
    /// Run local inference, optionally streaming decoded text deltas to `on_token`.
    pub fn infer_local_model_streaming<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, None, on_token, None)
    }

    /// Run local inference with a caller-owned cooperative cancellation token.
    pub fn infer_local_model_controlled<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        control: DecodeControl,
        on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, Some(control), on_token, None)
    }

    /// Runs local inference.
    pub(super) fn infer_local_model(
        &self,
        prompt: &str,
        graph_context: &str,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner::<fn(String)>(prompt, graph_context, None, None, None)
    }

    /// Explicit developer/debug/lab execution through the same numerical backend.
    /// Neither diagnostics nor environment settings select this request mode.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn infer_local_model_with_execution<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        execution: super::native_request::NativeDecodeRequest,
        control: Option<DecodeControl>,
        on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, control, on_token, Some(execution))
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables, unused_mut))]
    fn infer_local_model_inner<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        control: Option<DecodeControl>,
        on_token: Option<F>,
        #[cfg(not(target_arch = "wasm32"))] execution: Option<
            super::native_request::NativeDecodeRequest,
        >,
        #[cfg(target_arch = "wasm32")] _execution: Option<()>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        let prov_hash = crate::q_hash(graph_context);
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

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.infer_native_partition(
                prompt,
                graph_context,
                control,
                on_token,
                prov_hash,
                use_sieve,
                sieve_spec,
                sieve_lex_path,
                execution,
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.infer_wasm_partition(
                prompt,
                graph_context,
                control,
                on_token,
                prov_hash,
                use_sieve,
                sieve_spec,
                sieve_lex_path,
            )
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
type WorkerResult = (String, u32, Option<NQuin>, bool, StageTrace);

#[cfg(not(target_arch = "wasm32"))]
mod messages {
    // Fixed-size types keep the hot-path allocation-free in the ring buffer.
    #[derive(Clone, Copy)]
    pub(super) struct LogitSummary {
        pub(super) _top_id: u32,
        pub(super) anomaly: u8,
    }
    #[derive(Clone)]
    pub(super) enum LlmMsg {
        Logit(LogitSummary),
        Eos,
    }
    #[derive(Clone)]
    pub(super) enum SentMsg {
        DenyRollback,
    }
}
#[cfg(not(target_arch = "wasm32"))]
use messages::{LlmMsg, LogitSummary, SentMsg};
#[cfg(not(target_arch = "wasm32"))]
mod admission;
#[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
mod browser_select;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod cancellation_tests;
#[cfg(not(target_arch = "wasm32"))]
mod cold;
#[cfg(not(target_arch = "wasm32"))]
mod diagnostic;
#[cfg(not(target_arch = "wasm32"))]
mod forced;
#[cfg(not(target_arch = "wasm32"))]
mod model;
#[cfg(not(target_arch = "wasm32"))]
mod prefill;
#[cfg(not(target_arch = "wasm32"))]
mod publication;
#[cfg(not(target_arch = "wasm32"))]
mod qwen35;
#[cfg(not(target_arch = "wasm32"))]
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod select;
#[cfg(not(target_arch = "wasm32"))]
mod speculative;
#[cfg(not(target_arch = "wasm32"))]
mod worker;
