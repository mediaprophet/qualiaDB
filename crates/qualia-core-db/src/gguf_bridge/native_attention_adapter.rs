use super::*;
use etalon_inference::native_attention::{AttentionResources, GpuAttentionProvider};
use etalon_model::inference::ModelHyperparams;
impl GpuAttentionProvider for QTensorEngine {
    type Layout = KvCacheLayout;
    type Mask = [u32; KV_ATTENTION_MASK_WORDS];
    fn attention_resources(&self) -> AttentionResources<'_> {
        AttentionResources {
            gemm_input_buf: self.gemm_input_buf.as_ref(),
            gemm_output_buf: self.gemm_output_buf.as_ref(),
            gemm_aux_buf: self.gemm_aux_buf.as_ref(),
            gemm_output_staging: self.gemm_output_staging.as_ref(),
            attention_params_buf: self.attention_params_buf.as_ref(),
            attention_mask_buf: self.attention_mask_buf.as_ref(),
            gemm_params_buf: self.gemm_params_buf.as_ref(),
            kv_cache_gpu: self.kv_cache_gpu.as_ref(),
            attention_kv_gemm_params: self.attention_kv_gemm_params.as_ref(),
            attention_kv_params: self.attention_kv_params.as_ref(),
            attention_bind_layout: &self.attention_bind_layout,
            attention_pipeline: &self.attention_pipeline,
            coop_gemv_pipeline: &self.coop_gemv_pipeline,
            pipeline: &self.pipeline,
            gemm_max_input_floats: self.gemm_max_input_floats,
            gemm_max_out_dim: self.gemm_max_out_dim,
            max_tensor_bytes: self.max_tensor_bytes,
        }
    }
    fn resident_weights_enabled(&self) -> bool {
        crate::llm_bench::resident_weights_enabled()
    }
    fn coop_gemv_enabled(&self) -> bool {
        crate::llm_bench::coop_gemv_enabled()
    }
    fn gpu_device(&self) -> &etalon::Device {
        QTensorEngine::gpu_device(self)
    }
    fn gpu_queue(&self) -> &etalon::Queue {
        QTensorEngine::gpu_queue(self)
    }
    fn native_gemm_bind_layout(&self, coop: bool) -> &etalon::BindGroupLayout {
        QTensorEngine::native_gemm_bind_layout(self, coop)
    }
    fn resident_weight_buffer(&self, key: u64, raw: &[u8]) -> Option<etalon::Buffer> {
        QTensorEngine::resident_weight_buffer(self, key, raw)
    }
    fn submit(&self, command: etalon::CommandBuffer) -> bool {
        self.submit_commands([command]).is_some()
    }
    fn pass_writes_begin(&self) -> Option<etalon::ComputePassTimestampWrites<'static>> {
        crate::llm_gpu_profiler::pass_writes_begin()
    }
    fn pass_writes_end(&self) -> Option<etalon::ComputePassTimestampWrites<'static>> {
        crate::llm_gpu_profiler::pass_writes_end()
    }
    fn resolve(&self, encoder: &mut etalon::CommandEncoder) {
        crate::llm_gpu_profiler::resolve(encoder)
    }
    fn accumulate_attention(&self) {
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention)
    }
    fn layer_stride(layout: &Self::Layout) -> u32 {
        layout.layer_stride
    }
    fn attention_kv_mask_for_dispatch(
        layout: &Self::Layout,
        token_idx: u32,
        proj_kind: u32,
    ) -> (Self::Mask, u32, u32) {
        QTensorEngine::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind)
    }
    fn attention_gpu_params(
        h: &ModelHyperparams,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        info: &GgufTensorInfo,
        raw_len: usize,
        proj_kind: u32,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        mask_active: u32,
        mask_word_count: u32,
        out_stride_elems: u32,
    ) -> AttentionGpuParams {
        QTensorEngine::attention_gpu_params(
            &(*h).into(),
            layout,
            layer,
            token_idx,
            info,
            raw_len,
            proj_kind,
            num_tokens_in_batch,
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            out_stride_elems,
        )
    }
    fn readback(&self, staging: &etalon::Buffer, out_bytes: u64, out: &mut [f32]) -> bool {
        let slice = staging.slice(..out_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(etalon::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle
                .block_on(rx)
                .ok()
                .map(|mapped| mapped.is_ok())
                .unwrap_or(false)
            {
                let data = slice
                    .get_mapped_range()
                    .expect("wgpu buffer map_range failed");
                let floats: &[f32] = bytemuck::cast_slice(&data);
                out.copy_from_slice(&floats[..out.len()]);
                drop(data);
                staging.unmap();
                return true;
            }
        }
        let _ = staging.unmap();
        false
    }
}
