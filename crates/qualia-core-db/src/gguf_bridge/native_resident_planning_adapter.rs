use super::*;
use etalon_inference::native_resident::planning::{
    PlanEvent, ResidentPlanOptions, ResidentPlanProvider, ResidentPlanResources,
};
use etalon_model::inference::{LayerTensors, ModelHyperparams};

pub struct PlanningModelLease(model_bytes::ModelByteHandle);
impl AsRef<[u8]> for PlanningModelLease {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

fn enabled(name: &str) -> bool {
    matches!(
        super::execution_env::var(name).ok().as_deref(),
        Some("1") | Some("true")
    )
}

impl ResidentPlanProvider for QTensorEngine {
    type Index = crate::gguf_sharder::GgufTensorIndex;
    type ModelLease = PlanningModelLease;
    type Layout = KvCacheLayout;
    fn model_lease(&self) -> Option<Self::ModelLease> {
        self.clone_model_bytes().map(PlanningModelLease)
    }
    fn hyperparams(index: &Self::Index) -> ModelHyperparams {
        index.hyperparams.into()
    }
    fn data_start(index: &Self::Index) -> u64 {
        index.tensor_data_start
    }
    fn tensors(index: &Self::Index, layer: u32) -> LayerTensors {
        index.get_layer_tensors(layer)
    }
    fn logits_info(index: &Self::Index) -> Option<GgufTensorInfo> {
        index.logits_projection_info().copied()
    }
    fn output_norm_info(index: &Self::Index) -> Option<GgufTensorInfo> {
        index.output_norm_info().copied()
    }
    fn tensor_bytes<'a>(bytes: &'a [u8], start: u64, info: &GgufTensorInfo) -> Option<&'a [u8]> {
        crate::ggml_quants::fetch_tensor_bytes(bytes, start, info).ok()
    }
    fn norm_row(bytes: &[u8], start: u64, info: &GgufTensorInfo, out: &mut [f32]) -> usize {
        dequant_norm_row_into(bytes, start, info, out)
    }
    fn plan_layout(&self) -> Option<Self::Layout> {
        self.kv_layout
    }
    fn max_context(layout: &Self::Layout) -> u32 {
        layout.max_context
    }
    fn layer_stride(layout: &Self::Layout) -> u32 {
        layout.layer_stride
    }
    fn plan_options(&self) -> ResidentPlanOptions {
        ResidentPlanOptions {
            coop_gemv: crate::llm_bench::coop_gemv_enabled(),
            ffn_fusion: crate::llm_bench::ffn_fusion_enabled(),
            triple_qkv: enabled("QUALIA_LLM_TRIPLE_QKV"),
            multirow: enabled("QUALIA_LLM_MULTIROW"),
            warp_gemv: enabled("QUALIA_LLM_WARP_GEMV"),
            ffn_warp: enabled("QUALIA_LLM_FFN_WARP"),
            ffn_mr: enabled("QUALIA_LLM_FFN_MR"),
            topk_block_size: crate::topk::TOPK_BLOCK_SIZE,
        }
    }
    fn plan_resources(&self) -> ResidentPlanResources<'_> {
        ResidentPlanResources {
            output_topk_pipeline: self.output_topk_pipeline.as_ref(),
            rms_pipeline: self.elem_gpu_pipeline(ELEM_OP_RMS_NORM),
            add_pipeline: self.elem_gpu_pipeline(ELEM_OP_ADD_RESIDUAL),
            attention_mask_buf: self.attention_mask_buf.as_ref(),
            kv_cache_gpu: self.kv_cache_gpu.as_ref(),
            mc8_logits_resident_buf: self.mc8_logits_resident_buf.as_ref(),
            mc8_logits_row_bytes: self.mc8_logits_row_bytes,
            topk_cand_val_buf: self.topk_cand_val_buf.as_ref(),
            topk_cand_idx_buf: self.topk_cand_idx_buf.as_ref(),
            attention_bind_layout: &self.attention_bind_layout,
            elem_silu_mul_bind_layout: &self.elem_silu_mul_bind_layout,
            output_topk_bind_layout: self.output_topk_bind_layout.as_ref(),
            coop_gemv_residual_bind_layout: &self.coop_gemv_residual_bind_layout,
            triple_gemv_bind_layout: &self.triple_gemv_bind_layout,
            dual_gemv_bind_layout: &self.dual_gemv_bind_layout,
            ffn_fused_bind_layout: &self.ffn_fused_bind_layout,
            gemm_max_out_dim: self.gemm_max_out_dim,
        }
    }
    fn gpu_device(&self) -> &wgpu::Device {
        QTensorEngine::gpu_device(self)
    }
    fn gpu_queue(&self) -> &wgpu::Queue {
        QTensorEngine::gpu_queue(self)
    }
    fn native_gemm_bind_layout(&self, coop: bool) -> &wgpu::BindGroupLayout {
        QTensorEngine::native_gemm_bind_layout(self, coop)
    }
    fn resident_weight_buffer(&self, key: u64, raw: &[u8]) -> Option<wgpu::Buffer> {
        QTensorEngine::resident_weight_buffer(self, key, raw)
    }
    fn promote_matrix_to_f16_resident(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
    ) -> Option<(wgpu::Buffer, u32, u32, u32)> {
        QTensorEngine::promote_matrix_to_f16_resident(self, info, raw)
    }
    fn prewarm_layer(&self, layer: u32, matrices: &[(&GgufTensorInfo, &[u8], usize, usize)]) {
        #[cfg(feature = "cuda")]
        prewarm_cuda_layer(self, layer, matrices);
        #[cfg(not(feature = "cuda"))]
        let _ = (layer, matrices);
    }
    fn set_ffn_fusion_in_resident(&self, fused: bool) {
        crate::llm_bench::set_ffn_fusion_in_resident(fused);
    }
    fn event(&self, event: PlanEvent) {
        match event {
            PlanEvent::FfnF16(g, u, d) => log::info!(
                "LLM_LOAD|ffn-f16|promoted gate/up/down (types g={} u={} d={})", g, u, d),
            PlanEvent::DecodeBuilt { layers, passes, fused, triple, dual, ffn_mr, multirow, ffn_warp } => {
                #[cfg(feature = "cuda")]
                log::info!(
                    "LLM_DECODE|resident-plan|built: {} layers, {} passes/token, fused_ffn={} triple_qkv={} dual_kv={} ffn_mr={} multirow={} cuda_soa_weights={} cuda_dense_cache={}",
                    layers, passes, fused, triple, dual, ffn_mr, multirow,
                    crate::q4k_device_weight_count(), crate::weight_cache_len(),
                );
                #[cfg(not(feature = "cuda"))]
                log::info!(
                    "LLM_DECODE|resident-plan|built: {} layers, {} passes/token, fused_ffn={} ffn_mr={} ffn_warp={}",
                    layers, passes, fused, ffn_mr, ffn_warp,
                );
                let _ = (triple, dual, multirow, ffn_warp);
            }
            PlanEvent::PrefillBuilt { layers, passes, fused, dual } => log::info!(
                "LLM_PREFILL|resident-arena|built: {} layers, {} dispatches/layer (1 compute pass/chunk), fused_ffn={} dual_kv={}",
                layers, passes, fused, dual,
            ),
            PlanEvent::VerifyBuilt { layers, chunks, batch } => log::info!(
                "LLM_VERIFY|batched-verify|built: {} layers + {} vocab chunks, B_max={}", layers, chunks, batch,
            ),
        }
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
}

#[cfg(feature = "cuda")]
fn prewarm_cuda_layer(
    engine: &QTensorEngine,
    layer: u32,
    matrices: &[(&GgufTensorInfo, &[u8], usize, usize)],
) {
    if !crate::prefer_tensor_core_gemm() || !enabled("QUALIA_LLM_CUDA_DECODE") {
        return;
    }
    use crate::ggml_quants::GGML_TYPE_Q4_K_SOA;
    if layer == 0 {
        if let Some(layout) = engine.kv_layout.as_ref() {
            if !layout.int8 && layout.dict_k == 0 {
                let _ = crate::ensure_device_kv_cache(
                    layout.max_context,
                    layout.n_layer,
                    layout.n_kv_head,
                    layout.head_dim,
                    layout.slot_kv_elems,
                    layout.layer_stride,
                    layout.total_f32_elems,
                );
            }
        }
    }
    let mut pack: Vec<(&[u8], usize, usize)> = Vec::with_capacity(7);
    for &(info, raw, n_in, n_out) in matrices {
        if info.ggml_type == GGML_TYPE_Q4_K_SOA {
            pack.push((raw, n_in, n_out));
        }
    }
    if !pack.is_empty() {
        let _ = crate::preload_q4k_soa_weights(&pack);
    }
    if enabled("QUALIA_LLM_CUDA_TC_PREWARM") {
        let mut warmed = 0u32;
        for &(info, raw, n_in, n_out) in matrices {
            if info.ggml_type == GGML_TYPE_Q4_K_SOA
                && QTensorEngine::prewarm_cuda_weight(info, raw, n_in, n_out)
            {
                warmed += 1;
            }
        }
        if layer == 0 && warmed > 0 {
            log::info!(
                "LLM_LOAD|cuda_tc_prewarm|layer0|dense_entries={}|cache_len={}",
                warmed,
                crate::weight_cache_len()
            );
        }
    }
}
