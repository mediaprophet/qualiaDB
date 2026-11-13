//! Routing glue: retained source, exact directory, explicit f32 KV and complete graph.
use super::{Profile, Session};
use crate::gguf_bridge::{GgufLoadReport, KvCacheLayout, ModelByteHandle, QTensorEngine};
use crate::gguf_sharder::{GgufHyperparams, GgufTensorIndex};
use std::sync::Arc;

impl QTensorEngine {
    pub(crate) fn load_dense_qwen_gguf(
        &mut self,
        mmap: memmap2::Mmap,
        index: GgufTensorIndex,
        profile: Profile,
    ) -> Result<GgufLoadReport, String> {
        let mapped_bytes = mmap.len() as u64;
        let mmap = Arc::new(mmap);
        let source = ModelByteHandle::Legacy(mmap.clone());
        let session = Session::new(source, index.clone(), profile, self.ssm_state_byte_budget)?;
        KvCacheLayout::from_hyperparams_mode(&index.hyperparams, false, 0, 0)
            .ok_or("Qwen f32 KV budget")?;
        // Architecture admission is complete before changing this engine's selected model.
        self.gemma4 = None;
        self.gemma4_required = false;
        self.qwen35_gguf_required = true;
        self.qwen35_gguf = Some(session);
        self.qwen35_ftw = None;
        self.ftw_package = None;
        self.prepared_native_bytes = None;
        self.prepared_native_sources = None;
        self.ssm_recurrent_state = Box::new([]);
        self.hyperparams = index.hyperparams;
        self.tensor_data_offset = index.tensor_data_start;
        self.gguf_mmap = Some(mmap);
        self.p64_index = None;
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
        self.ensure_kv_cache(&index.hyperparams);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            self.qwen35_gguf = None; // required marker prevents fallback after a failed admission.
            return Err("Qwen f32 KV reservation refused".into());
        }
        Ok(GgufLoadReport {
            mapped_bytes,
            tensor_data_offset: index.tensor_data_start,
            n_layer: index.hyperparams.n_layer,
            n_head: index.hyperparams.n_head,
            n_kv_head: index.hyperparams.effective_n_kv_head(),
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
    pub(crate) fn dense_qwen_kv_layout(&self, h: &GgufHyperparams) -> Option<KvCacheLayout> {
        if self.qwen35_gguf_required
            && self
                .qwen35_gguf
                .as_ref()
                .is_some_and(|s| s.index.hyperparams == *h)
        {
            KvCacheLayout::from_hyperparams_mode(h, false, 0, 0)
        } else {
            KvCacheLayout::from_hyperparams(h)
        }
    }
    fn dense_qwen_matches(&self, index: &GgufTensorIndex) -> bool {
        let Some(s) = &self.qwen35_gguf else {
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
            && same_index(&s.index, index)
            && same_index(&s.index, current)
            && self.hyperparams == s.index.hyperparams
            && self.tensor_data_offset == s.index.tensor_data_start
    }
    pub(crate) fn dense_qwen_ready(&self, index: &GgufTensorIndex) -> bool {
        self.dense_qwen_matches(index)
            && self
                .qwen35_gguf
                .as_ref()
                .is_some_and(|s| s.position > 0 && s.next_layer == 0)
    }
    pub(crate) fn dense_qwen_layer(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        position: u32,
        hidden: &mut [f32],
        width: usize,
        a: &mut [f32],
        b: &mut [f32],
    ) -> bool {
        if !self.dense_qwen_matches(index) || hidden.len() != width {
            return false;
        }
        // Taking the owner avoids aliasing mutable engine state with mutable recurrent state.
        // The separate required marker survives unwinding: no generic fallback is then possible.
        let Some(mut session) = self.qwen35_gguf.take() else {
            return false;
        };
        let ok = session.layer(self, layer as usize, position, hidden, a, b);
        self.qwen35_gguf = Some(session);
        ok
    }
    pub(crate) fn dense_qwen_forward(
        &mut self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
        position: u32,
        limit: u32,
        a: &mut [f32],
        b: &mut [f32],
    ) -> u32 {
        if !self.dense_qwen_matches(index) || (limit != 0 && limit != index.hyperparams.n_layer) {
            return 0;
        }
        for layer in 0..index.hyperparams.n_layer {
            if !self.dense_qwen_layer(index, layer, position, hidden, width, a, b) {
                return 0;
            }
        }
        index.hyperparams.n_layer
    }
    pub(crate) fn dense_qwen_output_norm(
        &self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
    ) -> bool {
        if !self.dense_qwen_ready(index)
            || width != index.hyperparams.n_embd as usize
            || hidden.len() != width
            || width > 4096
        {
            return false;
        }
        let Some(info) = index.output_norm else {
            return false;
        };
        let mut weights = [0.; 4096];
        let mut out = [0.; 4096];
        if super::linear::norm(
            self,
            &info,
            hidden,
            index.hyperparams.rms_norm_eps,
            &mut weights[..width],
            &mut out[..width],
        )
        .is_none()
        {
            return false;
        }
        hidden.copy_from_slice(&out[..width]);
        true
    }
}
fn same_index(a: &GgufTensorIndex, b: &GgufTensorIndex) -> bool {
    a.entries == b.entries
        && a.hyperparams == b.hyperparams
        && a.tensor_data_start == b.tensor_data_start
        && a.token_embd == b.token_embd
        && a.output_weight == b.output_weight
        && a.output_norm == b.output_norm
        && a.moe_routing == b.moe_routing
        && a.ple_config == b.ple_config
}
