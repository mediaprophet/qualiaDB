use super::*;
use etalon_inference::browser_projection::{BrowserProjectionProvider, BrowserProjectionResources};
impl BrowserProjectionProvider for QTensorEngine {
    type Top1Plan = browser::webgpu::BrowserTop1Plan;
    fn top1_plan(vocab_size: usize, chunks: u32) -> Option<Self::Top1Plan> {
        Self::Top1Plan::new(vocab_size, chunks)
    }
    fn prepare_browser_top1(&self, plan: Self::Top1Plan) -> bool {
        QTensorEngine::prepare_browser_top1(self, plan)
    }
    fn encode_browser_top1_chunk(
        &self,
        encoder: &mut etalon::CommandEncoder,
        logits: &etalon::Buffer,
        staging: &etalon::Buffer,
        plan: Self::Top1Plan,
        chunk: usize,
    ) -> bool {
        QTensorEngine::encode_browser_top1_chunk(self, encoder, logits, staging, plan, chunk)
    }
    async fn read_browser_top1(
        &self,
        staging: &etalon::Buffer,
        plan: Self::Top1Plan,
    ) -> Option<StreamingArgmaxResult> {
        QTensorEngine::read_browser_top1(self, staging, plan).await
    }
    fn tensor_data_offset(&self) -> u64 {
        self.tensor_data_offset
    }
    fn accumulate_fused_block(&self) {
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock)
    }
    fn record_fused_ops(&self, ops: usize) {
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(ops, std::sync::atomic::Ordering::Relaxed);
    }

    fn resources(&self) -> BrowserProjectionResources<'_> {
        BrowserProjectionResources {
            gemm_input_buf: self.gemm_input_buf.as_ref(),
            gemm_weight_buf: self.gemm_weight_buf.as_ref(),
            gemm_output_buf: self.gemm_output_buf.as_ref(),
            gemm_params_buf: self.gemm_params_buf.as_ref(),
            gemm_output_staging: self.gemm_output_staging.as_ref(),
            mc8_logits_resident_buf: self.mc8_logits_resident_buf.as_ref(),
            pipeline: &self.pipeline,
            mmv_q8_0_pipeline: &self.mmv_q8_0_pipeline,
            gemm_max_out_dim: self.gemm_max_out_dim,
            max_tensor_bytes: self.max_tensor_bytes,
            mc8_logits_row_bytes: self.mc8_logits_row_bytes,
        }
    }
    fn gpu_device(&self) -> &etalon::Device {
        QTensorEngine::gpu_device(self)
    }
    fn gpu_queue(&self) -> &etalon::Queue {
        QTensorEngine::gpu_queue(self)
    }
    fn write_weight_words(&self, raw: &[u8], max: usize) {
        QTensorEngine::write_weight_words(self, raw, max)
    }
    fn submit(&self, command: etalon::CommandBuffer) -> bool {
        // WebGPU submits successfully without a native SubmissionIndex; await_map validates completion.
        self.submit_commands([command]);
        true
    }
    fn timestamps(&self) -> Option<etalon::ComputePassTimestampWrites<'static>> {
        crate::llm_gpu_profiler::pass_writes_both()
    }
    fn resolve(&self, encoder: &mut etalon::CommandEncoder) {
        crate::llm_gpu_profiler::resolve(encoder)
    }
    fn accumulate_gemm(&self) {
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm)
    }
    fn buffers_ready(&self) -> bool {
        self.mc8_buffers_ready()
    }
    fn log(&self, message: &str) {
        wlog(message)
    }
    async fn await_map(&self, slice: etalon::BufferSlice<'_>) -> bool {
        await_wgpu_map(slice).await
    }
}
