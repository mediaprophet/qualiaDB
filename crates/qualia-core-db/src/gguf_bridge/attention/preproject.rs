use crate::gguf_bridge::*;
impl QTensorEngine {
    pub(crate) fn dispatch_attention_kv_preproject_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        kv_dim: usize,
    ) -> bool {
        etalon_inference::native_attention::AttentionGpuEngine { host: self }
            .dispatch_attention_kv_preproject_fused(
                hidden,
                n_embd,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                k_info,
                k_raw,
                v_info,
                v_raw,
                kv_dim,
            )
    }
}
