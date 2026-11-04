//! Decode-tail dispatch: final logits projection + async GEMM / async attention paths.
//! Split from gguf_bridge/forward.rs (structural; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Final logits via chunked projection into `logits_out` (fills min(vocab, buf) rows).
    pub fn dispatch_output_logits_into(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        logits_out: &mut [f32],
    ) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref pkg) = self.ftw_package {
            if let (Some(packed), Some(scales), Some(globals)) = (
                pkg.fetch_tensor_bytes("lm_head.weight"),
                pkg.fetch_tensor_bytes("lm_head.weight_scale"),
                pkg.fetch_tensor_bytes("lm_head.weight_global"),
            ) {
                let vocab_size = globals.len() / 2;
                let fill = vocab_size.min(logits_out.len());
                let row_bytes = emb_dim / 2;
                let scale_bytes = emb_dim / 16;
                if emb_dim == 0 || hidden.len() < emb_dim || fill == 0 {
                    return 0;
                }
                if packed.len() < vocab_size * row_bytes || scales.len() < vocab_size * scale_bytes
                {
                    return 0;
                }
                for token_id in 0..fill {
                    let g_val = half::f16::from_le_bytes([
                        globals[token_id * 2],
                        globals[token_id * 2 + 1],
                    ])
                    .to_f32();
                    let p_row = &packed[token_id * row_bytes..(token_id + 1) * row_bytes];
                    let s_row = &scales[token_id * scale_bytes..(token_id + 1) * scale_bytes];
                    logits_out[token_id] = etalon_inference::ordinary::output::nvfp4_output_row(
                        p_row, s_row, g_val, hidden, emb_dim,
                    );
                }
                return fill;
            }
        }

        let Some(info) = index.logits_projection_info() else {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            return n;
        };
        let (n_in, vocab_size) = Self::matmul_dims(info);
        let fill = vocab_size.min(logits_out.len());
        if n_in > emb_dim || fill == 0 {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            return n;
        }
        let mmap = match self.model_bytes() {
            Some(m) => m,
            None => {
                let n = emb_dim.min(logits_out.len());
                logits_out[..n].copy_from_slice(&hidden[..n]);
                return n;
            }
        };
        let mut written = 0usize;
        let n_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        for chunk_idx in 0..n_chunks {
            if written >= fill {
                break;
            }
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
            let raw = match crate::ggml_quants::fetch_tensor_row_range_bytes(
                mmap,
                index.tensor_data_start,
                info,
                row_start,
                chunk_rows,
            ) {
                Ok(s) => s,
                Err(_) => break,
            };
            let out_rows = chunk_rows.min(fill - written);
            if !self.dispatch_gemm_raw_into(
                info,
                raw,
                &hidden[..n_in],
                &mut logits_out[written..written + out_rows],
                n_in,
                out_rows,
            ) {
                break;
            }
            written += out_rows;
        }
        if written > 0 {
            written
        } else {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            n
        }
    }

    /// Decodes `lexicon_bound`; fails closed on malformed input.
    pub fn decode_lexicon_bound(&self, _logits: &[f32], valid_lexicon_ids: &[u64]) -> u64 {
        if valid_lexicon_ids.is_empty() {
            0
        } else {
            valid_lexicon_ids[0]
        }
    }

    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: gemm-raw-into.
    pub(crate) async fn dispatch_gemm_raw_into_async(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_gemm_raw_into_async(info, raw, input, out, n_in, n_out)
            .await
    }

    /// Phase 5.3: quantized GEMM against a **resident** weight sub-range (no per-call weight upload).
    /// Same dispatch+readback as `dispatch_gemm_raw_into_async`, but binds the caller-provided
    /// `weight` BindingResource (a slice of the resident output-projection buffer) instead of
    /// `write_buffer`-ing the chunk. Used by the decode argmax once the logits projection is resident.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn dispatch_gemm_resident_chunk_async(
        &self,
        weight_ggml_type: u32,
        weight_row_elems: u32,
        weight: wgpu::BindingResource<'_>,
        weight_byte_len: u32,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_gemm_resident_chunk_async(
                weight_ggml_type,
                weight_row_elems,
                weight,
                weight_byte_len,
                input,
                out,
                n_in,
                n_out,
            )
            .await
    }
    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: gemm-into.
    pub async fn dispatch_gemm_into_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        etalon_inference::browser_projection::BrowserProjectionEngine { host: self }
            .dispatch_gemm_into_async(index, info, input, out, n_in, n_out)
            .await
    }
    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: attention-layer.
    pub(crate) async fn dispatch_attention_layer_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &[f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> Option<usize> {
        etalon_inference::ordinary::async_attention::dispatch_attention_layer_async(
            self, index, layer, token_idx, hidden, emb_dim, tensors, scratch_a, scratch_b,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    /// Async dispatch: attention-pass.
    pub(crate) async fn dispatch_attention_pass_async(
        &self,
        hidden: &[f32],
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        readback_out: Option<&mut [f32]>,
    ) -> bool {
        if !ggml_gpu_quant_supported(info.ggml_type) {
            wlog(&format!(
                "[attn_pass_async] GUARD unsupported quant kind={proj_kind}"
            ));
            return false;
        }
        if !ggml_gpu_attention_shader_supported(info.ggml_type) {
            return self.cpu_attention_pass(
                hidden,
                n_embd,
                num_tokens_in_batch,
                batch_start_token_idx,
                layout,
                layer,
                h,
                info,
                raw_weights,
                proj_kind,
                None,
                readback_out,
            );
        }
        let batch = num_tokens_in_batch.max(1) as usize;
        let hidden_elems = n_embd.checked_mul(batch).unwrap_or(0);
        if hidden_elems > hidden.len()
            || hidden_elems > self.gemm_max_input_floats
            || raw_weights.len() > self.max_tensor_bytes
            || self.gemm_input_buf.is_none()
            || self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
            || self.attention_mask_buf.is_none()
        {
            wlog(&format!(
                "[attn_pass_async] GUARD buffers kind={proj_kind} hidden_elems={hidden_elems} hidden={} gemm_in={} raw_w={} max_w={}",
                hidden.len(),
                self.gemm_max_input_floats,
                raw_weights.len(),
                self.max_tensor_bytes,
            ));
            return false;
        }

        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
        let params = Self::attention_gpu_params(
            h,
            layout,
            layer,
            token_idx,
            info,
            raw_weights.len(),
            proj_kind,
            num_tokens_in_batch.max(1),
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            0,
        );
        let input_buf = self.gemm_input_buf.as_ref().unwrap();
        let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
        let output_buf = self.gemm_output_buf.as_ref().unwrap();
        let params_buf = self.attention_params_buf.as_ref().unwrap();
        let mask_buf = self.attention_mask_buf.as_ref().unwrap();
        let kv_buf = self.kv_cache_gpu.as_ref().unwrap();
        let staging = self.gemm_output_staging.as_ref().unwrap();

        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..hidden_elems]));
        self.write_weight_words(raw_weights, self.max_tensor_bytes);
        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));
        self.gpu_queue()
            .write_buffer(mask_buf, 0, bytemuck::cast_slice(&mask_words));

        let resources = etalon_inference::gpu_attention::PassResources {
            input: input_buf,
            weight: weight_buf,
            output: output_buf,
            params: params_buf,
            mask: mask_buf,
            kv: kv_buf,
            staging,
            bind_layout: &self.attention_pipeline.get_bind_group_layout(0),
            pipeline: &self.attention_pipeline,
        };
        let readback_elems = readback_out.as_ref().map(|o| o.len()).unwrap_or(0);
        let mut encoder = match etalon_inference::gpu_attention::encode_pass(
            self.gpu_device(),
            self.gpu_queue(),
            &resources,
            &params,
            &mask_words,
            n_workgroups,
            readback_elems,
            crate::llm_gpu_profiler::pass_writes_both(),
        ) {
            Some(encoder) => encoder,
            None => return false,
        };
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.submit_commands(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention);

        if readback_elems == 0 {
            return true;
        }

        let out_bytes = (readback_elems * 4) as wgpu::BufferAddress;
        let slice = staging.slice(..out_bytes);
        if await_wgpu_map(slice).await {
            let data = slice
                .get_mapped_range()
                .expect("wgpu buffer map_range failed");
            let floats: &[f32] = bytemuck::cast_slice(&data);
            if let Some(out) = readback_out {
                out[..readback_elems].copy_from_slice(&floats[..readback_elems]);
            }
            drop(data);
            staging.unmap();
            return true;
        }
        let _ = staging.unmap();
        false
    }
    #[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
    /// Async dispatch: attention-q-ffn-token.
    pub(crate) async fn dispatch_attention_q_ffn_token_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        etalon_inference::ordinary::async_attention_token::dispatch_attention_q_ffn_token_async(
            self, index, layer, token_idx, hidden, emb_dim, tensors, scratch_a, scratch_b,
        )
        .await
    }
}
