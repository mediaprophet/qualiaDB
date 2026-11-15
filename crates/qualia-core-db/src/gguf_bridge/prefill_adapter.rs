use super::*;
use etalon_inference::browser_prefill::{PrefillProvider, PrefillResources};
use etalon_model::inference::{LayerTensors, ModelHyperparams};
impl PrefillProvider for QTensorEngine {
    type Pipeline = WasmGpuPipeline;
    fn weight_role_buffer(&self, role: Mc8WeightRole) -> &etalon::Buffer {
        self.mc8_weight_role_buf(role)
    }
    fn encoding_resources(
        &self,
    ) -> etalon_inference::browser_prefill::encoding::EncodingResources<'_> {
        etalon_inference::browser_prefill::encoding::EncodingResources {
            device: self.gpu_device(),
            role_strides: self.mc8_weight_role_stride,
            attention_mask_buf: self.attention_mask_buf.as_ref(),
            attention_params_buf: self.attention_params_buf.as_ref(),
            attention_pipeline: &self.attention_pipeline,
            elem_add_residual_pipeline: &self.elem_add_residual_pipeline,
            elem_params_buf: self.elem_params_buf.as_ref(),
            elem_rms_norm_pipeline: &self.elem_rms_norm_pipeline,
            elem_silu_mul_pipeline: &self.elem_silu_mul_pipeline,
            gemm_max_input_floats: self.gemm_max_input_floats,
            gemm_max_out_dim: self.gemm_max_out_dim,
            gemm_params_buf: self.gemm_params_buf.as_ref(),
            gemm_weight_buf: self.gemm_weight_buf.as_ref(),
            kv_cache_gpu: self.kv_cache_gpu.as_ref(),
            max_tensor_bytes: self.max_tensor_bytes,
            mc8_attn_bind_layout: &self.mc8_attn_bind_layout,
            mc8_bg_cache: &self.mc8_bg_cache,
            mc8_elem_bind_layout: &self.mc8_elem_bind_layout,
            mc8_ffn_fused_bind_layout: &self.mc8_ffn_fused_bind_layout,
            mc8_ffn_fused_pipeline: &self.mc8_ffn_fused_pipeline,
            mc8_gemm_bind_layout: &self.mc8_gemm_bind_layout,
            mc8_weights_resident: self.mc8_weights_resident,
            mmv_q8_0_pipeline: &self.mmv_q8_0_pipeline,
            pipeline: &self.pipeline,
        }
    }
    fn layer_stride(layout: &Self::Layout) -> u32 {
        layout.layer_stride
    }
    fn shader_supported(ggml_type: u32) -> bool {
        ggml_gpu_attention_shader_supported(ggml_type)
    }
    fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        QTensorEngine::write_weight_words(self, raw, max_bytes)
    }
    fn attention_kv_mask_for_dispatch(
        layout: &Self::Layout,
        token: u32,
        kind: u32,
    ) -> ([u32; KV_ATTENTION_MASK_WORDS], u32, u32) {
        QTensorEngine::attention_kv_mask_for_dispatch(layout, token, kind)
    }

    fn prefill_resources(&self) -> PrefillResources<'_> {
        PrefillResources {
            attention_params_buf: self.attention_params_buf.as_ref(),
            elem_params_buf: self.elem_params_buf.as_ref(),
            gemm_params_buf: self.gemm_params_buf.as_ref(),
            prefill_scratch_buf: self.prefill_scratch_buf.as_ref(),
            gemm_input_buf: self.gemm_input_buf.as_ref(),
            gemm_output_buf: self.gemm_output_buf.as_ref(),
            norm_weight_buf: self.norm_weight_buf.as_ref(),
            mc8_k_proj_buf: self.mc8_k_proj_buf.as_ref(),
            mc8_v_proj_buf: self.mc8_v_proj_buf.as_ref(),
            prefill_work_buf_a: self.prefill_work_buf_a.as_ref(),
            prefill_work_buf_b: self.prefill_work_buf_b.as_ref(),
            gemm_aux_buf: self.gemm_aux_buf.as_ref(),
            mc8_q_proj_buf: self.mc8_q_proj_buf.as_ref(),
            gemm_max_out_dim: self.gemm_max_out_dim,
            gemm_max_input_floats: self.gemm_max_input_floats,
            mc8_weights_resident: self.mc8_weights_resident,
        }
    }
    fn gpu_queue(&self) -> &etalon::Queue {
        QTensorEngine::gpu_queue(self)
    }
    fn mc8_buffers_ready(&self) -> bool {
        QTensorEngine::mc8_buffers_ready(self)
    }
    fn begin(&self) -> Self::Pipeline {
        WasmGpuPipeline::begin(self)
    }
    fn encoder(pipeline: &mut Self::Pipeline) -> &mut etalon::CommandEncoder {
        &mut pipeline.encoder
    }
    fn max_context(layout: &Self::Layout) -> u32 {
        layout.max_context
    }
    fn mask_word_count() -> u32 {
        KV_ATTENTION_MASK_WORDS as u32
    }
    fn tensors(index: &Self::Index, layer: u32) -> LayerTensors {
        index.get_layer_tensors(layer)
    }
    fn log(&self, message: &str) {
        wlog(message)
    }
    fn attention_gpu_params(
        h: &ModelHyperparams,
        layout: &Self::Layout,
        layer: u32,
        token: u32,
        info: &GgufTensorInfo,
        raw: usize,
        kind: u32,
        batch: u32,
        start: u32,
        active: u32,
        words: u32,
        stride: u32,
    ) -> AttentionGpuParams {
        QTensorEngine::attention_gpu_params(
            &(*h).into(),
            layout,
            layer,
            token,
            info,
            raw,
            kind,
            batch,
            start,
            active,
            words,
            stride,
        )
    }
    fn encode_gemm_bufs_offset(
        &self,
        pipeline: &mut Self::Pipeline,
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
        input: &etalon::Buffer,
        in_off: etalon::BufferAddress,
        in_bytes: etalon::BufferAddress,
        output: &etalon::Buffer,
        out_off: etalon::BufferAddress,
        out_bytes: etalon::BufferAddress,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
        gemm_dyn_offset: u32,
        layer: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        QTensorEngine::encode_gemm_bufs_offset(
            self,
            pipeline,
            info,
            raw,
            n_in,
            n_out,
            input,
            in_off,
            in_bytes,
            output,
            out_off,
            out_bytes,
            n_batch,
            in_row_stride,
            out_row_stride,
            gemm_dyn_offset,
            layer,
            weight_role,
        )
    }
    fn encode_attention_batched_q_prefill(
        &self,
        pipeline: &mut Self::Pipeline,
        input: &etalon::Buffer,
        in_off: etalon::BufferAddress,
        in_bytes: etalon::BufferAddress,
        output: &etalon::Buffer,
        out_off: etalon::BufferAddress,
        out_bytes: etalon::BufferAddress,
        n_embd: usize,
        n_tokens: u32,
        layout: &Self::Layout,
        layer: u32,
        h: &ModelHyperparams,
        raw_weights: &[u8],
        attn_dyn_offset: u32,
    ) -> bool {
        QTensorEngine::encode_attention_batched_q_prefill(
            self,
            pipeline,
            input,
            in_off,
            in_bytes,
            output,
            out_off,
            out_bytes,
            n_embd,
            n_tokens,
            layout,
            layer,
            &(*h).into(),
            raw_weights,
            attn_dyn_offset,
        )
    }
    fn encode_elem_offset(
        &self,
        pipeline: &mut Self::Pipeline,
        op: u32,
        n: u32,
        batch: u32,
        a: &etalon::Buffer,
        a_off: etalon::BufferAddress,
        a_bytes: etalon::BufferAddress,
        b: &etalon::Buffer,
        b_off: etalon::BufferAddress,
        b_bytes: etalon::BufferAddress,
        out: &etalon::Buffer,
        out_off: etalon::BufferAddress,
        out_bytes: etalon::BufferAddress,
        a_row_stride: u32,
        b_row_stride: u32,
        out_row_stride: u32,
        a_slot: u32,
        b_slot: u32,
        out_slot: u32,
        elem_dyn_offset: u32,
    ) {
        QTensorEngine::encode_elem_offset(
            self,
            pipeline,
            op,
            n,
            batch,
            a,
            a_off,
            a_bytes,
            b,
            b_off,
            b_bytes,
            out,
            out_off,
            out_bytes,
            a_row_stride,
            b_row_stride,
            out_row_stride,
            a_slot,
            b_slot,
            out_slot,
            elem_dyn_offset,
        )
    }
    fn encode_fused_ffn_expansion(
        &self,
        pipeline: &mut Self::Pipeline,
        layer: u32,
        input: &etalon::Buffer,
        in_off: etalon::BufferAddress,
        in_bytes: etalon::BufferAddress,
        output: &etalon::Buffer,
        out_off: etalon::BufferAddress,
        out_bytes: etalon::BufferAddress,
        n_out: u32,
        n_batch: u32,
        gemm_dyn_offset: u32,
    ) -> bool {
        QTensorEngine::encode_fused_ffn_expansion(
            self,
            pipeline,
            layer,
            input,
            in_off,
            in_bytes,
            output,
            out_off,
            out_bytes,
            n_out,
            n_batch,
            gemm_dyn_offset,
        )
    }
    fn mc8_norm_source(
        &self,
        mmap: &[u8],
        tensor_data_start: u64,
        info: &GgufTensorInfo,
        n_embd: usize,
        layer: u32,
        is_ffn: bool,
    ) -> Option<(&etalon::Buffer, etalon::BufferAddress)> {
        QTensorEngine::mc8_norm_source(self, mmap, tensor_data_start, info, n_embd, layer, is_ffn)
    }
    fn encode_attention_pass_gpu(
        &self,
        pipeline: &mut Self::Pipeline,
        input: &etalon::Buffer,
        output: &etalon::Buffer,
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        h: &ModelHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        attn_dyn_offset: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        QTensorEngine::encode_attention_pass_gpu(
            self,
            pipeline,
            input,
            output,
            n_embd,
            num_tokens_in_batch,
            batch_start_token_idx,
            layout,
            layer,
            token_idx,
            &(*h).into(),
            info,
            raw_weights,
            proj_kind,
            n_workgroups,
            attn_dyn_offset,
            weight_role,
        )
    }
    fn mc8_flush(&self, pipeline: &mut Self::Pipeline) {
        QTensorEngine::mc8_flush(self, pipeline)
    }
    fn encode_attn_ffn_tail_gpu(
        &self,
        pipeline: &mut Self::Pipeline,
        index: &Self::Index,
        layer: u32,
        token_idx: u32,
        emb_dim: usize,
        tensors: &LayerTensors,
        token_hidden: &etalon::Buffer,
        attn_input: Option<&etalon::Buffer>,
        work_aliases_hidden: bool,
    ) -> bool {
        QTensorEngine::encode_attn_ffn_tail_gpu(
            self,
            pipeline,
            index,
            layer,
            token_idx,
            emb_dim,
            tensors,
            token_hidden,
            attn_input,
            work_aliases_hidden,
        )
    }
}

const _: [(); KV_ATTENTION_MASK_WORDS] =
    [(); etalon_inference::browser_prefill::types::KV_ATTENTION_MASK_WORDS];
