//! Hybrid numerical execution adapter; provider dispatch stays in QTensorEngine.
#![allow(unused_imports, unused_variables, unused_mut)]
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};
use etalon_inference::ordinary::ssm_layout::{
    ssm_layout_for, SsmLayerStateLayout, SsmStateError, CONV_HISTORY_DEPTH,
};
use etalon_inference::ordinary::{HybridEngine, LayerProvider};
impl LayerProvider for super::QTensorEngine {
    type Index = GgufTensorIndex;
    fn hyperparams(index: &Self::Index) -> etalon_model::inference::ModelHyperparams {
        index.hyperparams.into()
    }
    fn moe_routing(
        index: &Self::Index,
    ) -> etalon_model::inference::moe_routing::MoeRoutingSettings {
        index.moe_routing
    }
    fn data_start(index: &Self::Index) -> u64 {
        index.tensor_data_start
    }
    fn tensors(index: &Self::Index, layer: u32) -> LayerTensors {
        index.get_layer_tensors(layer)
    }
    fn model_bytes(&self) -> Option<&[u8]> {
        super::QTensorEngine::model_bytes(self)
    }
    fn state(&self) -> &[f32] {
        &self.ssm_recurrent_state
    }
    fn state_mut(&mut self) -> &mut [f32] {
        &mut self.ssm_recurrent_state
    }
    fn gemm(
        &self,
        index: &Self::Index,
        info: &crate::gguf_sharder::GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        self.dispatch_gemm_into(index, info, input, out, n_in, n_out)
    }
    fn norm(&self, info: &crate::gguf_sharder::GgufTensorInfo, out: &mut [f32]) -> usize {
        self.dequant_norm_tensor_into(info, out)
    }
    fn raw(&self, info: &crate::gguf_sharder::GgufTensorInfo) -> Option<&[u8]> {
        self.fetch_tensor_raw_bytes(info)
    }
    fn event(&self, event: etalon_inference::ordinary::LayerEvent) {
        use etalon_inference::ordinary::LayerEvent;
        match event {
            LayerEvent::StateMissing(layer) => {
                super::wlog(&format!("[ssm] layer={layer} refused: state not allocated"))
            }
            LayerEvent::SsmUnrecognized(layer) => {
                super::wlog(&format!("[ssm] layer={layer} unrecognised SSM flavour"))
            }
            LayerEvent::GraniteModelMissing(layer) => {
                super::wlog(&format!("[granite_ssm] layer={layer} mmap None"))
            }
            LayerEvent::GraniteInputProjection(layer) => {
                super::wlog(&format!("[granite_ssm] layer={layer} ssm_in GEMM fail"))
            }
            LayerEvent::GraniteConvolutionFetch(layer) => {
                super::wlog(&format!("[granite_ssm] layer={layer} conv fetch fail"))
            }
            LayerEvent::QwenModelMissing(layer) => {
                super::wlog(&format!("[qwen_ssm] layer={layer} mmap None"))
            }
            LayerEvent::QwenQkvProjection(layer) => {
                super::wlog(&format!("[qwen_ssm] layer={layer} QKV GEMM fail"))
            }
        }
    }
    fn moe(
        &mut self,
        index: &Self::Index,
        emb_dim: usize,
        tensors: &LayerTensors,
        out: &mut [f32],
        input: &[f32],
    ) -> bool {
        self.dispatch_moe_ffn(index, emb_dim, tensors, out, input)
    }
    fn record_ffn_input(&self, input: &[f32]) {
        crate::llm_awq::record_ffn_input(input)
    }
    fn attention(
        &mut self,
        index: &Self::Index,
        layer: u32,
        token: u32,
        hidden: &[f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        a: &mut [f32],
        b: &mut [f32],
    ) -> Option<usize> {
        self.dispatch_attention_layer(index, layer, token, hidden, emb_dim, tensors, a, b)
    }
    fn ffn_dispatch(
        &mut self,
        index: &Self::Index,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        a: &mut [f32],
        b: &mut [f32],
    ) -> bool {
        self.dispatch_ffn_block_pre_norm(index, hidden, emb_dim, tensors, a, b)
    }
    fn profile_attention(&self, ns: u64) {
        #[cfg(not(target_arch = "wasm32"))]
        crate::llm_bench::add_decode_attn_ns(ns);
    }
    fn profile_ffn(&self, ns: u64) {
        #[cfg(not(target_arch = "wasm32"))]
        crate::llm_bench::add_decode_ffn_ns(ns);
    }
    fn layer_diagnostic(
        &self,
        layer: u32,
        attn_ok: bool,
        tensors: &LayerTensors,
        attention: &[f32],
        hidden: &[f32],
    ) {
        if layer < 3 && crate::gguf_bridge::execution_env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
            let max_attn = attention.iter().fold(0f32, |m, &v| m.max(v.abs()));
            let max_hid = hidden.iter().fold(0f32, |m, &v| m.max(v.abs()));
            eprintln!("[layer-dbg] L{} attn_ok={} attn_norm={} ffn_norm={} max|attn_out|={:.4} max|hidden_postattn|={:.4}", layer, attn_ok, tensors.attn_norm.is_some(), tensors.ffn_norm.is_some(), max_attn, max_hid);
        }
    }
    fn expert_source(
        &self,
    ) -> Option<&dyn etalon_inference::ordinary::expert_source::ExpertSource> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.ftw_package
                .as_deref()
                .map(|s| s as &dyn etalon_inference::ordinary::expert_source::ExpertSource)
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }
}
impl super::QTensorEngine {
    /// Select the external recurrent-state budget for subsequent cold model loads.
    /// This arena is separate from the 42 MiB Sentinel arena.
    pub fn set_ssm_state_byte_budget(&mut self, byte_budget: usize) {
        self.ssm_state_byte_budget = byte_budget;
    }

    pub fn init_ssm_state_with_budget(
        &mut self,
        index: &GgufTensorIndex,
        byte_budget: usize,
    ) -> Result<usize, SsmStateError> {
        use etalon_inference::ordinary::ssm_layout::{
            allocate_recurrent_state, checked_ssm_layout_for,
        };
        let h = index.hyperparams.into();
        let state = allocate_recurrent_state(
            (0..index.hyperparams.n_layer).filter_map(|layer| {
                let tensors = index.get_layer_tensors(layer);
                tensors.is_hybrid_ssm_layer().then(|| {
                    checked_ssm_layout_for(&tensors, &h, index.hyperparams.n_embd as usize)
                        .map(|layout| layout.stride)
                })
            }),
            byte_budget,
        )?;
        let count = state.len();
        self.ssm_recurrent_state = state;
        Ok(count)
    }

    /// Compatibility entry point. Loaders use fallible admission; a refused arena
    /// remains empty and hybrid dispatch fails closed.
    pub fn init_ssm_state(&mut self, index: &GgufTensorIndex) -> usize {
        match self.init_ssm_state_with_budget(index, self.ssm_state_byte_budget) {
            Ok(count) => count,
            Err(_) => {
                self.ssm_recurrent_state = Box::new([]);
                0
            }
        }
    }

    pub fn reset_recurrent_state(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(session) = &mut self.qwen35_ftw {
            session.reset();
        }
        self.ssm_recurrent_state.fill(0.0);
    }

    /// Export a zero-heap snapshot of the current hybrid SSM / GatedDeltaNet recurrent state
    /// (`ssm_recurrent_state`) into caller-supplied `out` (`f32` slice).
    /// Returns `Some(self.ssm_recurrent_state.len())` when `out` has sufficient capacity.
    #[must_use]
    pub fn snapshot_recurrent_state_into(&self, out: &mut [f32]) -> Option<usize> {
        let len = self.ssm_recurrent_state.len();
        if out.len() < len {
            return None;
        }
        out[..len].copy_from_slice(&self.ssm_recurrent_state);
        Some(len)
    }

    /// Restore a hybrid SSM / GatedDeltaNet recurrent state snapshot from a `.p64` neural
    /// extension cartridge or pre-warmed scene seed into `self.ssm_recurrent_state` (zero-heap).
    #[must_use]
    pub fn restore_recurrent_state_from_slice(&mut self, src: &[f32]) -> bool {
        let len = self.ssm_recurrent_state.len();
        if len == 0 || src.len() != len {
            return false;
        }
        self.ssm_recurrent_state.copy_from_slice(src);
        true
    }

    pub(crate) fn dispatch_hybrid_ssm_layer(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            #[cfg(not(target_arch = "wasm32"))]
            return self.qwen35_attention(index, layer, hidden, emb_dim, tensors);
            #[cfg(target_arch = "wasm32")]
            return false;
        }
        HybridEngine { host: self }
            .dispatch(index, layer, hidden, emb_dim, tensors, scratch_a, scratch_b)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gguf_sharder::{GgufHyperparams, LayerTensors};

    #[test]
    fn ssm_layout_new_granite_style() {
        let layout = SsmLayerStateLayout::new(4096, 128, 64, 1);
        assert_eq!(layout.conv_offset, 0);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 4096);
        assert_eq!(layout.delta_offset, layout.conv_len);
        assert_eq!(layout.delta_len, 128 * 64 * 1);
        assert_eq!(layout.stride, layout.conv_len + layout.delta_len);
    }

    #[test]
    fn ssm_layout_new_qwen_style() {
        let layout = SsmLayerStateLayout::new(2048, 128, 128, 16);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 2048);
        assert_eq!(layout.delta_len, 128 * 128 * 16);
        assert_eq!(layout.stride, layout.conv_len + layout.delta_len);
    }

    #[test]
    fn ssm_layout_for_uses_explicit_ssm_inner_size() {
        let mut h = GgufHyperparams::default();
        h.ssm_inner_size = 1024;
        h.ssm_state_size = 64;
        h.ssm_group_count = 4;
        h.n_embd = 512;
        let tensors = LayerTensors::default();
        let layout = ssm_layout_for(&tensors, &h.into(), 512);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 1024);
        assert!(layout.delta_len > 0);
    }

    #[test]
    fn ssm_state_size_example_external_model_arena() {
        // 96-layer Granite: stride per SSM layer = 3*4096 + 128*64 = 20480 f32
        let layout = SsmLayerStateLayout::new(4096, 128, 64, 1);
        let bytes = layout.stride * 96 * 4;
        // 96 * 20480 * 4 = 7,864,320 < 8 MB
        assert!(
            bytes < 8 * 1024 * 1024,
            "SSM example external state exceeds 8 MB: {bytes} bytes"
        );
    }
    #[test]
    fn reset_sequence_clears_recurrence_without_kv_layout() {
        use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
        let mut engine = super::super::QTensorEngine::new();
        engine.ssm_recurrent_state = vec![1.0, -2.0, 3.0].into_boxed_slice();
        engine.kv_layout = None;
        assert_zero_alloc("reset sequence", || engine.reset_kv_cache());
        assert_eq!(&*engine.ssm_recurrent_state, &[0.0; 3]);
    }

    #[test]
    fn attention_only_replacement_drops_old_recurrence() {
        let mut engine = super::super::QTensorEngine::new();
        engine.ssm_recurrent_state = vec![9.0; 8].into_boxed_slice();
        let index = GgufTensorIndex {
            entries: Vec::new(),
            tensor_data_start: 0,
            token_embd: None,
            output_weight: None,
            output_norm: None,
            ple_ngram_embedding: None,
            moe_routing: Default::default(),
            ple_config: None,
            hyperparams: GgufHyperparams::default(),
            max_tensor_bytes: 0,
            max_layer_tensor_bytes: 0,
        };
        assert_eq!(engine.init_ssm_state_with_budget(&index, 0), Ok(0));
        assert!(engine.ssm_recurrent_state.is_empty());
    }
}
