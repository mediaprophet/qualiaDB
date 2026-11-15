//! Async + MC8-fused prefill, fused-block async, async output-argmax, async engine init.
//! Split from gguf_bridge/forward.rs (structural; no behaviour change).
#![allow(dead_code, unused_variables, unused_mut)]
use super::*;

impl QTensorEngine {
    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: prefill-chunk.
    pub async fn dispatch_prefill_chunk_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
        l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        if self
            .dispatch_prefill_chunk_async_mc8_gpu(
                index,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                scratch_a,
                scratch_b,
                max_layers,
                l1_hidden_out,
            )
            .await
        {
            return true;
        }
        wlog("[MC8] GPU prefill FAILED — CPU fallback blocked (manifold unification)");
        false
    }

    /// Part 3u: stage K/V/Q + tail uniforms in three single `write_buffer` calls (pre-encoder).
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn mc8_stage_prefill_layer_super_arena(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_start_token_idx: u32,
        n_tokens: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        cursors: &mut Mc8ChunkUniformCursors,
    ) -> Option<(Mc8PrefillLayerUniforms, Mc8PrefillLayerGeom)> {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .mc8_stage_prefill_layer_super_arena(
                index,
                layer,
                tensors,
                batch_start_token_idx,
                n_tokens,
                emb_dim,
                used_attn_norm,
                k_info,
                k_raw,
                v_info,
                v_raw,
                cursors,
            )
    }

    /// MC8 Part 3m/3u: batched Q+FFN tail — uniforms pre-staged in super-arena.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_prefill_q_ffn_tail_fused(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_buf: &etalon::Buffer,
        attn_src: &etalon::Buffer,
        work_a: &etalon::Buffer,
        work_b: &etalon::Buffer,
        n_tokens: u32,
        batch_start_token_idx: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        uniforms: &Mc8PrefillLayerUniforms,
        geom: &Mc8PrefillLayerGeom,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_prefill_q_ffn_tail_fused(
                pipeline,
                index,
                layer,
                tensors,
                batch_buf,
                attn_src,
                work_a,
                work_b,
                n_tokens,
                batch_start_token_idx,
                emb_dim,
                used_attn_norm,
                uniforms,
                geom,
            )
    }

    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: prefill-chunk-mc8-gpu.
    pub(crate) async fn dispatch_prefill_chunk_async_mc8_gpu(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        _scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
        max_layers: u32,
        mut l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        if index.hyperparams.n_layer == 0 || n_tokens == 0 || !self.mc8_buffers_ready() {
            return false;
        }
        if !self.mc8_weights_resident {
            let _ = self.mc8_upload_all_resident_weights(index);
        }
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .dispatch_prefill_chunk_async_mc8_gpu(
                index,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                _scratch_a,
                _scratch_b,
                max_layers,
                l1_hidden_out,
            )
            .await
    }
    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: fused-transformer-block.
    pub async fn dispatch_fused_transformer_block_async(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        let rows = tensor.shape.get(0).copied().unwrap_or(4096);
        let cols = tensor.shape.get(1).copied().unwrap_or(4096);

        // ── DirectML path (Windows) ───────────────────────────────────────────
        #[cfg(target_os = "windows")]
        if let Some(dml) = &self.dml {
            if let Some(mmap) = self.model_bytes() {
                let offset = self.tensor_data_offset + tensor.byte_offset;
                let q4_bytes_needed = (rows * cols / crate::directml_bridge::Q4_K_BLOCK_SIZE)
                    * crate::directml_bridge::Q4_K_BLOCK_BYTES;
                if (offset as usize + q4_bytes_needed) <= mmap.len() {
                    let q4_slice = &mmap[offset as usize..offset as usize + q4_bytes_needed];
                    let weights_f32 =
                        crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT
                            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                        return result;
                    }
                }
            }
        }

        // ── Accelerate BLAS path (macOS / Apple Silicon AMX) ─────────────────────
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(mmap) = self.model_bytes() {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let q4_bytes_needed = (rows * cols / crate::metal_bridge::Q4_K_BLOCK_SIZE)
                * crate::metal_bridge::Q4_K_BLOCK_BYTES;
            if offset + q4_bytes_needed <= mmap.len() {
                let q4_slice = &mmap[offset..offset + q4_bytes_needed];
                let weights_f32 =
                    crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows,
                    cols,
                    rows,
                    input_activations,
                    &weights_f32,
                );
                crate::telemetry::SIEVE_OPS_COUNT
                    .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_fused_transformer_block_async(
                rows,
                cols,
                tensor.byte_offset,
                tensor.is_quantized_q4_k,
                input_activations,
            )
            .await
    }
    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: output-argmax-chunked.
    pub async fn dispatch_output_argmax_chunked_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        // Robust per-chunk submit + readback with exact CPU argmax
        self.dispatch_output_argmax_chunked_async_mc8_fused(
            index,
            hidden,
            emb_dim,
            chunk_logits,
            max_chunks,
            sieve_mask,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    /// Per-chunk `dispatch_gemm_raw_into_async` + streaming CPU argmax.
    ///
    /// There is no WGSL argmax reduction shader. The Endgame "fused" batched encoder
    /// (`encode_gemm_bufs` loop + single staging readback) produced garbled tokens because
    /// `queue.write_buffer` weight uploads race across chunks in one submit scope (pt3c
    /// analogue). Per-chunk submit/readback matches the proven sync path semantics.
    pub(crate) async fn dispatch_output_argmax_chunked_async_mc8_fused(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_output_argmax_chunked_async_mc8_fused(
                index,
                hidden,
                emb_dim,
                chunk_logits,
                max_chunks,
                sieve_mask,
            )
            .await
            .map(|result| StreamingArgmaxResult {
                best_token_id: result.best_token_id,
                max_logit: result.max_logit,
            })
    }

    /// Batched argmax: all vocab chunks in one encoder + one submit + one readback.
    /// Eliminates 4 of 5 async map yields per token vs the per-chunk path.
    #[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
    #[expect(
        dead_code,
        reason = "Staged browser argmax experiment; not connected to production decode"
    )]
    pub(crate) async fn dispatch_output_argmax_batched_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_output_argmax_batched_async(
                index,
                hidden,
                emb_dim,
                chunk_logits,
                max_chunks,
                sieve_mask,
            )
            .await
    }

    #[cfg(target_arch = "wasm32")]
    /// `QTensorEngine::new_async`.
    pub async fn new_async() -> Self {
        Self::try_new()
            .await
            .expect("Failed to initialize native GGUF engine")
    }
}
