use super::*;
use etalon_inference::native_resident::{ResidentProvider, ResidentResources};
impl ResidentProvider for QTensorEngine {
    type Mask = [u32; KV_ATTENTION_MASK_WORDS];
    fn resident_resources(&self) -> ResidentResources<'_> {
        ResidentResources {
            add_pipeline: self.elem_gpu_pipeline(ELEM_OP_ADD_RESIDUAL),
            attention_mask_buf: self.attention_mask_buf.as_ref(),
            attention_pipeline: &self.attention_pipeline,
            coop_gemv_mr_pipeline: &self.coop_gemv_mr_pipeline,
            coop_gemv_pipeline: &self.coop_gemv_pipeline,
            coop_gemv_residual_mr_pipeline: &self.coop_gemv_residual_mr_pipeline,
            coop_gemv_residual_pipeline: &self.coop_gemv_residual_pipeline,
            coop_gemv_residual_warp_pipeline: &self.coop_gemv_residual_warp_pipeline,
            coop_gemv_warp_pipeline: &self.coop_gemv_warp_pipeline,
            dual_gemv_mr_pipeline: &self.dual_gemv_mr_pipeline,
            dual_gemv_pipeline: &self.dual_gemv_pipeline,
            elem_silu_mul_pipeline: &self.elem_silu_mul_pipeline,
            ffn_fused_coop_pipeline: &self.ffn_fused_coop_pipeline,
            ffn_fused_mr_pipeline: &self.ffn_fused_mr_pipeline,
            ffn_fused_pipeline: &self.ffn_fused_pipeline,
            ffn_fused_warp_pipeline: &self.ffn_fused_warp_pipeline,
            output_topk_pipeline: self.output_topk_pipeline.as_ref(),
            pipeline: &self.pipeline,
            rms_pipeline: self.elem_gpu_pipeline(ELEM_OP_RMS_NORM),
            topk_cand_idx_buf: self.topk_cand_idx_buf.as_ref(),
            topk_cand_val_buf: self.topk_cand_val_buf.as_ref(),
            triple_gemv_pipeline: &self.triple_gemv_pipeline,
        }
    }
    fn gpu_device(&self) -> &wgpu::Device {
        QTensorEngine::gpu_device(self)
    }
    fn gpu_queue(&self) -> &wgpu::Queue {
        QTensorEngine::gpu_queue(self)
    }
    fn submit(&self, command: wgpu::CommandBuffer) -> bool {
        self.submit_commands([command]).is_some()
    }
    fn poll_wait(&self) -> bool {
        let result = self
            .gpu_device()
            .poll(wgpu::PollType::wait_indefinitely())
            .is_ok();
        GPU_WAIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        result
    }
    fn attention_kv_mask(&self, token: u32, capacity: u32) -> (Self::Mask, u32) {
        crate::compute_universe::attention_kv_mask_u32(token, capacity)
    }
    fn dual_multirow(&self) -> bool {
        matches!(
            super::execution_env::var("QUALIA_LLM_DUAL_MR")
                .ok()
                .as_deref(),
            Some("1") | Some("true")
        )
    }
    fn coop_gemv_workgroups(&self, rows: u32) -> u32 {
        crate::llm_bench::coop_gemv_workgroups(rows)
    }
    fn pass_writes(&self) -> Option<wgpu::ComputePassTimestampWrites<'static>> {
        crate::llm_gpu_profiler::pass_writes_both()
    }
    fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        crate::llm_gpu_profiler::resolve(encoder);
    }
    fn accumulate_fused(&self) {
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);
    }
}
