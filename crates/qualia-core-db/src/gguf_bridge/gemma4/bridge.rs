//! Admission and token-aware entry points for the actual retained Gemma graph.
use super::{Profile, Session};
use crate::{
    gguf_bridge::{GgufLoadReport, ModelByteHandle, QTensorEngine},
    gguf_sharder::GgufTensorIndex,
};
use std::sync::Arc;
impl QTensorEngine {
    pub(crate) fn load_gemma4_gguf(
        &mut self,
        mmap: memmap2::Mmap,
        index: GgufTensorIndex,
        p: Profile,
        context: usize,
        state_budget: usize,
    ) -> Result<GgufLoadReport, String> {
        let mapped_bytes = mmap.len() as u64;
        let mmap = Arc::new(mmap);
        let session = Session::new(
            ModelByteHandle::Legacy(mmap.clone()),
            index,
            p,
            context,
            state_budget,
        )?;
        let index = session.plan.index.clone();
        self.gemma4_required = true;
        self.gemma4 = Some(session);
        self.qwen35_gguf = None;
        self.qwen35_gguf_required = false;
        self.qwen35_ftw = None;
        self.ftw_package = None;
        self.prepared_native_bytes = None;
        self.prepared_native_sources = None;
        self.ssm_recurrent_state = Box::new([]);
        self.p64_index = None;
        self.hyperparams = index.hyperparams;
        self.tensor_data_offset = index.tensor_data_start;
        self.max_tensor_bytes = index.max_layer_tensor_bytes;
        self.gguf_mmap = Some(mmap);
        self.tensor_index_cache = Some(index.clone());
        self.kv_layout = None;
        self.kv_cache_cpu = None;
        #[cfg(feature = "gpu-runtime")]
        {
            self.invalidate_resident_projection_weights();
            self.kv_cache_gpu = None;
            let staging = index
                .max_layer_tensor_bytes
                .max(4096)
                .min(crate::gguf_bridge::MAX_WGPU_WEIGHT_STAGING);
            self.ensure_gemm_buffers(staging, crate::gguf_bridge::MAX_STACK_GEMM_OUT as u32);
        }
        Ok(GgufLoadReport {
            mapped_bytes,
            tensor_data_offset: index.tensor_data_start,
            n_layer: index.hyperparams.n_layer,
            n_head: index.hyperparams.n_head,
            n_kv_head: index.hyperparams.n_kv_head,
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes: self.kv_cache_bytes(),
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }
    fn gemma4_matches(&self, index: &GgufTensorIndex) -> bool {
        let Some(s) = &self.gemma4 else {
            return false;
        };
        let Some(source) = self.clone_model_bytes() else {
            return false;
        };
        let Some(current) = &self.tensor_index_cache else {
            return false;
        };
        !s.poisoned
            && s.source.same_owner(&source)
            && same_index(index, &s.plan.index)
            && same_index(current, &s.plan.index)
            && self.hyperparams == s.plan.index.hyperparams
            && self.tensor_data_offset == s.plan.index.tensor_data_start
    }
    pub(crate) fn gemma4_ready(&self, index: &GgufTensorIndex) -> bool {
        self.gemma4_matches(index) && self.gemma4.as_ref().is_some_and(|s| s.position > 0)
    }
    /// Text-only Gemma4 token step. The owner reads the actual retained embedding
    /// and PLE token row, so hidden-only callers cannot silently substitute token0.
    /// Returns all layers or zero; an interrupted/failed token requires reset.
    pub fn dispatch_gemma4_token(
        &mut self,
        index: &GgufTensorIndex,
        token: u32,
        position: u32,
        hidden: &mut [f32],
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> u32 {
        if !self.gemma4_required || !self.gemma4_matches(index) {
            return 0;
        }
        let Some(mut session) = self.gemma4.take() else {
            return 0;
        };
        let count = session.token(self, token, position, hidden, scratch_a, scratch_b);
        self.gemma4 = Some(session);
        count
    }
    pub(crate) fn gemma4_output_norm(
        &self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
    ) -> bool {
        if !self.gemma4_ready(index)
            || hidden.len() != width
            || width != index.hyperparams.n_embd as usize
        {
            return false;
        }
        let Some(info) = index.output_norm else {
            return false;
        };
        super::attention::norm(self, &info, hidden, index.hyperparams.rms_norm_eps).is_some()
    }
}
fn same_index(a: &GgufTensorIndex, b: &GgufTensorIndex) -> bool {
    a.entries == b.entries
        && a.hyperparams == b.hyperparams
        && a.tensor_data_start == b.tensor_data_start
        && a.token_embd == b.token_embd
        && a.output_weight == b.output_weight
        && a.output_norm == b.output_norm
        && a.ple_ngram_embedding == b.ple_ngram_embedding
        && a.ple_config == b.ple_config
        && a.moe_routing == b.moe_routing
}
