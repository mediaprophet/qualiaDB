//! Hybrid numerical execution adapter; provider dispatch stays in QTensorEngine.
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};
use etalon_inference::ordinary::ssm_layout::{
    ssm_layout_for, SsmLayerStateLayout, CONV_HISTORY_DEPTH,
};
use etalon_inference::ordinary::{HybridEngine, LayerProvider};
impl LayerProvider for super::QTensorEngine {
    type Index = GgufTensorIndex;
    fn hyperparams(index: &Self::Index) -> etalon_model::inference::ModelHyperparams {
        index.hyperparams.into()
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
                super::wlog(&format!("[ssm] layer={layer} skipped: state not allocated"))
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
    /// Allocate (once) the flat recurrent-state arena for all hybrid SSM layers.
    ///
    /// Called after the GGUF index is parsed. Returns the number of f32 elements
    /// allocated (zero if the model has no hybrid SSM layers).
    pub fn init_ssm_state(&mut self, index: &GgufTensorIndex) -> usize {
        let h = &index.hyperparams;
        let n_layer = h.n_layer as usize;
        let n_embd = h.n_embd as usize;
        let mut total_elems = 0usize;
        let mut any_ssm = false;
        for layer in 0..n_layer as u32 {
            let tensors = index.get_layer_tensors(layer);
            if tensors.is_hybrid_ssm_layer() {
                any_ssm = true;
                let layout = ssm_layout_for(&tensors, &(*h).into(), n_embd);
                total_elems += layout.stride;
            }
        }
        if !any_ssm || total_elems == 0 {
            return 0;
        }
        // Sentinel budget check (36 MB ceiling).
        const MAX_SSM_BYTES: usize = 36 * 1024 * 1024;
        let byte_budget = total_elems * 4;
        if byte_budget > MAX_SSM_BYTES {
            super::wlog(&format!(
                "[ssm_init] SSM state too large: {byte_budget} bytes > {MAX_SSM_BYTES} ceiling -- SSM layers skipped"
            ));
            return 0;
        }
        // Tier-2 cold allocation: single Box<[f32]> created once at boot.
        let state = vec![0.0f32; total_elems].into_boxed_slice();
        self.ssm_recurrent_state = state;
        super::wlog(&format!(
            "[ssm_init] allocated SSM state: {total_elems} f32 ({} kB) for {n_layer} layers",
            byte_budget / 1024,
        ));
        total_elems
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
    fn ssm_state_size_within_sentinel_budget() {
        // 96-layer Granite: stride per SSM layer = 3*4096 + 128*64 = 20480 f32
        let layout = SsmLayerStateLayout::new(4096, 128, 64, 1);
        let bytes = layout.stride * 96 * 4;
        // 96 * 20480 * 4 = 7,864,320 < 8 MB
        assert!(
            bytes < 8 * 1024 * 1024,
            "SSM state exceeds 8 MB sentinel: {bytes} bytes"
        );
    }
}
