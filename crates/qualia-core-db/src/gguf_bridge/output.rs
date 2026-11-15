//! Output projection: chunked argmax + GPU top-k, output RMSNorm
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Chunked vocabulary projection with streaming argmax (zero heap, stack chunk buffer only).
    /// `max_chunks`: `0` sweeps the full vocabulary; otherwise caps chunk iterations (tests).
    pub fn dispatch_output_argmax_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.gemma4_required && !self.gemma4_ready(index) {
            return None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.qwen35_gguf_required && !self.dense_qwen_ready(index) {
            return None;
        }
        if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            #[cfg(not(target_arch = "wasm32"))]
            return self.qwen35_argmax(index, hidden, emb_dim, max_chunks, sieve_mask);
            #[cfg(target_arch = "wasm32")]
            return None;
        }
        etalon_inference::projections::ProjectionEngine { host: self }
            .dispatch_output_argmax_chunked(
                index,
                hidden,
                emb_dim,
                chunk_logits,
                max_chunks,
                sieve_mask,
            )
            .map(|r| {
                let softcap = index.hyperparams.logit_softcap;
                let max_logit = if softcap > 0.0 {
                    (r.max_logit / softcap).tanh() * softcap
                } else {
                    r.max_logit
                };
                StreamingArgmaxResult {
                    best_token_id: r.best_token_id,
                    max_logit,
                }
            })
    }

    /// A1a: create the persistent GPU top-k pipeline + small candidate/staging buffers (once).
    #[cfg(feature = "gpu-runtime")]
    pub(crate) fn init_output_topk(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let cache = self.native_pipeline_cache_ref();
        #[cfg(target_arch = "wasm32")]
        let cache = None;
        let resources = etalon_inference::projections::topk_resources::TopkResources::build(
            self.gpu_device(),
            cache,
            etalon_inference::gpu_resources::ResourceBudget::for_device(self.gpu_device()),
        )
        .expect("output top-k resources exceed device limits");
        self.topk_cand_val_buf = Some(resources.values);
        self.topk_cand_idx_buf = Some(resources.indices);
        self.topk_cand_staging = Some(resources.staging);
        self.topk_params_buf = Some(resources.params);
        self.output_topk_bind_layout = Some(resources.layout);
        self.output_topk_pipeline = Some(resources.pipeline);
    }

    /// Native decode fast path: output projection plus GPU block argmax (`k=1`) with one tiny
    /// candidate readback after all vocab chunks have been submitted. This avoids the full
    /// chunk-logit readback in [`Self::dispatch_output_argmax_chunked`] and avoids heap allocation
    /// in the decode loop.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn dispatch_output_top1_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
    ) -> Option<StreamingArgmaxResult> {
        if self.qwen35_gguf_required || self.gemma4_required {
            return None;
        }
        etalon_inference::projections::ProjectionGpuEngine::new(self)
            .dispatch_output_top1_chunked(index, hidden, emb_dim)
            .map(|r| {
                let softcap = index.hyperparams.logit_softcap;
                let max_logit = if softcap > 0.0 {
                    (r.max_logit / softcap).tanh() * softcap
                } else {
                    r.max_logit
                };
                StreamingArgmaxResult {
                    best_token_id: r.best_token_id,
                    max_logit,
                }
            })
    }

    /// A1a: GPU top-k over the output projection — the logits stay on-GPU (`gemm_output_buf`), the
    /// top-k kernel reduces them per chunk, and only K `(id, logit)` candidates are read back (vs the
    /// 196 KB/token full-logit readback + CPU argmax in `dispatch_output_argmax_chunked`). Returns the
    /// merged global top-K, or `None` to signal the caller to fall back to the argmax path. v1: no
    /// sieve coupling (caller routes here only when no sieve mask is active).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn dispatch_output_topk_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        k: usize,
    ) -> Option<Vec<crate::topk::TopKItem>> {
        if self.qwen35_gguf_required || self.gemma4_required {
            return None;
        }
        let mut selected = [crate::topk::TopKItem {
            token_id: 0,
            logit: f32::NEG_INFINITY,
        }; crate::topk::TOPK_MAX_K];
        let count = etalon_inference::projections::ProjectionGpuEngine::new(self)
            .dispatch_output_topk_chunked_into(index, hidden, emb_dim, k, &mut selected)?;
        Some(selected[..count].to_vec())
    }

    /// Final `output_norm` RMSNorm in-place before vocabulary projection (Pre-Norm LLM tail).
    /// REQUIRED on all targets — native previously skipped it → logits from an un-normed hidden.
    pub fn apply_output_norm_inplace(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
    ) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.gemma4_required {
            return self.gemma4_output_norm(index, hidden, emb_dim);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.qwen35_gguf_required {
            return self.dense_qwen_output_norm(index, hidden, emb_dim);
        }
        if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            #[cfg(not(target_arch = "wasm32"))]
            return self.qwen35_output_norm(index, hidden, emb_dim);
            #[cfg(target_arch = "wasm32")]
            return false;
        }
        etalon_inference::projections::ProjectionEngine { host: self }
            .apply_output_norm_inplace(index, hidden, emb_dim)
    }
}
