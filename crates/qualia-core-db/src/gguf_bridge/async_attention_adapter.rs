use super::*;
use etalon_inference::ordinary::async_attention::AsyncAttentionProvider;
use etalon_model::inference::ModelHyperparams;
impl AsyncAttentionProvider for QTensorEngine {
    async fn dispatch_ffn_block_pre_norm_async(
        &mut self,
        index: &Self::Index,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        QTensorEngine::dispatch_ffn_block_pre_norm_async(
            self, index, hidden, emb_dim, tensors, scratch_a, scratch_b,
        )
        .await
    }

    async fn dispatch_gemm_into_async(
        &self,
        index: &Self::Index,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        QTensorEngine::dispatch_gemm_into_async(self, index, info, input, out, n_in, n_out).await
    }
    async fn dispatch_attention_pass_async(
        &self,
        hidden: &[f32],
        n_embd: usize,
        batch: u32,
        start: u32,
        layout: &Self::Layout,
        layer: u32,
        token: u32,
        h: &ModelHyperparams,
        info: &GgufTensorInfo,
        weights: &[u8],
        kind: u32,
        groups: u32,
        out: Option<&mut [f32]>,
    ) -> bool {
        QTensorEngine::dispatch_attention_pass_async(
            self,
            hidden,
            n_embd,
            batch,
            start,
            layout,
            layer,
            token,
            &(*h).into(),
            info,
            weights,
            kind,
            groups,
            out,
        )
        .await
    }
}
