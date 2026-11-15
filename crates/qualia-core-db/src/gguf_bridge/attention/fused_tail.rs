use crate::gguf_bridge::*;
impl QTensorEngine {
    pub(crate) fn dispatch_attention_q_o_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        q_info: &GgufTensorInfo,
        q_raw: &[u8],
        o_info: &GgufTensorInfo,
        o_raw: &[u8],
        out: &mut [f32],
        o_in: usize,
        o_out: usize,
    ) -> bool {
        etalon_inference::native_attention::AttentionGpuEngine { host: self }
            .dispatch_attention_q_o_fused(
                hidden,
                n_embd,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                q_info,
                q_raw,
                o_info,
                o_raw,
                out,
                o_in,
                o_out,
            )
    }
}
