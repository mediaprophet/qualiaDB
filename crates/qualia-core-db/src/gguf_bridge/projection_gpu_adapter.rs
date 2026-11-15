#![allow(unused_variables, unused_mut)]
use super::*;
use etalon_inference::projections::{
    Phase, Profile, ProjectionGpuProvider, ProjectionResources, ProjectionTensor,
};
impl QTensorEngine {
    /// Fallible native projection route. Governed callers may configure the admitted budget with
    /// set_projection_cold_byte_budget; ordinary preparation inherits the native host/device policy.
    pub fn try_native_projection_block(
        &self,
        tensor: &ProjectionTensor<'_>,
        input: &[f32],
    ) -> Result<Option<Vec<f32>>, &'static str> {
        // Admission exhaustion must not reinterpret legacy native Q4 bytes through a
        // different fallback layout. The existing Vec API has no recoverable error channel.
        if self.model_bytes().is_some() {
            let rows = tensor.shape.first().copied().unwrap_or(4096);
            let cols = tensor.shape.get(1).copied().unwrap_or(4096);
            let payload = rows
                .checked_mul(cols)
                .and_then(|n| n.checked_mul(2))
                .and_then(|n| n.checked_add(input.len()))
                .and_then(|n| {
                    rows.checked_mul(input.len().checked_div(cols)?)?
                        .checked_mul(3)?
                        .checked_add(n)
                })
                .and_then(|n| n.checked_mul(4));
            if !payload.is_some_and(|n| n <= self.projection_cold_byte_budget) {
                return Err("native projection payload exceeds caller budget or shape overflowed");
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
        let result: Option<Vec<f32>> = None;
        #[cfg(target_os = "windows")]
        let result = self.dml.as_ref().and_then(|device| {
            etalon_inference::projections::backend_q4::directml(
                device,
                self.model_bytes()?,
                self.tensor_data_offset,
                tensor,
                input,
                self.projection_cold_byte_budget,
            )
        });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let result = self.model_bytes().and_then(|model| {
            etalon_inference::projections::backend_q4::accelerate(
                model,
                self.tensor_data_offset,
                tensor,
                input,
                self.projection_cold_byte_budget,
            )
        });
        if result.is_some() {
            let rows = tensor.shape.first().copied().unwrap_or(4096);
            let cols = tensor.shape.get(1).copied().unwrap_or(4096);
            self.record_operations(rows.saturating_mul(cols));
        }
        Ok(result)
    }
}
impl ProjectionGpuProvider for QTensorEngine {
    fn tensor_data_offset(&self) -> u64 {
        self.tensor_data_offset
    }
    fn backend_fused_block(
        &self,
        tensor: &ProjectionTensor<'_>,
        input: &[f32],
    ) -> Option<Vec<f32>> {
        self.try_native_projection_block(tensor, input)
            .unwrap_or_else(|error| panic!("{error}"))
    }
    fn record_operations(&self, count: usize) {
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
    }
    fn readback_owned(
        &self,
        buffer: &etalon::Buffer,
        bytes: u64,
        panic_on_error: bool,
    ) -> Option<Vec<f32>> {
        let result = etalon_inference::projections::readback::readback_owned(
            &self.projection_readback,
            buffer,
            bytes,
            self.projection_cold_byte_budget,
            || self.poll_wait(),
        );
        if panic_on_error && result.is_none() {
            panic!("fused block readback failed or exceeded readback budget");
        }
        result
    }
    fn projection_resources(&self) -> ProjectionResources<'_> {
        ProjectionResources {
            gemm_input_buf: &self.gemm_input_buf,
            gemm_weight_buf: &self.gemm_weight_buf,
            gemm_output_buf: &self.gemm_output_buf,
            gemm_params_buf: &self.gemm_params_buf,
            gemm_output_staging: &self.gemm_output_staging,
            topk_params_buf: &self.topk_params_buf,
            topk_cand_val_buf: &self.topk_cand_val_buf,
            topk_cand_idx_buf: &self.topk_cand_idx_buf,
            topk_cand_staging: &self.topk_cand_staging,
            mc8_logits_resident_buf: &self.mc8_logits_resident_buf,
            coop_gemv_pipeline: &self.coop_gemv_pipeline,
            coop_gemv_mr_pipeline: &self.coop_gemv_mr_pipeline,
            pipeline: &self.pipeline,
            embedding_pipeline: &self.embedding_pipeline,
            mock_pipeline: &self.mock_pipeline,
            embedding_bind_layout: &self.embedding_bind_layout,
            pipeline_bind_layout: &self.pipeline_bind_layout,
            output_topk_pipeline: &self.output_topk_pipeline,
            output_topk_bind_layout: &self.output_topk_bind_layout,
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
    fn coop_gemv_enabled(&self) -> bool {
        crate::llm_bench::coop_gemv_enabled()
    }
    fn coop_gemv_workgroups(&self, rows: u32) -> u32 {
        crate::llm_bench::coop_gemv_workgroups(rows)
    }
    fn resident_weights_enabled(&self) -> bool {
        crate::llm_bench::resident_weights_enabled()
    }
    fn native_gemm_bind_layout(&self, coop: bool) -> &etalon::BindGroupLayout {
        QTensorEngine::native_gemm_bind_layout(self, coop)
    }
    fn resident_weight_buffer(&self, key: u64, raw: &[u8]) -> Option<etalon::Buffer> {
        QTensorEngine::resident_weight_buffer(self, key, raw)
    }
    fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        QTensorEngine::write_weight_words(self, raw, max_bytes)
    }
    fn submit(&self, commands: etalon::CommandBuffer) {
        self.submit_commands(Some(commands));
    }
    fn map_read(&self, buffer: &etalon::Buffer, bytes: u64) -> bool {
        etalon_inference::projections::readback::map_read(
            &self.projection_readback,
            buffer,
            bytes,
            || self.poll_wait(),
        )
    }
    fn readback(&self, buffer: &etalon::Buffer, bytes: u64, out: &mut [f32]) -> bool {
        etalon_inference::projections::readback::readback_into(
            &self.projection_readback,
            buffer,
            bytes,
            out,
            || self.poll_wait(),
        )
    }
    fn pass_writes(&self, profile: Profile) -> Option<etalon::ComputePassTimestampWrites<'static>> {
        match profile {
            Profile::Both => crate::llm_gpu_profiler::pass_writes_both(),
            Profile::Begin => crate::llm_gpu_profiler::pass_writes_begin(),
            Profile::End => crate::llm_gpu_profiler::pass_writes_end(),
        }
    }
    fn resolve(&self, encoder: &mut etalon::CommandEncoder) {
        crate::llm_gpu_profiler::resolve(encoder)
    }
    fn accumulate(&self, phase: Phase) {
        crate::llm_gpu_profiler::accumulate(match phase {
            Phase::Gemm => crate::llm_gpu_profiler::Phase::Gemm,
            Phase::OutputTopk => crate::llm_gpu_profiler::Phase::OutputTopk,
            Phase::Embedding => crate::llm_gpu_profiler::Phase::Embedding,
            Phase::FusedBlock => crate::llm_gpu_profiler::Phase::FusedBlock,
        })
    }
    fn cpu_gemm(
        &self,
        raw: &[u8],
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }
    fn cuda_gemm(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        // Mode=cuda acceleration (fail closed → wgpu path below).
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            if crate::prefer_tensor_core_gemm() {
                // M2b: on-device Q4_K SoA dequant-GEMV (no host densify) — preferred for .soa.p64.
                // This path has CPU differential tests; keep it on for mode=cuda.
                if info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                    && crate::try_q4k_soa_gemv(n_in, n_out, &input[..n_in], raw, &mut out[..n_out])
                {
                    return true;
                }
                // Dense densify + CUDA GEMM for F16/Q8/etc. was measured **incoherent** on
                // single-token decode (pad-to-16 + f16-WMMA reduced path → garbage tokens,
                // 2026-07-24 SmolLM f16 package). Default OFF for decode GEMV (always batch=1
                // here). Opt-in: QUALIA_LLM_CUDA_TC_DECODE=1 (lab only). Prefill should use
                // multi-token dispatch or the resident mega-pass, not this broken default.
                let densify_tc_decode = matches!(
                    crate::gguf_bridge::execution_env::var("QUALIA_LLM_CUDA_TC_DECODE")
                        .ok()
                        .as_deref(),
                    Some("1") | Some("true")
                );
                if densify_tc_decode
                    && n_in >= 16
                    && n_out >= 16
                    && n_in.saturating_mul(n_out) <= crate::inference::cuda_lane::MAX_DENSE_ELEMS
                {
                    let key = crate::weight_fingerprint(raw, n_in, n_out);
                    if crate::try_cuda_batch_gemv_cached_only(
                        key,
                        &input[..n_in],
                        1,
                        &mut out[..n_out],
                    ) {
                        return true;
                    }
                    if let Some(dense) = Self::dequant_weight_dense_f32(info, raw, n_in, n_out) {
                        if crate::try_cuda_batch_gemv_cached(
                            key,
                            &input[..n_in],
                            1,
                            n_in,
                            n_out,
                            &dense,
                            &mut out[..n_out],
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}
